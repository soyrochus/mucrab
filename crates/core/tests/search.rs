// SPDX-License-Identifier: GPL-3.0-only
//! Behavioural regression coverage for search using disposable fixtures.
mod support;
use mucrab_core::*;
use std::{
    fs,
    path::PathBuf,
    time::{Duration, UNIX_EPOCH},
};
struct Tree(PathBuf);
impl Tree {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "mucrab-w12-{}-{}",
            std::process::id(),
            support::nonce()
        ));
        fs::create_dir_all(&p).unwrap();
        Self(fs::canonicalize(p).unwrap())
    }
    fn query(&self) -> SearchQuery {
        SearchQuery {
            root: self.0.clone(),
            ..Default::default()
        }
    }
}
impl Drop for Tree {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn run(q: SearchQuery) -> SearchReport {
    SearchPlan::new(q)
        .unwrap()
        .run(&JobControl::default(), |_| true)
        .unwrap()
}
#[cfg(target_os = "macos")]
#[test]
fn macos_search_matches_visible_composed_names_and_opens_raw_paths() {
    let t = Tree::new();
    fs::write(t.0.join("cafe\u{301}.txt"), b"accented").unwrap();
    let report = run(SearchQuery {
        pattern: "café.*".into(),
        ..t.query()
    });
    assert_eq!(report.hits.len(), 1);
    assert_eq!(fs::read(&report.hits[0].path).unwrap(), b"accented");
    let sorter = NameSorter::en_us().unwrap();
    let mut pane = BrowserPane::default();
    report.hits[0].open_containing(&mut pane, &sorter).unwrap();
    assert_eq!(
        pane.selection
            .snapshot(&pane.entries, &pane.order)
            .focused
            .as_deref(),
        Some("café.txt")
    );
}
#[cfg(target_os = "macos")]
#[test]
#[ignore = "requires MUCRAB_MOUNT_FIXTURE with a real mounted child and disposable marker files"]
fn macos_real_mounted_child_is_omitted_by_search_and_folder_totals() {
    use std::os::unix::fs::MetadataExt;
    let root = fs::canonicalize(std::env::var_os("MUCRAB_MOUNT_FIXTURE").unwrap()).unwrap();
    let mounted = root.join("mounted");
    assert_ne!(
        fs::metadata(&root).unwrap().dev(),
        fs::metadata(&mounted).unwrap().dev()
    );
    assert_eq!(fs::read(root.join("outside.txt")).unwrap(), b"outside");
    assert_eq!(fs::read(mounted.join("inside.txt")).unwrap(), b"inside");
    let report = run(SearchQuery {
        root: root.clone(),
        ..Default::default()
    });
    assert!(report.partial());
    assert_eq!(report.hits.len(), 2);
    assert!(
        report
            .hits
            .iter()
            .all(|h| h.relative_path != std::path::Path::new("mounted/inside.txt"))
    );
    assert!(report.issues.iter().any(|s| s.contains("Mount boundary")));
    let totals = folder_size(&root, &JobControl::default()).unwrap();
    assert!(totals.partial());
    assert_eq!((totals.bytes, totals.files), (7, 1));
    assert!(
        totals
            .details
            .iter()
            .any(|s| s.to_lowercase().contains("mount"))
    );
}
#[test]
fn unicode_globs_escapes_validation_and_utc_boundaries() {
    let t = Tree::new();
    let mut q = t.query();
    for (pattern, name, sensitive, expected) in [
        ("a?pha.*", "ALPHA.TXT", false, true),
        ("?", "😀", false, true),
        ("?", "İ", false, true),
        ("é", "e\u{301}", false, false),
        ("É*", "é.txt", false, true),
        (r"a\*b.txt", "a*b.txt", false, true),
        ("*x?", "xxx", false, true),
        ("A*", "alpha", true, false),
    ] {
        q.pattern = pattern.into();
        q.case_sensitive = sensitive;
        assert_eq!(
            SearchPlan::new(q.clone()).unwrap().matches_name(name),
            expected,
            "{pattern}: {name}"
        );
    }
    q.pattern = "trailing\\".into();
    assert!(SearchPlan::new(q.clone()).is_err());
    q.pattern = "?".repeat(257);
    assert!(SearchPlan::new(q.clone()).is_err());
    q.pattern = "*".into();
    q.min_size = Some(2);
    q.max_size = Some(1);
    assert!(SearchPlan::new(q).is_err());
    assert_eq!(search_time_bound("1970-01-01", false).unwrap(), Some(0));
    assert_eq!(
        search_time_bound("1970-01-01", true).unwrap(),
        Some(86_400_000)
    );
    assert_eq!(
        search_time_bound("1970-01-01T00:00:01Z", true).unwrap(),
        Some(1000)
    );
    assert!(search_time_bound("yesterday", false).is_err());
    let before = t.0.join("before-epoch");
    fs::write(&before, b"x").unwrap();
    fs::File::options()
        .write(true)
        .open(before)
        .unwrap()
        .set_modified(UNIX_EPOCH - Duration::from_micros(500))
        .unwrap();
    let q = SearchQuery {
        from_ms: Some(-1),
        to_ms: Some(0),
        ..t.query()
    };
    assert_eq!(run(q).hits.len(), 1);
}
#[test]
fn prunes_hidden_links_and_applies_size_type_extension_time_filters() {
    let t = Tree::new();
    fs::create_dir_all(t.0.join(".hidden/deep")).unwrap();
    fs::create_dir(t.0.join("folder")).unwrap();
    fs::write(t.0.join(".hidden/deep/a.txt"), b"123").unwrap();
    fs::write(t.0.join("folder/a.TXT"), b"123").unwrap();
    fs::write(t.0.join("b.txt"), b"1234").unwrap();
    for p in [t.0.join("folder/a.TXT"), t.0.join("b.txt")] {
        fs::File::options()
            .write(true)
            .open(p)
            .unwrap()
            .set_modified(UNIX_EPOCH + Duration::from_millis(1000))
            .unwrap();
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("..", t.0.join("folder/cycle")).unwrap();
        std::os::unix::fs::symlink("missing", t.0.join("broken")).unwrap();
    }
    let mut q = t.query();
    q.extension = "txt".into();
    q.min_size = Some(3);
    q.max_size = Some(3);
    q.from_ms = Some(1000);
    q.to_ms = Some(1001);
    let r = run(q.clone());
    assert_eq!(r.hits.len(), 1);
    assert_eq!(r.hits[0].relative_path, PathBuf::from("folder/a.TXT"));
    q.to_ms = Some(1000);
    q.from_ms = Some(999);
    assert!(run(q).hits.is_empty());
    let r = run(t.query());
    assert!(
        !r.hits
            .iter()
            .any(|h| h.relative_path.to_string_lossy().contains("deep"))
    );
    assert!(r.hits.len() < 10);
    let mut q = t.query();
    q.include_hidden = true;
    assert!(
        run(q)
            .hits
            .iter()
            .any(|h| h.relative_path == std::path::Path::new(".hidden/deep/a.txt"))
    );
}
#[test]
fn limits_cancellation_batches_and_stale_result_navigation() {
    let t = Tree::new();
    for i in 0..150 {
        fs::write(t.0.join(format!("file{i:03}")), b"x").unwrap();
    }
    let plan = SearchPlan::new(t.query()).unwrap();
    let control = JobControl::default();
    let mut lengths = Vec::new();
    let r = plan
        .run(&control, |b| {
            lengths.push(b.len());
            control.cancel();
            true
        })
        .unwrap();
    assert!(r.cancelled);
    assert_eq!(lengths[0], 64);
    assert_eq!(r.hits.len(), 64);
    let r = plan
        .run_with_limits(
            &JobControl::default(),
            SearchLimits {
                hits: 10,
                ..Default::default()
            },
            |_| true,
        )
        .unwrap();
    assert_eq!(r.hits.len(), 10);
    assert!(r.partial());
    let r = plan
        .run_with_limits(
            &JobControl::default(),
            SearchLimits {
                visited: 2,
                ..Default::default()
            },
            |_| true,
        )
        .unwrap();
    assert_eq!(r.visited, 2);
    assert!(r.partial());
    let r = run(t.query());
    assert!(
        r.hits
            .windows(2)
            .all(|w| w[0].relative_path < w[1].relative_path)
    );
    let hit = &r.hits[0];
    let sorter = NameSorter::en_us().unwrap();
    let mut pane = BrowserPane::default();
    pane.open(&t.0, &sorter);
    pane.apply_event(&PaneEvent::All, &sorter).unwrap();
    let marks = pane.selection().marked_count();
    assert!(hit.open_containing(&mut pane, &sorter).is_ok());
    assert_eq!(
        pane.selection().marked_count(),
        marks,
        "Opening a result in the same folder preserves marks"
    );
    let location = pane.location().cloned();
    let focus = pane
        .selection()
        .snapshot(pane.entries(), pane.order())
        .focused;
    fs::remove_file(&hit.path).unwrap();
    assert!(hit.open_containing(&mut pane, &sorter).is_err());
    assert_eq!(pane.location().cloned(), location);
    assert_eq!(
        pane.selection()
            .snapshot(pane.entries(), pane.order())
            .focused,
        focus
    );
}
#[test]
fn frozen_default_and_reserved_metadata_cases() {
    let t = Tree::new();
    for payload in [
        include_str!("../../../fixtures/W12-file-search/cases.json"),
        include_str!("../../../fixtures/W12-file-search/reserved.json"),
    ] {
        let cases: Vec<serde_json::Value> = serde_json::from_str(payload).unwrap();
        for case in cases {
            let name = case["name"].as_str().unwrap();
            let file = t.0.join(name);
            let size = case["size"].as_u64().unwrap();
            let modified = case["modified_ms"].as_i64().unwrap();
            fs::write(&file, vec![0; size as usize]).unwrap();
            let stamp = if modified < 0 {
                UNIX_EPOCH - Duration::from_millis(modified.unsigned_abs())
            } else {
                UNIX_EPOCH + Duration::from_millis(modified as u64)
            };
            fs::File::options()
                .write(true)
                .open(&file)
                .unwrap()
                .set_modified(stamp)
                .unwrap();
            let q = SearchQuery {
                root: t.0.clone(),
                pattern: case["pattern"].as_str().unwrap().into(),
                case_sensitive: case["case_sensitive"].as_bool().unwrap(),
                include_hidden: true,
                min_size: case["min_size"].as_u64(),
                max_size: case["max_size"].as_u64(),
                from_ms: case["from_ms"].as_i64(),
                to_ms: case["to_ms"].as_i64(),
                ..Default::default()
            };
            let invalid = q.min_size.zip(q.max_size).is_some_and(|(a, b)| a > b)
                || q.from_ms.zip(q.to_ms).is_some_and(|(a, b)| a >= b);
            let plan = SearchPlan::new(q.clone());
            assert_eq!(plan.is_err(), invalid, "{}", case["id"]);
            if let Ok(plan) = plan {
                let r = plan.run(&JobControl::default(), |_| true).unwrap();
                let metadata = q.min_size.is_none_or(|v| size >= v)
                    && q.max_size.is_none_or(|v| size <= v)
                    && q.from_ms.is_none_or(|v| modified >= v)
                    && q.to_ms.is_none_or(|v| modified < v);
                assert_eq!(
                    r.hits.len(),
                    usize::from(metadata && plan.matches_name(name)),
                    "{}",
                    case["id"]
                );
            }
            fs::remove_file(file).unwrap();
        }
    }
}

#[cfg(unix)]
#[test]
fn denied_subtrees_and_diagnostic_storage_are_bounded_and_depth_is_explicit() {
    use std::os::unix::fs::PermissionsExt;
    let t = Tree::new();
    for i in 0..300 {
        let p = t.0.join(format!("denied{i}"));
        fs::create_dir(&p).unwrap();
        fs::write(p.join("unreadable"), b"x").unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o000)).unwrap();
    }
    let report = run(t.query());
    for p in fs::read_dir(&t.0).unwrap() {
        fs::set_permissions(p.unwrap().path(), fs::Permissions::from_mode(0o700)).unwrap();
    }
    assert_eq!(
        report.error_count, 300,
        "Run permission checks unprivileged"
    );
    assert_eq!(report.issues.len(), 256);
    assert_eq!(report.hits.len(), 300);
    assert!(report.partial());
    fs::create_dir_all(t.0.join("outer/inner")).unwrap();
    fs::write(t.0.join("outer/inner/deep.txt"), b"x").unwrap();
    let r = SearchPlan::new(t.query())
        .unwrap()
        .run_with_limits(
            &JobControl::default(),
            SearchLimits {
                depth: 1,
                ..Default::default()
            },
            |_| true,
        )
        .unwrap();
    assert!(
        !r.hits
            .iter()
            .any(|h| h.relative_path.components().count() > 1)
    );
    assert!(r.partial());
}
