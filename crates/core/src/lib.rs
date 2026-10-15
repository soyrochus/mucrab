// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

//! Framework-independent file-management state.
//!
//! This scaffold contains pane identity only. Recovered filesystem operations
//! and sorting semantics will be introduced with binary comparison evidence.

/// Identifies one of the application's two file panes.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PaneSide {
    #[default]
    Left,
    Right,
}
