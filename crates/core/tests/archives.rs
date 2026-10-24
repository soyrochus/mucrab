// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
//! Behavioural regression coverage for archives using disposable fixtures.
#![cfg(unix)]
mod support;

use mucrab_core::{
    BrowserPane, CopyPolicy, NameSorter, PaneEvent, ZipCatalog, ZipExtractPlan, extract_zip,
};
use std::{
    fs,
    io::Write,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
};
use zip::{CompressionMethod, DateTime, ZipWriter, write::SimpleFileOptions};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "mucrab-w06-{}-{}",
            std::process::id(),
            support::nonce()
        ));
        fs::create_dir_all(root.join("destination")).unwrap();
        Self(root)
    }
    fn zip(&self, entries: &[(&str, &[u8])]) -> PathBuf {
        let path = self.0.join("fixture.zip");
        let mut writer = ZipWriter::new(fs::File::create(&path).unwrap());
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .last_modified_time(DateTime::from_date_and_time(2020, 6, 7, 8, 9, 10).unwrap())
            .unix_permissions(0o644);
        for (name, data) in entries {
            if name.ends_with('/') {
                writer.add_directory(*name, options).unwrap();
            } else {
                writer.start_file(*name, options).unwrap();
                writer.write_all(data).unwrap();
            }
        }
        writer.finish().unwrap();
        path
    }
    fn plan(&self, prefix: &str, names: Option<&[String]>, policy: CopyPolicy) -> ZipExtractPlan {
        ZipExtractPlan::new(
            &self.0.join("fixture.zip"),
            prefix,
            names,
            &self.0.join("destination"),
            policy,
        )
        .unwrap()
    }
}
fn relax(path: &Path) {
    if fs::symlink_metadata(path).unwrap().is_dir() {
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        for entry in fs::read_dir(path).unwrap() {
            relax(&entry.unwrap().path());
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        relax(&self.0);
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn implicit_and_explicit_folders_preserve_encounter_order_and_nested_bytes() {
    let f = Fixture::new();
    let archive = f.zip(&[
        ("z/nested/a.txt", b"nested"),
        ("root", b"root"),
        ("z/", b""),
        ("empty/", b""),
        (".hidden", b"hidden"),
    ]);
    let before = fs::read(&archive).unwrap();
    let catalog = ZipCatalog::open(&archive).unwrap();
    assert_eq!(
        catalog
            .list("")
            .unwrap()
            .iter()
            .map(|e| &*e.name)
            .collect::<Vec<_>>(),
        ["z", "root", "empty", ".hidden"]
    );
    assert!(catalog.list("missing").is_err());
    assert!(catalog.list("root").is_err());
    let report = extract_zip(&f.plan("z/", Some(&["nested".into()]), CopyPolicy::Skip)).unwrap();
    assert!(report.success);
    assert_eq!(
        fs::read(f.0.join("destination/nested/a.txt")).unwrap(),
        b"nested"
    );
    assert!(!f.0.join("destination/root").exists());
    assert_eq!(fs::read(archive).unwrap(), before);
}

#[test]
fn archive_navigation_refresh_hidden_filter_and_parent_preserve_local_behavior() {
    let f = Fixture::new();
    let archive = f.zip(&[
        ("folder/file", b"data"),
        (".hidden", b"hidden"),
        ("root", b"root"),
    ]);
    let sorter = NameSorter::en_us().unwrap();
    let mut pane = BrowserPane::default();
    assert!(pane.open(&archive, &sorter));
    assert!(pane.archive().is_some());
    pane.refresh(&sorter);
    assert!(pane.error.is_none());
    assert_eq!(pane.entries().len(), 2);
    pane.apply_event(&PaneEvent::All, &sorter).unwrap();
    pane.set_show_hidden(true, &sorter);
    assert_eq!(pane.entries().len(), 3);
    assert_eq!(pane.selection().marked_count(), 2);
    assert!(pane.open(&archive.join("folder"), &sorter));
    let old = pane.location().cloned();
    assert!(!pane.open(&archive.join("missing"), &sorter));
    assert_eq!(pane.location().cloned(), old);
    pane.parent(&sorter);
    assert_eq!(
        pane.selection()
            .snapshot(pane.entries(), pane.order())
            .focused
            .as_deref(),
        Some("folder")
    );
    pane.parent(&sorter);
    assert!(pane.archive().is_none());
    assert_eq!(
        pane.selection()
            .snapshot(pane.entries(), pane.order())
            .focused
            .as_deref(),
        Some("fixture.zip")
    );
    let hidden_archive = f.0.join(".zip");
    fs::copy(&archive, &hidden_archive).unwrap();
    assert!(pane.open(&hidden_archive, &sorter));
    assert!(pane.archive().is_some());
    assert!(
        ZipExtractPlan::new(
            &archive,
            "//",
            None,
            &f.0.join("destination"),
            CopyPolicy::Skip
        )
        .is_err()
    );
}

#[test]
fn collisions_preserve_skips_and_stop_after_first_error_without_rollback() {
    let f = Fixture::new();
    f.zip(&[
        ("kept", b"new"),
        ("good", b"completed"),
        ("blocked/child", b"fail"),
        ("later", b"later"),
    ]);
    fs::write(f.0.join("destination/kept"), b"existing").unwrap();
    fs::write(f.0.join("destination/blocked"), b"file blocks directory").unwrap();
    let report = extract_zip(&f.plan("", None, CopyPolicy::Skip)).unwrap();
    assert!(!report.success);
    assert_eq!(report.files_extracted, 1);
    assert_eq!(fs::read(f.0.join("destination/kept")).unwrap(), b"existing");
    assert_eq!(
        fs::read(f.0.join("destination/good")).unwrap(),
        b"completed"
    );
    assert!(!f.0.join("destination/later").exists());
    assert_eq!(report.issues.len(), 2);
}

#[test]
fn reviewed_source_or_destination_replacement_is_rejected_before_writing() {
    let f = Fixture::new();
    let archive = f.zip(&[("file", b"contents")]);
    let plan = f.plan("", None, CopyPolicy::Overwrite);
    fs::rename(&archive, f.0.join("old.zip")).unwrap();
    fs::copy(f.0.join("old.zip"), &archive).unwrap();
    assert!(extract_zip(&plan).is_err());
    assert!(!f.0.join("destination/file").exists());
    let plan = f.plan("", None, CopyPolicy::Overwrite);
    fs::rename(f.0.join("destination"), f.0.join("old-destination")).unwrap();
    fs::create_dir(f.0.join("destination")).unwrap();
    assert!(extract_zip(&plan).is_err());
    assert!(!f.0.join("destination/file").exists());
    let plan = f.plan("", None, CopyPolicy::Overwrite);
    fs::remove_file(&archive).unwrap();
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&archive)
            .status()
            .unwrap()
            .success()
    );
    // Reject a non-regular replacement before opening it: opening a FIFO blocks.
    let error = extract_zip(&plan).unwrap_err();
    assert!(error.to_string().contains("regular file"));
    assert!(!f.0.join("destination/file").exists());
}

#[test]
fn unsafe_paths_and_file_folder_conflicts_are_rejected_before_mutation() {
    for name in [
        "../escape",
        "/absolute",
        "dir/../escape",
        "back\\slash",
        "C:drive",
        "two//parts",
        "./dot",
    ] {
        let f = Fixture::new();
        let archive = f.zip(&[(name, b"unsafe")]);
        assert!(ZipCatalog::open(&archive).is_err(), "{name}");
        assert!(
            fs::read_dir(f.0.join("destination"))
                .unwrap()
                .next()
                .is_none()
        );
    }
    let f = Fixture::new();
    let archive = f.zip(&[("conflict", b"file"), ("conflict/child", b"child")]);
    assert!(ZipCatalog::open(&archive).is_err());
}

#[test]
fn destination_symlinks_hardlinks_and_archive_aliases_cannot_be_written_through() {
    let f = Fixture::new();
    f.zip(&[("good", b"data"), ("alias/child", b"escape")]);
    fs::create_dir(f.0.join("outside")).unwrap();
    symlink("../outside", f.0.join("destination/alias")).unwrap();
    assert!(
        ZipExtractPlan::new(
            &f.0.join("fixture.zip"),
            "",
            None,
            &f.0.join("destination"),
            CopyPolicy::Overwrite
        )
        .is_err()
    );
    assert!(!f.0.join("destination/good").exists());
    assert!(!f.0.join("outside/child").exists());
    fs::remove_file(f.0.join("destination/alias")).unwrap();
    fs::write(f.0.join("outside/original"), b"retained").unwrap();
    fs::hard_link(f.0.join("outside/original"), f.0.join("destination/good")).unwrap();
    assert!(
        ZipExtractPlan::new(
            &f.0.join("fixture.zip"),
            "",
            None,
            &f.0.join("destination"),
            CopyPolicy::Overwrite
        )
        .is_err()
    );
    assert_eq!(fs::read(f.0.join("outside/original")).unwrap(), b"retained");
    f.zip(&[("fixture.zip", b"overwrite source")]);
    assert!(
        ZipExtractPlan::new(
            &f.0.join("fixture.zip"),
            "",
            None,
            &f.0,
            CopyPolicy::Overwrite
        )
        .is_err()
    );
}

#[test]
fn crc_corruption_is_reported_and_later_entries_are_not_extracted() {
    let f = Fixture::new();
    let archive = f.zip(&[
        ("first", b"first"),
        ("bad", b"UNIQUE-CONTENTS"),
        ("later", b"later"),
    ]);
    let mut bytes = fs::read(&archive).unwrap();
    let offset = bytes
        .windows(15)
        .position(|part| part == b"UNIQUE-CONTENTS")
        .unwrap();
    bytes[offset] ^= 0xff;
    fs::write(&archive, &bytes).unwrap();
    let report = extract_zip(&f.plan("", None, CopyPolicy::Skip)).unwrap();
    assert!(!report.success);
    assert_eq!(report.files_extracted, 1);
    assert_eq!(fs::read(f.0.join("destination/first")).unwrap(), b"first");
    assert!(!f.0.join("destination/later").exists());
    assert_eq!(fs::read(&archive).unwrap(), bytes);
}

#[test]
fn declared_expansion_symlinks_duplicates_and_excessive_depth_are_rejected() {
    let f = Fixture::new();
    let archive = f.zip(&[("a", b"a"), ("b", b"b")]);
    let original = fs::read(&archive).unwrap();
    let central = original
        .windows(4)
        .position(|part| part == b"PK\x01\x02")
        .unwrap();
    let mut oversized = original.clone();
    oversized[central + 24..central + 28]
        .copy_from_slice(&(512_u32 * 1024 * 1024 + 1).to_le_bytes());
    fs::write(&archive, oversized).unwrap();
    assert!(ZipCatalog::open(&archive).is_err());
    let mut symlink = original.clone();
    symlink[central + 38..central + 42].copy_from_slice(&(0o120777_u32 << 16).to_le_bytes());
    fs::write(&archive, symlink).unwrap();
    assert!(ZipCatalog::open(&archive).is_err());
    let mut duplicates = original;
    let headers: Vec<_> = duplicates
        .windows(4)
        .enumerate()
        .filter(|(_, part)| *part == b"PK\x01\x02")
        .map(|(i, _)| i)
        .collect();
    duplicates[headers[1] + 46] = b'a';
    fs::write(&archive, duplicates).unwrap();
    assert!(ZipCatalog::open(&archive).is_err());
    let name = format!("{}file", "folder/".repeat(65));
    let archive = f.zip(&[(&name, b"data")]);
    assert!(ZipCatalog::open(&archive).is_err());
}
