// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
use crate::{PaneLocation, is_valid_leaf_name};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bookmark {
    pub id: u64,
    pub label: String,
    pub location: PaneLocation,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    version: u32,
    next_id: u64,
    entries: Vec<Bookmark>,
}
impl Default for Document {
    fn default() -> Self {
        Self {
            version: 1,
            next_id: 1,
            entries: Vec::new(),
        }
    }
}
pub struct BookmarkStore {
    path: PathBuf,
    document: Document,
    baseline: Option<Vec<u8>>,
}
fn invalid(message: impl ToString) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.to_string())
}
fn read(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
        Ok(m) => {
            if !m.is_file() || m.len() > 1024 * 1024 {
                return Err(invalid(
                    "Bookmark file must be a regular file smaller than 1 MiB",
                ));
            }
            let mut bytes = Vec::new();
            fs::File::open(path)?
                .take(1024 * 1024 + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() > 1024 * 1024 {
                return Err(invalid("Bookmark file exceeds 1 MiB"));
            }
            Ok(Some(bytes))
        }
    }
}
fn validate(d: &Document) -> io::Result<()> {
    if d.version != 1 || d.entries.len() > 256 || d.next_id == 0 {
        return Err(invalid("Unsupported bookmark version or count"));
    }
    let mut ids = HashSet::new();
    for (i, e) in d.entries.iter().enumerate() {
        if e.id == 0
            || e.id >= d.next_id
            || !ids.insert(e.id)
            || e.label.trim().is_empty()
            || e.label.chars().count() > 128
            || e.label.chars().any(char::is_control)
        {
            return Err(invalid("Invalid bookmark identity or label"));
        }
        match &e.location {
            PaneLocation::Sftp(loc) => {
                if crate::sftp::RemoteLocation::new(&loc.profile, &loc.path)? != *loc {
                    return Err(invalid("Invalid remote bookmark"));
                }
            }
            PaneLocation::Local(p) => {
                if !valid_path(p) {
                    return Err(invalid("Absolute local bookmark path required"));
                }
            }
            PaneLocation::Zip(z) => {
                if !valid_path(&z.archive)
                    || (!z.prefix.is_empty()
                        && (!z.prefix.ends_with('/')
                            || z.prefix
                                .strip_suffix('/')
                                .unwrap_or(&z.prefix)
                                .split('/')
                                .any(|p| !is_valid_leaf_name(p))))
                {
                    return Err(invalid("Invalid ZIP bookmark location"));
                }
            }
        }
        if d.entries[..i]
            .iter()
            .any(|old| old.label == e.label && old.location == e.location)
        {
            return Err(invalid("This location and label are already bookmarked"));
        }
    }
    Ok(())
}
fn valid_path(p: &Path) -> bool {
    p.is_absolute() && p.to_str().is_some_and(|s| !s.contains('\0'))
}
impl BookmarkStore {
    pub fn load(path: PathBuf) -> io::Result<Self> {
        let baseline = read(&path)?;
        let document = match &baseline {
            Some(b) => serde_json::from_slice(b).map_err(invalid)?,
            None => Document::default(),
        };
        validate(&document)?;
        Ok(Self {
            path,
            document,
            baseline,
        })
    }
    pub fn entries(&self) -> &[Bookmark] {
        &self.document.entries
    }
    pub fn add(&mut self, label: &str, location: PaneLocation) -> io::Result<u64> {
        let mut d = self.document.clone();
        let id = d.next_id;
        d.next_id = d
            .next_id
            .checked_add(1)
            .ok_or_else(|| invalid("Bookmark IDs exhausted"))?;
        d.entries.push(Bookmark {
            id,
            label: label.trim().into(),
            location,
        });
        self.commit(d)?;
        Ok(id)
    }
    pub fn rename(&mut self, id: u64, label: &str) -> io::Result<()> {
        let mut d = self.document.clone();
        let e = d
            .entries
            .iter_mut()
            .find(|e| e.id == id)
            .ok_or_else(|| invalid("Bookmark no longer exists"))?;
        e.label = label.trim().into();
        self.commit(d)
    }
    pub fn remove(&mut self, id: u64) -> io::Result<()> {
        let mut d = self.document.clone();
        let i = d
            .entries
            .iter()
            .position(|e| e.id == id)
            .ok_or_else(|| invalid("Bookmark no longer exists"))?;
        d.entries.remove(i);
        self.commit(d)
    }
    pub fn reorder(&mut self, id: u64, up: bool) -> io::Result<()> {
        let mut d = self.document.clone();
        let i = d
            .entries
            .iter()
            .position(|e| e.id == id)
            .ok_or_else(|| invalid("Bookmark no longer exists"))?;
        let j = if up {
            i.saturating_sub(1)
        } else {
            (i + 1).min(d.entries.len() - 1)
        };
        d.entries.swap(i, j);
        self.commit(d)
    }
    fn commit(&mut self, d: Document) -> io::Result<()> {
        validate(&d)?;
        if read(&self.path)? != self.baseline {
            return Err(invalid(
                "Bookmark file changed externally; reload before saving",
            ));
        }
        let bytes = serde_json::to_vec_pretty(&d).map_err(invalid)?;
        if bytes.len() > 1024 * 1024 {
            return Err(invalid("Bookmark file exceeds 1 MiB"));
        }
        let parent = self
            .path
            .parent()
            .ok_or_else(|| invalid("Bookmark configuration directory unavailable"))?;
        fs::create_dir_all(parent)?;
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let mut owned = None;
        for _ in 0..100 {
            let p = parent.join(format!(
                ".bookmarks-{}-{}.tmp",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            match options.open(&p) {
                Ok(f) => {
                    owned = Some((p, f));
                    break;
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(e),
            }
        }
        let (temp, mut file) =
            owned.ok_or_else(|| invalid("Cannot create bookmark temporary file"))?;
        let result = (|| {
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            if read(&self.path)? != self.baseline {
                return Err(invalid("Bookmark file changed during save"));
            }
            fs::rename(&temp, &self.path)
        })();
        if let Err(e) = result {
            let _ = fs::remove_file(temp);
            return Err(e);
        }
        self.document = d;
        self.baseline = Some(bytes);
        Ok(())
    }
}
