// SPDX-License-Identifier: GPL-3.0-only
//! Owned browsing requests and results; filesystem preparation has no GUI dependency.
use crate::{BrowserPane, JobControl, NameSorter, PaneLocation, SortOptions};
use std::{
    io,
    path::{Path, PathBuf},
};

/// One requested navigation, including its history commit policy.
#[derive(Clone, Debug)]
pub struct BrowseRequest {
    path: PathBuf,
    generation: u64,
    record: bool,
    cursor: Option<usize>,
    options: SortOptions,
    hidden: bool,
}

/// A complete listing prepared by a worker; never partially installed in a pane.
#[derive(Debug)]
pub struct PreparedBrowse {
    request: BrowseRequest,
    pane: BrowserPane,
}

impl BrowseRequest {
    /// Requested path, which is not necessarily canonical until preparation.
    pub fn path(&self) -> &Path {
        &self.path
    }
    /// Monotonic identity within its originating tab.
    pub fn generation(&self) -> u64 {
        self.generation
    }
    /// Prepare canonical location, metadata and sorting on the calling worker.
    ///
    /// # Errors
    /// Reports cancellation, invalid archives, inaccessible paths and listing failures.
    pub fn prepare(self, control: &JobControl) -> io::Result<PreparedBrowse> {
        if control.is_cancelled() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Browse cancelled",
            ));
        }
        let sorter = NameSorter::en_us().map_err(io::Error::other)?;
        let mut pane = BrowserPane {
            options: self.options,
            show_hidden: self.hidden,
            ..Default::default()
        };
        if let Some(zip) = crate::ZipLocation::resolve(&self.path)? {
            let entries = crate::ZipCatalog::open(&zip.archive)?.list(&zip.prefix)?;
            let path = zip.display_path();
            let has_parent = path.parent().is_some();
            pane.load_listing(path, entries, has_parent, &sorter);
            pane.commit_location(PaneLocation::Zip(zip));
        } else {
            let path = std::fs::canonicalize(&self.path)?;
            let entries = crate::list_directory(&path)?;
            let has_parent = path.parent().is_some();
            pane.load_listing(path, entries, has_parent, &sorter);
        }
        if control.is_cancelled() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Browse cancelled",
            ));
        }
        Ok(PreparedBrowse {
            request: self,
            pane,
        })
    }
}

impl BrowserPane {
    /// Enable queued navigation for a desktop-owned pane. Headless callers stay synchronous.
    pub fn defer_browsing(&mut self) {
        self.deferred_browsing = true;
    }
    /// Whether the desktop must prepare a listing outside the event loop.
    pub fn browsing_deferred(&self) -> bool {
        self.deferred_browsing
    }
    pub(crate) fn queue_browse(&mut self, path: PathBuf, record: bool, cursor: Option<usize>) {
        let Some(generation) = self.browse_generation.checked_add(1) else {
            self.error = Some("Browse request identities exhausted".into());
            return;
        };
        self.browse_generation = generation;
        self.remote_request = None;
        self.browse_request = Some(BrowseRequest {
            path,
            generation,
            record,
            cursor,
            options: self.options,
            hidden: self.show_hidden,
        });
        self.error = None;
    }
    /// Take the latest request for scheduling; the generation remains attached to the pane.
    pub fn take_browse_request(&mut self) -> Option<BrowseRequest> {
        self.browse_request.take()
    }
    /// Generation currently requested, used to discard stale results and errors.
    pub fn browse_generation(&self) -> u64 {
        self.browse_generation
    }
    /// Publish a complete prepared listing only if its request and options are still current.
    /// Returns false for superseded results. Changed view options queue a fresh preparation.
    pub fn commit_browse(&mut self, prepared: PreparedBrowse) -> bool {
        let PreparedBrowse { request, mut pane } = prepared;
        if request.generation != self.browse_generation {
            return false;
        }
        if request.options != self.options || request.hidden != self.show_hidden {
            self.queue_browse(request.path, request.record, request.cursor);
            return false;
        }
        // Preserve selection/history from the latest committed state, while moving
        // enumeration and the potentially expensive sorting into preparation.
        let old = self.selection.snapshot(&self.entries, &self.order);
        let same = self.current_location() == pane.current_location();
        let returning = self
            .location()
            .map(|p| p.as_path())
            .and_then(|p| p.parent())
            == pane.location().map(|p| p.as_path());
        let returned_name = returning
            .then(|| {
                self.location()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .map(str::to_owned)
            })
            .flatten();
        if same {
            if let Some(name) = old.focused
                && !pane.focus_name(&name)
            {
                let count = pane.selection.row_count(&pane.order);
                if count > 0 {
                    pane.focus_row(old.focus_row.unwrap_or(0).min(count - 1));
                }
            }
            for (row, index) in pane.order.iter().enumerate() {
                if old.marked.contains(&pane.entries[*index].name) {
                    pane.selection.set_marked(
                        row + usize::from(pane.selection.has_parent),
                        true,
                        &pane.order,
                    );
                }
            }
        } else if let Some(name) = returned_name {
            pane.focus_name(&name);
        }
        if let Some(location) = pane.current_location() {
            self.commit_location(location);
        }
        self.remote_request = None;
        self.remote_disconnected = false;
        self.path_input = pane.path_input;
        self.entries = pane.entries;
        self.order = pane.order;
        self.selection = pane.selection;
        self.error = None;
        if let Some(cursor) = request.cursor {
            self.commit_history_cursor(cursor);
        } else if request.record {
            self.quick_find = None;
            self.record_location();
        }
        if let Some(query) = self.quick_find.clone() {
            self.find_prefix(&query, None);
        }
        if let Some(name) = self.browse_focus.take() {
            self.focus_name(&name);
        }
        true
    }
    /// Restore a renamed/result entry after the next successful listing commit.
    pub fn focus_after_browse(&mut self, name: &str) {
        if self.deferred_browsing {
            self.browse_focus = Some(name.into());
        }
        self.focus_name(name);
    }
    /// Queue a structured location without doing filesystem work.
    pub(crate) fn queue_location(
        &mut self,
        location: &PaneLocation,
        record: bool,
        cursor: Option<usize>,
    ) {
        match location {
            PaneLocation::Local(path) => self.queue_browse(path.clone(), record, cursor),
            PaneLocation::Zip(zip) => self.queue_browse(zip.display_path(), record, cursor),
            PaneLocation::Sftp(remote) => self.request_remote(remote.clone(), record, cursor),
        }
    }
}
