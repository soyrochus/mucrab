// SPDX-License-Identifier: GPL-3.0-only
mod support;
use mucrab_core::*;
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::{
    fs,
    path::PathBuf,
    time::{Duration, UNIX_EPOCH},
};
struct Tree(PathBuf);
impl Tree {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "mucrab-w13-{}-{}",
            std::process::id(),
            support::nonce()
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Tree {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn snapshot(path: &std::path::Path) -> PropertySnapshot {
    PropertySnapshot::capture(EntryTarget::Local(path.to_owned())).unwrap()
}
#[cfg(unix)]
#[test]
fn frozen_metadata_cases_preserve_special_bits_atime_bytes_and_hardlinks() {
    let tree = Tree::new();
    for payload in [
        include_str!("../../../fixtures/W13-properties-permissions/cases.json"),
        include_str!("../../../fixtures/W13-properties-permissions/reserved.json"),
    ] {
        let cases: Vec<serde_json::Value> = serde_json::from_str(payload).unwrap();
        for case in cases {
            let path = tree.0.join(case["name"].as_str().unwrap());
            let target = tree.0.join("target");
            let kind = case["kind"].as_str().unwrap();
            let size = case["size"].as_u64().unwrap() as usize;
            if kind == "folder" {
                fs::create_dir(&path).unwrap();
            } else if kind == "link" {
                fs::write(&target, vec![7; size]).unwrap();
                std::os::unix::fs::symlink("target", &path).unwrap();
            } else {
                fs::write(&path, vec![7; size]).unwrap();
                if kind == "hardlink" {
                    fs::hard_link(&path, &target).unwrap();
                }
            }
            if kind == "link" {
                let s = snapshot(&path);
                assert_eq!(s.kind, EntryKind::Link);
                assert_eq!(
                    s.link_target.as_deref(),
                    Some(std::path::Path::new("target"))
                );
                assert!(s.metadata_plan(Some(0o777), None).is_err());
                fs::remove_file(&path).unwrap();
                fs::remove_file(&target).unwrap();
                continue;
            }
            let mode =
                case["mode"].as_u64().unwrap() as u32 | case["special"].as_u64().unwrap() as u32;
            // Keep a descriptor for fixture cleanup/read-back even if mode becomes 000.
            let file = fs::File::open(&path).unwrap();
            let atime = UNIX_EPOCH + Duration::from_secs(1_500_000_000);
            let mtime = time_from_millis(case["mtime_ms"].as_i64().unwrap()).unwrap();
            file.set_times(fs::FileTimes::new().set_accessed(atime).set_modified(mtime))
                .unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
            let s = snapshot(&path);
            assert_eq!(s.uid, Some(fs::metadata(&path).unwrap().uid()));
            assert_eq!(s.gid, Some(fs::metadata(&path).unwrap().gid()));
            if kind == "hardlink" {
                assert_eq!(s.links, Some(2));
            }
            let changed = time_from_millis(case["new_mtime_ms"].as_i64().unwrap()).unwrap();
            let newmode = case["new_mode"].as_u64().unwrap() as u32;
            // Time plans require a readable handle before any mutation; owner chmod alone still works on mode 000.
            if mode & 0o400 == 0 {
                let result = s
                    .metadata_plan(Some(newmode), Some(changed))
                    .unwrap()
                    .apply(&JobControl::default());
                assert!(result.is_err());
                assert_eq!(snapshot(&path).mode, s.mode);
                fs::set_permissions(&path, fs::Permissions::from_mode(mode | 0o400)).unwrap();
            }
            let s = snapshot(&path);
            let r = s
                .metadata_plan(Some(newmode), Some(changed))
                .unwrap()
                .apply(&JobControl::default())
                .unwrap();
            assert!(r.steps.iter().all(|s| s.changed));
            let m = file.metadata().unwrap();
            assert_eq!(
                m.permissions().mode() & 0o7777,
                (s.mode.unwrap() & 0o7000) | newmode
            );
            assert_eq!(m.accessed().unwrap(), atime);
            assert_eq!(m.modified().unwrap(), changed);
            if kind == "hardlink" {
                assert_eq!(fs::metadata(&target).unwrap().modified().unwrap(), changed);
            }
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            if kind != "folder" {
                assert_eq!(fs::read(&path).unwrap(), vec![7; size]);
                fs::remove_file(&path).unwrap();
            } else {
                fs::remove_dir(&path).unwrap();
            }
            if target.exists() {
                fs::remove_file(&target).unwrap();
            }
        }
    }
}
#[cfg(unix)]
#[test]
fn stale_plans_cancel_and_partial_steps_never_modify_a_replacement() {
    let t = Tree::new();
    let p = t.0.join("entry");
    fs::write(&p, b"original").unwrap();
    let alias = t.0.join("alias");
    fs::hard_link(&p, &alias).unwrap();
    let s = snapshot(&p);
    let newtime = UNIX_EPOCH + Duration::from_secs(1_600_000_000);
    let path = p.clone();
    let control = JobControl::with_observer(move |_, progress| {
        if progress.completed_items == 1 {
            fs::remove_file(&path).unwrap();
            fs::write(&path, b"replacement").unwrap();
        }
    });
    let report = s
        .metadata_plan(Some(0o640), Some(newtime))
        .unwrap()
        .apply(&control)
        .unwrap();
    assert!(report.steps[0].changed);
    assert!(!report.steps[1].changed);
    assert_eq!(fs::metadata(&alias).unwrap().mode() & 0o777, 0o640);
    assert_ne!(fs::metadata(&alias).unwrap().modified().unwrap(), newtime);
    assert_eq!(fs::read(&p).unwrap(), b"replacement");
    assert!(
        s.metadata_plan(Some(0o777), None)
            .unwrap()
            .apply(&JobControl::default())
            .is_err()
    );
    let fresh = snapshot(&p);
    let control = JobControl::with_observer(|c, p| {
        if p.completed_items == 1 && p.state == JobState::Running {
            c.cancel();
        }
    });
    let r = fresh
        .metadata_plan(Some(0o600), Some(newtime))
        .unwrap()
        .apply(&control)
        .unwrap();
    assert!(r.cancelled);
    assert!(r.steps[0].changed);
    assert!(!r.steps[1].changed);
    assert_eq!(fs::metadata(&p).unwrap().mode() & 0o777, 0o600);
}
#[cfg(unix)]
#[test]
fn folder_totals_include_hidden_and_hardlinked_names_but_not_link_targets() {
    let t = Tree::new();
    fs::create_dir_all(t.0.join("sub/.hidden")).unwrap();
    fs::write(t.0.join("one"), b"123").unwrap();
    fs::hard_link(t.0.join("one"), t.0.join("two")).unwrap();
    fs::write(t.0.join("sub/.hidden/data"), b"12345").unwrap();
    std::os::unix::fs::symlink(".", t.0.join("cycle")).unwrap();
    std::os::unix::fs::symlink("absent", t.0.join("broken")).unwrap();
    let r = folder_size(&t.0, &JobControl::default()).unwrap();
    assert!(!r.partial());
    assert_eq!((r.bytes, r.files, r.folders, r.links), (11, 3, 2, 2));
    let control = JobControl::with_observer(|c, p| {
        if p.completed_items == 2 && p.state == JobState::Running {
            c.cancel()
        }
    });
    assert!(folder_size(&t.0, &control).unwrap().cancelled);
    let r = folder_size_with_limits(&t.0, &JobControl::default(), 1, 1_000_000).unwrap();
    assert!(r.partial());
    let r = folder_size_with_limits(&t.0, &JobControl::default(), 256, 2).unwrap();
    assert!(r.partial());
    assert_eq!(r.visited, 2);
    assert!(folder_size(&t.0.join("cycle"), &JobControl::default()).is_err());
}
#[cfg(unix)]
#[test]
fn denied_folder_and_metadata_handles_are_visible_without_elevation() {
    let t = Tree::new();
    let denied = t.0.join("denied");
    fs::create_dir(&denied).unwrap();
    fs::write(denied.join("retained"), b"x").unwrap();
    fs::set_permissions(&denied, fs::Permissions::from_mode(0o000)).unwrap();
    let r = folder_size(&t.0, &JobControl::default()).unwrap();
    fs::set_permissions(&denied, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(r.partial(), "Run permission checks unprivileged");
    assert_eq!(r.errors, 1);
    let file = t.0.join("no-read-access");
    fs::write(&file, b"unchanged").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o000)).unwrap();
    let s = snapshot(&file);
    let result = s
        .metadata_plan(Some(0o777), Some(UNIX_EPOCH + Duration::from_secs(1000)))
        .unwrap()
        .apply(&JobControl::default());
    assert!(result.is_err());
    assert_eq!(snapshot(&file).mode, s.mode);
    assert_eq!(snapshot(&file).modified, s.modified);
    fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(fs::read(file).unwrap(), b"unchanged");
}
#[test]
fn zip_properties_never_borrow_host_ownership_or_implicit_dates() {
    let archive = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/W09/open.zip");
    let p = PropertySnapshot::capture(EntryTarget::Zip {
        location: ZipLocation {
            archive: archive.clone(),
            prefix: "nested/".into(),
        },
        name: "- café & report.txt".into(),
    })
    .unwrap();
    assert_eq!(p.kind, EntryKind::File);
    assert!(p.size.is_some());
    assert!(p.modified.is_some());
    assert!(p.uid.is_none() && p.gid.is_none() && p.mode.is_none() && p.accessed.is_none());
    assert!(p.metadata_plan(Some(0o777), None).is_err());
}
