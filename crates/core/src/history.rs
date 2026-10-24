// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
//! Successful provider navigation history and generation-checked remote publication.
use crate::{BrowserPane, NameSorter, ZipLocation};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "location", deny_unknown_fields)]
/// One committed provider identity: local, virtual ZIP, or saved-profile SFTP.
pub enum PaneLocation {
    #[doc = "An existing local filesystem path."]
    Local(PathBuf),
    #[doc = "A virtual folder or member within a local read-only ZIP."]
    Zip(ZipLocation),
    #[doc = "A normalized path associated with a saved SFTP profile."]
    Sftp(crate::sftp::RemoteLocation),
}
#[derive(Clone, Debug, Default)]
/// Bounded successful location history with a cursor independent of pending navigation.
pub struct NavigationHistory {
    entries: Vec<PaneLocation>,
    cursor: usize,
}

impl BrowserPane {
    /// The one last successfully committed provider and location.
    pub fn current_location(&self) -> Option<PaneLocation> {
        self.committed.clone()
    }
    pub(crate) fn record_location(&mut self) {
        let Some(location) = self.current_location() else {
            return;
        };
        if self.history.entries.get(self.history.cursor) == Some(&location) {
            return;
        }
        self.history
            .entries
            .truncate(if self.history.entries.is_empty() {
                0
            } else {
                self.history.cursor + 1
            });
        self.history.entries.push(location);
        if self.history.entries.len() > 100 {
            self.history.entries.remove(0);
        }
        self.history.cursor = self.history.entries.len() - 1;
    }
    pub(crate) fn commit_history_cursor(&mut self, cursor: usize) {
        if self.history.entries.get(cursor) == self.current_location().as_ref() {
            self.history.cursor = cursor;
        } else {
            self.record_location();
        }
        self.quick_find = None;
    }
    /// Test whether an earlier successfully committed history location exists.
    pub fn can_back(&self) -> bool {
        !self.history.entries.is_empty() && self.history.cursor > 0
    }
    /// Test whether a later successfully committed history location exists.
    pub fn can_forward(&self) -> bool {
        !self.history.entries.is_empty() && self.history.cursor + 1 < self.history.entries.len()
    }
    /// Count retained successful navigation locations, bounded to one hundred entries.
    pub fn history_len(&self) -> usize {
        self.history.entries.len()
    }
    /// Navigate backward or forward without recording a duplicate history entry.
    pub fn traverse_history(&mut self, forward: bool, sorter: &NameSorter) -> bool {
        if if forward {
            !self.can_forward()
        } else {
            !self.can_back()
        } {
            return false;
        }
        let cursor = if forward {
            self.history.cursor + 1
        } else {
            self.history.cursor - 1
        };
        let target = self.history.entries[cursor].clone();
        if let PaneLocation::Sftp(location) = target {
            self.request_remote(location, false, Some(cursor));
            return false;
        }
        if self.deferred_browsing {
            self.queue_location(&target, false, Some(cursor));
            return false;
        }
        let success = self.load_location(&target, sorter);
        if success {
            self.history.cursor = cursor;
            self.quick_find = None;
        }
        success
    }
    /// Case-insensitive Unicode lowercase prefix; no canonical normalization.
    /// None starts at the first match; Some(false/true) moves previous/next.
    pub fn find_prefix(&mut self, query: &str, direction: Option<bool>) -> bool {
        if query.is_empty() {
            return true;
        }
        let query = query.to_lowercase();
        let matches: Vec<usize> = self
            .order
            .iter()
            .enumerate()
            .filter(|(_, i)| self.entries[**i].name.to_lowercase().starts_with(&query))
            .map(|(p, _)| p + usize::from(self.selection.has_parent))
            .collect();
        if matches.is_empty() {
            return false;
        }
        let current = self.selection.focus_row;
        let row = match direction {
            None => matches[0],
            Some(true) => matches
                .iter()
                .copied()
                .find(|r| Some(*r) > current)
                .unwrap_or(matches[0]),
            Some(false) => matches
                .iter()
                .rev()
                .copied()
                .find(|r| Some(*r) < current)
                .unwrap_or(*matches.last().unwrap()),
        };
        self.selection.focus(row, &self.order);
        true
    }
}

impl BrowserPane {
    /// Queue a validated remote location with generation and optional history cursor.
    pub fn request_remote(
        &mut self,
        location: crate::sftp::RemoteLocation,
        record: bool,
        cursor: Option<usize>,
    ) {
        self.browse_generation = self.browse_generation.saturating_add(1);
        self.browse_request = None;
        let Some(generation) = self.remote_generation.checked_add(1) else {
            self.error = Some("Remote navigation generations exhausted".into());
            return;
        };
        self.remote_generation = generation;
        self.remote_request = Some(crate::sftp::RemoteNavigation {
            location,
            generation,
            record,
            cursor,
        });
        self.error = None;
    }
    /// Commit only the currently requested remote generation and restore surviving identities.
    pub fn remote_loaded(
        &mut self,
        request: &crate::sftp::RemoteNavigation,
        mut entries: Vec<crate::FileEntry>,
        sorter: &NameSorter,
    ) -> bool {
        if self.remote_request.as_ref().map(|r| r.generation) != Some(request.generation) {
            return false;
        }
        let same = self.remote() == Some(&request.location);
        let old = self.selection.snapshot(&self.entries, &self.order);
        let previous = self.remote().cloned();
        if !self.show_hidden {
            entries.retain(|e| !e.name.starts_with('.'));
        }
        self.commit_location(PaneLocation::Sftp(request.location.clone()));
        self.remote_request = None;
        self.remote_disconnected = false;
        self.path_input = request.location.display();
        self.entries = entries;
        self.order = sorter.sorted_indices(&self.entries, self.options);
        self.selection
            .reload(self.entries.len(), request.location.path() != "/", Some(0));
        if same {
            if let Some(name) = old.focused {
                self.focus_name(&name);
            }
            for name in old.marked {
                if let Some(i) = self
                    .order
                    .iter()
                    .position(|i| self.entries[*i].name == name)
                {
                    self.selection.set_marked(
                        i + usize::from(self.selection.has_parent),
                        true,
                        &self.order,
                    );
                }
            }
        } else if let Some(previous) = previous
            && previous.profile() == request.location.profile()
            && previous.parent() == request.location
            && let Some(name) = previous.path().rsplit('/').next()
        {
            self.focus_name(name);
        }
        self.error = None;
        self.quick_find = None;
        if let Some(cursor) = request.cursor.filter(|i| {
            self.history.entries.get(*i) == Some(&PaneLocation::Sftp(request.location.clone()))
        }) {
            self.history.cursor = cursor;
        } else if request.record || request.cursor.is_some() {
            self.record_location();
        }
        true
    }
}
