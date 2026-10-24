// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

// Recovered from muCommander FileTable, FileTableModel and selection actions.
// See NOTICE.md and evidence/workflows/W02-selection-marking/contract.md.

//! Focus and marks tracked independently of file-table display ordering.
use crate::{FileEntry, SortCriterion, SortMode};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
/// A checked selection or ordering action independent of mouse/keyboard presentation.
pub enum PaneEvent {
    #[doc = "Move focus to a checked display row without changing marks."]
    Focus {
        #[doc = "Checked display row; the optional parent row is not a markable child."]
        row: usize,
    },
    #[doc = "Apply ordinary, toggle or range-selection pointer semantics to one row."]
    Click {
        #[doc = "Checked display row; the optional parent row is not a markable child."]
        row: usize,
        #[serde(default)]
        #[doc = "Toggle one child mark without clearing unrelated marks."]
        ctrl: bool,
        #[serde(default)]
        #[doc = "Extend a mark range from the retained focus anchor."]
        shift: bool,
    },
    #[doc = "Toggle the focused child mark and advance under the held-key guard."]
    Space,
    #[doc = "Reset the held mark-key guard after key release."]
    Release,
    #[doc = "Move focus to the next display row within bounds."]
    Next,
    #[doc = "Move focus to the previous display row within bounds."]
    Previous,
    #[doc = "Move focus to the first available display row."]
    First,
    #[doc = "Move focus to the last available display row."]
    Last,
    #[doc = "Extend marking toward the next display row."]
    MarkNext,
    #[doc = "Extend marking toward the previous display row."]
    MarkPrevious,
    #[doc = "Mark every child identity, excluding the parent row."]
    All,
    #[doc = "Clear all child marks without replacing the focused entry."]
    None,
    #[doc = "Invert every child mark without marking the parent row."]
    Invert,
    #[doc = "Apply explicit ordering choices while retaining focused and marked identities."]
    Sort {
        #[serde(default)]
        #[doc = "Primary name or metadata ordering field."]
        criterion: SortCriterion,
        #[doc = "Apply ascending direction after directory-first handling."]
        ascending: bool,
        #[doc = "Keep directory targets ahead of other entries independently of direction."]
        directories_first: bool,
        #[doc = "Natural or locale-lexical name comparison."]
        mode: SortMode,
    },
}

#[derive(Debug, Default)]
/// Focus and marks indexed by child identity, with an optional non-markable parent row.
pub struct FileSelection {
    /// Optional focused display row, separate from marked child identities.
    pub focus_row: Option<usize>,
    /// Whether display row zero represents a non-markable parent entry.
    pub has_parent: bool,
    marked: Vec<bool>,
    last_row: usize,
    mark_key_repeated: bool,
    last_row_marked: bool,
}

#[derive(Debug, Serialize)]
/// Owned visible, focused, marked and effective selected names for comparison or display.
pub struct SelectionSnapshot {
    /// Owned entry names in current display order, including the optional parent row.
    pub visible: Vec<String>,
    /// Optional focused display row, separate from marked child identities.
    pub focus_row: Option<usize>,
    /// Owned focused display name, if a valid row is focused.
    pub focused: Option<String>,
    /// Mark flags aligned with the visible display rows.
    pub marks: Vec<bool>,
    /// Owned names of marked child entries in display order.
    pub marked: Vec<String>,
    /// Effective operation selection: marked children, otherwise the focused child.
    pub selected: Vec<String>,
    /// Number of marked child entries, excluding the parent row.
    pub marked_count: usize,
}

impl FileSelection {
    /// Original reload preserves held-key guards and sets the range anchor to
    /// the resulting focus before restoring surviving marks.
    pub(crate) fn reload(&mut self, children: usize, has_parent: bool, focus: Option<usize>) {
        self.marked = vec![false; children];
        self.has_parent = has_parent;
        self.focus_row = focus;
        self.last_row = focus.unwrap_or(0);
    }

    /// Allocate unmarked child state and focus the first available display row.
    pub fn new(children: usize, has_parent: bool) -> Self {
        Self {
            focus_row: (children + usize::from(has_parent) > 0).then_some(0),
            has_parent,
            marked: vec![false; children],
            ..Default::default()
        }
    }

    /// Count displayed children and the optional non-markable parent row.
    pub fn row_count(&self, order: &[usize]) -> usize {
        order.len() + usize::from(self.has_parent)
    }

    /// Resolve a display row to an optional child index through the supplied order.
    pub fn entry_at(&self, row: usize, order: &[usize]) -> Option<usize> {
        row.checked_sub(usize::from(self.has_parent))
            .and_then(|index| order.get(index).copied())
    }

