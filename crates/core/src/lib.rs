// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

//! Recovered W01 name ordering and local browsing, independent of the GUI.

mod archives;
mod batch_renaming;
mod browsing;
mod copying;
mod everyday;
mod history;
mod java_case_data;
mod jobs;
pub use everyday::{DeletePlan, PastePlan, PinnedDirectory};
pub use history::{NavigationHistory, PaneLocation};
mod bookmarks;
mod moving;
mod selection;
mod sorting;
mod tabs;
pub use bookmarks::{Bookmark, BookmarkStore};
pub use tabs::{BrowserTab, TabId, TabStack, TabbedPanes};

pub use archives::{
    ZipCatalog, ZipExtractPlan, ZipExtractReport, ZipLocation, extract_zip,
    extract_zip_with_control,
};
pub use batch_renaming::{
    BatchRenameOptions, BatchRenameOutcome, BatchRenamePlan, BatchRenamePreview, BatchRenameReport,
    BatchRenameRow, apply_batch_rename, apply_batch_rename_with_control,
};
pub use browsing::{BrowserPane, FileEntry, FileMetadata, filename_extension, list_directory};
pub use copying::{
    CopyIssue, CopyIssueKind, CopyOutcome, CopyPolicy, CopyReport, CopyRequest, copy_entries,
    copy_entries_with_control, is_valid_leaf_name,
};
pub use jobs::{JobControl, JobProgress, JobState};
pub use moving::{MoveReport, MoveRequest, move_entries, move_entries_with_control};
pub use selection::{FileSelection, PaneEvent, SelectionSnapshot};
pub use sorting::{NameSorter, SortCriterion, SortMode, SortOptions};

/// Identifies one of the application's two file panes.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum PaneSide {
    #[default]
    Left,
    Right,
}

mod search;
pub use search::{
    SearchHit, SearchKind, SearchLimits, SearchPlan, SearchQuery, SearchReport, search_time_bound,
};

mod properties;
pub use properties::{
    EntryKind, EntryTarget, FolderSizeReport, MetadataPlan, MetadataReport, MetadataStep,
    PropertyRequest, PropertySnapshot, folder_size, folder_size_with_limits, parse_modified_time,
    time_from_millis,
};

mod viewer;
pub use viewer::{
    Preview, PreviewContent, PreviewImage, TextDocument, ViewRequest, ViewerLease, decode_preview,
};
