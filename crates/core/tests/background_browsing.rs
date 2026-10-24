//! Commit-on-success and stale-result isolation without a GUI or live services.
use mucrab_core::{BrowserPane, JobControl, NameSorter};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

struct Tree(PathBuf);
impl Tree {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "mucrab-browse-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::create_dir(path.join("child")).unwrap();
        fs::write(path.join("visible.txt"), b"source").unwrap();
        fs::write(path.join(".hidden"), b"hidden").unwrap();
        Self(path)
    }
}
impl Drop for Tree {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn pending_and_failed_navigation_preserve_committed_state_and_history() {
    let t = Tree::new();
    let sorter = NameSorter::en_us().unwrap();
    let mut pane = BrowserPane::default();
    assert!(pane.open(&t.0, &sorter));
    pane.mark(1, true);
    let before = pane.selection().snapshot(pane.entries(), pane.order());
    let history = pane.history_len();
    pane.defer_browsing();
    assert!(!pane.open(&t.0.join("missing"), &sorter));
    assert_eq!(pane.location(), Some(&t.0));
    let request = pane.take_browse_request().unwrap();
    assert!(request.prepare(&JobControl::default()).is_err());
    assert_eq!(pane.history_len(), history);
    assert_eq!(
        pane.selection()
            .snapshot(pane.entries(), pane.order())
            .marked,
        before.marked
    );
}

#[test]
fn superseded_preparation_cannot_commit_and_changed_options_are_reprepared() {
    let t = Tree::new();
    let sorter = NameSorter::en_us().unwrap();
    let mut pane = BrowserPane::default();
    pane.defer_browsing();
    pane.open(&t.0, &sorter);
    let old = pane
        .take_browse_request()
        .unwrap()
        .prepare(&JobControl::default())
        .unwrap();
    pane.open(&t.0.join("child"), &sorter);
    assert!(!pane.commit_browse(old));
    let child = pane
        .take_browse_request()
        .unwrap()
        .prepare(&JobControl::default())
        .unwrap();
    assert!(pane.commit_browse(child));
    assert_eq!(pane.history_len(), 1);
    pane.parent(&sorter);
    let prepared = pane
        .take_browse_request()
        .unwrap()
        .prepare(&JobControl::default())
        .unwrap();
    pane.show_hidden = true;
    assert!(!pane.commit_browse(prepared));
    let current = pane
        .take_browse_request()
        .unwrap()
        .prepare(&JobControl::default())
        .unwrap();
    assert!(pane.commit_browse(current));
    assert!(pane.entries().iter().any(|e| e.name == ".hidden"));
    assert_eq!(pane.history_len(), 2);
    pane.traverse_history(false, &sorter);
    let prepared = pane
        .take_browse_request()
        .unwrap()
        .prepare(&JobControl::default())
        .unwrap();
    assert!(pane.commit_browse(prepared));
    assert_eq!(pane.location(), Some(&t.0.join("child")));
    assert_eq!(
        pane.history_len(),
        2,
        "history traversal must not append entries"
    );
}

#[test]
fn cancellation_and_remote_transition_reject_old_local_preparation() {
    let t = Tree::new();
    let sorter = NameSorter::en_us().unwrap();
    let mut pane = BrowserPane::default();
    pane.defer_browsing();
    pane.open(&t.0, &sorter);
    let cancelled = pane.take_browse_request().unwrap();
    let control = JobControl::default();
    control.cancel();
    assert!(cancelled.prepare(&control).is_err());
    pane.open(&t.0, &sorter);
    let old = pane
        .take_browse_request()
        .unwrap()
        .prepare(&JobControl::default())
        .unwrap();
    pane.request_remote(
        mucrab_core::sftp::RemoteLocation::new("server", "/").unwrap(),
        true,
        None,
    );
    assert!(!pane.commit_browse(old));
    assert!(pane.location().is_none());
}

#[test]
fn deferred_zip_navigation_and_refresh_preserve_focus_and_marked_identities() {
    use std::io::Write;
    let t = Tree::new();
    let archive = t.0.join("preview.zip");
    let mut zip = zip::ZipWriter::new(fs::File::create(&archive).unwrap());
    for name in ["folder/first.txt", "folder/second.txt"] {
        zip.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"archive source").unwrap();
    }
    zip.finish().unwrap();
    let sorter = NameSorter::en_us().unwrap();
    let mut pane = BrowserPane::default();
    pane.defer_browsing();
    pane.open(&archive.join("folder"), &sorter);
    let prepared = pane
        .take_browse_request()
        .unwrap()
        .prepare(&JobControl::default())
        .unwrap();
    assert!(pane.commit_browse(prepared));
    assert!(matches!(
        pane.current_location(),
        Some(mucrab_core::PaneLocation::Zip(_))
    ));
    pane.focus_name("second.txt");
    pane.mark(1, true);
    let before = pane.selection().snapshot(pane.entries(), pane.order());
    pane.refresh(&sorter);
    let prepared = pane
        .take_browse_request()
        .unwrap()
        .prepare(&JobControl::default())
        .unwrap();
    assert!(pane.commit_browse(prepared));
    let after = pane.selection().snapshot(pane.entries(), pane.order());
    assert_eq!(after.focused, before.focused);
    assert_eq!(after.marked, before.marked);
    assert_eq!(pane.history_len(), 1, "refresh must not append ZIP history");
}

#[test]
fn explicit_listing_commit_supersedes_an_older_worker_result() {
    let t = Tree::new();
    let sorter = NameSorter::en_us().unwrap();
    let mut pane = BrowserPane::default();
    pane.defer_browsing();
    pane.open(&t.0.join("child"), &sorter);
    let prepared = pane
        .take_browse_request()
        .unwrap()
        .prepare(&JobControl::default())
        .unwrap();
    pane.load_listing(t.0.clone(), Vec::new(), true, &sorter);
    assert!(!pane.commit_browse(prepared));
    assert_eq!(pane.location(), Some(&t.0));
    assert!(pane.entries().is_empty());
}
