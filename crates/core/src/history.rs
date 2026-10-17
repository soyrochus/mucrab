// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
use crate::{BrowserPane, NameSorter, ZipLocation};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "location", deny_unknown_fields)]
pub enum PaneLocation {
    Local(PathBuf),
    Zip(ZipLocation),
}
#[derive(Clone, Default)]
pub struct NavigationHistory {
    entries: Vec<PaneLocation>,
    cursor: usize,
}

impl BrowserPane {
    pub fn current_location(&self) -> Option<PaneLocation> {
        self.archive
            .clone()
            .map(PaneLocation::Zip)
            .or_else(|| self.location.clone().map(PaneLocation::Local))
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
    pub fn can_back(&self) -> bool {
        !self.history.entries.is_empty() && self.history.cursor > 0
    }
    pub fn can_forward(&self) -> bool {
        !self.history.entries.is_empty() && self.history.cursor + 1 < self.history.entries.len()
    }
    pub fn history_len(&self) -> usize {
        self.history.entries.len()
    }
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
