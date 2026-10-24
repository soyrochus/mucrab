// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
// Recovered listing/extraction rules: evidence/workflows/W06-zip-browsing-extraction/contract.md.
//! Validated bounded ZIP browsing and reviewed extraction without archive writeback.
use crate::{JobControl, jobs::copy_stream};

use crate::{CopyIssue, CopyIssueKind, CopyPolicy, FileEntry, FileMetadata, filename_extension};
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use zip::{CompressionMethod, ZipArchive};

const MAX_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_ENTRIES: usize = 20_000;
const MAX_EXPANDED_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_ENTRY_BYTES: u64 = 512 * 1024 * 1024;
const MAX_DEPTH: usize = 64;

fn invalid(message: impl ToString) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.to_string())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ArchiveStamp {
    length: u64,
    modified: SystemTime,
    #[cfg(unix)]
    identity: (u64, u64),
}
impl ArchiveStamp {
    fn read(metadata: &fs::Metadata) -> io::Result<Self> {
        if !metadata.is_file() || metadata.len() > MAX_ARCHIVE_BYTES {
            return Err(invalid("ZIP must be a regular file no larger than 512 MiB"));
        }
        Ok(Self {
            length: metadata.len(),
            modified: metadata.modified()?,
            #[cfg(unix)]
            identity: (metadata.dev(), metadata.ino()),
        })
    }
}

/// A virtual folder in a local, read-only ZIP.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZipLocation {
    /// Local archive path, retained unchanged by browsing and extraction.
    pub archive: PathBuf,
    /// Empty for the root; otherwise a relative path ending in `/`.
    pub prefix: String,
}
impl ZipLocation {
    #[doc = "Build the virtual path used to display and navigate this archive folder."]
    pub fn display_path(&self) -> PathBuf {
        if self.prefix.is_empty() {
            self.archive.clone()
        } else {
            self.archive.join(self.prefix.trim_end_matches('/'))
        }
    }

    pub(crate) fn resolve(path: &Path) -> io::Result<Option<Self>> {
        for ancestor in path.ancestors() {
            if ancestor
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.to_ascii_lowercase().ends_with(".zip"))
                && ancestor.is_file()
            {
                if fs::symlink_metadata(ancestor)?.file_type().is_symlink() {
                    return Err(invalid("Opening a symbolic link as a ZIP is unsupported"));
                }
                let relative = path.strip_prefix(ancestor).map_err(invalid)?;
                let relative = relative
                    .to_str()
                    .ok_or_else(|| invalid("ZIP path must be Unicode"))?;
                let prefix = if relative.is_empty() {
                    String::new()
                } else {
                    validate_path(relative)?;
                    format!("{}/", relative.trim_end_matches('/'))
                };
                return Ok(Some(Self {
                    archive: fs::canonicalize(ancestor)?,
                    prefix,
                }));
            }
        }
        Ok(None)
    }
}

#[derive(Clone, Debug)]
struct Member {
    path: String,
    directory: bool,
    size: u64,
    modified_ms: i64,
    mode: u32,
    index: usize,
}
#[derive(Clone, Debug)]
struct Node {
    path: String,
    entry: FileEntry,
    explicit: bool,
}

/// Validated ZIP32 metadata in original encounter order. No extraction convenience API is used.
#[derive(Clone, Debug)]
pub struct ZipCatalog {
    /// Captured or reported filesystem/remote path; inspect the enclosing operation contract.
    pub path: PathBuf,
    stamp: ArchiveStamp,
    members: Vec<Member>,
    nodes: Vec<Node>,
    by_path: HashMap<String, usize>,
}

pub(crate) fn validate_path(path: &str) -> io::Result<()> {
    let path = path.strip_suffix('/').unwrap_or(path);
    let parts: Vec<_> = path.split('/').collect();
    if path.is_empty()
        || path.contains(['\\', '\0', ':'])
        || parts.len() > MAX_DEPTH
        || parts
            .iter()
            .any(|part| part.is_empty() || *part == "." || *part == ".." || part.len() > 255)
    {
        return Err(invalid("Unsafe or unsupported ZIP entry path"));
    }
    Ok(())
}

/// Bound metadata parsing and detect duplicate names that the ZIP reader would coalesce.
fn zip32_directory(file: &mut File) -> io::Result<(usize, u64)> {
    let length = file.metadata()?.len();
    let tail_len = length.min(65_557) as usize;
    file.seek(SeekFrom::End(-(tail_len as i64)))?;
    let mut tail = vec![0; tail_len];
    file.read_exact(&mut tail)?;
    let result = directory_tail(&tail, length)?;
    file.rewind()?;
    Ok(result)
}