    /// Inspect a display row without marking the optional parent entry.
    pub fn is_marked(&self, row: usize, order: &[usize]) -> bool {
        self.entry_at(row, order)
            .and_then(|index| self.marked.get(index).copied())
            .unwrap_or(false)
    }

    /// Count marked child identities independently of display ordering.
    pub fn marked_count(&self) -> usize {
        self.marked.iter().filter(|mark| **mark).count()
    }

    /// Set a display row mark; invalid rows or child indices are ignored.
    pub fn set_marked(&mut self, row: usize, marked: bool, order: &[usize]) {
        if let Some(index) = self.entry_at(row, order)
            && let Some(slot) = self.marked.get_mut(index)
        {
            *slot = marked;
        }
    }

    /// Focus a valid display row and retain the prior row as range anchor.
    pub fn focus(&mut self, row: usize, order: &[usize]) {
        if row < self.row_count(order) {
            self.last_row = self.focus_row.unwrap_or(0);
            self.focus_row = Some(row);
        }
    }

    fn range(&mut self, start: usize, end: usize, marked: bool, order: &[usize]) {
        for row in start.min(end)..=start.max(end) {
            self.set_marked(row, marked, order);
        }
    }

    pub(crate) fn apply(&mut self, event: &PaneEvent, order: &[usize]) -> Result<(), &'static str> {
        if let PaneEvent::Focus { row } | PaneEvent::Click { row, .. } = event
            && *row >= self.row_count(order)
        {
            return Err("Invalid selection row");
        }
        let Some(current) = self.focus_row else {
            return Ok(());
        };
        let last = self.row_count(order) - 1;
        match *event {
            PaneEvent::Focus { row } => self.focus(row, order),
            PaneEvent::Click { row, ctrl, shift } => {
                self.focus(row, order);
                if shift {
                    self.range(row, self.last_row, !self.is_marked(row, order), order);
                } else if ctrl {
                    self.set_marked(row, !self.is_marked(row, order), order);
                }
            }
            PaneEvent::Space => {
                if self.mark_key_repeated && self.last_row_marked {
                    return Ok(());
                }
                self.set_marked(current, !self.is_marked(current, order), order);
                if current != last {
                    self.focus(current + 1, order);
                } else if !self.last_row_marked {
                    self.last_row_marked = true;
                }
                self.mark_key_repeated = true;
            }
            PaneEvent::Release => {
                self.mark_key_repeated = false;
                self.last_row_marked = false;
            }
            PaneEvent::Next => self.focus((current + 1).min(last), order),
            PaneEvent::Previous => self.focus(current.saturating_sub(1), order),
            PaneEvent::First => self.focus(0, order),
            PaneEvent::Last => self.focus(last, order),
            PaneEvent::MarkNext | PaneEvent::MarkPrevious => {
                self.set_marked(current, !self.is_marked(current, order), order);
                let next = if matches!(event, PaneEvent::MarkNext) {
                    (current + 1).min(last)
                } else {
                    current.saturating_sub(1)
                };
                self.focus(next, order);
            }
            PaneEvent::All => self.marked.fill(true),
            PaneEvent::None => self.marked.fill(false),
            PaneEvent::Invert => self.marked.iter_mut().for_each(|mark| *mark = !*mark),
            PaneEvent::Sort { .. } => return Err("Sorting requires the pane"),
        }
        Ok(())
    }

    /// Capture visible names, focus and effective selection using a matching listing/order.
    ///
    /// # Panics
    /// Panics if the supplied order indexes outside `entries`. Pane accessors provide
    /// a matching validated order; standalone callers must preserve that relationship.
    pub fn snapshot(&self, entries: &[FileEntry], order: &[usize]) -> SelectionSnapshot {
        let mut visible = Vec::with_capacity(self.row_count(order));
        if self.has_parent {
            visible.push("..".to_string());
        }
        visible.extend(order.iter().map(|index| entries[*index].name.clone()));
        let marks: Vec<_> = (0..visible.len())
            .map(|row| self.is_marked(row, order))
            .collect();
        let marked: Vec<_> = visible
            .iter()
            .zip(&marks)
            .filter(|(_, mark)| **mark)
            .map(|(name, _)| name.clone())
            .collect();
        let focused = self.focus_row.and_then(|row| visible.get(row)).cloned();
        let selected = if marked.is_empty() {
            self.focus_row
                .and_then(|row| self.entry_at(row, order))
                .map(|index| vec![entries[index].name.clone()])
                .unwrap_or_default()
        } else {
            marked.clone()
        };
        SelectionSnapshot {
            visible,
            focus_row: self.focus_row,
            focused,
            marks,
            marked_count: marked.len(),
            marked,
            selected,
        }
    }
}
