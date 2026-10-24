// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
//! Behavioural regression coverage for copying using disposable fixtures.
#![cfg(unix)]
mod support;

use mucrab_core::{
    BrowserPane, CopyIssueKind, CopyPolicy, CopyRequest, NameSorter, SortCriterion, copy_entries,
};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::PathBuf,
    time::{Duration, UNIX_EPOCH},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let nonce = support::nonce();
        let root = std::env::temp_dir().join(format!("mucrab-copy-{}-{nonce}", std::process::id()));
        fs::create_dir_all(root.join("source")).unwrap();
        fs::create_dir(root.join("dest")).unwrap();
        Self(root)
    }
    fn request(&self, names: &[&str], policy: CopyPolicy) -> CopyRequest {
        CopyRequest {
            source_directory: self.0.join("source"),
            destination_directory: self.0.join("dest"),
            names: names.iter().map(|s| s.to_string()).collect(),
            new_name: None,
            policy,
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn recursive_merge_preserves_bytes_and_recovered_metadata() {
    let f = Fixture::new();
    let s = f.0.join("source");
    let d = f.0.join("dest");
    fs::create_dir_all(s.join("tree/sub/empty")).unwrap();
    fs::create_dir(d.join("tree")).unwrap();
    let data: Vec<u8> = (0..262151).map(|i| (i % 251) as u8).collect();
    fs::write(s.join("tree/sub/binary"), &data).unwrap();
    fs::write(s.join("tree/.hidden"), b"hidden").unwrap();
    fs::write(d.join("tree/keep"), b"retained").unwrap();
    fs::set_permissions(s.join("tree/sub/binary"), fs::Permissions::from_mode(0o751)).unwrap();
    fs::File::open(s.join("tree/sub/binary"))
        .unwrap()
        .set_modified(UNIX_EPOCH + Duration::from_millis(1577836800123))
        .unwrap();
    fs::write(d.join("mode-reference"), b"creation mode").unwrap();
    let created_other_bits = fs::metadata(d.join("mode-reference"))
        .unwrap()
        .permissions()
        .mode()
        & 0o77;
    let report = copy_entries(&f.request(&["tree"], CopyPolicy::Skip)).unwrap();
    assert!(report.issues.is_empty());
    assert!(report.outcomes[0].success);
    assert_eq!(fs::read(d.join("tree/sub/binary")).unwrap(), data);
    assert_eq!(fs::read(s.join("tree/sub/binary")).unwrap(), data);
    assert!(d.join("tree/sub/empty").is_dir());
    assert_eq!(fs::read(d.join("tree/.hidden")).unwrap(), b"hidden");
    assert_eq!(fs::read(d.join("tree/keep")).unwrap(), b"retained");
    assert_eq!(
        fs::metadata(d.join("tree/sub/binary"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700 | created_other_bits
    );
    assert_eq!(
        fs::metadata(d.join("tree/sub/binary"))
            .unwrap()
            .modified()
            .unwrap(),
        UNIX_EPOCH + Duration::from_millis(1577836800123)
    );
    fs::write(s.join("past"), b"past").unwrap();
    fs::File::open(s.join("past"))
        .unwrap()
        .set_modified(UNIX_EPOCH - Duration::from_millis(1))
        .unwrap();
    let mut request = f.request(&["past"], CopyPolicy::Overwrite);
    request.new_name = Some("renamed".into());
    assert!(copy_entries(&request).unwrap().outcomes[0].success);
    assert_eq!(
        fs::metadata(d.join("renamed")).unwrap().modified().unwrap(),
        UNIX_EPOCH
    );
}

#[test]
fn collisions_and_child_errors_leave_visible_partial_results() {
    let f = Fixture::new();
    let s = f.0.join("source");
    let d = f.0.join("dest");
    fs::create_dir(s.join("tree")).unwrap();
    fs::write(s.join("tree/locked"), b"cannot read").unwrap();
    fs::write(s.join("tree/good"), b"copied").unwrap();
    fs::set_permissions(s.join("tree/locked"), fs::Permissions::from_mode(0o0)).unwrap();
    assert!(
        fs::File::open(s.join("tree/locked")).is_err(),
        "Permission cases require an unprivileged user"
    );
    let report = copy_entries(&f.request(&["tree", "missing"], CopyPolicy::Skip)).unwrap();
    fs::set_permissions(s.join("tree/locked"), fs::Permissions::from_mode(0o600)).unwrap();
    assert!(report.outcomes[0].success);
    assert!(!report.outcomes[1].success);
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.kind == CopyIssueKind::Failed)
    );
    assert_eq!(fs::read(d.join("tree/good")).unwrap(), b"copied");
    assert!(!d.join("tree/locked").exists());
    fs::write(s.join("file"), b"new").unwrap();
    fs::write(d.join("file"), b"old").unwrap();
    assert!(
        !copy_entries(&f.request(&["file"], CopyPolicy::OverwriteIfSizeDiffers))
            .unwrap()
            .outcomes[0]
            .success
    );
    assert_eq!(fs::read(d.join("file")).unwrap(), b"old");
    fs::write(s.join("file"), b"new longer").unwrap();
    assert!(
        copy_entries(&f.request(&["file"], CopyPolicy::OverwriteIfSizeDiffers))
            .unwrap()
            .outcomes[0]
            .success
    );
    assert_eq!(fs::read(d.join("file")).unwrap(), b"new longer");
}

#[test]
fn copy_boundaries_prevent_source_damage_and_outside_writes() {
    let f = Fixture::new();
    let s = f.0.join("source");
    let d = f.0.join("dest");
    fs::write(s.join("file"), b"source intact").unwrap();
    fs::write(f.0.join("outside"), b"outside intact").unwrap();
    for name in [
        "../outside",
        "/tmp/outside",
        "file/",
        "file/.",
        ".",
        "..",
        "",
    ] {
        let mut request = f.request(&["file"], CopyPolicy::Overwrite);
        request.new_name = Some(name.into());
        assert!(copy_entries(&request).is_err());
        assert!(!d.join("file").exists());
    }
    symlink("../outside", s.join("escape")).unwrap();
    symlink("../outside", d.join("file")).unwrap();
    let report = copy_entries(&f.request(&["file", "escape"], CopyPolicy::Overwrite)).unwrap();
    assert!(report.outcomes.iter().all(|o| !o.success));
    assert!(!d.join("escape").exists());
    assert_eq!(fs::read(f.0.join("outside")).unwrap(), b"outside intact");
    assert_eq!(fs::read(s.join("file")).unwrap(), b"source intact");
    fs::remove_file(d.join("file")).unwrap();
    fs::hard_link(s.join("file"), d.join("file")).unwrap();
    assert!(
        !copy_entries(&f.request(&["file"], CopyPolicy::Overwrite))
            .unwrap()
            .outcomes[0]
            .success
    );
    assert_eq!(fs::read(s.join("file")).unwrap(), b"source intact");
    fs::create_dir_all(s.join("folder/inside")).unwrap();
    let mut nested = f.request(&["folder"], CopyPolicy::Overwrite);
    nested.destination_directory = s.join("folder/inside");
    assert!(!copy_entries(&nested).unwrap().outcomes[0].success);
    assert!(!s.join("folder/inside/folder").exists());
}

#[test]
fn symlinks_and_copy_completion_preserve_selection_identity() {
    let f = Fixture::new();
    let s = f.0.join("source");
    let d = f.0.join("dest");
    fs::create_dir(s.join("pack")).unwrap();
    fs::write(s.join("pack/target"), b"target").unwrap();
    symlink("target", s.join("pack/link")).unwrap();
    symlink("missing", s.join("pack/broken")).unwrap();
    fs::write(s.join("other"), b"keep mark").unwrap();
    let sorter = NameSorter::en_us().unwrap();
    let mut pane = BrowserPane::default();
    pane.open(&s, &sorter);
    pane.options.criterion = SortCriterion::Extension;
    pane.resort(&sorter);
    for row in 1..pane.selection().row_count(pane.order()) {
        pane.mark(row, true);
    }
    let report = copy_entries(&f.request(&["pack"], CopyPolicy::Skip)).unwrap();
    assert!(report.outcomes[0].success);
    assert_eq!(report.links_copied, 2);
    assert_eq!(
        fs::read_link(d.join("pack/link")).unwrap(),
        PathBuf::from("target")
    );
    assert!(
        fs::symlink_metadata(d.join("pack/broken"))
            .unwrap()
            .is_symlink()
    );
    pane.unmark_copy_outcomes(&report.outcomes);
    pane.refresh(&sorter);
    assert_eq!(pane.options.criterion, SortCriterion::Extension);
    assert_eq!(
        pane.selection()
            .snapshot(pane.entries(), pane.order())
            .marked,
        vec!["other"]
    );
}
