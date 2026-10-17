// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

mod support;

use mucrab_core::{BrowserPane, NameSorter, PaneEvent, SortMode, list_directory};
use std::{fs, path::PathBuf};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let nonce = support::nonce();
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

#[test]
fn hidden_filter_discards_marks_and_falls_back_without_resurrecting_selection() {
    let fixture = Fixture::new();
    fs::write(fixture.0.join(".hidden"), b"hidden").unwrap();
    let sorter = NameSorter::en_us().unwrap();
    let mut pane = BrowserPane {
        show_hidden: true,
        ..Default::default()
    };
    assert!(pane.open(&fixture.0, &sorter));
    let hidden_row = pane
        .order
        .iter()
        .position(|i| pane.entries[*i].name == ".hidden")
        .unwrap()
        + 1;
    pane.apply_event(&PaneEvent::All, &sorter).unwrap();
    pane.apply_event(&PaneEvent::Focus { row: hidden_row }, &sorter)
        .unwrap();
    pane.set_show_hidden(false, &sorter);
    let hidden = pane.selection.snapshot(&pane.entries, &pane.order);
    assert!(
        !hidden
            .visible
            .iter()
            .any(|name| name.starts_with('.') && name != "..")
    );
    assert_eq!(
        hidden.focus_row,
        Some(hidden_row.min(hidden.visible.len() - 1))
    );
    assert_eq!(hidden.marked.len(), 3);
    pane.set_show_hidden(true, &sorter);
    let shown = pane.selection.snapshot(&pane.entries, &pane.order);
    assert!(shown.visible.iter().any(|name| name == ".hidden"));
    assert!(!shown.marked.iter().any(|name| name == ".hidden"));
    assert_eq!(shown.focused, hidden.focused);
    assert_eq!(shown.marked.len(), 3);
    // The original table's held-Space guard survives a filter-triggered reload.
    pane.apply_event(&PaneEvent::None, &sorter).unwrap();
    pane.apply_event(&PaneEvent::Last, &sorter).unwrap();
    pane.apply_event(&PaneEvent::Space, &sorter).unwrap();
    pane.set_show_hidden(false, &sorter);
    pane.apply_event(&PaneEvent::Space, &sorter).unwrap();
    assert_eq!(pane.selection.marked_count(), 1);
    pane.apply_event(&PaneEvent::Release, &sorter).unwrap();
    pane.apply_event(&PaneEvent::Space, &sorter).unwrap();
    assert_eq!(pane.selection.marked_count(), 0);
}

#[cfg(unix)]
#[test]
fn hidden_setting_follows_navigation_and_classifies_links_by_their_own_names() {
    let fixture = Fixture::new();
    fs::write(fixture.0.join(".hidden"), b"hidden").unwrap();
    fs::write(fixture.0.join("child/.child-hidden"), b"child").unwrap();
    std::os::unix::fs::symlink(".hidden", fixture.0.join("visible-link")).unwrap();
    std::os::unix::fs::symlink("missing", fixture.0.join(".hidden-broken-link")).unwrap();
    let sorter = NameSorter::en_us().unwrap();
    let mut pane = BrowserPane::default();
    assert!(pane.open(&fixture.0, &sorter));
    assert!(
        pane.entries
            .iter()
            .any(|entry| entry.name == "visible-link")
    );
    assert!(!pane.entries.iter().any(|entry| entry.name.starts_with('.')));
    pane.set_show_hidden(true, &sorter);
    assert!(
        pane.entries
            .iter()
            .any(|entry| entry.name == ".hidden-broken-link")
    );
    assert!(pane.open(&fixture.0.join("child"), &sorter));
    assert_eq!(pane.entries[0].name, ".child-hidden");
    pane.set_show_hidden(false, &sorter);
    assert!(pane.entries.is_empty());
    assert_eq!(
        pane.selection
            .snapshot(&pane.entries, &pane.order)
            .focused
            .as_deref(),
        Some("..")
    );
    assert!(!pane.open(&fixture.0.join("absent"), &sorter));
    assert!(pane.entries.is_empty());
    pane.parent(&sorter);
    assert!(!pane.show_hidden);
    assert!(!pane.entries.iter().any(|entry| entry.name.starts_with('.')));
}

#[cfg(unix)]
#[test]
fn local_metadata_follows_links_but_extensions_use_the_entry_name() {
    use std::time::{Duration, UNIX_EPOCH};
    let fixture = Fixture::new();
    let target = fixture.0.join("target.TxT");
    fs::write(&target, b"four").unwrap();
    fs::File::open(&target)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(UNIX_EPOCH - Duration::from_millis(1)))
        .unwrap();
    std::os::unix::fs::symlink("target.TxT", fixture.0.join("visible.other")).unwrap();
    std::os::unix::fs::symlink("missing", fixture.0.join("broken.zz")).unwrap();
    fs::write(fixture.0.join(".config"), b"").unwrap();
    fs::write(fixture.0.join("trailing."), b"").unwrap();
    let entries = list_directory(&fixture.0).unwrap();
    let metadata = |name: &str| {
        entries
            .iter()
            .find(|e| e.name == name)
            .unwrap()
            .metadata
            .as_ref()
            .unwrap()
    };
    assert_eq!(
        (
            metadata("visible.other").size,
            metadata("visible.other").modified_ms
        ),
        (4, -1)
    );
    assert_eq!(
        metadata("visible.other").extension.as_deref(),
        Some("other")
    );
    assert_eq!(
        (
            metadata("broken.zz").size,
            metadata("broken.zz").modified_ms
        ),
        (0, 0)
    );
    assert_eq!(metadata(".config").extension, None);
    assert_eq!(metadata("trailing.").extension, None);
}

#[test]
fn metadata_sort_refresh_retains_criterion_focus_marks_and_pane_independence() {
    use mucrab_core::SortCriterion;
    let fixture = Fixture::new();
    let sorter = NameSorter::en_us().unwrap();
    let mut left = BrowserPane::default();
    let mut right = BrowserPane::default();
    assert!(left.open(&fixture.0, &sorter));
    assert!(right.open(&fixture.0, &sorter));
    let row = left
        .order
        .iter()
        .position(|i| left.entries[*i].name == "file2")
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
    left.options.criterion = SortCriterion::Size;
    left.resort(&sorter);
    fs::write(fixture.0.join("file2"), vec![0; 1024]).unwrap();
    left.refresh(&sorter);
    let snapshot = left.selection.snapshot(&left.entries, &left.order);
    assert_eq!(snapshot.visible.last().map(String::as_str), Some("file2"));
    assert_eq!(snapshot.focused.as_deref(), Some("file2"));
    assert_eq!(snapshot.marked, ["file2"]);
    left.set_show_hidden(true, &sorter);
    assert_eq!(left.options.criterion, SortCriterion::Size);
    assert_eq!(right.options.criterion, SortCriterion::Name);
}

#[cfg(target_os = "macos")]
#[test]
fn macos_listing_composes_names_before_extension_sorting_and_selection() {
    let f = Fixture::new();
    fs::write(f.0.join("e\u{301}.I\u{307}"), b"unicode name").unwrap();
    let sorter = NameSorter::en_us().unwrap();
    let mut pane = BrowserPane::default();
    assert!(pane.open(&f.0, &sorter));
    let entry = pane
        .entries
        .iter()
        .find(|entry| entry.name == "é.İ")
        .unwrap();
    assert_eq!(
        entry.metadata.as_ref().unwrap().extension.as_deref(),
        Some("İ")
    );
    assert_eq!(fs::read(f.0.join(&entry.name)).unwrap(), b"unicode name");
}
