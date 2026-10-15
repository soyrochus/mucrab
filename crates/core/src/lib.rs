// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

//! Recovered W01 name ordering and local browsing, independent of the GUI.

mod browsing;
mod sorting;

pub use browsing::{BrowserPane, FileEntry, list_directory};
pub use sorting::{NameSorter, SortMode, SortOptions};

/// Identifies one of the application's two file panes.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PaneSide {
    #[default]
    Left,
    Right,
}
