// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

//! Native application dispatch. Archive copies deliberately survive application exit.
use mucrab_core::{CopyPolicy, JobControl, ZipExtractPlan, ZipLocation, extract_zip_with_control};
use std::{
    fs, io,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread,
};

pub enum Target {
    Local(PathBuf),
    Remote(mucrab_core::sftp::RemoteLocation),
    Archived(ZipLocation, String),
}
type OpenResult = io::Result<(Child, bool)>;

#[derive(Default)]
pub struct Opener {
    pending: Option<(JobControl, mpsc::Receiver<OpenResult>)>,
    children: Vec<Child>,
    pub message: Option<String>,
}
impl Opener {
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }
    pub fn cancel(&self) {
        if let Some((control, _)) = &self.pending {
            control.cancel();
        }
    }
    pub fn start(&mut self, target: Target) {
        if self.busy() || self.children.len() >= 64 {
            self.message = Some("Please wait for the pending Open request.".into());
            return;
        }
        let control = JobControl::default();
        let worker_control = control.clone();
        let (tx, rx) = mpsc::channel();
        self.message = Some("Preparing Open request…".into());
        self.pending = Some((control, rx));
        thread::spawn(move || {
            let result = (|| {
                let archived = matches!(target, Target::Archived(..));
                let path = match target {
                    Target::Remote(_) => {
                        return Err(io::Error::other(
                            "Remote Open requires connected SFTP preparation",
                        ));
                    }
                    Target::Local(path) => fs::canonicalize(path)?,
                    Target::Archived(location, name) => {
                        extract_member(&location, &name, &worker_control)?
                    }
                };
                let dispatched = if worker_control.is_cancelled() {
                    Err(io::Error::other("Open cancelled"))
                } else {
                    launch(&path)
                };
                if archived && dispatched.is_err() {
                    let _ = fs::remove_dir_all(path.parent().expect("private archive directory"));
                }
                dispatched.map(|child| (child, archived))
            })();
            let _ = tx.send(result);
        });
    }
    pub fn poll(&mut self) {
        if let Some((_, rx)) = &self.pending {
            match rx.try_recv() {
                Ok(result) => {
                    self.pending = None;
                    match result {
                        Ok((child, archived)) => {
                            self.children.push(child);
                            self.message = Some(if archived { "Open request sent. Changes to the retained temporary copy do not update the ZIP." } else { "Open request sent to the default application." }.into());
                        }
                        Err(error) => self.message = Some(format!("Could not open file: {error}")),
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.pending = None;
                    self.message = Some("Open worker stopped unexpectedly.".into());
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        self.children.retain_mut(|child| match child.try_wait() {
            Ok(None) => true,
            Ok(Some(status)) => { if !status.success() { self.message = Some(format!("Default application launcher failed ({status}). Check the file association.")); } false }
            Err(error) => { self.message = Some(format!("Could not check application launcher: {error}")); false }
        });
    }
    pub fn watching(&self) -> bool {
        self.busy() || !self.children.is_empty()
    }
}
fn launch(path: &Path) -> io::Result<Child> {
    if !fs::metadata(path)?.is_file() {
        return Err(io::Error::other("The selected item is not a regular file"));
    }
    #[cfg(target_os = "linux")]
    let mut command = Command::new("xdg-open");
    #[cfg(target_os = "macos")]
    let mut command = Command::new("/usr/bin/open");
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    return Err(io::Error::other(
        "Native opening is supported on Linux and macOS",
    ));
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    command
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
}
fn extract_member(location: &ZipLocation, name: &str, control: &JobControl) -> io::Result<PathBuf> {
    if !mucrab_core::is_valid_leaf_name(name) {
        return Err(io::Error::other("Invalid archive filename"));
    }
    // A unique private directory prevents collisions and stays available to apps
    // which read lazily or keep a document open after muCrab exits.
    let root = std::env::temp_dir();
    let directory = private_directory(&root)?;
    let result = (|| {
        let plan = ZipExtractPlan::new(
            &location.archive,
            &location.prefix,
            Some(&[name.to_owned()]),
            &directory,
            CopyPolicy::Skip,
        )?;
        let report = extract_zip_with_control(&plan, control)?;
        if !report.success || report.cancelled {
            return Err(io::Error::other(format!(
                "Archive extraction failed: {:?}",
                report.issues
            )));
        }
        let path = directory.join(name);
        if !fs::symlink_metadata(&path)?.is_file() {
            return Err(io::Error::other("Archive member is not a regular file"));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        }
        Ok(path)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(directory);
    }
    result
}
fn private_directory(root: &Path) -> io::Result<PathBuf> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    for _ in 0..100 {
        let path = root.join(format!(
            "mucrab-open-{}-{stamp}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        match builder.create(&path) {
            Ok(()) => return Ok(path),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::other(
        "Could not allocate a private temporary directory",
    ))
}

pub fn target(pane: &mucrab_core::BrowserPane, name: &str) -> Target {
    if let Some(location) = &pane.remote {
        return Target::Remote(location.child(name).expect("validated listed remote name"));
    }
    match &pane.archive {
        Some(location) => Target::Archived(location.clone(), name.to_owned()),
        None => Target::Local(pane.location.as_deref().unwrap_or(Path::new("")).join(name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn archived_copy_preserves_name_contents_and_lifetime() {
        let location = ZipLocation {
            archive: Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/W09/open.zip"),
            prefix: "nested/".into(),
        };
        let path =
            extract_member(&location, "- café & report.txt", &JobControl::default()).unwrap();
        assert_eq!(path.file_name().unwrap(), "- café & report.txt");
        assert_eq!(fs::read_to_string(&path).unwrap(), "W09 archive document\n");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
            assert_eq!(
                fs::metadata(path.parent().unwrap())
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o700
            );
        }
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
    #[test]
    fn unsafe_missing_and_cancelled_members_are_not_prepared() {
        let mut location = ZipLocation {
            archive: Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/W09/open.zip"),
            prefix: "nested/".into(),
        };
        assert!(extract_member(&location, "../escape", &JobControl::default()).is_err());
        assert!(extract_member(&location, "missing.txt", &JobControl::default()).is_err());
        let control = JobControl::default();
        control.cancel();
        assert!(extract_member(&location, "- café & report.txt", &control).is_err());
        location.archive.set_file_name("corrupt.zip");
        location.prefix.clear();
        assert!(extract_member(&location, "broken.txt", &JobControl::default()).is_err());
        location.archive.set_file_name("unsafe.zip");
        location.prefix.clear();
        assert!(extract_member(&location, "escape.txt", &JobControl::default()).is_err());
    }
    #[test]
    fn launch_rejects_non_files_and_broken_links() {
        assert!(launch(Path::new("/definitely-missing-mucrab-file")).is_err());
        assert!(launch(&std::env::temp_dir()).is_err());
    }
}

#[cfg(all(test, unix))]
mod lifecycle_tests {
    use super::*;
    #[test]
    fn late_launcher_failure_is_visible_without_blocking() {
        let child = Command::new("sh").args(["-c", "exit 7"]).spawn().unwrap();
        let mut opener = Opener {
            children: vec![child],
            ..Opener::default()
        };
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while opener.watching() && std::time::Instant::now() < deadline {
            opener.poll();
            thread::yield_now();
        }
        assert!(!opener.watching());
        assert!(opener.message.unwrap().contains("launcher failed"));
    }
    #[test]
    fn busy_guard_keeps_original_worker_and_cancel_reaches_it() {
        let control = JobControl::default();
        let (_tx, rx) = mpsc::channel();
        let mut opener = Opener {
            pending: Some((control.clone(), rx)),
            ..Opener::default()
        };
        opener.start(Target::Local(PathBuf::from("unused")));
        assert!(opener.busy());
        assert!(opener.message.as_ref().unwrap().contains("Please wait"));
        opener.cancel();
        assert!(control.is_cancelled());
    }
}
