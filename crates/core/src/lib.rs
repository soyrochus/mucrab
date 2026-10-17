// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

//! Recovered W01 name ordering and local browsing, independent of the GUI.

mod archives;
mod batch_renaming;
mod browsing;
mod copying;
mod java_case_data;
mod moving;
mod selection;
mod sorting;

pub use archives::{ZipCatalog, ZipExtractPlan, ZipExtractReport, ZipLocation, extract_zip};
pub use batch_renaming::{
    BatchRenameOptions, BatchRenameOutcome, BatchRenamePlan, BatchRenamePreview, BatchRenameReport,
    BatchRenameRow, apply_batch_rename,
};
pub use browsing::{BrowserPane, FileEntry, FileMetadata, filename_extension, list_directory};
pub use copying::{
    CopyIssue, CopyIssueKind, CopyOutcome, CopyPolicy, CopyReport, CopyRequest, copy_entries,
    is_valid_leaf_name,
};
pub use moving::{MoveReport, MoveRequest, move_entries};
pub use selection::{FileSelection, PaneEvent, SelectionSnapshot};
pub use sorting::{NameSorter, SortCriterion, SortMode, SortOptions};

/// Identifies one of the application's two file panes.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PaneSide {
    #[default]
    Left,
    Right,
}
