// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

//! Committed local/provider browsing state with checked selection and deferred preparation.
use crate::{FileSelection, NameSorter, PaneEvent, SortOptions, ZipCatalog, ZipLocation};
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// One visible Unicode child with independent link identity and target classification.
pub struct FileEntry {
    /// Direct entry name, independent of a display path or target resolution.
    pub name: String,
    /// Whether the entry target is a directory; link identity is reported separately.
    pub is_directory: bool,
    /// Whether the entry itself is a symbolic link, even when its target is missing.
    pub is_symlink: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Optional target metadata used for ordering and display.
    pub metadata: Option<FileMetadata>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Compatibility metadata: byte size and signed UTC epoch milliseconds used by sorting.
pub struct FileMetadata {
    /// File size in bytes; compatibility listing metadata can use -1 for unknown size.
    pub size: i64,
    /// Signed milliseconds relative to the UTC Unix epoch.
    pub modified_ms: i64,
    /// Optional final filename extension, excluding empty or leading-dot suffixes.
    pub extension: Option<String>,
}

impl FileMetadata {
    /// Bounds for terminating metadata sort transitions. Extreme arithmetic
    /// remains available to the comparator, separately from table sorting.
    pub fn is_sortable(&self) -> bool {
        self.size >= -1 && self.modified_ms.unsigned_abs() <= (i64::MAX / 2) as u64
    }
}

/// Borrow the final extension, excluding empty suffixes and leading-dot names.
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

/// Enumerate Unicode local children with metadata and link identity.
///
/// # Errors
/// Returns an error for invalid selection input; filesystem navigation errors are retained in pane state.
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
            // The original macOS directory listing exposes NFC names even when
            // the filesystem preserves a decomposed spelling. APFS resolves
            // either spelling; normalize before extension and ordering rules.
            #[cfg(target_os = "macos")]
            let name = icu_normalizer::ComposingNormalizerBorrowed::new_nfc()
                .normalize(&name)
                .into_owned();
            Ok(FileEntry {
                metadata: Some(local_metadata(&item.path(), &name)?),
                name,
                is_directory: item.path().is_dir(),
                is_symlink: item.file_type()?.is_symlink(),
            })
        })
        .collect()
}

#[derive(Debug, Default)]
/// Committed provider/listing state plus independent queued navigation and presentation options.
pub struct BrowserPane {
    pub(crate) deferred_browsing: bool,
    pub(crate) browse_generation: u64,
    pub(crate) browse_request: Option<crate::BrowseRequest>,
    pub(crate) browse_focus: Option<String>,
    pub(crate) zip_display: Option<PathBuf>,
    pub(crate) committed: Option<crate::PaneLocation>,
    /// Optional pending remote navigation, separate from the committed provider.
    pub remote_request: Option<crate::sftp::RemoteNavigation>,
    /// Whether the committed remote listing is retained after disconnect.
    pub remote_disconnected: bool,
    /// Monotonic remote request identity; stale completions cannot publish.
    pub remote_generation: u64,
    /// Editable display input; only successful navigation changes the committed location.
    pub path_input: String,
    pub(crate) entries: Vec<FileEntry>,
    pub(crate) order: Vec<usize>,
    /// Per-pane ordering options, independent of shared hidden-file visibility.
    pub options: SortOptions,
    /// Optional diagnostic explaining failure or why a field was not attempted.
    pub error: Option<String>,
    pub(crate) selection: FileSelection,
    /// Shared by the desktop application's panes; Unix dotfiles are hidden by default.
    pub show_hidden: bool,
    /// Optional active prefix query, independent of file marks.
    pub quick_find: Option<String>,
    /// Bounded successful navigation context belonging to this pane/tab.
    pub history: crate::NavigationHistory,
}

impl BrowserPane {
    /// Last successfully loaded local path or ZIP display path.
    pub fn location(&self) -> Option<&PathBuf> {
        match &self.committed {
            Some(crate::PaneLocation::Local(path)) => Some(path),
            Some(crate::PaneLocation::Zip(_)) => self.zip_display.as_ref(),
            _ => None,
        }
    }
    /// Committed read-only ZIP folder, if this pane browses an archive.
    pub fn archive(&self) -> Option<&ZipLocation> {
        match &self.committed {
            Some(crate::PaneLocation::Zip(zip)) => Some(zip),
            _ => None,
        }
    }
    /// Committed remote folder, independently of pending navigation.
    pub fn remote(&self) -> Option<&crate::sftp::RemoteLocation> {
        match &self.committed {
            Some(crate::PaneLocation::Sftp(remote)) => Some(remote),
            _ => None,
        }
    }
    /// Visible entry storage in identity order.
    pub fn entries(&self) -> &[FileEntry] {
        &self.entries
    }
    /// Valid permutation of visible entries in display order.
    pub fn order(&self) -> &[usize] {
        &self.order
    }
    /// Read-only focus and marking state.
    pub fn selection(&self) -> &FileSelection {
        &self.selection
    }
    /// Mark a display row using the pane's own validated ordering.
    pub fn mark(&mut self, row: usize, marked: bool) {
        self.selection.set_marked(row, marked, &self.order);
    }
    /// Focus a display row using the pane's own ordering.
    pub fn focus_row(&mut self, row: usize) {
        self.selection.focus(row, &self.order);
    }
    pub(crate) fn commit_location(&mut self, location: crate::PaneLocation) {
        self.zip_display = match &location {
            crate::PaneLocation::Zip(zip) => Some(zip.display_path()),
            _ => None,
        };
        self.committed = Some(location);
    }

