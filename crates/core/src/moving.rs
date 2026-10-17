// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

// Recovered from original MoveJob, LocalFile.renameTo and transfer/delete callers.
// See NOTICE.md and evidence/workflows/W04-move-rename/contract.md.
use crate::copying::{apply_metadata, modified_ms, resolved};
use crate::{CopyIssue, CopyIssueKind, CopyOutcome, CopyPolicy, CopyRequest, is_valid_leaf_name};
use crate::{JobControl, jobs::copy_stream};
use serde::Deserialize;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::{
    fs::{self, File, OpenOptions},
    io,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Deserialize)]
pub struct MoveRequest {
    #[serde(flatten)]
    pub transfer: CopyRequest,
    #[serde(default)]
    pub rename_mode: bool,
}
#[derive(Clone, Debug, Default)]
pub struct MoveReport {
    pub cancelled: bool,
    pub outcomes: Vec<CopyOutcome>,
    pub issues: Vec<CopyIssue>,
    pub renamed_entries: usize,
    pub files_transferred: usize,
    pub sources_removed: usize,
}
struct Mover<'a> {
    control: &'a JobControl,
    source_root: PathBuf,
    destination_root: PathBuf,
    policy: CopyPolicy,
    report: MoveReport,
}
impl Mover<'_> {
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
        let entry = match fs::symlink_metadata(source) {
            Ok(value) => value,
            Err(error) => return self.issue(source, CopyIssueKind::Failed, error),
        };
        if !self
            .control
            .entry(source, entry.is_file().then_some(entry.len()))
        {
            return false;
        }
        if !entry.is_file() && !entry.is_dir() && !entry.is_symlink() {
            return self.issue(
                source,
                CopyIssueKind::Unsupported,
                "Only local regular files, folders and supported link renames are available",
            );
        }
        let metadata = fs::metadata(source).ok();
        let dest_metadata = fs::metadata(destination).ok();
        // A case-insensitive filesystem resolves a case-only target to the
        // source inode. Allow its atomic spelling change, while still rejecting
        // a distinct hard-link entry that aliases the source.
        #[cfg(unix)]
        if source != destination
            && source.parent() == destination.parent()
            && fs::symlink_metadata(destination)
                .is_ok_and(|dest| entry.dev() == dest.dev() && entry.ino() == dest.ino())
            && source.parent().is_some_and(|parent| {
                fs::read_dir(parent).is_ok_and(|mut entries| {
                    !entries.any(|entry| {
                        entry.is_err()
                            || entry.is_ok_and(|entry| {
                                Some(entry.file_name().as_os_str()) == destination.file_name()
                            })
                    })
                })
            })
        {
            if !self.control.checkpoint() {
                return false;
            }
            return match fs::rename(source, destination) {
                Ok(()) => {
                    self.report.renamed_entries += 1;
                    self.report.sources_removed += 1;
                    true
                }
                Err(error) => self.issue(destination, CopyIssueKind::Failed, error),
            };
        }
        let src_path = match resolved(source) {
            Ok(path) => path,
            Err(error) => return self.issue(source, CopyIssueKind::Failed, error),
        };
        let dst_path = match resolved(destination) {
            Ok(path) => path,
            Err(error) => return self.issue(destination, CopyIssueKind::Failed, error),
        };
        if src_path == dst_path
            || (metadata.as_ref().is_some_and(fs::Metadata::is_dir)
                && dst_path.starts_with(&src_path))
        {
            return self.issue(
                destination,
                CopyIssueKind::Skipped,
                "Source and destination overlap",
            );
        }
        #[cfg(unix)]
        if let (Some(source), Some(dest)) = (&metadata, &dest_metadata)
            && source.dev() == dest.dev()
            && source.ino() == dest.ino()
        {
            return self.issue(
                destination,
                CopyIssueKind::Unsupported,
                "Destination aliases the source",
            );
        }
        if let Some(dest) = &dest_metadata
            && !dest.is_dir()
        {
            let replace = match self.policy {
                CopyPolicy::Skip => false,
                CopyPolicy::Overwrite => true,
                CopyPolicy::OverwriteIfOlder => {
                    modified_ms(metadata.as_ref()) > modified_ms(Some(dest))
                }
                CopyPolicy::OverwriteIfSizeDiffers => {
                    metadata.as_ref().map_or(0, fs::Metadata::len) != dest.len()
                }
            };
            if !replace {
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
        if !dst_path.starts_with(&self.destination_root) {
            return self.issue(
                destination,
                CopyIssueKind::Unsupported,
                "Destination leaves the selected folder",
            );
        }
        if entry.is_symlink() {
            if metadata.is_none() {
                return self.issue(
                    source,
                    CopyIssueKind::Unsupported,
                    "Broken-link move is unsupported; source retained",
                );
            }
            let target = match fs::read_link(source) {
                Ok(value) => value,
                Err(error) => return self.issue(source, CopyIssueKind::Failed, error),
            };
            if target.is_absolute()
                || !resolved(&source.parent().unwrap().join(&target))
                    .is_ok_and(|p| p.starts_with(&self.source_root))
                || !resolved(&destination.parent().unwrap().join(&target))
                    .is_ok_and(|p| p.starts_with(&self.destination_root))
            {
                return self.issue(
                    source,
                    CopyIssueKind::Unsupported,
                    "Link target leaves the selected folders; source retained",
                );
            }
        }
        // Original attempts local rename before every fallback, including when
        // permissions deny rename while still allowing copy or in-place writing.
        if !self.control.checkpoint() {
            return false;
        }
        if fs::rename(source, destination).is_ok() {
            self.report.renamed_entries += 1;
            self.report.sources_removed += 1;
            return true;
        }
        // Never reproduce original fallback's conversion/consumption of targets.
        if entry.is_symlink() {
            return self.issue(source,CopyIssueKind::Unsupported,"Link could not be renamed; fallback through its target is disabled and source retained");
        }
        if entry.is_dir() {
            if !dest_metadata.as_ref().is_some_and(fs::Metadata::is_dir)
                && let Err(error) = fs::create_dir(destination)
            {
                return self.issue(destination, CopyIssueKind::Failed, error);
            }
            let children = match fs::read_dir(source) {
                Ok(value) => value,
                Err(error) => return self.issue(source, CopyIssueKind::Failed, error),
            };
            let mut all_succeeded = true;
            for child in children {
                let child = match child {
                    Ok(value) => value,
                    Err(error) => return self.issue(source, CopyIssueKind::Failed, error),
                };
                if !self.control.checkpoint() {
                    return false;
                }
                if !self.process(&child.path(), &destination.join(child.file_name())) {
                    all_succeeded = false;
                }
            }
            if !self.control.checkpoint() {
                return false;
            }
            apply_metadata(metadata.as_ref(), destination, false);
            if !all_succeeded {
                return false;
            }
            if !self.control.checkpoint() {
                return false;
            }
            if let Err(error) = fs::remove_dir(source) {
                return self.issue(
                    source,
                    CopyIssueKind::Failed,
                    format!("Children moved, but source folder could not be removed: {error}"),
                );
            }
            self.report.sources_removed += 1;
            return true;
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
                format!("Copy stopped; source retained: {error}"),
            );
        }
        drop(output);
        apply_metadata(metadata.as_ref(), destination, true);
        self.report.files_transferred += 1;
        if !self.control.checkpoint() {
            return false;
        }
        if let Err(error) = fs::remove_file(source) {
            return self.issue(
                source,
                CopyIssueKind::Failed,
                format!("Copied to destination, but source could not be removed: {error}"),
            );
        }
        self.report.sources_removed += 1;
        true
    }
}

