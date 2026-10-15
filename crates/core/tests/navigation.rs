// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

use mucrab_core::{BrowserPane, NameSorter, SortMode, list_directory};
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
