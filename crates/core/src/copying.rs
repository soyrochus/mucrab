// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

// Recovered from original muCommander CopyJob and its transfer/collision callers.
// See NOTICE.md and evidence/workflows/W03-recursive-copy/contract.md.
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
pub enum CopyPolicy {
    #[default]
    Skip,
    Overwrite,
    OverwriteIfOlder,
    OverwriteIfSizeDiffers,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CopyRequest {
    pub source_directory: PathBuf,
    pub destination_directory: PathBuf,
    pub names: Vec<String>,
    pub new_name: Option<String>,
    pub policy: CopyPolicy,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CopyOutcome {
    pub name: String,
    /// Original processFile return value; a directory may contain failed children.
    pub success: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CopyIssueKind {
    Skipped,
    Failed,
    Unsupported,
}
#[derive(Clone, Debug)]
pub struct CopyIssue {
    pub path: PathBuf,
    pub kind: CopyIssueKind,
    pub message: String,
}
#[derive(Clone, Debug, Default)]
pub struct CopyReport {
    pub outcomes: Vec<CopyOutcome>,
    pub issues: Vec<CopyIssue>,
    pub files_copied: usize,
    pub directories_processed: usize,
    pub links_copied: usize,
}

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

struct Copier {
    source_root: PathBuf,
    destination_root: PathBuf,
    policy: CopyPolicy,
    report: CopyReport,
}
impl Copier {
    fn issue(&mut self, path: &Path, kind: CopyIssueKind, message: impl ToString) -> bool {
        self.report.issues.push(CopyIssue {
            path: path.to_owned(),
            kind,
            message: message.to_string(),
        });
        false
    }
    fn process(&mut self, source: &Path, destination: &Path) -> bool {
        let link_metadata = match fs::symlink_metadata(source) {
            Ok(value) => value,
            Err(error) => return self.issue(source, CopyIssueKind::Failed, error),
        };
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
                self.process(&child.path(), &destination.join(child.file_name()));
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
        if let Err(error) = io::copy(&mut input, &mut output) {
            return self.issue(destination, CopyIssueKind::Failed, error);
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
        source_root: source.clone(),
        destination_root: destination.clone(),
        policy: request.policy,
        report: CopyReport::default(),
    };
    for name in &request.names {
        let success = copier.process(
            &source.join(name),
            &destination.join(request.new_name.as_ref().unwrap_or(name)),
        );
        copier.report.outcomes.push(CopyOutcome {
            name: name.clone(),
            success,
        });
    }
    Ok(copier.report)
}
