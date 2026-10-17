// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
#![cfg(unix)]
mod support;

use mucrab_core::{BatchRenameOptions, BatchRenamePlan, apply_batch_rename};
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt, symlink},
    path::{Path, PathBuf},
    time::{Duration, UNIX_EPOCH},
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir()
            .join(format!(
                "mucrab-w05-{}-{}",
                std::process::id(),
                support::nonce()
            ))
            .join("source");
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }
    fn file(&self, name: &str, data: &[u8]) {
        let p = self.0.join(name);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, data).unwrap();
    }
    fn plan(&self, names: &[&str], mask: &str) -> BatchRenamePlan {
        BatchRenamePlan::new(
            &self.0,
            &names.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            &BatchRenameOptions {
                mask: mask.into(),
                ..Default::default()
            },
        )
        .unwrap()
    }
}
fn relax(path: &Path) {
    let m = fs::symlink_metadata(path).unwrap();
    if m.is_dir() {
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        for child in fs::read_dir(path).unwrap() {
            relax(&child.unwrap().path());
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        relax(&self.0);
        fs::remove_dir_all(self.0.parent().unwrap()).unwrap();
    }
}
#[test]
fn recovered_tokens_use_utf16_independent_wrapping_counters_and_utc_dates() {
    let f = Fixture::new();
    f.file("🦀ab.txt", b"one");
    f.file("Beta.TXT", b"two");
    for name in ["🦀ab.txt", "Beta.TXT"] {
        fs::File::open(f.0.join(name))
            .unwrap()
            .set_modified(UNIX_EPOCH + Duration::from_millis(1577836800123))
            .unwrap();
    }
    let p = f.plan(
        &["🦀ab.txt", "Beta.TXT"],
        "[YMD]-[hms]-[N1-2]-[C2147483647,1]-[C3,2,3].[E]",
    );
    assert_eq!(
        p.preview.rows[0].new_name,
        "20200101-000000-🦀-2147483647-003.txt"
    );
    assert_eq!(
        p.preview.rows[1].new_name,
        "20200101-000000-Be--2147483648-005.TXT"
    );
    let p = f.plan(&["Beta.TXT"], "[N2--2]-[E-2]-[P1,3]-[[]-[]-[Q]");
    assert_eq!(p.preview.rows[0].new_name, "et-X-sou-[--[Q]");
}
#[test]
fn ordered_apply_preserves_bytes_metadata_links_and_unchanged_entries() {
    let f = Fixture::new();
    f.file("old-one.txt", b"binary\0\xff");
    f.file("keep.dat", b"keep");
    f.file("old-package/.hidden", b"hidden");
    symlink(".hidden", f.0.join("old-package/link")).unwrap();
    fs::set_permissions(f.0.join("old-one.txt"), fs::Permissions::from_mode(0o751)).unwrap();
    let before = fs::metadata(f.0.join("old-one.txt")).unwrap();
    let p = BatchRenamePlan::new(
        &f.0,
        &[
            "old-package".into(),
            "keep.dat".into(),
            "old-one.txt".into(),
        ],
        &BatchRenameOptions {
            search: "old-".into(),
            replacement: "new-".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let r = apply_batch_rename(&p).unwrap();
    assert_eq!(r.outcomes.len(), 2);
    assert!(r.outcomes.iter().all(|o| o.success));
    assert_eq!(r.outcomes[0].name, "old-package");
    assert_eq!(r.outcomes[1].name, "old-one.txt");
    assert_eq!(fs::read(f.0.join("new-one.txt")).unwrap(), b"binary\0\xff");
    assert_eq!(fs::read(f.0.join("keep.dat")).unwrap(), b"keep");
    let after = fs::metadata(f.0.join("new-one.txt")).unwrap();
    assert_eq!(
        (after.ino(), after.mode(), after.modified().unwrap()),
        (before.ino(), before.mode(), before.modified().unwrap())
    );
    assert_eq!(
        fs::read_link(f.0.join("new-package/link")).unwrap(),
        PathBuf::from(".hidden")
    );
    assert_eq!(
        fs::read(f.0.join("new-package/.hidden")).unwrap(),
        b"hidden"
    );
    assert!(!f.0.join("old-package").exists());
}
#[test]
fn collisions_and_partial_merges_keep_skipped_bytes_and_continue() {
    let f = Fixture::new();
    f.file("a", b"retain source");
    f.file("done-a", b"retain destination");
    f.file("b", b"move sibling");
    let r = apply_batch_rename(&f.plan(&["a", "b"], "done-[N]")).unwrap();
    assert!(!r.outcomes[0].success);
    assert!(r.outcomes[1].success);
    assert!(!r.issues.is_empty());
    assert_eq!(fs::read(f.0.join("a")).unwrap(), b"retain source");
    assert_eq!(fs::read(f.0.join("done-a")).unwrap(), b"retain destination");
    assert_eq!(fs::read(f.0.join("done-b")).unwrap(), b"move sibling");
    f.file("old/both", b"old child");
    f.file("old/good", b"good");
    f.file("new/both", b"existing child");
    let r = apply_batch_rename(&f.plan(&["old"], "new")).unwrap();
    assert!(!r.outcomes[0].success);
    assert_eq!(fs::read(f.0.join("old/both")).unwrap(), b"old child");
    assert_eq!(fs::read(f.0.join("new/both")).unwrap(), b"existing child");
    assert_eq!(fs::read(f.0.join("new/good")).unwrap(), b"good");
    assert!(!f.0.join("old/good").exists());
}
#[test]
fn stale_conflicting_and_unsafe_plans_cannot_mutate_files() {
    let f = Fixture::new();
    f.file("a", b"one");
    f.file("b", b"two");
    let duplicate = f.plan(&["a", "b"], "same");
    assert!(duplicate.preview.duplicate_names);
    assert!(apply_batch_rename(&duplicate).is_err());
    let conflict = f.plan(&["a", "b"], "b");
    assert!(conflict.preview.old_names_conflict);
    assert!(apply_batch_rename(&conflict).is_err());
    let invalid = f.plan(&["a"], "../outside");
    assert!(!invalid.preview.can_apply());
    assert!(apply_batch_rename(&invalid).is_err());
    let p = f.plan(&["a", "b"], "new-[N]");
    let modified = fs::metadata(f.0.join("a")).unwrap().modified().unwrap();
    f.file("replacement", b"one");
    fs::File::open(f.0.join("replacement"))
        .unwrap()
        .set_modified(modified)
        .unwrap();
    fs::rename(f.0.join("replacement"), f.0.join("a")).unwrap();
    assert!(apply_batch_rename(&p).is_err());
    assert!(!f.0.join("new-a").exists());
    assert!(!f.0.join("new-b").exists());
    f.file("🦀a", b"unicode");
    assert!(
        BatchRenamePlan::new(
            &f.0,
            &["🦀a".into()],
            &BatchRenameOptions {
                mask: "[N1]".into(),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        BatchRenamePlan::new(
            &f.0,
            &["🦀a".into()],
            &BatchRenameOptions {
                case_mode: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        BatchRenamePlan::new(
            &f.0,
            &["a".into()],
            &BatchRenameOptions {
                mask: "[C1,1,100000]".into(),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(fs::read(f.0.join("b")).unwrap(), b"two");
}
#[test]
fn denied_renames_keep_every_source_and_produce_complete_failures() {
    let f = Fixture::new();
    f.file("a", b"first");
    f.file("b", b"second");
    fs::set_permissions(&f.0, fs::Permissions::from_mode(0o555)).unwrap();
    let r = apply_batch_rename(&f.plan(&["a", "b"], "renamed-[N]")).unwrap();
    assert_eq!(r.outcomes.len(), 2);
    assert!(
        r.outcomes.iter().all(|o| !o.success),
        "Run permission tests unprivileged"
    );
    assert_eq!(r.issues.len(), 2);
    assert_eq!(fs::read(f.0.join("a")).unwrap(), b"first");
    assert_eq!(fs::read(f.0.join("b")).unwrap(), b"second");
}
