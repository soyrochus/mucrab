// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

use mucrab_core::{BrowserPane, NameSorter, PaneEvent, SortMode, list_directory};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("mucrab-navigation-{}-{nonce}", std::process::id()));
        fs::create_dir_all(path.join("child")).unwrap();
        fs::write(path.join("file10"), b"ten").unwrap();
        fs::write(path.join("file2"), b"two").unwrap();
        Self(fs::canonicalize(path).unwrap())
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn failed_navigation_retains_last_location_and_panes_are_independent() {
    let fixture = Fixture::new();
    let sorter = NameSorter::en_us().unwrap();
    let mut left = BrowserPane::default();
    let mut right = BrowserPane::default();
    assert!(left.open(&fixture.0, &sorter));
    assert!(right.open(&fixture.0.join("child"), &sorter));
    let previous_entries = left.entries.clone();
    assert!(!left.open(&fixture.0.join("missing"), &sorter));
    assert_eq!(left.location.as_deref(), Some(fixture.0.as_path()));
    assert_eq!(left.entries, previous_entries);
    assert!(left.error.is_some());
    left.options.mode = SortMode::Lexicographic;
    left.options.ascending = false;
    left.resort(&sorter);
    assert_eq!(right.options.mode, SortMode::Natural);
    assert!(right.entries.is_empty());
    assert!(left.open(&fixture.0.join("child"), &sorter));
    assert!(left.error.is_none());
    assert!(left.entries.is_empty());
    left.parent(&sorter);
    assert_eq!(left.location.as_deref(), Some(fixture.0.as_path()));
    assert_eq!(left.entries.len(), 3);
    assert!(left.open(&fixture.0.join("child/.."), &sorter));
    assert_eq!(left.location.as_deref(), Some(fixture.0.as_path()));
    left.parent(&sorter);
    assert_eq!(left.location.as_deref(), fixture.0.parent());
}

#[test]
fn marks_survive_sort_refresh_and_failures_but_remain_pane_local() {
    let fixture = Fixture::new();
    let sorter = NameSorter::en_us().unwrap();
    let mut left = BrowserPane::default();
    let mut right = BrowserPane::default();
    assert!(left.open(&fixture.0, &sorter));
    assert!(right.open(&fixture.0, &sorter));
    let row = left
        .order
        .iter()
        .position(|index| left.entries[*index].name == "file2")
        .unwrap()
        + 1;
    left.apply_event(
        &PaneEvent::Click {
            row,
            ctrl: true,
            shift: false,
        },
        &sorter,
    )
    .unwrap();
    left.options.ascending = false;
    left.resort(&sorter);
    assert_eq!(
        left.selection
            .snapshot(&left.entries, &left.order)
            .focused
            .as_deref(),
        Some("file2")
    );
    assert!(!left.open(&fixture.0.join("absent"), &sorter));
    left.refresh(&sorter);
    let state = left.selection.snapshot(&left.entries, &left.order);
    assert_eq!(state.marked, ["file2"]);
    assert_eq!(state.focused.as_deref(), Some("file2"));
    assert_eq!(right.selection.marked_count(), 0);
    assert_eq!(
        right
            .selection
            .snapshot(&right.entries, &right.order)
            .focused
            .as_deref(),
        Some("..")
    );
    // Removing a selected entry makes refresh choose a valid nearby row and
    // discard the vanished mark, rather than leave a stale index/identity.
    fs::remove_file(fixture.0.join("file2")).unwrap();
    left.refresh(&sorter);
    let state = left.selection.snapshot(&left.entries, &left.order);
    assert!(state.focus_row.unwrap() < state.visible.len());
    assert!(state.marked.is_empty());
}

#[test]
fn folder_changes_clear_marks_and_parent_focuses_departed_folder() {
    let fixture = Fixture::new();
    let sorter = NameSorter::en_us().unwrap();
    let mut pane = BrowserPane::default();
    assert!(pane.open(&fixture.0, &sorter));
    pane.apply_event(&PaneEvent::All, &sorter).unwrap();
    assert!(pane.open(&fixture.0.join("child"), &sorter));
    assert_eq!(pane.selection.marked_count(), 0);
    assert_eq!(
        pane.selection
            .snapshot(&pane.entries, &pane.order)
            .focused
            .as_deref(),
        Some("..")
    );
    pane.parent(&sorter);
    let state = pane.selection.snapshot(&pane.entries, &pane.order);
    assert_eq!(state.focused.as_deref(), Some("child"));
    assert!(state.marked.is_empty());
    // A separate press after Space release can toggle the final row again.
    pane.apply_event(&PaneEvent::Last, &sorter).unwrap();
    pane.apply_event(&PaneEvent::Space, &sorter).unwrap();
    pane.apply_event(&PaneEvent::Space, &sorter).unwrap();
    assert_eq!(pane.selection.marked_count(), 1);
    pane.apply_event(&PaneEvent::Release, &sorter).unwrap();
    pane.apply_event(&PaneEvent::Space, &sorter).unwrap();
    assert_eq!(pane.selection.marked_count(), 0);
}

#[cfg(unix)]
#[test]
fn links_and_dotfiles_are_visible_and_directory_flags_follow_links() {
    let fixture = Fixture::new();
    std::os::unix::fs::symlink("child", fixture.0.join("link")).unwrap();
    std::os::unix::fs::symlink("missing", fixture.0.join("broken")).unwrap();
    fs::write(fixture.0.join(".hidden"), b"hidden").unwrap();
    let entries = list_directory(&fixture.0).unwrap();
    let link = entries.iter().find(|entry| entry.name == "link").unwrap();
    assert!(link.is_directory && link.is_symlink);
    let broken = entries.iter().find(|entry| entry.name == "broken").unwrap();
    assert!(!broken.is_directory && broken.is_symlink);
    assert!(entries.iter().any(|entry| entry.name == ".hidden"));
}
