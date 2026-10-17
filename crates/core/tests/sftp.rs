// SPDX-License-Identifier: GPL-3.0-only
use mucrab_core::{CopyPolicy, JobControl, sftp::*};
use std::{fs, path::PathBuf, sync::Arc};
fn scratch(label: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("mucrab-sftp-test-{}-{label}", std::process::id()));
    if p.exists() {
        fs::remove_dir_all(&p).unwrap();
    }
    fs::create_dir(&p).unwrap();
    p
}
fn profile() -> Profile {
    Profile {
        id: "fixture".into(),
        label: "Disposable".into(),
        host: "127.0.0.1".into(),
        port: 22,
        user: "fixture".into(),
        authentication: Authentication::Password,
        directory: "/".into(),
        remembered: false,
        credential_context: None,
    }
}
#[test]
fn typed_locations_and_nonsecret_atomic_store_reject_unsafe_configuration() {
    let dir = scratch("store");
    let path = dir.join("sftp.json");
    let mut store = ConnectionStore::load(path.clone()).unwrap();
    let p = profile();
    store.save_profile(p.clone()).unwrap();
    assert_eq!(store.profiles(), &[p]);
    for s in [
        "sftp://user:secret@host/a",
        "sftp://user@host/a",
        "https://host/a",
        "sftp://bad%id/a",
        "sftp://fixture/path%zz",
    ] {
        assert!(RemoteLocation::parse(s).is_err());
    }
    let loc = RemoteLocation::new("fixture", "/a/../café %?#/file.txt").unwrap();
    assert_eq!(RemoteLocation::parse(&loc.display()).unwrap(), loc);
    assert_eq!(loc.parent().path, "/café %?#");
    let key = HostIdentity {
        endpoint: "[127.0.0.1]:22".into(),
        algorithm: "Ed25519".into(),
        fingerprint: "SHA256:test".into(),
    };
    assert!(!store.trust_status(&key).unwrap());
    store.trust(key.clone()).unwrap();
    assert!(store.trust_status(&key).unwrap());
    let changed = HostIdentity {
        fingerprint: "SHA256:changed".into(),
        ..key.clone()
    };
    assert!(store.trust_status(&changed).is_err());
    assert!(store.trust(changed).is_err());
    store.forget_host(&key.endpoint).unwrap();
    assert!(!store.trust_status(&key).unwrap());
    fs::write(&path, b"external edit").unwrap();
    assert!(store.save_profile(profile()).is_err());
    assert_eq!(fs::read(&path).unwrap(), b"external edit");
    fs::remove_dir_all(dir).unwrap();
}
#[test]
fn remote_navigation_only_commits_matching_results_and_preserves_local_history() {
    use mucrab_core::{BrowserPane, FileEntry, NameSorter, PaneLocation};
    let dir = scratch("nav");
    let sorter = NameSorter::en_us().unwrap();
    let mut pane = BrowserPane::default();
    assert!(pane.open(&dir, &sorter));
    let original = pane.current_location();
    pane.path_input = "sftp://user:private-secret@host/a".into();
    assert!(!pane.open(&PathBuf::from(pane.path_input.clone()), &sorter));
    assert!(!pane.path_input.contains("private-secret"));
    assert!(!pane.error.as_ref().unwrap().contains("private-secret"));
    let loc = RemoteLocation::new("fixture", "/folder").unwrap();
    pane.request_remote(loc.clone(), true, None);
    let old = pane.remote_request.clone().unwrap();
    pane.request_remote(loc, true, None);
    let request = pane.remote_request.clone().unwrap();
    assert!(!pane.remote_loaded(&old, vec![], &sorter));
    assert_eq!(pane.current_location(), original);
    assert!(pane.remote_loaded(
        &request,
        vec![FileEntry {
            name: "x".into(),
            ..Default::default()
        }],
        &sorter
    ));
    assert!(pane.location.is_none());
    assert!(matches!(
        pane.current_location(),
        Some(PaneLocation::Sftp(_))
    ));
    assert!(pane.can_back());
    assert!(pane.traverse_history(false, &sorter));
    assert_eq!(pane.current_location(), original);
    fs::remove_dir_all(dir).unwrap();
}
fn server() -> (PathBuf, Profile) {
    let root =
        PathBuf::from(std::env::var_os("MUCRAB_W15_RUN").expect("test-only server run path"));
    let endpoint: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("endpoint.json")).unwrap()).unwrap();
    let mut p = profile();
    p.port = endpoint["port"].as_u64().unwrap() as u16;
    (root, p)
}
fn login(p: Profile, secret: &str) -> Arc<RemoteSession> {
    let c = JobControl::default();
    let h = Handshake::connect(&p, &c).unwrap();
    let key = h.identity.clone();
    h.authenticate(p, &key, secret, &c).unwrap()
}
#[test]
#[ignore = "requires a real disposable loopback SFTP server; tools/w15_prepare.py + w15_server.py"]
fn real_sftp_auth_list_transfer_collision_links_cancel_and_retained_open() {
    let (root, p) = server();
    let c = JobControl::default();
    let h = Handshake::connect(&p, &c).unwrap();
    let identity = h.identity.clone();
    assert!(
        h.authenticate(p.clone(), &identity, "wrong-disposable-secret", &c)
            .is_err()
    );
    let session = login(p.clone(), "w15-disposable-only");
    let location = RemoteLocation::new("fixture", "/").unwrap();
    let entries = session.list(&location, &c).unwrap();
    assert!(
        entries
            .iter()
            .any(|e| e.name == "notes.txt" && !e.is_symlink)
    );
    assert!(entries.iter().any(|e| e.name == "link" && e.is_symlink));
    let mut key = p.clone();
    key.authentication = Authentication::PrivateKey(root.join("client.key"));
    login(key, "").disconnect();
    let mut encrypted = p.clone();
    encrypted.authentication = Authentication::PrivateKey(root.join("encrypted.key"));
    login(encrypted, "w15-key-passphrase").disconnect();
    let recipes: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/W15-sftp-connections/cases.json"
    ))
    .unwrap();
    let reserved: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/W15-sftp-connections/reserved.json"
    ))
    .unwrap();
    for case in recipes.iter().chain(reserved.iter()) {
        let name = case["name"].as_str().unwrap();
        let len = case["size"].as_u64().unwrap() as usize;
        let content: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
        let upload = case["direction"] == "upload";
        let policy = if case["policy"] == "skip" {
            CopyPolicy::Skip
        } else {
            CopyPolicy::Overwrite
        };
        let existing = case["existing"].as_bool().unwrap();
        let local = root.join("local");
        let remote = root.join("remote");
        let (src, dst) = if upload {
            (&local, &remote)
        } else {
            (&remote, &local)
        };
        fs::write(src.join(name), &content).unwrap();
        if dst.join(name).exists() {
            fs::remove_file(dst.join(name)).unwrap();
        }
        if existing {
            fs::write(dst.join(name), b"prior destination").unwrap();
        }
        let endpoint = Endpoint::Remote {
            session: session.clone(),
            location: location.clone(),
        };
        let (source, destination) = if upload {
            (Endpoint::Local(local.clone()), endpoint)
        } else {
            (endpoint, Endpoint::Local(local.clone()))
        };
        let plan =
            TransferPlan::capture(source, destination, vec![name.into()], policy, &c).unwrap();
        let r = plan.apply(&c).unwrap();
        let skipped = existing && policy == CopyPolicy::Skip;
        assert_eq!(
            r.outcomes[0].success, !skipped,
            "case {:?}, issues {:?}",
            case, r.issues
        );
        assert_eq!(fs::read(src.join(name)).unwrap(), content);
        assert_eq!(
            fs::read(dst.join(name)).unwrap(),
            if skipped {
                b"prior destination".to_vec()
            } else {
                content
            }
        );
    }
    // Reads on the same lease must remain usable while transfer pause waits.
    let paused_name = "pause-concurrent.bin";
    fs::write(root.join("local").join(paused_name), vec![5u8; 256 * 1024]).unwrap();
    let remote_path = root.join("remote").join(paused_name);
    let _ = fs::remove_file(&remote_path);
    let pause_plan = TransferPlan::capture(
        Endpoint::Local(root.join("local")),
        Endpoint::Remote {
            session: session.clone(),
            location: location.clone(),
        },
        vec![paused_name.into()],
        CopyPolicy::Skip,
        &c,
    )
    .unwrap();
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let once = std::sync::atomic::AtomicBool::new(false);
    let pause_control = JobControl::with_observer(move |ctl, p| {
        if p.state == mucrab_core::JobState::Running
            && p.transferred_bytes >= 32768
            && !once.swap(true, std::sync::atomic::Ordering::AcqRel)
        {
            ctl.set_paused(true);
            let _ = tx.send(());
        }
    });
    let worker_control = pause_control.clone();
    let worker = std::thread::spawn(move || pause_plan.apply(&worker_control));
    rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
    let start = std::time::Instant::now();
    assert!(!session.list(&location, &c).unwrap().is_empty());
    assert!(start.elapsed() < std::time::Duration::from_secs(3));
    pause_control.cancel();
    assert!(worker.join().unwrap().unwrap().cancelled);
    let remote = Endpoint::Remote {
        session: session.clone(),
        location: location.clone(),
    };
    let local = root.join("download-tree");
    if local.exists() {
        fs::remove_dir_all(&local).unwrap();
    }
    fs::create_dir_all(&local).unwrap();
    let plan = TransferPlan::capture(
        remote.clone(),
        Endpoint::Local(local.clone()),
        vec!["folder".into()],
        CopyPolicy::Skip,
        &c,
    )
    .unwrap();
    assert!(plan.apply(&c).unwrap().outcomes[0].success);
    assert_eq!(
        fs::read(local.join("folder/.hidden/data")).unwrap(),
        b"hidden"
    );
    let link = TransferPlan::capture(
        remote,
        Endpoint::Local(local),
        vec!["link".into()],
        CopyPolicy::Overwrite,
        &c,
    )
    .unwrap();
    assert!(!link.apply(&c).unwrap().issues.is_empty());
    let opened = download_open(&session, &location.child("notes.txt").unwrap(), &c).unwrap();
    assert_eq!(
        fs::read(&opened).unwrap(),
        fs::read(root.join("remote/notes.txt")).unwrap()
    );
    session.disconnect();
    assert!(opened.exists());
    assert!(session.list(&location, &c).is_err());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&opened).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    fs::remove_dir_all(opened.parent().unwrap()).unwrap();
}