pub(crate) fn directory_tail(tail: &[u8], length: u64) -> io::Result<(usize, u64)> {
    if tail.len() > 65_557 || length < tail.len() as u64 {
        return Err(invalid("Invalid ZIP directory tail size"));
    }
    let tail_len = tail.len();
    for offset in (0..tail_len.saturating_sub(21)).rev() {
        let bytes = &tail[offset..];
        if bytes[..4] != *b"PK\x05\x06" {
            continue;
        }
        let u16_at = |n| u16::from_le_bytes([bytes[n], bytes[n + 1]]);
        let u32_at = |n| u32::from_le_bytes([bytes[n], bytes[n + 1], bytes[n + 2], bytes[n + 3]]);
        if offset + 22 + usize::from(u16_at(20)) != tail_len {
            continue;
        }
        let count = usize::from(u16_at(10));
        let start = u64::from(u32_at(16));
        let size = u64::from(u32_at(12));
        if u16_at(4) != 0
            || u16_at(6) != 0
            || u16_at(8) != u16_at(10)
            || count == 65_535
            || count > MAX_ENTRIES
            || size > 16 * 1024 * 1024
            || start + size != length - tail_len as u64 + offset as u64
        {
            return Err(invalid(
                "Unsupported ZIP directory, ZIP64, multi-disk or metadata limit",
            ));
        }
        return Ok((count, start));
    }
    Err(invalid(
        "ZIP end-of-directory record is missing or truncated",
    ))
}

impl ZipCatalog {
    /// Read a bounded ZIP catalog and validate member paths, sizes and metadata.
    ///
    /// # Errors
    /// Returns errors for unsupported, unsafe, changed or inaccessible archive/destination data.
    pub fn open(path: &Path) -> io::Result<Self> {
        if fs::symlink_metadata(path)?.file_type().is_symlink() {
            return Err(invalid("ZIP source cannot be a symbolic link"));
        }
        ArchiveStamp::read(&fs::symlink_metadata(path)?)?;
        let path = fs::canonicalize(path)?;
        let mut file = File::open(&path)?;
        let stamp = ArchiveStamp::read(&file.metadata()?)?;
        let (count, directory_start) = zip32_directory(&mut file)?;
        let zip = ZipArchive::new(file).map_err(invalid)?;
        if zip.len() != count
            || zip.offset() != 0
            || zip.central_directory_start() != directory_start
        {
            return Err(invalid(
                "Ambiguous or duplicate ZIP entries are unsupported",
            ));
        }
        let mut catalog = Self {
            path,
            stamp,
            members: Vec::new(),
            nodes: Vec::new(),
            by_path: HashMap::new(),
        };
        let mut expanded = 0_u64;
        for index in 0..zip.len() {
            let entry = zip.by_index_data(index).map_err(invalid)?;
            let path = std::str::from_utf8(entry.name_raw())
                .map_err(invalid)?
                .to_owned();
            validate_path(&path)?;
            let directory = path.ends_with('/');
            let mode = entry
                .unix_mode()
                .unwrap_or(if directory { 0o40755 } else { 0o100644 });
            if entry.encrypted()
                || !matches!(
                    entry.compression(),
                    CompressionMethod::Stored | CompressionMethod::Deflated
                )
                || entry.is_symlink()
                || !matches!(mode & 0o170000, 0 | 0o040000 | 0o100000)
                || directory != entry.is_dir()
                || (!directory && mode & 0o170000 == 0o040000)
            {
                return Err(invalid(
                    "Only unencrypted stored/deflated regular ZIP files and folders are supported",
                ));
            }
            expanded = expanded
                .checked_add(entry.size())
                .ok_or_else(|| invalid("ZIP size overflow"))?;
            if entry.size() > MAX_ENTRY_BYTES || expanded > MAX_EXPANDED_BYTES {
                return Err(invalid(
                    "ZIP expanded size exceeds the 512 MiB entry / 2 GiB total limit",
                ));
            }
            let date = entry
                .last_modified()
                .ok_or_else(|| invalid("Invalid ZIP timestamp"))?;
            let modified_ms = chrono::NaiveDate::from_ymd_opt(
                date.year().into(),
                date.month().into(),
                date.day().into(),
            )
            .and_then(|day| {
                day.and_hms_opt(
                    date.hour().into(),
                    date.minute().into(),
                    date.second().into(),
                )
            })
            .ok_or_else(|| invalid("Unsupported ZIP timestamp"))?
            .and_utc()
            .timestamp_millis();
            let member = Member {
                path,
                directory,
                size: entry.size(),
                modified_ms,
                mode,
                index,
            };
            catalog.add_member(&member)?;
            catalog.members.push(member);
        }
        if ArchiveStamp::read(&fs::metadata(&catalog.path)?)? != catalog.stamp {
            return Err(invalid("ZIP changed while reading its directory"));
        }
        Ok(catalog)
    }

