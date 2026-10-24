// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

// Recovered from original muCommander CopyJob and its transfer/collision callers.
// See NOTICE.md and evidence/workflows/W03-recursive-copy/contract.md.
//! Reviewed local copy operations retaining explicit partial-failure outcomes.
use crate::{JobControl, jobs::copy_stream};
use serde::{Deserialize, Serialize};
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::{
    fs::{self, File, OpenOptions},
    io,
    path::{Component, Path, PathBuf},
    time::{Duration, UNIX_EPOCH},
};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
/// The reviewed collision decision for an existing destination file.
pub enum CopyPolicy {
    #[default]
    #[doc = "Preserve an existing destination file and report a skipped outcome."]
    Skip,
    #[doc = "Replace existing destination file contents under the reviewed operation contract."]
    Overwrite,
    #[doc = "Replace only when the source modification time is later."]
    OverwriteIfOlder,
    #[doc = "Replace only when source and destination byte lengths differ."]
    OverwriteIfSizeDiffers,
}

#[derive(Clone, Debug, Deserialize)]
/// An ordered direct-child transfer request validated against canonical roots before writing.
pub struct CopyRequest {
    /// Existing local directory containing the selected direct children.
    pub source_directory: PathBuf,
    /// Existing target directory, revalidated before writes.
    pub destination_directory: PathBuf,
    /// Ordered distinct direct-child source selection.
    pub names: Vec<String>,
    /// Proposed replacement direct-child name; validated before mutation.
    pub new_name: Option<String>,
    /// Reviewed handling of existing destination files.
    pub policy: CopyPolicy,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Success of one selected top-level item; detailed child failures remain in the report.
pub struct CopyOutcome {
    /// Direct entry name, independent of a display path or target resolution.
    pub name: String,
    /// Original processFile return value; a directory may contain failed children.
    pub success: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// A recoverable item outcome category used independently of its diagnostic text.
pub enum CopyIssueKind {
    #[doc = "The collision or overlap policy intentionally retained this item."]
    Skipped,
    #[doc = "The operation failed; inspect diagnostics and any retained partial results."]
    Failed,
    #[doc = "This object or operation is outside the supported safe subset."]
    Unsupported,
    #[doc = "The operation stopped in response to cancellation; completed writes remain."]
    Cancelled,
}
#[derive(Clone, Debug)]
/// One path-associated skip, failure, unsupported operation or cancellation diagnostic.
pub struct CopyIssue {
    /// Captured or reported filesystem/remote path; inspect the enclosing operation contract.
    pub path: PathBuf,
    /// Typed category independent of user-facing diagnostic wording.
    pub kind: CopyIssueKind,
    /// Human-readable diagnostic; callers use the accompanying kind for decisions.
    pub message: String,
}
#[derive(Clone, Debug, Default)]
/// Completed local operations and per-item issues; partial destinations can remain.
pub struct CopyReport {
    /// Whether cooperative cancellation stopped or prevented the remaining work.
    pub cancelled: bool,
    /// Ordered results for selected top-level items, including partial-operation outcomes.
    pub outcomes: Vec<CopyOutcome>,
    /// Detailed per-path failures, skips, unsupported cases or cancellation notices.
    pub issues: Vec<CopyIssue>,
    /// Number of regular files copied successfully.
    pub files_copied: usize,
    /// Number of enumerated source directories processed.
    pub directories_processed: usize,
    /// Number of supported symbolic links copied.
    pub links_copied: usize,
}

/// Reject empty, parent, root and separator-bearing direct-child names.
pub fn is_valid_leaf_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('\0')
        && !name.contains('/')
        && !name.contains(std::path::MAIN_SEPARATOR)
        && Path::new(name).components().count() == 1
        && matches!(
            Path::new(name).components().next(),
            Some(Component::Normal(_))
        )
}
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
fn normalize(path: &Path) -> PathBuf {
    let mut clean = PathBuf::new();
    for part in path.components() {
        match part {
            Component::ParentDir => {
                clean.pop();
            }
            Component::CurDir => {}
            _ => clean.push(part),
        }
    }
    clean
}
pub(crate) fn resolved(path: &Path) -> io::Result<PathBuf> {
    if let Ok(path) = fs::canonicalize(path) {
        return Ok(path);
    }
    let path = normalize(path);
    let parent = path
        .parent()
        .ok_or_else(|| invalid("Cannot resolve destination"))?;
    let name = path
        .file_name()
        .ok_or_else(|| invalid("Cannot resolve filename"))?;
    Ok(resolved(parent)?.join(name))
}
pub(crate) fn modified_ms(metadata: Option<&fs::Metadata>) -> i128 {
    metadata.and_then(|m| m.modified().ok()).map_or(0, |time| {
        match time.duration_since(UNIX_EPOCH) {
            Ok(value) => value.as_millis() as i128,
            Err(error) => -(error.duration().as_nanos().div_ceil(1_000_000) as i128),
        }
    })
}
pub(crate) fn apply_metadata(source: Option<&fs::Metadata>, destination: &Path, permissions: bool) {
    let millis = modified_ms(source).max(0);
    if let Ok(millis) = u64::try_from(millis)
        && let Some(time) = UNIX_EPOCH.checked_add(Duration::from_millis(millis))
        && let Ok(file) =
            File::open(destination).or_else(|_| OpenOptions::new().write(true).open(destination))
    {
        let _ = file.set_modified(time);
    }
    #[cfg(unix)]
    if permissions && let (Some(source), Ok(dest)) = (source, fs::metadata(destination)) {
        let mode = (dest.permissions().mode() & !0o700) | (source.permissions().mode() & 0o700);
        let _ = fs::set_permissions(destination, fs::Permissions::from_mode(mode));
    }
}

struct Copier<'a> {
    control: &'a JobControl,
    source_root: PathBuf,
    destination_root: PathBuf,
    policy: CopyPolicy,
    report: CopyReport,
}
impl Copier<'_> {
    fn issue(&mut self, path: &Path, kind: CopyIssueKind, message: impl ToString) -> bool {
        self.report.issues.push(CopyIssue {
            path: path.to_owned(),
            kind,
            message: message.to_string(),
        });
        false
    }
    fn process(&mut self, source: &Path, destination: &Path) -> bool {
        if !self.control.checkpoint() {
            return false;
        }
        let link_metadata = match fs::symlink_metadata(source) {
            Ok(value) => value,
            Err(error) => return self.issue(source, CopyIssueKind::Failed, error),
        };
        if !self.control.entry(
            source,
            link_metadata.is_file().then_some(link_metadata.len()),
        ) {
            return false;
        }
        let source_metadata = fs::metadata(source).ok();
        let destination_metadata = fs::metadata(destination).ok();
        let source_is_dir = source_metadata.as_ref().is_some_and(fs::Metadata::is_dir);
        let source_path = match resolved(source) {
            Ok(path) => path,
            Err(error) => return self.issue(source, CopyIssueKind::Failed, error),
        };
        let destination_path = match resolved(destination) {
            Ok(path) => path,
            Err(error) => return self.issue(destination, CopyIssueKind::Failed, error),
        };
        if source_path == destination_path
            || (source_is_dir && destination_path.starts_with(&source_path))
        {
            return self.issue(
                destination,
                CopyIssueKind::Skipped,
                "Source and destination overlap",
            );
        }
        #[cfg(unix)]
        if let (Some(source), Some(dest)) = (&source_metadata, &destination_metadata)
            && source.dev() == dest.dev()
            && source.ino() == dest.ino()
        {
            return self.issue(
                destination,
                CopyIssueKind::Unsupported,
                "Destination aliases the source",
            );
        }
        // Existing directories merge under every original collision policy.
        if let Some(dest) = &destination_metadata
            && !dest.is_dir()
        {
            let overwrite = match self.policy {
                CopyPolicy::Skip => false,
                CopyPolicy::Overwrite => true,
                CopyPolicy::OverwriteIfOlder => {
                    modified_ms(source_metadata.as_ref()) > modified_ms(Some(dest))
                }
                CopyPolicy::OverwriteIfSizeDiffers => {
                    source_metadata.as_ref().map_or(0, fs::Metadata::len) != dest.len()
                }
            };
            if !overwrite {
                return self.issue(
                    destination,
                    CopyIssueKind::Skipped,
                    "Existing destination skipped",
                );
            }
        }
        if fs::symlink_metadata(destination).is_ok_and(|m| m.is_symlink()) {
            return self.issue(
                destination,
                CopyIssueKind::Unsupported,
                "Existing destination link is not followed",
            );
        }
        if !destination_path.starts_with(&self.destination_root) {
            return self.issue(
                destination,
                CopyIssueKind::Unsupported,
                "Destination leaves the selected folder",
            );
        }
        if link_metadata.is_symlink() {
            let target = match fs::read_link(source) {
                Ok(value) => value,
                Err(error) => return self.issue(source, CopyIssueKind::Failed, error),
            };
            let source_target = resolved(&source.parent().unwrap().join(&target));
            let destination_target = resolved(&destination.parent().unwrap().join(&target));
            if target.is_absolute()
                || !source_target.is_ok_and(|p| p.starts_with(&self.source_root))
                || !destination_target.is_ok_and(|p| p.starts_with(&self.destination_root))
            {
                return self.issue(
                    source,
                    CopyIssueKind::Unsupported,
                    "Link target leaves the selected folders",
                );
            }
            #[cfg(unix)]
            {
                if let Err(error) = symlink(target, destination) {
                    return self.issue(destination, CopyIssueKind::Failed, error);
                }
                apply_metadata(source_metadata.as_ref(), destination, true);
                self.report.links_copied += 1;
                return true;
            }
            #[cfg(not(unix))]
            return self.issue(
                source,
                CopyIssueKind::Unsupported,
                "Link copying is available on Unix only",
            );
        }
        if link_metadata.is_dir() {
            if !destination_metadata
                .as_ref()
                .is_some_and(fs::Metadata::is_dir)
                && let Err(error) = fs::create_dir(destination)
            {
                return self.issue(destination, CopyIssueKind::Failed, error);
            }
            let children = match fs::read_dir(source) {
                Ok(children) => children,
                Err(error) => return self.issue(source, CopyIssueKind::Failed, error),
            };
            // Original recursion continues through failed children and reports true
            // when enumeration itself succeeded. Detailed issues remain visible.
            for child in children {
                let child = match child {
                    Ok(child) => child,
                    Err(error) => return self.issue(source, CopyIssueKind::Failed, error),
                };
                if !self.control.checkpoint() {
                    return false;
                }
                self.process(&child.path(), &destination.join(child.file_name()));
                if self.control.is_cancelled() {
                    return false;
                }
            }
            apply_metadata(source_metadata.as_ref(), destination, false);
            self.report.directories_processed += 1;
            return true;
        }
        if !link_metadata.is_file() {
            return self.issue(
                source,
                CopyIssueKind::Unsupported,
                "Only regular files, folders and contained relative links are supported",
            );
        }
        let mut input = match File::open(source) {
            Ok(file) => file,
            Err(error) => return self.issue(source, CopyIssueKind::Failed, error),
        };
        let mut output = match OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(destination)
        {
            Ok(file) => file,
            Err(error) => return self.issue(destination, CopyIssueKind::Failed, error),
        };
        if let Err(error) = copy_stream(&mut input, &mut output, self.control) {
            return self.issue(
                destination,
                if self.control.is_cancelled() {
                    CopyIssueKind::Cancelled
                } else {
                    CopyIssueKind::Failed
                },
                error,
            );
        }
        drop(output);
        apply_metadata(source_metadata.as_ref(), destination, true);
        self.report.files_copied += 1;
        true
    }
}

