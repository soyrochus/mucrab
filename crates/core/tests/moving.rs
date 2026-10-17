// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
#![cfg(unix)]
mod support;

use mucrab_core::{
    BrowserPane, CopyIssueKind, CopyPolicy, CopyRequest, MoveRequest, NameSorter, SortCriterion,
    move_entries,
};
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt, symlink},
    path::{Path, PathBuf},
    time::{Duration, UNIX_EPOCH},
};
struct Fixture {
    root: PathBuf,
    cross: PathBuf,
}
fn relax(path: &Path) {
    let meta = fs::symlink_metadata(path).unwrap();
    if meta.is_dir() {
        fs::set_permissions(
            path,
            fs::Permissions::from_mode(meta.permissions().mode() | 0o700),
        )
        .unwrap();
        for child in fs::read_dir(path).unwrap() {
            relax(&child.unwrap().path());
        }
    }
}
impl Fixture {
    fn new() -> Self {
        let tag = format!("{}-{}", std::process::id(), support::nonce());
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../artifacts/tests")
            .join("w04-".to_owned() + &tag);
        let cross = std::env::temp_dir().join("mucrab-w04-test-".to_owned() + &tag);
        fs::create_dir_all(root.join("source")).unwrap();
        fs::create_dir(root.join("dest")).unwrap();
        fs::create_dir(&cross).unwrap();
        assert_ne!(
            fs::metadata(&root).unwrap().dev(),
            fs::metadata(&cross).unwrap().dev(),
            "W04 requires the temporary directory on a genuinely different filesystem"
        );
        Self {
            root: fs::canonicalize(root).unwrap(),
            cross: fs::canonicalize(cross).unwrap(),
        }
    }
    fn source(&self) -> PathBuf {
        self.root.join("source")
    }
    fn request(&self, names: &[&str], cross: bool, policy: CopyPolicy) -> MoveRequest {
        MoveRequest {
            transfer: CopyRequest {
                source_directory: self.source(),
                destination_directory: if cross {
                    self.cross.clone()
                } else {
                    self.root.join("dest")
                },
                names: names.iter().map(|s| s.to_string()).collect(),
                new_name: None,
                policy,
            },
            rename_mode: false,
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        relax(&self.root);
        relax(&self.cross);
        fs::remove_dir_all(&self.root).unwrap();
        fs::remove_dir_all(&self.cross).unwrap();
    }
}

#[test]
fn local_rename_preserves_inode_metadata_and_replaces_readonly_files() {
    let f = Fixture::new();
    let source = f.source();
    let dest = f.root.join("dest");
    fs::write(source.join("old"), b"data intact").unwrap();
    fs::set_permissions(source.join("old"), fs::Permissions::from_mode(0o751)).unwrap();
    fs::File::open(source.join("old"))
        .unwrap()
        .set_modified(UNIX_EPOCH - Duration::from_millis(1234))
        .unwrap();
    fs::write(dest.join("old"), b"old destination").unwrap();
    fs::set_permissions(dest.join("old"), fs::Permissions::from_mode(0o444)).unwrap();
    let before = fs::metadata(source.join("old")).unwrap();
    let report = move_entries(&f.request(&["old"], false, CopyPolicy::Overwrite)).unwrap();
    assert!(report.outcomes[0].success);
    assert_eq!(report.renamed_entries, 1);
    assert!(!source.join("old").exists());
    let after = fs::metadata(dest.join("old")).unwrap();
    assert_eq!(before.ino(), after.ino());
    assert_eq!(after.permissions().mode() & 0o777, 0o751);
    assert_eq!(before.modified().unwrap(), after.modified().unwrap());
    assert_eq!(fs::read(dest.join("old")).unwrap(), b"data intact");
    fs::write(source.join("File.txt"), b"case").unwrap();
    let mut request = f.request(&["File.txt"], false, CopyPolicy::Skip);
    request.rename_mode = true;
    request.transfer.destination_directory = source.clone();
    request.transfer.new_name = Some("file.txt".into());
    assert!(move_entries(&request).unwrap().outcomes[0].success);
    // On case-insensitive APFS the old spelling still resolves, so inspect the
    // actual directory entry to verify that the rename changed its spelling.
    let names: Vec<_> = fs::read_dir(&source)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert!(!names.iter().any(|name| name == "File.txt"));
    assert!(names.iter().any(|name| name == "file.txt"));
    assert_eq!(fs::read(source.join("file.txt")).unwrap(), b"case");
}

#[test]
fn real_cross_filesystem_moves_preserve_data_and_retain_skipped_children() {
    let f = Fixture::new();
    let source = f.source();
    fs::write(source.join("past"), b"cross device").unwrap();
    fs::File::open(source.join("past"))
        .unwrap()
        .set_modified(UNIX_EPOCH - Duration::from_millis(1))
        .unwrap();
    let report = move_entries(&f.request(&["past"], true, CopyPolicy::Skip)).unwrap();
    assert!(report.outcomes[0].success);
    assert_eq!(report.files_transferred, 1);
    assert_eq!(report.renamed_entries, 0);
    assert!(!source.join("past").exists());
    assert_eq!(fs::read(f.cross.join("past")).unwrap(), b"cross device");
    assert_eq!(
        fs::metadata(f.cross.join("past"))
            .unwrap()
            .modified()
            .unwrap(),
        UNIX_EPOCH
    );
    fs::create_dir(source.join("tree")).unwrap();
    fs::write(source.join("tree/both"), b"source retained").unwrap();
    fs::write(source.join("tree/new"), b"moved").unwrap();
    fs::create_dir(f.cross.join("tree")).unwrap();
    fs::write(f.cross.join("tree/both"), b"destination retained").unwrap();
    let report = move_entries(&f.request(&["tree"], true, CopyPolicy::Skip)).unwrap();
    assert!(!report.outcomes[0].success);
    assert_eq!(
        fs::read(source.join("tree/both")).unwrap(),
        b"source retained"
    );
    assert!(!source.join("tree/new").exists());
    assert_eq!(fs::read(f.cross.join("tree/new")).unwrap(), b"moved");
    assert_eq!(
        fs::read(f.cross.join("tree/both")).unwrap(),
        b"destination retained"
    );
}

#[test]
fn failed_source_deletion_retains_both_copies_and_reports_failure() {
    let f = Fixture::new();
    let source = f.source();
    fs::write(source.join("file"), b"cannot remove source").unwrap();
    fs::set_permissions(&source, fs::Permissions::from_mode(0o555)).unwrap();
    let report = move_entries(&f.request(&["file"], false, CopyPolicy::Skip)).unwrap();
    assert!(
        !report.outcomes[0].success,
        "Permission cases require an unprivileged user"
    );
    assert_eq!(report.files_transferred, 1);
    assert_eq!(report.sources_removed, 0);
    assert_eq!(
        fs::read(source.join("file")).unwrap(),
        b"cannot remove source"
    );
    assert_eq!(
        fs::read(f.root.join("dest/file")).unwrap(),
        b"cannot remove source"
    );
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.message.contains("source could not be removed"))
    );
}

