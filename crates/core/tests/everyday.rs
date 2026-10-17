// SPDX-License-Identifier: GPL-3.0-only
use mucrab_core::*;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
struct Tree(PathBuf);
impl Tree {
    fn new() -> Self {
        static ID: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "mucrab-w10-{}-{}",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn dir(&self, n: &str) -> PathBuf {
        let p = self.0.join(n);
        fs::create_dir_all(&p).unwrap();
        p
    }
    fn file(&self, n: &str) -> PathBuf {
        let p = self.0.join(n);
        fs::write(&p, n).unwrap();
        p
    }
}
impl Drop for Tree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn folder_creation_rejects_invalid_collisions_and_stale_review() {
    let t = Tree::new();
    let pin = PinnedDirectory::new(&t.0).unwrap();
    for n in ["New folder", "café 中", ".hidden", "- leading"] {
        assert!(pin.create_folder(n, &JobControl::default()).unwrap());
    }
    for n in ["", ".", "..", "a/b", "New folder"] {
        assert!(pin.create_folder(n, &JobControl::default()).is_err());
    }
    let cancelled = JobControl::default();
    cancelled.cancel();
    assert!(!pin.create_folder("cancelled", &cancelled).unwrap());
    assert!(!t.0.join("cancelled").exists());
    let d = t.dir("reviewed");
    let stale = PinnedDirectory::new(&d).unwrap();
    fs::rename(&d, t.0.join("old")).unwrap();
    fs::create_dir(&d).unwrap();
    assert!(stale.create_folder("new", &JobControl::default()).is_err());
}
#[test]
fn deletion_is_reviewed_and_never_falls_back_from_trash() {
    let t = Tree::new();
    let p = t.file("document");
    let plan = DeletePlan::new(std::slice::from_ref(&p), &[]).unwrap();
    let mut fail = |_: &Path| Err(std::io::Error::other("Trash unavailable"));
    let r = plan.apply(&JobControl::default(), Some(&mut fail)).unwrap();
    assert!(!r.outcomes[0].success);
    assert!(p.exists());
    assert_eq!(r.issues.len(), 1);
    fs::rename(&p, t.0.join("old")).unwrap();
    fs::write(&p, "replacement").unwrap();
    assert!(plan.apply(&JobControl::default(), None).is_err());
    assert_eq!(fs::read_to_string(p).unwrap(), "replacement");
}
#[cfg(unix)]
#[test]
fn recursive_deletion_includes_hidden_entries_and_unlinks_external_and_broken_links() {
    use std::os::unix::fs::symlink;
    let t = Tree::new();
    let folder = t.dir("selected/nested");
    let outside = t.file("outside");
    fs::write(folder.join(".hidden"), "hidden").unwrap();
    symlink(&outside, folder.join("link")).unwrap();
    symlink("missing", folder.join("broken")).unwrap();
    let p = t.0.join("selected");
    let r = DeletePlan::new(std::slice::from_ref(&p), std::slice::from_ref(&t.0))
        .unwrap()
        .apply(&JobControl::default(), None)
        .unwrap();
    assert!(r.outcomes[0].success);
    assert!(!p.exists());
    assert!(outside.exists());
    let link = t.0.join("link-to-root");
    symlink(&t.0, &link).unwrap();
    assert!(
        DeletePlan::new(std::slice::from_ref(&link), std::slice::from_ref(&t.0))
            .unwrap()
            .apply(&JobControl::default(), None)
            .unwrap()
            .outcomes[0]
            .success
    );
    assert!(outside.exists());
}
#[test]
fn protected_roots_and_cooperative_cancellation_are_preserved() {
    let t = Tree::new();
    let d = t.dir("folder/nested");
    t.file("folder/a");
    t.file("folder/b");
    assert!(DeletePlan::new(&[t.0.join("folder")], &[d]).is_err());
    assert!(DeletePlan::new(&[PathBuf::from("/")], &[]).is_err());
    let control = JobControl::with_observer(|c, p| {
        if p.state == JobState::Running
            && p.current_path
                .as_ref()
                .is_some_and(|p| p.file_name().is_some_and(|n| n == "b"))
        {
            c.cancel();
        }
    });
    let r = DeletePlan::new(&[t.0.join("folder")], &[])
        .unwrap()
        .apply(&control, None)
        .unwrap();
    assert!(r.cancelled);
    assert!(!r.outcomes[0].success);
    assert!(t.0.join("folder/b").exists());
}
#[test]
fn paste_multi_parent_deduplicates_and_preserves_sources_and_collisions() {
    let t = Tree::new();
    t.dir("a");
    t.dir("b");
    let dest = t.dir("dest");
    let a = t.file("a/space café %#.txt");
    let b = t.file("b/other");
    let plan = PastePlan::new(&[a.clone(), b.clone(), a.clone()], &dest).unwrap();
    assert_eq!(plan.paths().len(), 2);
    let c = JobControl::default();
    let r = plan.apply(CopyPolicy::Skip, &c).unwrap();
    assert_eq!(r.outcomes.len(), 2);
    assert!(r.outcomes.iter().all(|o| o.success));
    assert_eq!(c.snapshot().completed_items, 2);
    assert_eq!(c.snapshot().total_items, 2);
    assert!(a.exists() && b.exists());
    fs::write(dest.join("other"), "existing").unwrap();
    plan.apply(CopyPolicy::Skip, &JobControl::default())
        .unwrap();
    assert_eq!(fs::read_to_string(dest.join("other")).unwrap(), "existing");
}
#[test]
fn paste_rejects_overlaps_and_all_stale_sources_before_first_write() {
    let t = Tree::new();
    t.dir("a");
    let dest = t.dir("dest");
    let a = t.file("a/first");
    let b = t.file("a/second");
    assert!(PastePlan::new(std::slice::from_ref(&a), &t.0.join("a")).is_err());
    assert!(PastePlan::new(std::slice::from_ref(&t.0), &dest).is_err());
    let plan = PastePlan::new(&[a, b.clone()], &dest).unwrap();
    fs::rename(&b, t.0.join("old")).unwrap();
    fs::write(&b, "changed").unwrap();
    assert!(
        plan.apply(CopyPolicy::Overwrite, &JobControl::default())
            .is_err()
    );
    assert_eq!(fs::read_dir(dest).unwrap().count(), 0);
}
#[test]
fn paste_rejects_replaced_destination_and_cancelled_jobs_do_not_write() {
    let t = Tree::new();
    t.dir("a");
    let dest = t.dir("dest");
    let p = t.file("a/document");
    let plan = PastePlan::new(std::slice::from_ref(&p), &dest).unwrap();
    let c = JobControl::default();
    c.cancel();
    assert!(plan.apply(CopyPolicy::Skip, &c).unwrap().cancelled);
    assert!(!dest.join("document").exists());
    fs::rename(&dest, t.0.join("old")).unwrap();
    fs::create_dir(&dest).unwrap();
    assert!(
        plan.apply(CopyPolicy::Skip, &JobControl::default())
            .is_err()
    );
}
#[test]
fn history_bounds_branches_duplicates_and_failed_navigation() {
    let t = Tree::new();
    let s = NameSorter::en_us().unwrap();
    let mut pane = BrowserPane::default();
    for name in ["a", "b", "c"] {
        pane.open(&t.dir(name), &s);
    }
    assert!(pane.traverse_history(false, &s));
    assert!(pane.location.as_ref().unwrap().ends_with("b"));
    pane.refresh(&s);
    pane.options.ascending = false;
    pane.resort(&s);
    pane.set_show_hidden(true, &s);
    assert_eq!(pane.history_len(), 3);
    pane.open(&t.0.join("b"), &s);
    assert_eq!(pane.history_len(), 3);
    assert!(pane.can_forward());
    fs::remove_dir(t.0.join("a")).unwrap();
    assert!(!pane.traverse_history(false, &s));
    assert!(pane.location.as_ref().unwrap().ends_with("b"));
    assert!(pane.can_forward());
    pane.open(&t.dir("d"), &s);
    assert!(!pane.can_forward());
    assert_eq!(pane.history_len(), 3);
    for i in 0..105 {
        pane.open(&t.dir(&format!("dir{i}")), &s);
    }
    assert_eq!(pane.history_len(), 100);
    let mut steps = 0;
    while pane.traverse_history(false, &s) {
        steps += 1;
    }
    assert_eq!(steps, 99);
}
#[test]
fn history_retains_zip_identity_and_is_pane_local() {
    let t = Tree::new();
    let s = NameSorter::en_us().unwrap();
    let mut a = BrowserPane::default();
    let mut b = BrowserPane::default();
    a.open(&t.0, &s);
    b.open(&t.0, &s);
    let zip = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/W09/open.zip");
    assert!(a.open(&zip.join("nested"), &s));
    assert!(a.traverse_history(false, &s));
    assert!(a.archive.is_none());
    assert!(a.traverse_history(true, &s));
    assert_eq!(a.archive.as_ref().unwrap().prefix, "nested/");
    assert_eq!(b.history_len(), 1);
    assert!(b.archive.is_none());
}
#[test]
fn quick_find_focuses_prefixes_wraps_and_preserves_marks_and_filters() {
    let t = Tree::new();
    for n in [
        "Alpha.txt",
        "alphabet.txt",
        "xAlpha.txt",
        "École",
        "中 文",
        ".Alpha",
    ] {
        t.file(n);
    }
    let s = NameSorter::en_us().unwrap();
    let mut pane = BrowserPane::default();
    pane.open(&t.0, &s);
    pane.focus_name("xAlpha.txt");
    pane.apply_event(&PaneEvent::Space, &s).unwrap();
    let marks = pane.selection.snapshot(&pane.entries, &pane.order).marked;
    assert!(pane.find_prefix("al", None));
    assert_eq!(
        pane.selection
            .snapshot(&pane.entries, &pane.order)
            .focused
            .as_deref(),
        Some("Alpha.txt")
    );
    assert!(pane.find_prefix("al", Some(true)));
    assert_eq!(
        pane.selection
            .snapshot(&pane.entries, &pane.order)
            .focused
            .as_deref(),
        Some("alphabet.txt")
    );
    pane.find_prefix("al", Some(true));
    assert_eq!(
        pane.selection
            .snapshot(&pane.entries, &pane.order)
            .focused
            .as_deref(),
        Some("Alpha.txt")
    );
    assert_eq!(
        pane.selection.snapshot(&pane.entries, &pane.order).marked,
        marks
    );
    assert!(pane.find_prefix("é", None));
    assert!(pane.find_prefix("中 ", None));
    let row = pane.selection.focus_row;
    assert!(!pane.find_prefix("missing", None));
    assert_eq!(pane.selection.focus_row, row);
    assert!(!pane.find_prefix(".Al", None));
}

#[cfg(unix)]
#[test]
fn denied_folder_creation_and_partial_deletion_keep_undeleted_sources() {
    use std::os::unix::fs::PermissionsExt;
    let t = Tree::new();
    let denied = t.dir("denied");
    let file = t.file("denied/retained");
    let pin = PinnedDirectory::new(&denied).unwrap();
    let plan = DeletePlan::new(std::slice::from_ref(&file), &[]).unwrap();
    fs::set_permissions(&denied, fs::Permissions::from_mode(0o555)).unwrap();
    let create = pin.create_folder("blocked", &JobControl::default());
    let deleted = plan.apply(&JobControl::default(), None);
    fs::set_permissions(&denied, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(
        create.is_err(),
        "Run permission fixtures as an unprivileged user"
    );
    let report = deleted.unwrap();
    assert!(!report.outcomes[0].success);
    assert!(file.exists());
    assert!(!report.issues.is_empty());
}