/// Copy the ordered selection into an existing folder. Validate all names before
/// writing. Scope assumes stable local trees; this is not a race-proof sandbox.
pub fn copy_entries(request: &CopyRequest) -> io::Result<CopyReport> {
    copy_entries_with_control(request, &JobControl::default())
}

/// Copy an ordered direct-child selection with cooperative pause/cancellation.
///
/// # Errors
/// Returns invalid-request or root-access errors; individual and partial failures are recorded in the report.
pub fn copy_entries_with_control(
    request: &CopyRequest,
    control: &JobControl,
) -> io::Result<CopyReport> {
    control.total(request.names.len());
    copy_entries_in_job(request, control)
}

pub(crate) fn copy_entries_in_job(
    request: &CopyRequest,
    control: &JobControl,
) -> io::Result<CopyReport> {
    if !control.checkpoint() {
        return Ok(CopyReport {
            cancelled: true,
            ..Default::default()
        });
    }
    if request.names.iter().any(|name| !is_valid_leaf_name(name))
        || request
            .names
            .iter()
            .enumerate()
            .any(|(i, name)| request.names[..i].contains(name))
        || request
            .new_name
            .as_ref()
            .is_some_and(|name| !is_valid_leaf_name(name) || request.names.len() != 1)
    {
        return Err(invalid(
            "Use distinct direct child names and rename only one item",
        ));
    }
    let source = fs::canonicalize(&request.source_directory)?;
    let destination = fs::canonicalize(&request.destination_directory)?;
    if !source.is_dir() || !destination.is_dir() {
        return Err(invalid("Existing source and destination folders required"));
    }
    let mut copier = Copier {
        control,
        source_root: source.clone(),
        destination_root: destination.clone(),
        policy: request.policy,
        report: CopyReport::default(),
    };
    for name in &request.names {
        if !control.checkpoint() {
            break;
        }
        let success = copier.process(
            &source.join(name),
            &destination.join(request.new_name.as_ref().unwrap_or(name)),
        );
        copier.report.outcomes.push(CopyOutcome {
            name: name.clone(),
            success: success && !control.is_cancelled(),
        });
        if control.is_cancelled() {
            break;
        }
        control.completed();
    }
    copier.report.cancelled = control.is_cancelled();
    if copier.report.cancelled
        && !copier
            .report
            .issues
            .iter()
            .any(|issue| issue.kind == CopyIssueKind::Cancelled)
    {
        copier.report.issues.push(CopyIssue {
            path: control.snapshot().current_path.unwrap_or(source),
            kind: CopyIssueKind::Cancelled,
            message: "Copy cancelled; completed and partial destinations retained".to_owned(),
        });
    }
    Ok(copier.report)
}