    fn add_member(&mut self, member: &Member) -> io::Result<()> {
        let parts: Vec<_> = member.path.trim_end_matches('/').split('/').collect();
        for depth in 1..=parts.len() {
            let path = parts[..depth].join("/");
            let leaf = depth == parts.len();
            let directory = !leaf || member.directory;
            let entry = FileEntry {
                name: parts[depth - 1].to_owned(),
                is_directory: directory,
                is_symlink: false,
                metadata: Some(FileMetadata {
                    size: if leaf { member.size as i64 } else { 0 },
                    modified_ms: member.modified_ms,
                    extension: filename_extension(parts[depth - 1]).map(str::to_owned),
                }),
            };
            if let Some(index) = self.by_path.get(&path).copied() {
                let node = &mut self.nodes[index];
                if !node.entry.is_directory || !directory || (leaf && node.explicit) {
                    return Err(invalid("Conflicting or duplicate ZIP paths"));
                }
                if leaf {
                    node.entry = entry;
                    node.explicit = true;
                }
            } else {
                if self.nodes.len() >= MAX_ENTRIES {
                    return Err(invalid("ZIP folder tree exceeds 20000 entries"));
                }
                self.by_path.insert(path.clone(), self.nodes.len());
                self.nodes.push(Node {
                    path,
                    entry,
                    explicit: leaf,
                });
            }
        }
        Ok(())
    }

    /// Member metadata only; implicit folders have no timestamp/size of their own.
    pub fn explicit_metadata(
        &self,
        prefix: &str,
        name: &str,
    ) -> io::Result<Option<crate::FileMetadata>> {
        if !crate::is_valid_leaf_name(name) {
            return Err(invalid("One ZIP child required"));
        }
        let path = format!("{}{}", prefix, name);
        let node = self
            .by_path
            .get(path.trim_end_matches('/'))
            .and_then(|i| self.nodes.get(*i))
            .ok_or_else(|| invalid("ZIP child missing"))?;
        Ok(node.explicit.then(|| node.entry.metadata.clone()).flatten())
    }
    /// Enumerate direct children of a validated virtual archive folder.
    ///
    /// # Errors
    /// Returns errors for unsupported, unsafe, changed or inaccessible archive/destination data.
    pub fn list(&self, prefix: &str) -> io::Result<Vec<FileEntry>> {
        let prefix = prefix.trim_end_matches('/');
        if !prefix.is_empty() {
            validate_path(prefix)?;
            if !self
                .by_path
                .get(prefix)
                .is_some_and(|index| self.nodes[*index].entry.is_directory)
            {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "ZIP folder does not exist",
                ));
            }
        }
        Ok(self
            .nodes
            .iter()
            .filter(|node| node.path.rsplit_once('/').map_or("", |(parent, _)| parent) == prefix)
            .map(|node| node.entry.clone())
            .collect())
    }
}

/// A reviewed extraction. Archive identity/metadata are checked again before mutation.
#[derive(Clone, Debug)]
pub struct ZipExtractPlan {
    archive: PathBuf,
    prefix: String,
    names: Vec<String>,
    destination: PathBuf,
    #[cfg(unix)]
    destination_identity: (u64, u64),
    /// Reviewed handling of existing destination files.
    pub policy: CopyPolicy,
    expanded_bytes: u64,
    catalog: ZipCatalog,
    indices: Vec<usize>,
}
#[derive(Debug, Default)]
/// Completed extraction counts and failures; cancellation does not roll back prior writes.
pub struct ZipExtractReport {
    /// Whether cooperative cancellation stopped or prevented the remaining work.
    pub cancelled: bool,
    /// Whether this item met the operation success contract; inspect detailed issues separately.
    pub success: bool,
    /// Detailed per-path failures, skips, unsupported cases or cancellation notices.
    pub issues: Vec<CopyIssue>,
    #[doc = "Number of regular ZIP members successfully extracted."]
    pub files_extracted: usize,
    /// Number of enumerated source directories processed.
    pub directories_processed: usize,
}