#[test]
#[ignore = "requires disposable SFTP server; run ignored SFTP tests with --test-threads=1"]
fn real_sftp_failure_boundaries_preserve_sources_destinations_and_owned_partials() {
    use mucrab_core::JobState;
    use std::{
        io::Write,
        thread,
        time::{Duration, Instant},
    };
    let (root, p) = server();
    let session = login(p.clone(), "w15-disposable-only");
    let loc = RemoteLocation::new("fixture", "/").unwrap();
    let local = root.join("local");
    let remote = root.join("remote");
    let c = JobControl::default();
    let name = "failure-data.bin";
    let data = vec![0x57; 2 * 1024 * 1024];
    fs::write(local.join(name), &data).unwrap();
    fs::write(remote.join(name), b"keep existing").unwrap();
    let endpoints = || {
        (
            Endpoint::Local(local.clone()),
            Endpoint::Remote {
                session: session.clone(),
                location: loc.clone(),
            },
        )
    };
    let (a, b) = endpoints();
    let stale = TransferPlan::capture(a, b, vec![name.into()], CopyPolicy::Overwrite, &c).unwrap();
    fs::write(remote.join(name), b"changed after review").unwrap();
    let report = stale.apply(&c).unwrap();
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.message.contains("changed after review"))
    );
    assert_eq!(
        fs::read(remote.join(name)).unwrap(),
        b"changed after review"
    );
    fs::write(root.join("no-posix"), b"1").unwrap();
    let (a, b) = endpoints();
    let no_replace =
        TransferPlan::capture(a, b, vec![name.into()], CopyPolicy::Overwrite, &c).unwrap();
    let r = no_replace.apply(&c).unwrap();
    assert!(r.issues.iter().any(|i| i.message.contains("POSIX rename")));
    assert_eq!(
        fs::read(remote.join(name)).unwrap(),
        b"changed after review"
    );
    fs::remove_file(root.join("no-posix")).unwrap();
    let (a, b) = endpoints();
    let plan = TransferPlan::capture(a, b, vec![name.into()], CopyPolicy::Overwrite, &c).unwrap();
    let cancelled = JobControl::with_observer(|ctl, p| {
        if p.state == JobState::Running && p.transferred_bytes >= 32768 {
            ctl.cancel();
        }
    });
    let report = plan.apply(&cancelled).unwrap();
    assert!(report.cancelled);
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.message.contains("Owned remote stage"))
    );
    assert_eq!(
        fs::read(remote.join(name)).unwrap(),
        b"changed after review"
    );
    assert_eq!(fs::read(local.join(name)).unwrap(), data);
    assert!(
        fs::read_dir(&remote)
            .unwrap()
            .filter_map(Result::ok)
            .any(|p| p
                .file_name()
                .to_string_lossy()
                .starts_with("failure-data.bin.mucrab-stage-"))
    );
    fs::write(remote.join(name), &data).unwrap();
    fs::write(local.join(name), b"local prior").unwrap();
    let plan = TransferPlan::capture(
        Endpoint::Remote {
            session: session.clone(),
            location: loc.clone(),
        },
        Endpoint::Local(local.clone()),
        vec![name.into()],
        CopyPolicy::Overwrite,
        &c,
    )
    .unwrap();
    let cancelled = JobControl::with_observer(|ctl, p| {
        if p.state == JobState::Running && p.transferred_bytes >= 32768 {
            ctl.cancel();
        }
    });
    assert!(plan.apply(&cancelled).unwrap().cancelled);
    assert_eq!(fs::read(local.join(name)).unwrap(), b"local prior");
    assert!(
        fs::read_dir(&local)
            .unwrap()
            .filter_map(Result::ok)
            .any(|p| p
                .file_name()
                .to_string_lossy()
                .starts_with(".mucrab-stage-"))
    );
    let link_dir = remote.join("directory-link");
    if !link_dir.exists() {
        std::os::unix::fs::symlink("folder", &link_dir).unwrap();
    }
    assert!(
        session
            .list(&loc.child("directory-link").unwrap(), &c)
            .is_err()
    );
    assert!(session.list(&loc.child("denied").unwrap(), &c).is_err());
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let bad = remote.join(std::ffi::OsStr::from_bytes(b"non-unicode-\xff"));
        fs::write(&bad, b"data").unwrap();
        assert!(session.list(&loc, &c).is_err());
        assert_eq!(fs::read(&bad).unwrap(), b"data");
        fs::remove_file(bad).unwrap();
    }
    // A replaced source cannot launch and an oversized sparse source is rejected before download.
    let huge = remote.join("oversized.bin");
    let mut f = fs::File::create(&huge).unwrap();
    f.write_all(b"a").unwrap();
    f.set_len(512 * 1024 * 1024 + 1).unwrap();
    assert!(download_open(&session, &loc.child("oversized.bin").unwrap(), &c).is_err());
    fs::remove_file(huge).unwrap();
    let start = Instant::now();
    assert!(session.list(&loc.child("slow").unwrap(), &c).is_err());
    assert!(start.elapsed() < Duration::from_secs(9));
    let session2 = login(p.clone(), "w15-disposable-only");
    let background = session2.clone();
    let slow = loc.child("slow").unwrap();
    let start = Instant::now();
    let worker = thread::spawn(move || background.list(&slow, &JobControl::default()));
    thread::sleep(Duration::from_millis(100));
    session2.disconnect();
    assert!(worker.join().unwrap().is_err());
    assert!(start.elapsed() < Duration::from_secs(2));
    // Server commits, then loses the acknowledgment: the client must report uncertainty, never retry/delete-first.
    let session3 = login(p, "w15-disposable-only");
    fs::write(local.join("ambiguous.txt"), b"new committed bytes").unwrap();
    fs::write(remote.join("ambiguous.txt"), b"old destination").unwrap();
    let plan = TransferPlan::capture(
        Endpoint::Local(local.clone()),
        Endpoint::Remote {
            session: session3.clone(),
            location: loc,
        },
        vec!["ambiguous.txt".into()],
        CopyPolicy::Overwrite,
        &c,
    )
    .unwrap();
    fs::write(root.join("ambiguous"), b"1").unwrap();
    let result = plan.apply(&c).unwrap();
    fs::remove_file(root.join("ambiguous")).unwrap();
    assert!(
        result
            .issues
            .iter()
            .any(|i| i.message.contains("uncertain"))
    );
    assert_eq!(
        fs::read(remote.join("ambiguous.txt")).unwrap(),
        b"new committed bytes"
    );
    assert_eq!(
        fs::read(local.join("ambiguous.txt")).unwrap(),
        b"new committed bytes"
    );
    session.disconnect();
    session3.disconnect();

    // Loss after a successful chunk but before publication preserves the destination.
    let lost = login(session.profile.clone(), "w15-disposable-only");
    let lost_path = local.join("network-loss.bin");
    fs::write(&lost_path, &data).unwrap();
    fs::write(
        remote.join("network-loss.bin"),
        b"prior network destination",
    )
    .unwrap();
    let plan = TransferPlan::capture(
        Endpoint::Local(local.clone()),
        Endpoint::Remote {
            session: lost.clone(),
            location: RemoteLocation::new("fixture", "/").unwrap(),
        },
        vec!["network-loss.bin".into()],
        CopyPolicy::Overwrite,
        &c,
    )
    .unwrap();
    let interrupt = lost.clone();
    let control = JobControl::with_observer(move |_, p| {
        if p.transferred_bytes >= 32768 {
            interrupt.disconnect();
        }
    });
    let failed = plan.apply(&control).unwrap();
    assert!(!failed.outcomes[0].success);
    assert!(
        failed
            .issues
            .iter()
            .any(|i| i.message.contains("Owned remote stage"))
    );
    assert_eq!(
        fs::read(remote.join("network-loss.bin")).unwrap(),
        b"prior network destination"
    );
    assert_eq!(fs::read(lost_path).unwrap(), data);
    // A source changed by another writer during Open must never yield a launchable copy.
    let changed = login(session.profile.clone(), "w15-disposable-only");
    let source = remote.join("open-changed.bin");
    fs::write(&source, vec![9u8; 131072]).unwrap();
    let once = std::sync::atomic::AtomicBool::new(false);
    let control = JobControl::with_observer(move |_, p| {
        if p.transferred_bytes >= 32768 && !once.swap(true, std::sync::atomic::Ordering::AcqRel) {
            fs::OpenOptions::new()
                .write(true)
                .open(&source)
                .unwrap()
                .set_len(32769)
                .unwrap();
        }
    });
    assert!(
        download_open(
            &changed,
            &RemoteLocation::new("fixture", "/open-changed.bin").unwrap(),
            &control
        )
        .is_err()
    );
    assert!(
        !std::env::temp_dir()
            .read_dir()
            .unwrap()
            .filter_map(Result::ok)
            .any(|e| e
                .file_name()
                .to_string_lossy()
                .starts_with(&format!("mucrab-sftp-open-{}-", std::process::id())))
    );
    changed.disconnect();
}