/// Move selected direct children, retaining sources on failed fallback/deletion.
/// Stable owned local trees only; native guards do not establish race safety.
pub fn move_entries(request: &MoveRequest) -> io::Result<MoveReport> {
    move_entries_with_control(request, &JobControl::default())
}

pub fn move_entries_with_control(
    request: &MoveRequest,
    control: &JobControl,
) -> io::Result<MoveReport> {
    move_entries_controlled(request, control, true)
}

pub(crate) fn move_entries_controlled(
    request: &MoveRequest,
    control: &JobControl,
    top_level: bool,
) -> io::Result<MoveReport> {
    if top_level {
        control.total(request.transfer.names.len());
    }
    if !control.checkpoint() {
        return Ok(MoveReport {
            cancelled: true,
            ..Default::default()
        });
    }
    let input = &request.transfer;
    if input.names.iter().any(|n| !is_valid_leaf_name(n))
        || input
            .names
            .iter()
            .enumerate()
            .any(|(i, n)| input.names[..i].contains(n))
        || input
            .new_name
            .as_ref()
            .is_some_and(|n| !is_valid_leaf_name(n) || input.names.len() != 1)
        || (request.rename_mode && (input.names.len() != 1 || input.new_name.is_none()))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Use distinct direct child names and rename one item to a valid leaf name",
        ));
    }
    let source = fs::canonicalize(&input.source_directory)?;
    let dest = fs::canonicalize(&input.destination_directory)?;
    if !source.is_dir() || !dest.is_dir() || (request.rename_mode && source != dest) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Use existing folders and rename within the source folder",
        ));
    }
    let mut mover = Mover {
        control,
        source_root: source.clone(),
        destination_root: dest.clone(),
        policy: input.policy,
        report: MoveReport::default(),
    };
    for name in &input.names {
        if !control.checkpoint() {
            break;
        }
        let success = mover.process(
            &source.join(name),
            &dest.join(input.new_name.as_ref().unwrap_or(name)),
        );
        mover.report.outcomes.push(CopyOutcome {
            name: name.clone(),
            success: success && !control.is_cancelled(),
        });
        if control.is_cancelled() {
            break;
        }
        if top_level {
            control.completed();
        }
    }
    mover.report.cancelled = control.is_cancelled();
    if mover.report.cancelled
        && !mover
            .report
            .issues
            .iter()
            .any(|issue| issue.kind == CopyIssueKind::Cancelled)
    {
        mover.report.issues.push(CopyIssue {
            path: control.snapshot().current_path.unwrap_or(source),
            kind: CopyIssueKind::Cancelled,
            message:
                "Move cancelled; completed destinations remain and unfinished sources are retained"
                    .to_owned(),
        });
    }
    Ok(mover.report)
}