fn check_target(root: &Path, relative: &str, archive: &Path) -> io::Result<()> {
    let mut path = root.to_owned();
    for component in relative.trim_end_matches('/').split('/') {
        path.push(component);
        match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                #[cfg(unix)]
                let aliased = metadata.is_file()
                    && (metadata.nlink() > 1 || {
                        let source = fs::metadata(archive)?;
                        (metadata.dev(), metadata.ino()) == (source.dev(), source.ino())
                    });
                #[cfg(not(unix))]
                let aliased = false;
                if metadata.file_type().is_symlink() || aliased || path == archive {
                    return Err(invalid(
                        "Extraction cannot traverse links or overwrite an archive/file alias",
                    ));
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
                ) =>
            {
                break;
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

impl ZipExtractPlan {
    /// Borrow the captured archive path used for source revalidation.
    pub fn archive(&self) -> &Path {
        &self.archive
    }
    /// Borrow the normalized virtual folder prefix, empty at the archive root.
    pub fn prefix(&self) -> &str {
        &self.prefix
    }
    /// Borrow the ordered direct-child selection reviewed for extraction.
    pub fn names(&self) -> &[String] {
        &self.names
    }
    /// Borrow the canonical extraction destination captured by the plan.
    pub fn destination(&self) -> &Path {
        &self.destination
    }
    /// Return the validated sum of expanded member bytes before extraction.
    pub fn expanded_bytes(&self) -> u64 {
        self.expanded_bytes
    }
    /// Capture an ordered extraction plan with source and destination validation.
    ///
    /// # Errors
    /// Returns errors for unsupported, unsafe, changed or inaccessible archive/destination data.
    pub fn new(
        archive: &Path,
        prefix: &str,
        selected: Option<&[String]>,
        destination: &Path,
        policy: CopyPolicy,
    ) -> io::Result<Self> {
        let catalog = ZipCatalog::open(archive)?;
        if !prefix.is_empty() {
            validate_path(prefix)?;
        }
        let prefix = if prefix.is_empty() {
            String::new()
        } else {
            format!("{}/", prefix.trim_end_matches('/'))
        };
        let children = catalog.list(&prefix)?;
        let names = selected.map_or_else(
            || children.iter().map(|entry| entry.name.clone()).collect(),
            <[String]>::to_vec,
        );
        let mut distinct = HashSet::new();
        if names.iter().any(|name| {
            !crate::is_valid_leaf_name(name)
                || !distinct.insert(name.clone())
                || !children.iter().any(|entry| entry.name == *name)
        }) {
            return Err(invalid(
                "ZIP selection must contain distinct direct children of the current folder",
            ));
        }
        if fs::symlink_metadata(destination)?.file_type().is_symlink() || !destination.is_dir() {
            return Err(invalid(
                "Extraction requires an existing local destination folder",
            ));
        }
        let destination = fs::canonicalize(destination)?;
        #[cfg(unix)]
        let destination_metadata = fs::metadata(&destination)?;
        let mut indices = Vec::new();
        let mut expanded_bytes = 0;
        for (index, entry) in catalog.members.iter().enumerate() {
            if let Some(relative) = entry.path.strip_prefix(&prefix)
                && names
                    .iter()
                    .any(|name| relative == name || relative.starts_with(&format!("{name}/")))
            {
                check_target(&destination, relative, &catalog.path)?;
                expanded_bytes += entry.size;
                indices.push(index);
            }
        }
        Ok(Self {
            archive: catalog.path.clone(),
            prefix,
            names,
            destination,
            #[cfg(unix)]
            destination_identity: (destination_metadata.dev(), destination_metadata.ino()),
            policy,
            expanded_bytes,
            catalog,
            indices,
        })
    }
}

/// Extract the reviewed selection, retaining completed writes after an error.
///
/// # Errors
/// Returns errors for unsupported, unsafe, changed or inaccessible archive/destination data.
pub fn extract_zip(plan: &ZipExtractPlan) -> io::Result<ZipExtractReport> {
    extract_zip_with_control(plan, &JobControl::default())
}

/// Extract with cooperative cancellation and explicit retained-partial reporting.
///
/// # Errors
/// Returns errors for unsupported, unsafe, changed or inaccessible archive/destination data.
pub fn extract_zip_with_control(
    plan: &ZipExtractPlan,
    control: &JobControl,
) -> io::Result<ZipExtractReport> {
    control.total(plan.indices.len());
    if !control.checkpoint() {
        return Ok(ZipExtractReport {
            cancelled: true,
            ..Default::default()
        });
    }
    let source_metadata = fs::symlink_metadata(&plan.archive)?;
    if source_metadata.file_type().is_symlink()
        || ArchiveStamp::read(&source_metadata)? != plan.catalog.stamp
    {
        return Err(invalid(
            "ZIP changed since review; open the extraction dialog again",
        ));
    }
    let file = File::open(&plan.archive)?;
    if ArchiveStamp::read(&file.metadata()?)? != plan.catalog.stamp {
        return Err(invalid(
            "ZIP changed since review; open the extraction dialog again",
        ));
    }
    if !plan.destination.is_dir()
        || fs::symlink_metadata(&plan.destination)?
            .file_type()
            .is_symlink()
    {
        return Err(invalid("Extraction destination changed since review"));
    }
    #[cfg(unix)]
    {
        let metadata = fs::metadata(&plan.destination)?;
        if (metadata.dev(), metadata.ino()) != plan.destination_identity {
            return Err(invalid("Extraction destination was replaced since review"));
        }
    }
    for index in &plan.indices {
        if !control.checkpoint() {
            return Ok(ZipExtractReport {
                cancelled: true,
                ..Default::default()
            });
        }
        let relative = &plan.catalog.members[*index].path[plan.prefix.len()..];
        check_target(&plan.destination, relative, &plan.archive)?;
    }
    let mut zip = ZipArchive::new(file).map_err(invalid)?;
    let mut report = ZipExtractReport {
        success: true,
        ..Default::default()
    };
    for index in &plan.indices {
        let entry = &plan.catalog.members[*index];
        let relative = &entry.path[plan.prefix.len()..];
        let target = plan.destination.join(relative.trim_end_matches('/'));
        if !control.entry(&target, (!entry.directory).then_some(entry.size)) {
            report.success = false;
            break;
        }
        let result = (|| {
            check_target(&plan.destination, relative, &plan.archive)?;
            let existing = fs::metadata(&target).ok();
            if let Some(metadata) = existing.as_ref().filter(|metadata| !metadata.is_dir()) {
                let skip = match plan.policy {
                    CopyPolicy::Skip => true,
                    CopyPolicy::Overwrite => false,
                    CopyPolicy::OverwriteIfOlder => {
                        i128::from(entry.modified_ms) <= crate::copying::modified_ms(Some(metadata))
                    }
                    CopyPolicy::OverwriteIfSizeDiffers => entry.size == metadata.len(),
                };
                if skip {
                    report.issues.push(CopyIssue {
                        path: target.clone(),
                        kind: CopyIssueKind::Skipped,
                        message: "Existing destination skipped by policy".to_owned(),
                    });
                    return Ok(());
                }
            }
            if entry.directory {
                fs::create_dir_all(&target)?;
                report.directories_processed += 1;
                return Ok(());
            }
            // Open the real entry before truncating a destination. Read validates CRC.
            let member = zip.by_index(entry.index).map_err(invalid)?;
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut output = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&target)?;
            let copied = copy_stream(&mut member.take(entry.size + 1), &mut output, control)?;
            if copied != entry.size {
                return Err(invalid("Expanded ZIP data differs from its declared size"));
            }
            let _ =
                output.set_modified(UNIX_EPOCH + Duration::from_millis(entry.modified_ms as u64));
            #[cfg(unix)]
            if let Ok(metadata) = output.metadata() {
                let mode = (metadata.permissions().mode() & 0o077) | (entry.mode & 0o700);
                let _ = output.set_permissions(fs::Permissions::from_mode(mode));
            }
            report.files_extracted += 1;
            Ok::<_, io::Error>(())
        })();
        if let Err(error) = result {
            report.success = false;
            report.issues.push(CopyIssue {
                path: target,
                kind: if control.is_cancelled() {
                    CopyIssueKind::Cancelled
                } else {
                    CopyIssueKind::Failed
                },
                message: error.to_string(),
            });
            break;
        }
        if control.is_cancelled() {
            report.success = false;
            break;
        }
        control.completed();
    }
    report.cancelled = control.is_cancelled();
    if report.cancelled {
        report.success = false;
        if !report
            .issues
            .iter()
            .any(|issue| issue.kind == CopyIssueKind::Cancelled)
        {
            report.issues.push(CopyIssue {
                path: control
                    .snapshot()
                    .current_path
                    .unwrap_or_else(|| plan.destination.clone()),
                kind: CopyIssueKind::Cancelled,
                message: "Extraction cancelled; completed and partial destinations retained"
                    .to_owned(),
            });
        }
    }
    Ok(report)
}
