// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

use crate::{FileSelection, NameSorter, PaneEvent, SortOptions, ZipCatalog, ZipLocation};
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct FileEntry {
    pub name: String,
    pub is_directory: bool,
    pub is_symlink: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<FileMetadata>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FileMetadata {
    pub size: i64,
    pub modified_ms: i64,
    pub extension: Option<String>,
}

impl FileMetadata {
    /// Bounds for terminating metadata sort transitions. Extreme arithmetic
    /// remains available to the comparator, separately from table sorting.
    pub fn is_sortable(&self) -> bool {
        self.size >= -1 && self.modified_ms.unsigned_abs() <= (i64::MAX / 2) as u64
    }
}

pub fn filename_extension(name: &str) -> Option<&str> {
    let (prefix, extension) = name.rsplit_once('.')?;
    (!prefix.is_empty() && !extension.is_empty()).then_some(extension)
}

fn local_metadata(path: &Path, name: &str) -> io::Result<FileMetadata> {
    let (size, modified_ms) = if let Ok(metadata) = fs::metadata(path) {
        let size = i64::try_from(metadata.len()).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "File size exceeds supported range",
            )
        })?;
        let modified_ms = metadata
            .modified()
            .map(|time| match time.duration_since(std::time::UNIX_EPOCH) {
                Ok(duration) => i64::try_from(duration.as_millis()).unwrap_or(i64::MAX),
                Err(error) => i64::try_from(error.duration().as_nanos().div_ceil(1_000_000))
                    .map_or(i64::MIN, |value| -value),
            })
            .unwrap_or(0);
        (size, modified_ms)
    } else {
        // java.io.File.length/lastModified return zero for unavailable targets.
        (0, 0)
    };
    let result = FileMetadata {
        size,
        modified_ms,
        extension: filename_extension(name).map(str::to_owned),
    };
    if !result.is_sortable() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Modified date exceeds supported sort range",
        ));
    }
    Ok(result)
}

pub fn list_directory(path: &Path) -> io::Result<Vec<FileEntry>> {
    fs::read_dir(path)?
        .map(|item| {
            let item = item?;
            let name = item.file_name().into_string().map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "W01 does not support non-Unicode file names",
                )
            })?;
            Ok(FileEntry {
                metadata: Some(local_metadata(&item.path(), &name)?),
                name,
                is_directory: item.path().is_dir(),
                is_symlink: item.file_type()?.is_symlink(),
            })
        })
        .collect()
}

#[derive(Default)]
pub struct BrowserPane {
    pub location: Option<PathBuf>,
    pub archive: Option<ZipLocation>,
    pub path_input: String,
    pub entries: Vec<FileEntry>,
    pub order: Vec<usize>,
    pub options: SortOptions,
    pub error: Option<String>,
    pub selection: FileSelection,
    /// Shared by the desktop application's panes; Unix dotfiles are hidden by default.
    pub show_hidden: bool,
}