    /// Create an empty pane with explicit presentation options and hidden-file policy.
    pub fn with_options(options: SortOptions, show_hidden: bool) -> Self {
        Self {
            options,
            show_hidden,
            ..Default::default()
        }
    }

    /// Open a location synchronously or queue it when desktop deferral is enabled.
    ///
    /// Returns true only for a successful synchronous local/ZIP commit; inspect pending
    /// requests for deferred navigation. Failed preparation preserves committed state.
    pub fn open(&mut self, path: &Path, sorter: &NameSorter) -> bool {
        if let Some(text) = path.to_str()
            && text.contains("://")
        {
            match crate::sftp::RemoteLocation::parse(text) {Ok(loc)=>self.request_remote(loc,true,None),Err(_)=>self.error=Some("Invalid remote location. Use sftp://saved-profile/path; credentials in URLs are not accepted.".into())};
            self.path_input = self
                .current_location()
                .map(|l| match l {
                    crate::PaneLocation::Local(p) => p.display().to_string(),
                    crate::PaneLocation::Zip(z) => z.display_path().display().to_string(),
                    crate::PaneLocation::Sftp(r) => r.display(),
                })
                .unwrap_or_default();
            return false;
        }
        if self.deferred_browsing {
            self.queue_browse(path.to_owned(), true, None);
            return false;
        }
        if !self.open_unrecorded(path, sorter) {
            return false;
        }
        self.quick_find = None;
        self.record_location();
        true
    }
    pub(crate) fn open_unrecorded(&mut self, path: &Path, sorter: &NameSorter) -> bool {
        if self.deferred_browsing {
            self.queue_browse(path.to_owned(), false, None);
            return false;
        }
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
                if let Some(zip) = archive {
                    self.commit_location(crate::PaneLocation::Zip(zip));
                }
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
        // An explicit caller-supplied commit supersedes pending preparation too.
        self.browse_generation = self.browse_generation.saturating_add(1);
        self.browse_request = None;
        self.remote_request = None;
        self.remote_disconnected = false;
        let old = self.selection.snapshot(&self.entries, &self.order);
        let same_folder = self.location() == Some(&path);
        let returning = self
            .location()
            .map(|p| p.as_path())
            .and_then(|p| p.parent())
            == Some(&path);
        let restore_focus = if same_folder {
            old.focused.filter(|name| name != "..")
        } else if returning {
            self.location()
                .map(|p| p.as_path())
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
        self.commit_location(crate::PaneLocation::Local(path));
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

    /// Change the hidden-file policy and refresh the current provider.
    pub fn set_show_hidden(&mut self, show: bool, sorter: &NameSorter) {
        if self.show_hidden != show {
            self.show_hidden = show;
            self.refresh(sorter);
        }
    }

    /// Reorder visible entries while preserving the focused entry identity.
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

    /// Apply a checked selection or sorting event against the owned listing.
    ///
    /// # Errors
    /// Returns an error for invalid selection input; filesystem navigation errors are retained in pane state.
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

    /// Navigate to the parent of the committed location, preserving provider semantics.
    pub fn parent(&mut self, sorter: &NameSorter) {
        if let Some(loc) = self.remote().cloned() {
            self.request_remote(loc.parent(), true, None);
            return;
        }
        if let Some(parent) = self
            .location()
            .map(|p| p.as_path())
            .and_then(|p| p.parent())
            .map(Path::to_path_buf)
        {
            self.open(&parent, sorter);
        }
    }

    /// Focus a visible identity after a successful single-item rename.
    pub fn focus_name(&mut self, name: &str) -> bool {
        #[cfg(target_os = "macos")]
        let normalized = icu_normalizer::ComposingNormalizerBorrowed::new_nfc().normalize(name);
        #[cfg(target_os = "macos")]
        let name = normalized.as_ref();
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

    /// Reload the committed folder without appending a navigation-history entry.
    pub fn refresh(&mut self, sorter: &NameSorter) {
        if let Some(loc) = self.remote().cloned() {
            self.request_remote(loc, false, None);
            return;
        }
        if let Some(path) = self.location().cloned() {
            self.open_unrecorded(&path, sorter);
            if let Some(query) = self.quick_find.clone() {
                self.find_prefix(&query, None);
            }
        }
    }
}