#[test]
fn move_guards_keep_link_targets_aliases_and_invalid_paths_intact() {
    let f = Fixture::new();
    let source = f.source();
    fs::create_dir(source.join("target")).unwrap();
    fs::write(source.join("target/child"), b"target intact").unwrap();
    symlink("target", source.join("link")).unwrap();
    let report = move_entries(&f.request(&["link"], true, CopyPolicy::Skip)).unwrap();
    assert!(!report.outcomes[0].success);
    assert!(
        fs::symlink_metadata(source.join("link"))
            .unwrap()
            .is_symlink()
    );
    assert_eq!(
        fs::read(source.join("target/child")).unwrap(),
        b"target intact"
    );
    assert!(!f.cross.join("link").exists());
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.kind == CopyIssueKind::Unsupported)
    );
    fs::create_dir(source.join("whole")).unwrap();
    fs::write(source.join("whole/data"), b"whole").unwrap();
    symlink("missing", source.join("whole/broken")).unwrap();
    assert!(
        move_entries(&f.request(&["whole"], false, CopyPolicy::Skip))
            .unwrap()
            .outcomes[0]
            .success
    );
    assert_eq!(
        fs::read_link(f.root.join("dest/whole/broken")).unwrap(),
        PathBuf::from("missing")
    );
    fs::write(source.join("file"), b"source intact").unwrap();
    fs::hard_link(source.join("file"), f.root.join("dest/file")).unwrap();
    assert!(
        !move_entries(&f.request(&["file"], false, CopyPolicy::Overwrite))
            .unwrap()
            .outcomes[0]
            .success
    );
    for bad in ["../escape", "/tmp/escape", "file/", ".", "..", ""] {
        let mut request = f.request(&["file"], false, CopyPolicy::Overwrite);
        request.transfer.new_name = Some(bad.into());
        assert!(move_entries(&request).is_err());
    }
    let mut request = f.request(&["target"], false, CopyPolicy::Overwrite);
    request.transfer.destination_directory = source.join("target");
    assert!(!move_entries(&request).unwrap().outcomes[0].success);
    assert_eq!(fs::read(source.join("file")).unwrap(), b"source intact");
}

#[test]
fn focused_rename_restores_new_identity_and_preserves_other_marks_and_sorting() {
    let f = Fixture::new();
    let source = f.source();
    fs::write(source.join("old"), b"rename").unwrap();
    fs::write(source.join("marked"), b"stay marked").unwrap();
    let sorter = NameSorter::en_us().unwrap();
    let mut pane = BrowserPane::default();
    assert!(pane.open(&source, &sorter));
    pane.options.criterion = SortCriterion::Date;
    pane.resort(&sorter);
    let marked = pane
        .order
        .iter()
        .position(|i| pane.entries[*i].name == "marked")
        .unwrap()
        + 1;
    pane.selection.set_marked(marked, true, &pane.order);
    assert!(pane.focus_name("old"));
    let mut request = f.request(&["old"], false, CopyPolicy::Skip);
    request.rename_mode = true;
    request.transfer.destination_directory = source;
    request.transfer.new_name = Some("renamed".into());
    let report = move_entries(&request).unwrap();
    pane.unmark_copy_outcomes(&report.outcomes);
    pane.refresh(&sorter);
    assert!(pane.focus_name("renamed"));
    let state = pane.selection.snapshot(&pane.entries, &pane.order);
    assert_eq!(state.focused.as_deref(), Some("renamed"));
    assert_eq!(state.marked, vec!["marked"]);
    assert_eq!(pane.options.criterion, SortCriterion::Date);
}