impl BrowserPane {
    pub fn open(&mut self, path: &Path, sorter: &NameSorter) -> bool {
        let result = (|| {
            if let Some(location) = ZipLocation::resolve(path)? {
                let entries = ZipCatalog::open(&location.archive)?.list(&location.prefix)?;
                return Ok((location.display_path(), entries, Some(location)));
            }
            // Store a real location so Parent works after input such as child/..
            // instead of returning to the lexical child component.
            let path = fs::canonicalize(path)?;
            if path.to_str().is_none() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "W01 requires a Unicode path",
                ));
            }
            let entries = list_directory(&path)?;
            Ok((path, entries, None))
        })();
        match result {
            Ok((path, entries, archive)) => {
                let has_parent = path.parent().is_some();
                self.load_listing(path, entries, has_parent, sorter);
                self.archive = archive;
                true
            }
            Err(error) => {
                self.error = Some(error.to_string());
                false
            }
        }
    }

    /// Load an enumerated canonical location, filtering before sorting.
    /// Explicit parent presence also supports the original-binary adapters.
    pub fn load_listing(
        &mut self,
        path: PathBuf,
        mut entries: Vec<FileEntry>,
        has_parent: bool,
        sorter: &NameSorter,
    ) {
        self.archive = None;
        let old = self.selection.snapshot(&self.entries, &self.order);
        let same_folder = self.location.as_ref() == Some(&path);
        let returning = self.location.as_deref().and_then(Path::parent) == Some(&path);
        let restore_focus = if same_folder {
            old.focused.filter(|name| name != "..")
        } else if returning {
            self.location
                .as_deref()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
                .map(str::to_owned)
        } else {
            None
        };
        if !self.show_hidden {
            entries.retain(|entry| !entry.name.starts_with('.'));
        }
        self.path_input = path.to_string_lossy().into_owned();
        self.location = Some(path);
        self.entries = entries;
        self.order = sorter.sorted_indices(&self.entries, self.options);
        let mut focus_row = (self.entries.len() + usize::from(has_parent) > 0).then_some(0);
        if let Some(name) = restore_focus {
            let row = self
                .order
                .iter()
                .position(|index| self.entries[*index].name == name)
                .map(|index| index + usize::from(has_parent))
                .or_else(|| {
                    same_folder.then(|| {
                        old.focus_row
                            .unwrap_or(0)
                            .min((self.order.len() + usize::from(has_parent)).saturating_sub(1))
                    })
                });
            if let Some(row) = row {
                focus_row = Some(row);
            }
        }
        if self.order.is_empty() && !has_parent {
            focus_row = None;
        }
        self.selection
            .reload(self.entries.len(), has_parent, focus_row);
        if same_folder {
            for row in 0..self.selection.row_count(&self.order) {
                if let Some(index) = self.selection.entry_at(row, &self.order)
                    && old.marked.contains(&self.entries[index].name)
                {
                    self.selection.set_marked(row, true, &self.order);
                }
            }
        }
        self.error = None;
    }

    pub fn set_show_hidden(&mut self, show: bool, sorter: &NameSorter) {
        if self.show_hidden != show {
            self.show_hidden = show;
            self.refresh(sorter);
        }
    }

    pub fn resort(&mut self, sorter: &NameSorter) {
        let focused = self
            .selection
            .focus_row
            .and_then(|row| self.selection.entry_at(row, &self.order));
        sorter.sort_indices(&self.entries, self.options, &mut self.order);
        if let Some(index) = focused
            && let Some(position) = self.order.iter().position(|item| *item == index)
        {
            self.selection.focus(
                position + usize::from(self.selection.has_parent),
                &self.order,
            );
        }
    }

    pub fn apply_event(
        &mut self,
        event: &PaneEvent,
        sorter: &NameSorter,
    ) -> Result<(), &'static str> {
        if let PaneEvent::Sort {
            criterion,
            ascending,
            directories_first,
            mode,
        } = *event
        {
            self.options = SortOptions {
                criterion,
                ascending,
                directories_first,
                mode,
            };
            self.resort(sorter);
            Ok(())
        } else {
            self.selection.apply(event, &self.order)
        }
    }

    pub fn parent(&mut self, sorter: &NameSorter) {
        if let Some(parent) = self
            .location
            .as_deref()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
        {
            self.open(&parent, sorter);
        }
    }

    /// Focus a visible identity after a successful single-item rename.
    pub fn focus_name(&mut self, name: &str) -> bool {
        if let Some(position) = self
            .order
            .iter()
            .position(|index| self.entries[*index].name == name)
        {
            self.selection.focus(
                position + usize::from(self.selection.has_parent),
                &self.order,
            );
            true
        } else {
            false
        }
    }

    /// Original FileJob unmarks only top-level true outcomes. Detailed child
    /// errors belong to the copy report, because directories may return true.
    pub fn unmark_copy_outcomes(&mut self, outcomes: &[crate::CopyOutcome]) {
        for outcome in outcomes.iter().filter(|outcome| outcome.success) {
            if let Some(position) = self
                .order
                .iter()
                .position(|index| self.entries[*index].name == outcome.name)
            {
                self.selection.set_marked(
                    position + usize::from(self.selection.has_parent),
                    false,
                    &self.order,
                );
            }
        }
    }

    pub fn refresh(&mut self, sorter: &NameSorter) {
        if let Some(path) = self.location.clone() {
            self.open(&path, sorter);
        }
    }
}
