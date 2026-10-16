// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

// Recovered from muCommander FileTable, FileTableModel and selection actions.
// See NOTICE.md and evidence/workflows/W02-selection-marking/contract.md.

use crate::{FileEntry, SortCriterion, SortMode};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum PaneEvent {
    Focus {
        row: usize,
    },
    Click {
        row: usize,
        #[serde(default)]
        ctrl: bool,
        #[serde(default)]
        shift: bool,
    },
    Space,
    Release,
    Next,
    Previous,
    First,
    Last,
    MarkNext,
    MarkPrevious,
    All,
    None,
    Invert,
    Sort {
        #[serde(default)]
        criterion: SortCriterion,
        ascending: bool,
        directories_first: bool,
        mode: SortMode,
    },
}

#[derive(Debug, Default)]
pub struct FileSelection {
    pub focus_row: Option<usize>,
    pub has_parent: bool,
    marked: Vec<bool>,
    last_row: usize,
    mark_key_repeated: bool,
    last_row_marked: bool,
}

#[derive(Debug, Serialize)]
pub struct SelectionSnapshot {
    pub visible: Vec<String>,
    pub focus_row: Option<usize>,
    pub focused: Option<String>,
    pub marks: Vec<bool>,
    pub marked: Vec<String>,
    pub selected: Vec<String>,
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

    pub fn new(children: usize, has_parent: bool) -> Self {
        Self {
            focus_row: (children + usize::from(has_parent) > 0).then_some(0),
            has_parent,
            marked: vec![false; children],
            ..Default::default()
        }
    }

    pub fn row_count(&self, order: &[usize]) -> usize {
        order.len() + usize::from(self.has_parent)
    }

    pub fn entry_at(&self, row: usize, order: &[usize]) -> Option<usize> {
        row.checked_sub(usize::from(self.has_parent))
            .and_then(|index| order.get(index).copied())
    }

    pub fn is_marked(&self, row: usize, order: &[usize]) -> bool {
        self.entry_at(row, order)
            .and_then(|index| self.marked.get(index).copied())
            .unwrap_or(false)
    }

    pub fn marked_count(&self) -> usize {
        self.marked.iter().filter(|mark| **mark).count()
    }

    pub fn set_marked(&mut self, row: usize, marked: bool, order: &[usize]) {
        if let Some(index) = self.entry_at(row, order) {
            self.marked[index] = marked;
        }
    }

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
