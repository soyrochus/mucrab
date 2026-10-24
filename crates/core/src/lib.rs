// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

//! File-management state, validated operation plans and bounded read preparation.
//!
//! This crate is independent of egui. Local, ZIP and SFTP operations expose owned
//! results, cooperative cancellation and explicit partial-failure reports.
//! Destructive operations assume stable, owned filesystem trees; identity checks
//! reject known changes but do not establish a race-proof filesystem sandbox.
//!
//! ```
//! use mucrab_core::sftp::RemoteLocation;
//! let folder = RemoteLocation::new("saved-server", "/documents/../photos")?;
//! assert_eq!(folder.path(), "/photos");
//! assert_eq!(folder.parent().path(), "/");
//! # Ok::<(), std::io::Error>(())
//! ```

mod archives;
mod batch_renaming;
mod browse_request;
mod browsing;
mod capture_error;
pub use browse_request::{BrowseRequest, PreparedBrowse};
pub use capture_error::CaptureChanged;
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
    #[doc = "The fixed left-hand file pane."]
    Left,
    #[doc = "The fixed right-hand file pane."]
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

pub mod sftp;

/// Side-effect-free, bounded parser entry points for fuzz and regression tooling.
#[cfg(feature = "test-util")]
pub mod fuzzing;
