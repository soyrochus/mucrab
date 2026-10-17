// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

//! Cooperative job control. Cancellation never implies rollback of writes.
use serde::Serialize;
use std::{
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Condvar, Mutex},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JobState {
    Running,
    Paused,
    Cancelling,
    Cancelled,
    Finished,
    Failed,
}
impl JobState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Cancelled | Self::Finished | Self::Failed)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct JobProgress {
    pub state: JobState,
    pub current_path: Option<PathBuf>,
    pub current_file_bytes: u64,
    pub current_file_size: Option<u64>,
    pub transferred_bytes: u64,
    pub completed_items: usize,
    pub total_items: usize,
}
impl Default for JobProgress {
    fn default() -> Self {
        Self {
            state: JobState::Running,
            current_path: None,
            current_file_bytes: 0,
            current_file_size: None,
            transferred_bytes: 0,
            completed_items: 0,
            total_items: 0,
        }
    }
}

type Observer = dyn Fn(&JobControl, &JobProgress) + Send + Sync;
struct Shared {
    progress: Mutex<JobProgress>,
    wake: Condvar,
    observer: Option<Box<Observer>>,
}
/// Cloneable handle for one job; snapshots are bounded and replace earlier values.
#[derive(Clone)]
pub struct JobControl(Arc<Shared>);
impl Default for JobControl {
    fn default() -> Self {
        Self(Arc::new(Shared {
            progress: Mutex::new(JobProgress::default()),
            wake: Condvar::new(),
            observer: None,
        }))
    }
}
impl JobControl {
    /// An optional synchronous observer for embedding and deterministic boundary
    /// checks. Called without internal locks; it must be fast and must not panic.
    pub fn with_observer(
        observer: impl Fn(&JobControl, &JobProgress) + Send + Sync + 'static,
    ) -> Self {
        Self(Arc::new(Shared {
            progress: Mutex::new(JobProgress::default()),
            wake: Condvar::new(),
            observer: Some(Box::new(observer)),
        }))
    }
    pub fn snapshot(&self) -> JobProgress {
        self.0
            .progress
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
    fn update(&self, f: impl FnOnce(&mut JobProgress)) {
        let snapshot = {
            let mut progress = self.0.progress.lock().unwrap_or_else(|e| e.into_inner());
            if progress.state.is_terminal() {
                return;
            }
            f(&mut progress);
            progress.clone()
        };
        self.0.wake.notify_all();
        if let Some(observer) = &self.0.observer {
            observer(self, &snapshot);
        }
    }
    pub fn cancel(&self) {
        self.update(|p| p.state = JobState::Cancelling);
    }
    pub fn set_paused(&self, paused: bool) {
        self.update(|p| match (p.state, paused) {
            (JobState::Running, true) => p.state = JobState::Paused,
            (JobState::Paused, false) => p.state = JobState::Running,
            _ => {}
        });
    }
    pub fn is_cancelled(&self) -> bool {
        matches!(
            self.snapshot().state,
            JobState::Cancelling | JobState::Cancelled
        )
    }
    /// Worker acknowledgement, after all streams and filesystem work have ended.
    pub fn finish(&self, failed: bool) {
        self.update(|p| {
            p.state = if failed {
                JobState::Failed
            } else if p.state == JobState::Cancelling {
                JobState::Cancelled
            } else {
                JobState::Finished
            }
        });
    }
    pub(crate) fn checkpoint(&self) -> bool {
        let mut progress = self.0.progress.lock().unwrap_or_else(|e| e.into_inner());
        while progress.state == JobState::Paused {
            progress = self
                .0
                .wake
                .wait(progress)
                .unwrap_or_else(|e| e.into_inner());
        }
        progress.state == JobState::Running
    }
    pub(crate) fn total(&self, total: usize) {
        self.update(|p| p.total_items = total);
    }
    pub(crate) fn entry(&self, path: &Path, size: Option<u64>) -> bool {
        if !self.checkpoint() {
            return false;
        }
        self.update(|p| {
            p.current_path = Some(path.to_owned());
            p.current_file_size = size;
            p.current_file_bytes = 0;
        });
        self.checkpoint()
    }
    pub(crate) fn completed(&self) {
        self.update(|p| p.completed_items += 1);
    }
    pub(crate) fn advance(&self, bytes: u64) {
        self.update(|p| {
            p.current_file_bytes = p.current_file_bytes.saturating_add(bytes);
            p.transferred_bytes = p.transferred_bytes.saturating_add(bytes);
        });
    }
}

pub(crate) fn copy_stream(
    input: &mut impl Read,
    output: &mut impl Write,
    control: &JobControl,
) -> io::Result<u64> {
    let mut buffer = [0_u8; 65536];
    let mut copied = 0;
    loop {
        if !control.checkpoint() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Operation cancelled; partial destination retained",
            ));
        }
        let count = match input.read(&mut buffer) {
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            result => result?,
        };
        if count == 0 {
            return Ok(copied);
        }
        if !control.checkpoint() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Operation cancelled; partial destination retained",
            ));
        }
        output.write_all(&buffer[..count])?;
        copied += count as u64;
        control.advance(count as u64);
    }
}