#[test]
#[ignore = "requires isolated DBus/Secret Service fixture; do not run against personal credential stores"]
fn isolated_native_credential_roundtrip_or_unavailable_has_no_plaintext_fallback() {
    let mode = std::env::var("MUCRAB_W15_CREDENTIAL_FIXTURE")
        .expect("explicit isolated credential fixture mode");
    let id = format!("w15-disposable-{}", std::process::id());
    if mode == "available" {
        save_secret(&id, "disposable-native-secret").unwrap();
        assert_eq!(
            load_secret(&id).unwrap().as_str(),
            "disposable-native-secret"
        );
        delete_secret(&id).unwrap();
        assert!(load_secret(&id).is_err());
    } else {
        assert_eq!(mode, "unavailable");
        assert!(save_secret(&id, "disposable-native-secret").is_err());
        assert!(load_secret(&id).is_err());
    }
}

#[test]
fn profiles_reject_secret_fields_invalid_endpoints_and_preserve_private_config() {
    let dir = scratch("validation");
    let path = dir.join("sftp.json");
    let mut store = ConnectionStore::load(path.clone()).unwrap();
    let valid = profile();
    for host in ["", "user@host", "https://host", "host/path", "host\nname"] {
        let mut p = valid.clone();
        p.host = host.into();
        assert!(p.validate().is_err());
    }
    let mut p = valid.clone();
    p.port = 0;
    assert!(p.validate().is_err());
    p = valid.clone();
    p.authentication = Authentication::PrivateKey("relative.key".into());
    assert!(p.validate().is_err());
    store.save_profile(valid.clone()).unwrap();
    let mut first = valid.clone();
    first.id = "server-1".into();
    store.save_profile(first).unwrap();
    let mut third = valid;
    third.id = "server-3".into();
    store.save_profile(third).unwrap();
    assert_eq!(store.unused_profile_id(), "server-2");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    let mut doc: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    doc["profiles"][0]["password"] = serde_json::Value::String("must-not-load".into());
    fs::write(&path, serde_json::to_vec(&doc).unwrap()).unwrap();
    assert!(ConnectionStore::load(path).is_err());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn remote_bookmarks_and_duplicated_tabs_keep_structured_history_and_marks_isolated() {
    use mucrab_core::{BookmarkStore, BrowserPane, FileEntry, NameSorter, PaneLocation, TabStack};
    let dir = scratch("tabs");
    let sorter = NameSorter::en_us().unwrap();
    let mut pane = BrowserPane::default();
    pane.open(&dir, &sorter);
    let loc = RemoteLocation::new("fixture", "/folder").unwrap();
    pane.request_remote(loc.clone(), true, None);
    let request = pane.remote_request.clone().unwrap();
    pane.remote_loaded(
        &request,
        vec![FileEntry {
            name: "focused.txt".into(),
            ..Default::default()
        }],
        &sorter,
    );
    pane.selection.set_marked(1, true, &pane.order);
    let mut tabs = TabStack::new(pane);
    let original = tabs.active().id;
    let duplicate = tabs.duplicate(&sorter).unwrap();
    assert_eq!(tabs.active().pane.remote.as_ref(), Some(&loc));
    assert_eq!(tabs.active().pane.selection.marked_count(), 0);
    assert_eq!(tabs.active().pane.history_len(), 2);
    tabs.select(original);
    assert_eq!(tabs.active().pane.selection.marked_count(), 1);
    assert!(tabs.close(duplicate));
    let mut bookmarks = BookmarkStore::load(dir.join("bookmarks.json")).unwrap();
    bookmarks
        .add("Remote", PaneLocation::Sftp(loc.clone()))
        .unwrap();
    drop(bookmarks);
    let loaded = BookmarkStore::load(dir.join("bookmarks.json")).unwrap();
    assert_eq!(loaded.entries()[0].location, PaneLocation::Sftp(loc));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn remembered_credentials_do_not_cross_connection_identity_changes() {
    let mut p = profile();
    p.remembered = true;
    p.credential_context = Some(p.credential_context_key());
    assert!(p.saved_credential_matches());
    let mut cosmetic = p.clone();
    cosmetic.label = "Renamed".into();
    cosmetic.directory = "/other".into();
    assert!(cosmetic.saved_credential_matches());
    let mut changed = p.clone();
    changed.host = "other.example".into();
    assert!(!changed.saved_credential_matches());
    changed = p.clone();
    changed.port = 2222;
    assert!(!changed.saved_credential_matches());
    changed = p.clone();
    changed.user = "other-user".into();
    assert!(!changed.saved_credential_matches());
    changed = p;
    changed.authentication = Authentication::PrivateKey("/disposable/key".into());
    assert!(!changed.saved_credential_matches());
}
