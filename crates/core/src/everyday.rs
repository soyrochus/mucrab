// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
//! Reviewed local mutations. Assumes stable trees; does not promise race-free IO.
use crate::{
    CopyIssue, CopyIssueKind, CopyOutcome, CopyPolicy, CopyReport, CopyRequest, JobControl,
    is_valid_leaf_name,
};
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::{
    collections::HashSet,
    fs, io,
    path::{Path, PathBuf},
    time::SystemTime,
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Identity {
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
    kind: u8,
    file: Option<(u64, Option<SystemTime>)>,
    link: Option<PathBuf>,
}
fn identity(path: &Path) -> io::Result<Identity> {
    let m = fs::symlink_metadata(path)?;
    Ok(Identity {
        #[cfg(unix)]
        device: m.dev(),
        #[cfg(unix)]
        inode: m.ino(),
        kind: if m.is_symlink() {
            2
        } else if m.is_dir() {
            1
        } else if m.is_file() {
            0
        } else {
            3
        },
        file: m.is_file().then(|| (m.len(), m.modified().ok())),
        link: if m.is_symlink() {
            Some(fs::read_link(path)?)
        } else {
            None
        },
    })
}
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
#[derive(Clone, Debug)]
pub struct PinnedDirectory {
    pub path: PathBuf,
    id: Identity,
}
impl PinnedDirectory {
    pub fn new(path: &Path) -> io::Result<Self> {
        let path = fs::canonicalize(path)?;
        let id = identity(&path)?;
        if id.kind != 1 {
            return Err(invalid("An existing local directory is required"));
        }
        Ok(Self { path, id })
    }
    pub fn validate(&self) -> io::Result<()> {
        if identity(&self.path)? != self.id {
            return Err(invalid("Reviewed directory changed; review again"));
        }
        Ok(())
    }
    pub fn create_folder(&self, name: &str, control: &JobControl) -> io::Result<bool> {
        self.validate()?;
        if !is_valid_leaf_name(name) {
            return Err(invalid("Enter one nonempty folder name without separators"));
        }
        control.total(1);
        let path = self.path.join(name);
        if !control.entry(&path, None) {
            return Ok(false);
        }
        fs::create_dir(path)?;
        control.completed();
        Ok(true)
    }
}
#[derive(Clone, Debug)]
struct Entry {
    path: PathBuf,
    id: Identity,
    parent: PinnedDirectory,
}
impl Entry {
    fn new(path: &Path) -> io::Result<Self> {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .filter(|n| is_valid_leaf_name(n))
            .ok_or_else(|| invalid("Direct local child required"))?;
        let parent = PinnedDirectory::new(
            path.parent()
                .ok_or_else(|| invalid("Root cannot be selected"))?,
        )?;
        let path = parent.path.join(name);
        let id = identity(&path)?;
        if id.kind == 3 {
            return Err(invalid("Special files are unsupported"));
        }
        Ok(Self { path, id, parent })
    }
    fn validate(&self) -> io::Result<()> {
        self.parent.validate()?;
        if identity(&self.path)? != self.id {
            return Err(invalid("Reviewed source changed; review again"));
        }
        Ok(())
    }
    fn name(&self) -> String {
        self.path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned()
    }
}
#[derive(Clone, Debug)]
pub struct DeletePlan {
    entries: Vec<Entry>,
    protected: Vec<PathBuf>,
}
pub type TrashAdapter<'a> = dyn FnMut(&Path) -> io::Result<()> + 'a;
impl DeletePlan {
    pub fn new(paths: &[PathBuf], protected: &[PathBuf]) -> io::Result<Self> {
        if paths.is_empty() {
            return Err(invalid("Select at least one local entry"));
        }
        let protected: Vec<_> = protected
            .iter()
            .map(fs::canonicalize)
            .collect::<io::Result<_>>()?;
        let entries = paths
            .iter()
            .map(|p| Entry::new(p))
            .collect::<io::Result<Vec<_>>>()?;
        let mounts = mounted_roots()?;
        let mut seen = HashSet::new();
        for e in &entries {
            if mounts.contains(&e.path) {
                return Err(invalid("Mounted filesystem entries cannot be deleted"));
            }
            #[cfg(unix)]
            if e.id.kind == 1 && e.id.device != e.parent.id.device {
                return Err(invalid(
                    "Filesystem roots and mount points cannot be deleted",
                ));
            }
            if entries.iter().any(|other| {
                other.path != e.path && other.id.kind == 1 && e.path.starts_with(&other.path)
            }) {
                return Err(invalid("Overlapping deletion roots"));
            }
            if !seen.insert(e.path.clone()) {
                return Err(invalid("Duplicate deletion source"));
            }
            if e.id.kind != 2 && protected.iter().any(|p| p.starts_with(&e.path)) {
                return Err(invalid(
                    "Cannot delete an open pane location or its ancestor",
                ));
            }
        }
        Ok(Self { entries, protected })
    }
    pub fn paths(&self) -> Vec<PathBuf> {
        self.entries.iter().map(|e| e.path.clone()).collect()
    }
    pub fn apply(
        &self,
        control: &JobControl,
        mut trash: Option<&mut TrashAdapter<'_>>,
    ) -> io::Result<CopyReport> {
        let mounts = mounted_roots()?;
        for e in &self.entries {
            e.validate()?;
            if mounts.contains(&e.path) {
                return Err(invalid("Mounted filesystem entry retained"));
            }
        }
        // Recheck protected locations at apply time, too.
        for p in &self.protected {
            if !p.exists() {
                return Err(invalid("Protected pane location changed"));
            }
        }
        let mut report = CopyReport::default();
        control.total(self.entries.len());
        for e in &self.entries {
            if !control.entry(&e.path, None) {
                break;
            }
            let result = e.validate().and_then(|()| match trash.as_mut() {
                Some(adapter) => adapter(&e.path),
                None => remove_tree(&e.path, control, 0, &mounts),
            });
            let success = result.is_ok();
            if let Err(error) = result {
                report.issues.push(CopyIssue {
                    path: e.path.clone(),
                    kind: if control.is_cancelled() {
                        CopyIssueKind::Cancelled
                    } else {
                        CopyIssueKind::Failed
                    },
                    message: error.to_string(),
                });
            }
            report.outcomes.push(CopyOutcome {
                name: e.name(),
                success,
            });
            if control.is_cancelled() {
                break;
            }
            control.completed();
        }
        report.cancelled = control.is_cancelled();
        Ok(report)
    }
}
fn remove_tree(
    path: &Path,
    control: &JobControl,
    depth: usize,
    mounts: &[PathBuf],
) -> io::Result<()> {
    if mounts.iter().any(|root| root == path) {
        return Err(invalid("Mounted filesystem entry retained"));
    }
    if depth > 256 {
        return Err(invalid(
            "Deletion depth exceeds 256; remaining tree retained",
        ));
    }
    if !control.entry(path, None) {
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "Deletion cancelled; partial tree retained",
        ));
    }
    let before = identity(path)?;
    if before.kind == 1 {
        for child in fs::read_dir(path)? {
            if identity(path)? != before {
                return Err(invalid("Directory changed during deletion"));
            }
            let child = child?.path();
            #[cfg(unix)]
            if identity(&child)?.kind == 1 && identity(&child)?.device != before.device {
                return Err(invalid("Mounted filesystem retained during deletion"));
            }
            remove_tree(&child, control, depth + 1, mounts)?;
        }
        if !control.checkpoint() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Deletion cancelled",
            ));
        }
        if identity(path)? != before {
            return Err(invalid("Directory changed during deletion"));
        }
        fs::remove_dir(path)
    } else if before.kind == 0 || before.kind == 2 {
        fs::remove_file(path)
    } else {
        Err(invalid("Special file retained"))
    }
}
#[derive(Clone, Debug)]
pub struct PastePlan {
    entries: Vec<Entry>,
    destination: PinnedDirectory,
}
impl PastePlan {
    pub fn new(paths: &[PathBuf], destination: &Path) -> io::Result<Self> {
        if paths.is_empty() {
            return Err(invalid("Clipboard contains no local files"));
        }
        let destination = PinnedDirectory::new(destination)?;
        let mut seen = HashSet::new();
        let mut entries = Vec::new();
        for p in paths {
            let e = Entry::new(p)?;
            if !seen.insert(e.path.clone()) {
                continue;
            }
            let target = destination.path.join(e.name());
            let resolved = crate::copying::resolved(&e.path)?;
            let dest_resolved = crate::copying::resolved(&target)?;
            if resolved == dest_resolved
                || (e.id.kind == 1
                    && (dest_resolved.starts_with(&resolved)
                        || resolved.starts_with(&dest_resolved)))
            {
                return Err(invalid("Clipboard source and destination overlap"));
            }
            entries.push(e);
        }
        Ok(Self {
            entries,
            destination,
        })
    }
    pub fn paths(&self) -> Vec<PathBuf> {
        self.entries.iter().map(|e| e.path.clone()).collect()
    }
    pub fn apply(&self, policy: CopyPolicy, control: &JobControl) -> io::Result<CopyReport> {
        self.destination.validate()?;
        for e in &self.entries {
            e.validate()?;
        }
        let mut all = CopyReport::default();
        control.total(self.entries.len());
        for e in &self.entries {
            if !control.checkpoint() {
                break;
            }
            let report = crate::copying::copy_entries_in_job(
                &CopyRequest {
                    source_directory: e.parent.path.clone(),
                    destination_directory: self.destination.path.clone(),
                    names: vec![e.name()],
                    new_name: None,
                    policy,
                },
                control,
            )?;
            all.outcomes.extend(report.outcomes);
            all.issues.extend(report.issues);
            all.files_copied += report.files_copied;
            all.links_copied += report.links_copied;
            all.directories_processed += report.directories_processed;
        }
        all.cancelled = control.is_cancelled();
        Ok(all)
    }
}

#[cfg(target_os = "linux")]
fn parse_mount_roots(text: &str) -> Vec<PathBuf> {
    text.lines()
        .filter_map(|line| line.split_whitespace().nth(4))
        .map(|path| {
            PathBuf::from(
                path.replace("\\040", " ")
                    .replace("\\011", "\t")
                    .replace("\\012", "\n")
                    .replace("\\134", "\\"),
            )
        })
        .collect()
}
pub(crate) fn mounted_roots() -> io::Result<Vec<PathBuf>> {
    #[cfg(target_os = "linux")]
    {
        Ok(parse_mount_roots(&fs::read_to_string(
            "/proc/self/mountinfo",
        )?))
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(Vec::new())
    }
}
#[cfg(all(test, target_os = "linux"))]
mod mount_tests {
    #[test]
    fn kernel_mount_paths_decode_spaces_and_backslashes() {
        let roots = super::parse_mount_roots(
            "1 2 0:0 / /some\\040folder rw - ext4 x rw\n2 3 0:0 / /literal\\134040 rw - ext4 x rw",
        );
        assert_eq!(
            roots,
            vec![
                std::path::PathBuf::from("/some folder"),
                std::path::PathBuf::from("/literal\\040")
            ]
        );
    }
}
