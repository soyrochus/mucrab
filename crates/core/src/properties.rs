// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
//! Captured local/ZIP properties, bounded folder totals and reviewed metadata edits.
use crate::{JobControl, PinnedDirectory, ZipCatalog, ZipLocation, is_valid_leaf_name};
use serde::Serialize;
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug)]
pub enum EntryTarget {
    Local(PathBuf),
    Zip { location: ZipLocation, name: String },
}
impl EntryTarget {
    pub fn display_path(&self) -> PathBuf {
        match self {
            Self::Local(p) => p.clone(),
            Self::Zip { location, name } => location.display_path().join(name),
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum EntryKind {
    File,
    Folder,
    Link,
    Other,
}
pub(crate) fn entry_kind(m: &fs::Metadata) -> EntryKind {
    if m.is_symlink() {
        EntryKind::Link
    } else if m.is_dir() {
        EntryKind::Folder
    } else if m.is_file() {
        EntryKind::File
    } else {
        EntryKind::Other
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EntryIdentity {
    #[cfg(unix)]
    dev: u64,
    #[cfg(unix)]
    ino: u64,
    kind: EntryKind,
    len: u64,
    mode: Option<u32>,
    uid: Option<u32>,
    gid: Option<u32>,
    modified: Option<SystemTime>,
    link: Option<PathBuf>,
}
impl EntryIdentity {
    pub(crate) fn read(path: &Path) -> io::Result<Self> {
        let m = fs::symlink_metadata(path)?;
        Self::from_metadata(path, &m)
    }
    pub(crate) fn from_metadata(path: &Path, m: &fs::Metadata) -> io::Result<Self> {
        Ok(Self {
            #[cfg(unix)]
            dev: m.dev(),
            #[cfg(unix)]
            ino: m.ino(),
            kind: entry_kind(m),
            len: m.len(),
            mode: mode(m),
            uid: uid(m),
            gid: gid(m),
            modified: m.modified().ok(),
            link: if m.is_symlink() {
                Some(fs::read_link(path)?)
            } else {
                None
            },
        })
    }
    pub(crate) fn validate(&self, path: &Path) -> io::Result<()> {
        if Self::read(path)? != *self {
            Err(invalid("Captured entry changed; refresh and review again"))
        } else {
            Ok(())
        }
    }
}
fn invalid(s: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, s)
}
#[cfg(unix)]
fn mode(m: &fs::Metadata) -> Option<u32> {
    Some(m.mode() & 0o7777)
}
#[cfg(not(unix))]
fn mode(_: &fs::Metadata) -> Option<u32> {
    None
}
#[cfg(unix)]
fn uid(m: &fs::Metadata) -> Option<u32> {
    Some(m.uid())
}
#[cfg(not(unix))]
fn uid(_: &fs::Metadata) -> Option<u32> {
    None
}
#[cfg(unix)]
fn gid(m: &fs::Metadata) -> Option<u32> {
    Some(m.gid())
}
#[cfg(not(unix))]
fn gid(_: &fs::Metadata) -> Option<u32> {
    None
}

#[derive(Clone, Debug)]
pub struct PropertyRequest {
    target: EntryTarget,
    identity: EntryIdentity,
}
impl PropertyRequest {
    pub fn capture(target: EntryTarget) -> io::Result<Self> {
        let path = match &target {
            EntryTarget::Local(p) => p,
            EntryTarget::Zip { location, .. } => &location.archive,
        };
        Ok(Self {
            identity: EntryIdentity::read(path)?,
            target,
        })
    }
    pub fn inspect(&self) -> io::Result<PropertySnapshot> {
        let path = match &self.target {
            EntryTarget::Local(p) => p,
            EntryTarget::Zip { location, .. } => &location.archive,
        };
        self.identity.validate(path)?;
        let mut snapshot = PropertySnapshot::capture(self.target.clone())?;
        snapshot.resolve_accounts();
        self.identity.validate(path)?;
        Ok(snapshot)
    }
}
#[derive(Clone, Debug)]
pub struct PropertySnapshot {
    pub target: EntryTarget,
    pub kind: EntryKind,
    pub size: Option<u64>,
    pub allocated_size: Option<u64>,
    pub modified: Option<SystemTime>,
    pub accessed: Option<SystemTime>,
    pub created: Option<SystemTime>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub owner: Option<String>,
    pub group: Option<String>,
    pub mode: Option<u32>,
    pub links: Option<u64>,
    pub link_target: Option<PathBuf>,
    pub(crate) identity: EntryIdentity,
}
impl PropertySnapshot {
    pub fn capture(target: EntryTarget) -> io::Result<Self> {
        match &target {
            EntryTarget::Local(path) => {
                let m = fs::symlink_metadata(path)?;
                let identity = EntryIdentity::from_metadata(path, &m)?;
                Ok(Self {
                    kind: entry_kind(&m),
                    size: Some(m.len()),
                    allocated_size: {
                        #[cfg(unix)]
                        {
                            m.blocks().checked_mul(512)
                        }
                        #[cfg(not(unix))]
                        {
                            None
                        }
                    },
                    modified: m.modified().ok(),
                    accessed: m.accessed().ok(),
                    created: m.created().ok(),
                    uid: uid(&m),
                    gid: gid(&m),
                    owner: None,
                    group: None,
                    mode: mode(&m),
                    links: {
                        #[cfg(unix)]
                        {
                            Some(m.nlink())
                        }
                        #[cfg(not(unix))]
                        {
                            None
                        }
                    },
                    link_target: identity.link.clone(),
                    identity,
                    target,
                })
            }
            EntryTarget::Zip { location, name } => {
                if !is_valid_leaf_name(name) {
                    return Err(invalid("One ZIP child is required"));
                }
                let identity = EntryIdentity::read(&location.archive)?;
                let entry = ZipCatalog::open(&location.archive)?
                    .list(&location.prefix)?
                    .into_iter()
                    .find(|e| e.name == *name)
                    .ok_or_else(|| invalid("ZIP member is missing"))?;
                let metadata = ZipCatalog::open(&location.archive)?
                    .explicit_metadata(&location.prefix, name)?;
                Ok(Self {
                    kind: if entry.is_directory {
                        EntryKind::Folder
                    } else {
                        EntryKind::File
                    },
                    allocated_size: None,
                    size: metadata.as_ref().and_then(|m| u64::try_from(m.size).ok()),
                    modified: metadata.and_then(|m| time_from_millis(m.modified_ms)),
                    accessed: None,
                    created: None,
                    uid: None,
                    gid: None,
                    owner: None,
                    group: None,
                    mode: None,
                    links: None,
                    link_target: None,
                    identity,
                    target,
                })
            }
        }
    }
    /// Best-effort name lookup, performed only by the properties read worker.
    pub fn resolve_accounts(&mut self) {
        self.owner = self.uid.and_then(|id| account_name(id, false));
        self.group = self.gid.and_then(|id| account_name(id, true));
    }
    pub fn validate(&self) -> io::Result<()> {
        match &self.target {
            EntryTarget::Local(p) => self.identity.validate(p),
            EntryTarget::Zip { location, .. } => self.identity.validate(&location.archive),
        }
    }
    pub fn editable(&self) -> bool {
        matches!(self.target, EntryTarget::Local(_))
            && matches!(self.kind, EntryKind::File | EntryKind::Folder)
            && self.mode.is_some()
    }
    pub fn metadata_plan(
        &self,
        rwx: Option<u32>,
        modified: Option<SystemTime>,
    ) -> io::Result<MetadataPlan> {
        if !self.editable() {
            return Err(invalid(
                "Permission/time editing requires a local non-link file or folder",
            ));
        }
        if rwx.is_some_and(|m| m > 0o777) {
            return Err(invalid("Only the nine ordinary rwx bits are editable"));
        }
        if rwx.is_none() && modified.is_none() {
            return Err(invalid("Choose a metadata change"));
        }
        Ok(MetadataPlan {
            snapshot: self.clone(),
            rwx,
            modified,
        })
    }
}
fn account_name(id: u32, group: bool) -> Option<String> {
    #[cfg(target_os = "linux")]
    let output = std::process::Command::new("getent")
        .args([if group { "group" } else { "passwd" }, &id.to_string()])
        .output()
        .ok()?;
    #[cfg(target_os = "macos")]
    let output = std::process::Command::new("/usr/bin/dscacheutil")
        .args([
            "-q",
            if group { "group" } else { "user" },
            "-a",
            if group { "gid" } else { "uid" },
            &id.to_string(),
        ])
        .output()
        .ok()?;
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = (id, group);
        None
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        if !output.status.success() {
            return None;
        }
        let text = std::str::from_utf8(&output.stdout).ok()?;
        #[cfg(target_os = "linux")]
        let name = text.lines().next()?.split(':').next()?;
        #[cfg(target_os = "macos")]
        let name = text.lines().find_map(|s| s.strip_prefix("name: "))?;
        (!name.is_empty() && name.len() <= 256 && !name.chars().any(char::is_control))
            .then(|| name.to_owned())
    }
}
#[derive(Clone, Debug)]
pub struct MetadataPlan {
    pub snapshot: PropertySnapshot,
    pub rwx: Option<u32>,
    pub modified: Option<SystemTime>,
}
#[derive(Clone, Debug, Serialize)]
pub struct MetadataStep {
    pub field: String,
    pub changed: bool,
    pub error: Option<String>,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct MetadataReport {
    pub steps: Vec<MetadataStep>,
    pub cancelled: bool,
}
impl MetadataPlan {
    pub fn apply(&self, control: &JobControl) -> io::Result<MetadataReport> {
        let EntryTarget::Local(path) = &self.snapshot.target else {
            return Err(invalid("Local metadata required"));
        };
        self.snapshot.identity.validate(path)?;
        // Acquire a time-capable handle before changing permissions. A failure to
        // preserve atime must not follow an already committed chmod.
        let handle = if self.modified.is_some() {
            let f = fs::File::open(path)?;
            if EntryIdentity::from_metadata(path, &f.metadata()?)? != self.snapshot.identity {
                return Err(invalid("Entry changed while opening"));
            }
            Some(f)
        } else {
            None
        };
        let accessed = handle
            .as_ref()
            .map(|f| f.metadata()?.accessed())
            .transpose()?;
        control.total(usize::from(self.rwx.is_some()) + usize::from(self.modified.is_some()));
        let mut report = MetadataReport::default();
        let mut expected = self.snapshot.identity.clone();
        let mut failed = false;
        for field in ["Permissions", "Modified time"] {
            if (field == "Permissions" && self.rwx.is_none())
                || (field == "Modified time" && self.modified.is_none())
            {
                continue;
            }
            if !control.entry(path, None) {
                report.cancelled = true;
                report.steps.push(MetadataStep {
                    field: field.into(),
                    changed: false,
                    error: Some("Cancelled before this field".into()),
                });
                continue;
            }
            if failed {
                report.steps.push(MetadataStep {
                    field: field.into(),
                    changed: false,
                    error: Some("Not attempted after an earlier failure".into()),
                });
                continue;
            }
            let outcome = (|| -> io::Result<()> {
                expected.validate(path)?;
                if field == "Permissions" {
                    #[cfg(unix)]
                    {
                        let bits = (expected
                            .mode
                            .ok_or_else(|| invalid("Unix mode unavailable"))?
                            & 0o7000)
                            | self.rwx.expect("permission step");
                        fs::set_permissions(path, fs::Permissions::from_mode(bits))?;
                    }
                    #[cfg(not(unix))]
                    {
                        return Err(invalid("Unix permissions unavailable"));
                    }
                } else {
                    handle.as_ref().expect("time handle").set_times(
                        fs::FileTimes::new()
                            .set_accessed(accessed.expect("preserved atime"))
                            .set_modified(self.modified.expect("time step")),
                    )?;
                }
                Ok(())
            })();
            let changed = outcome.is_ok();
            let error = outcome.err().map(|e| e.to_string());
            failed = error.is_some();
            report.steps.push(MetadataStep {
                field: field.into(),
                changed,
                error,
            });
            // Update assumptions before publishing the completed-step boundary.
            if changed {
                match EntryIdentity::read(path) {
                    Ok(id) => expected = id,
                    Err(_) => {
                        failed = true;
                    }
                }
            }
            control.completed();
        }
        report.cancelled |= control.is_cancelled();
        Ok(report)
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct FolderSizeReport {
    pub bytes: u64,
    pub files: usize,
    pub folders: usize,
    pub links: usize,
    pub visited: usize,
    pub errors: usize,
    pub details: Vec<String>,
    pub cancelled: bool,
}
impl FolderSizeReport {
    pub fn partial(&self) -> bool {
        self.cancelled || self.errors > 0
    }
    fn issue(&mut self, path: &Path, message: &str) {
        self.errors += 1;
        if self.details.len() < 256 {
            self.details.push(format!("{}: {message}", path.display()));
        }
    }
}
pub fn folder_size(path: &Path, control: &JobControl) -> io::Result<FolderSizeReport> {
    folder_size_with_limits(path, control, 256, 1_000_000)
}
pub fn folder_size_with_limits(
    path: &Path,
    control: &JobControl,
    depth_limit: usize,
    visit_limit: usize,
) -> io::Result<FolderSizeReport> {
    let root = PinnedDirectory::new(path)?;
    if fs::symlink_metadata(path)?.is_symlink() {
        return Err(invalid("Do not follow a folder link"));
    }
    let mounts = crate::everyday::mounted_roots()?;
    #[cfg(unix)]
    let device = fs::symlink_metadata(&root.path)?.dev();
    let mut stack = vec![(root.clone(), fs::read_dir(&root.path)?, 0usize)];
    let mut report = FolderSizeReport::default();
    while !stack.is_empty() {
        if !control.checkpoint() {
            report.cancelled = true;
            break;
        }
        if report.visited >= visit_limit.min(1_000_000) {
            report.issue(&root.path, "Visited-entry limit reached");
            break;
        }
        let (dir, iter, depth) = stack.last_mut().expect("nonempty stack");
        if let Err(e) = dir.validate() {
            report.issue(&dir.path, &e.to_string());
            stack.pop();
            continue;
        }
        let entry = match iter.next() {
            None => {
                stack.pop();
                continue;
            }
            Some(Err(e)) => {
                report.visited += 1;
                report.issue(&dir.path, &e.to_string());
                continue;
            }
            Some(Ok(e)) => e,
        };
        let depth = *depth;
        let path = entry.path();
        report.visited += 1;
        if !control.entry(&path, None) {
            report.cancelled = true;
            break;
        }
        control.completed();
        let m = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(e) => {
                report.issue(&path, &e.to_string());
                continue;
            }
        };
        match entry_kind(&m) {
            EntryKind::File => {
                report.files += 1;
                if let Some(bytes) = report.bytes.checked_add(m.len()) {
                    report.bytes = bytes;
                } else {
                    report.issue(&path, "Logical byte total overflow");
                    break;
                }
            }
            EntryKind::Link => report.links += 1,
            EntryKind::Other => report.issue(&path, "Special entry omitted"),
            EntryKind::Folder => {
                report.folders += 1;
                #[cfg(unix)]
                let same_device = device == m.dev();
                #[cfg(not(unix))]
                let same_device = true;
                if !same_device || mounts.contains(&path) {
                    report.issue(&path, "Mount boundary omitted");
                    continue;
                }
                if depth + 1 >= depth_limit.min(256) {
                    report.issue(&path, "Depth limit reached");
                    continue;
                }
                let next = (|| {
                    let pin = PinnedDirectory::new(&path)?;
                    if pin.path != path || fs::symlink_metadata(&path)?.is_symlink() {
                        return Err(invalid("Folder changed to a link"));
                    }
                    let iter = fs::read_dir(&pin.path)?;
                    Ok((pin, iter, depth + 1))
                })();
                match next {
                    Ok(next) => stack.push(next),
                    Err(e) => report.issue(&path, &e.to_string()),
                }
            }
        }
    }
    report.cancelled |= control.is_cancelled();
    Ok(report)
}
pub fn time_from_millis(ms: i64) -> Option<SystemTime> {
    if ms >= 0 {
        UNIX_EPOCH.checked_add(Duration::from_millis(ms as u64))
    } else {
        UNIX_EPOCH.checked_sub(Duration::from_millis(ms.unsigned_abs()))
    }
}
pub fn parse_modified_time(text: &str) -> Result<SystemTime, String> {
    let dt = chrono::DateTime::parse_from_rfc3339(text)
        .map_err(|_| "Use an RFC3339 UTC timestamp, for example 2026-10-17T12:00:00Z".to_owned())?;
    let seconds = dt.timestamp();
    let nanos = dt.timestamp_subsec_nanos();
    if seconds >= 0 {
        UNIX_EPOCH.checked_add(Duration::new(seconds as u64, nanos))
    } else {
        UNIX_EPOCH
            .checked_sub(Duration::from_secs(seconds.unsigned_abs()))
            .and_then(|t| t.checked_add(Duration::from_nanos(u64::from(nanos))))
    }
    .ok_or_else(|| "Timestamp is outside the platform range".into())
}
