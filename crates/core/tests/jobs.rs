// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
mod support;

use mucrab_core::*;
use std::{fs, path::PathBuf, sync::mpsc, time::Duration};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "mucrab-jobs-{}-{}",
            std::process::id(),
            support::nonce()
        ));
        fs::create_dir_all(path.join("source")).unwrap();
        fs::create_dir(path.join("target")).unwrap();
        fs::write(path.join("source/first"), b"earlier completed file").unwrap();
        fs::write(path.join("source/large"), vec![0x6b; 1024 * 1024]).unwrap();
        fs::write(path.join("source/later"), b"untouched").unwrap();
        Self(path)
    }
    fn request(&self) -> CopyRequest {
        CopyRequest {
            source_directory: self.0.join("source"),
            destination_directory: self.0.join("target"),
            names: vec!["first".into(), "large".into(), "later".into()],
            new_name: None,
            policy: CopyPolicy::Overwrite,
        }
    }
    fn zip(&self) -> ZipExtractPlan {
        use std::io::Write;
        let file = fs::File::create(self.0.join("input.zip")).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        for name in ["first", "large", "later"] {
            zip.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(&fs::read(self.0.join("source").join(name)).unwrap())
                .unwrap();
        }
        zip.finish().unwrap();
        ZipExtractPlan::new(
            &self.0.join("input.zip"),
            "",
            None,
            &self.0.join("target"),
            CopyPolicy::Overwrite,
        )
        .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn cancel_stream() -> JobControl {
    JobControl::with_observer(|control, p| {
        if p.state == JobState::Running
            && p.current_path
                .as_ref()
                .is_some_and(|p| p.ends_with("large"))
            && p.current_file_bytes >= 131072
        {
            control.cancel();
        }
    })
}
fn assert_prefix(f: &Fixture) {
    assert_eq!(
        fs::read(f.0.join("target/first")).unwrap(),
        b"earlier completed file"
    );
    let partial = fs::read(f.0.join("target/large")).unwrap();
    let source = fs::read(f.0.join("source/large")).unwrap();
    assert_eq!(partial.len(), 131072);
    assert_eq!(partial, source[..partial.len()]);
    assert!(!f.0.join("target/later").exists());
}

#[test]
fn copy_cancel_keeps_prefix_completed_root_and_unfinished_marks() {
    let f = Fixture::new();
    let control = cancel_stream();
    fs::write(
        f.0.join("target/large"),
        b"previous destination is overwritten, not rolled back",
    )
    .unwrap();
    let report = copy_entries_with_control(&f.request(), &control).unwrap();
    assert!(report.cancelled);
    assert_eq!(report.outcomes.len(), 2);
    assert!(report.outcomes[0].success);
    assert!(!report.outcomes[1].success);
    assert_eq!(report.issues[0].kind, CopyIssueKind::Cancelled);
    assert_prefix(&f);
    let sorter = NameSorter::en_us().unwrap();
    let mut pane = BrowserPane::default();
    pane.open(&f.0.join("source"), &sorter);
    pane.apply_event(&PaneEvent::All, &sorter).unwrap();
    pane.unmark_copy_outcomes(&report.outcomes);
    let snapshot = pane.selection.snapshot(&pane.entries, &pane.order);
    assert_eq!(snapshot.selected, vec!["large", "later"]);
    control.finish(false);
    assert_eq!(control.snapshot().state, JobState::Cancelled);
    assert_eq!(control.snapshot().completed_items, 1);
    assert_eq!(
        control.snapshot().transferred_bytes,
        131072 + b"earlier completed file".len() as u64
    );
}

#[test]
fn recursive_copy_cancel_does_not_report_the_interrupted_root_as_successful() {
    let f = Fixture::new();
    fs::create_dir(f.0.join("source/tree")).unwrap();
    fs::rename(f.0.join("source/large"), f.0.join("source/tree/large")).unwrap();
    let mut request = f.request();
    request.names = vec!["tree".into(), "later".into()];
    let control = cancel_stream();
    let report = copy_entries_with_control(&request, &control).unwrap();
    assert!(report.cancelled);
    assert_eq!(report.outcomes.len(), 1);
    assert!(!report.outcomes[0].success);
    assert!(!f.0.join("target/later").exists());
    assert!(f.0.join("source/tree/large").exists());
}

#[test]
fn cross_device_cancel_retains_the_active_move_source_and_earlier_commits() {
    let f = Fixture::new();
    // Workspace vs /tmp is a real cross-device arrangement on verified hosts.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../artifacts/workspace");
    fs::create_dir_all(&root).unwrap();
    let source = root.join(f.0.file_name().unwrap());
    fs::create_dir(&source).unwrap();
    for name in ["first", "large", "later"] {
        fs::copy(f.0.join("source").join(name), source.join(name)).unwrap();
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        assert_ne!(
            fs::metadata(&source).unwrap().dev(),
            fs::metadata(f.0.join("target")).unwrap().dev(),
            "real cross-device filesystem required"
        );
    }
    let mut request = f.request();
    request.source_directory = source.clone();
    let control = cancel_stream();
    let report = move_entries_with_control(
        &MoveRequest {
            transfer: request,
            rename_mode: false,
        },
        &control,
    )
    .unwrap();
    assert!(report.cancelled);
    assert!(!source.join("first").exists());
    assert_eq!(
        fs::read(source.join("large")).unwrap(),
        vec![0x6b; 1024 * 1024]
    );
    assert_eq!(fs::read(source.join("later")).unwrap(), b"untouched");
    assert_prefix(&f);
    fs::remove_dir_all(source).unwrap();
}

#[test]
fn zip_cancel_preserves_archive_and_completed_prefix() {
    let f = Fixture::new();
    let plan = f.zip();
    let archive = fs::read(f.0.join("input.zip")).unwrap();
    let control = cancel_stream();
    let report = extract_zip_with_control(&plan, &control).unwrap();
    assert!(report.cancelled);
    assert!(!report.success);
    assert_eq!(report.files_extracted, 1);
    assert_prefix(&f);
    assert_eq!(fs::read(f.0.join("input.zip")).unwrap(), archive);
}

#[test]
fn cancellation_before_start_prevents_all_mutations() {
    let f = Fixture::new();
    let control = JobControl::default();
    control.cancel();
    assert!(
        copy_entries_with_control(&f.request(), &control)
            .unwrap()
            .cancelled
    );
    assert!(
        move_entries_with_control(
            &MoveRequest {
                transfer: f.request(),
                rename_mode: false
            },
            &control
        )
        .unwrap()
        .cancelled
    );
    let plan = f.zip();
    assert!(extract_zip_with_control(&plan, &control).unwrap().cancelled);
    let options = BatchRenameOptions {
        mask: "new_[N].[E]".into(),
        ..Default::default()
    };
    let batch = BatchRenamePlan::new(&f.0.join("source"), &f.request().names, &options).unwrap();
    assert!(
        apply_batch_rename_with_control(&batch, &control)
            .unwrap()
            .cancelled
    );
    assert_eq!(fs::read_dir(f.0.join("target")).unwrap().count(), 0);
    assert_eq!(fs::read_dir(f.0.join("source")).unwrap().count(), 3);
}

#[test]
fn batch_cancel_between_atomic_renames_keeps_only_completed_changes() {
    let f = Fixture::new();
    let options = BatchRenameOptions {
        mask: "new_[N].[E]".into(),
        ..Default::default()
    };
    let plan = BatchRenamePlan::new(&f.0.join("source"), &f.request().names, &options).unwrap();
    let control = JobControl::with_observer(|control, p| {
        if p.state == JobState::Running && p.completed_items == 1 {
            control.cancel();
        }
    });
    let report = apply_batch_rename_with_control(&plan, &control).unwrap();
    assert!(report.cancelled);
    assert_eq!(report.outcomes.len(), 1);
    assert!(report.outcomes[0].success);
    assert!(!f.0.join("source/first").exists());
    assert!(f.0.join("source/new_first").exists());
    assert!(f.0.join("source/large").exists());
    assert!(f.0.join("source/later").exists());
}

#[test]
fn paused_job_wakes_on_cancel_and_terminal_states_ignore_late_requests() {
    let f = Fixture::new();
    let request = f.request();
    let (notify, events) = mpsc::channel();
    let control = JobControl::with_observer(move |control, p| {
        if p.state == JobState::Running && p.current_file_bytes >= 65536 {
            control.set_paused(true);
            let _ = notify.send(());
        }
    });
    let worker_control = control.clone();
    let worker =
        std::thread::spawn(move || copy_entries_with_control(&request, &worker_control).unwrap());
    events.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(control.snapshot().state, JobState::Paused);
    let before = control.snapshot().transferred_bytes;
    assert!(events.recv_timeout(Duration::from_millis(30)).is_err());
    assert_eq!(control.snapshot().transferred_bytes, before);
    control.cancel();
    control.cancel();
    assert!(worker.join().unwrap().cancelled);
    control.finish(false);
    control.set_paused(false);
    control.cancel();
    assert_eq!(control.snapshot().state, JobState::Cancelled);
    for failed in [false, true] {
        let control = JobControl::default();
        control.finish(failed);
        control.cancel();
        control.set_paused(true);
        assert_eq!(
            control.snapshot().state,
            if failed {
                JobState::Failed
            } else {
                JobState::Finished
            }
        );
    }
}
