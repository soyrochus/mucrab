// SPDX-License-Identifier: GPL-3.0-only
//! Behavioural regression coverage for tabs bookmarks using disposable fixtures.
mod support;
use mucrab_core::*;
use std::{fs, path::PathBuf};
struct Tree(PathBuf);
impl Tree {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "mucrab-w11-{}-{}",
            std::process::id(),
            support::nonce()
        ));
        fs::create_dir_all(p.join("child")).unwrap();
        fs::write(p.join("alpha"), b"a").unwrap();
        Self(fs::canonicalize(p).unwrap())
    }
}
impl Drop for Tree {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn pane(tree: &Tree, sorter: &NameSorter) -> BrowserPane {
    let mut p = BrowserPane::default();
    assert!(p.open(&tree.0, sorter));
    p
}
#[test]
fn duplicate_copies_history_view_and_focus_but_isolates_marks_and_navigation() {
    let t = Tree::new();
    let s = NameSorter::en_us().unwrap();
    let mut tabs = TabStack::new(pane(&t, &s));
    let origin = tabs.active().id;
    tabs.active_mut()
        .pane
        .apply_event(&PaneEvent::All, &s)
        .unwrap();
    tabs.active_mut().pane.focus_name("alpha");
    tabs.active_mut().widths = [80., 90., 100.];
    let duplicate = tabs.duplicate(&s).unwrap();
    assert_ne!(origin, duplicate);
    assert_eq!(tabs.active().pane.selection().marked_count(), 0);
    assert_eq!(tabs.active().widths, [80., 90., 100.]);
    assert_eq!(
        tabs.active()
            .pane
            .selection()
            .snapshot(tabs.active().pane.entries(), tabs.active().pane.order())
            .focused
            .as_deref(),
        Some("alpha")
    );
    assert!(tabs.active_mut().pane.open(&t.0.join("child"), &s));
    assert!(tabs.active_mut().pane.can_back());
    tabs.select(origin);
    assert!(!tabs.active().pane.can_back());
    assert_eq!(tabs.active().pane.selection().marked_count(), 2);
    tabs.select(duplicate);
    assert!(tabs.active_mut().pane.traverse_history(false, &s));
    tabs.select(origin);
    assert_eq!(tabs.active().pane.location(), Some(&t.0));
}
#[test]
fn limits_failed_navigation_and_ids() {
    let t = Tree::new();
    let s = NameSorter::en_us().unwrap();
    let mut tabs = TabStack::new(pane(&t, &s));
    let first = tabs.active().id;
    assert!(!tabs.close(first));
    assert!(tabs.add_home(&t.0.join("missing"), false, &s).is_err());
    assert_eq!(tabs.len(), 1);
    for _ in 1..32 {
        tabs.duplicate(&s).unwrap();
    }
    assert!(tabs.duplicate(&s).is_err());
    assert!(tabs.add_home(&t.0, false, &s).is_err());
    let closed = tabs.active().id;
    assert!(tabs.close(closed));
    let fresh = tabs.duplicate(&s).unwrap();
    assert_ne!(closed, fresh);
    assert!(!tabs.select(closed));
}
#[test]
fn bookmark_roundtrip_stable_ids_order_zip_and_missing_targets() {
    let t = Tree::new();
    let path = t.0.join("settings/bookmarks.json");
    let mut b = BookmarkStore::load(path.clone()).unwrap();
    let a = b.add("Home", PaneLocation::Local(t.0.clone())).unwrap();
    let z = b
        .add(
            "Archive",
            PaneLocation::Zip(ZipLocation {
                archive: t.0.join("missing.zip"),
                prefix: "inside/".into(),
            }),
        )
        .unwrap();
    b.rename(a, "Local").unwrap();
    b.reorder(z, true).unwrap();
    drop(b);
    let mut b = BookmarkStore::load(path).unwrap();
    assert_eq!(b.entries()[0].id, z);
    assert_eq!(b.entries()[1].label, "Local");
    assert!(b.add("Local", PaneLocation::Local(t.0.clone())).is_err());
    b.remove(a).unwrap();
    let next = b.add("Local", PaneLocation::Local(t.0.clone())).unwrap();
    assert!(next > z);
    let mut p = pane(&t, &NameSorter::en_us().unwrap());
    assert!(!p.open_location(&b.entries()[0].location, &NameSorter::en_us().unwrap()));
    assert_eq!(p.location(), Some(&t.0));
}
#[test]
fn rejected_data_and_failed_save_preserve_old_file_and_store() {
    let t = Tree::new();
    let path = t.0.join("bookmarks.json");
    let mut b = BookmarkStore::load(path.clone()).unwrap();
    let id = b.add("Valid", PaneLocation::Local(t.0.clone())).unwrap();
    let original = fs::read(&path).unwrap();
    for label in ["", "a\nb", &"x".repeat(129)] {
        assert!(b.rename(id, label).is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
    }
    assert!(
        b.add(
            "bad",
            PaneLocation::Zip(ZipLocation {
                archive: t.0.join("a.zip"),
                prefix: "a//".into()
            })
        )
        .is_err()
    );
    fs::write(&path, b"external invalid data").unwrap();
    assert!(b.rename(id, "Unsaved").is_err());
    assert_eq!(b.entries()[0].label, "Valid");
    assert_eq!(fs::read(&path).unwrap(), b"external invalid data");
    assert!(BookmarkStore::load(path.clone()).is_err());
    let mut doc: serde_json::Value = serde_json::from_slice(&original).unwrap();
    doc["version"] = 2.into();
    fs::write(&path, serde_json::to_vec(&doc).unwrap()).unwrap();
    assert!(BookmarkStore::load(path.clone()).is_err());
    doc["version"] = 1.into();
    doc["entries"][0]["id"] = 0.into();
    fs::write(&path, serde_json::to_vec(&doc).unwrap()).unwrap();
    assert!(BookmarkStore::load(path.clone()).is_err());
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();
    assert!(b.rename(id, "Unsaved").is_err());
    assert_eq!(b.entries()[0].label, "Valid");
}
#[cfg(unix)]
#[test]
fn bookmark_symlink_is_not_overwritten() {
    let t = Tree::new();
    let target = t.0.join("target");
    fs::write(&target, b"untouched").unwrap();
    let link = t.0.join("bookmarks.json");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    assert!(BookmarkStore::load(link).is_err());
    assert_eq!(fs::read(target).unwrap(), b"untouched");
}
#[test]
fn frozen_tab_sequences_exercise_native_state_invariants() {
    let t = Tree::new();
    let sorter = NameSorter::en_us().unwrap();
    for payload in [
        include_str!("../../../fixtures/W11-tabs-bookmarks/cases.json"),
        include_str!("../../../fixtures/W11-tabs-bookmarks/reserved.json"),
    ] {
        let cases: Vec<serde_json::Value> = serde_json::from_str(payload).unwrap();
        for case in cases {
            let mut tabs = TabStack::new(pane(&t, &sorter));
            let mut all_ids = std::collections::HashSet::from([tabs.active().id]);
            for event in case["events"].as_array().unwrap() {
                match event.as_str().unwrap() {
                    "duplicate" => {
                        if let Ok(id) = tabs.duplicate(&sorter) {
                            assert!(all_ids.insert(id));
                        }
                    }
                    "switch" => tabs.cycle(true),
                    "close" => {
                        tabs.close(tabs.active().id);
                    }
                    "mark" => {
                        tabs.active_mut()
                            .pane
                            .apply_event(&PaneEvent::All, &sorter)
                            .unwrap();
                    }
                    "navigate" => {
                        tabs.active_mut().pane.open(&t.0.join("child"), &sorter);
                    }
                    "back" => {
                        tabs.active_mut().pane.traverse_history(false, &sorter);
                    }
                    "hidden" => {
                        for id in tabs.tabs().iter().map(|t| t.id).collect::<Vec<_>>() {
                            tabs.select(id);
                            let p = &mut tabs.active_mut().pane;
                            p.set_show_hidden(!p.show_hidden, &sorter);
                        }
                    }
                    "refresh" => {
                        tabs.active_mut().pane.refresh(&sorter);
                    }
                    // Job and bookmark journeys have dedicated filesystem/desktop tests below this boundary.
                    "start-copy" | "finish-copy" | "close-origin" | "add-local" | "add-zip"
                    | "rename" | "reorder" | "save" | "restart" | "open-missing" => {}
                    other => panic!("Unrecognized frozen event {other}"),
                }
                assert!((1..=32).contains(&tabs.len()));
                let unique: std::collections::HashSet<_> =
                    tabs.tabs().iter().map(|t| t.id).collect();
                assert_eq!(unique.len(), tabs.len());
                assert!(unique.contains(&tabs.active().id));
            }
        }
    }
}

#[cfg(unix)]
#[test]
fn denied_bookmark_save_retains_last_valid_bytes_and_rejects_duplicate_ids() {
    use std::os::unix::fs::PermissionsExt;
    let t = Tree::new();
    let config = t.0.join("config");
    fs::create_dir(&config).unwrap();
    let path = config.join("bookmarks.json");
    let mut b = BookmarkStore::load(path.clone()).unwrap();
    let id = b.add("Original", PaneLocation::Local(t.0.clone())).unwrap();
    let bytes = fs::read(&path).unwrap();
    fs::set_permissions(&config, fs::Permissions::from_mode(0o555)).unwrap();
    let failed = b.rename(id, "Unsaved");
    fs::set_permissions(&config, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(failed.is_err(), "Run permission checks unprivileged");
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(b.entries()[0].label, "Original");
    assert_eq!(fs::read_dir(&config).unwrap().count(), 1);
    let mut doc: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let duplicate = doc["entries"][0].clone();
    doc["entries"].as_array_mut().unwrap().push(duplicate);
    fs::write(&path, serde_json::to_vec(&doc).unwrap()).unwrap();
    assert!(BookmarkStore::load(path.clone()).is_err());
    fs::write(&path, vec![b' '; 1024 * 1024 + 1]).unwrap();
    assert!(BookmarkStore::load(path).is_err());
}
#[test]
fn new_home_defaults_and_shared_hidden_reconcile_inactive_tabs() {
    let t = Tree::new();
    fs::write(t.0.join(".hidden"), b"h").unwrap();
    let sorter = NameSorter::en_us().unwrap();
    let mut panes = TabbedPanes::from([pane(&t, &sorter), pane(&t, &sorter)]);
    panes[0].options.mode = SortMode::Lexicographic;
    let original = panes.stack(0).active().id;
    panes.stack_mut(0).add_home(&t.0, true, &sorter).unwrap();
    assert_eq!(panes[0].options.mode, SortMode::Natural);
    assert_eq!(panes[0].history_len(), 1);
    panes[0].apply_event(&PaneEvent::All, &sorter).unwrap();
    for tab in panes.all_mut() {
        tab.pane.set_show_hidden(false, &sorter);
    }
    assert!(
        panes
            .all()
            .all(|tab| !tab.pane.entries().iter().any(|e| e.name == ".hidden"))
    );
    assert_eq!(panes.stack(0).len(), 2);
    assert!(panes.stack_mut(0).select(original));
    assert_eq!(panes[0].options.mode, SortMode::Lexicographic);
}
