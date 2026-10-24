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
/// Worker lifecycle; terminal states require acknowledgement after filesystem work ends.
pub enum JobState {
    #[doc = "The worker is active or ready to proceed at its next checkpoint."]
    Running,
    #[doc = "The worker waits at a cooperative checkpoint until resumed or cancelled."]
    Paused,
    #[doc = "Cancellation was requested but the worker has not yet acknowledged completion."]
    Cancelling,
    #[doc = "The operation stopped in response to cancellation; completed writes remain."]
    Cancelled,
    #[doc = "The worker acknowledged successful completion of its operation."]
    Finished,
    #[doc = "The operation failed; inspect diagnostics and any retained partial results."]
    Failed,
}
impl JobState {
    /// Test whether a worker has acknowledged final completion, failure or cancellation.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Cancelled | Self::Finished | Self::Failed)
    }
}

#[derive(Clone, Debug, Serialize)]
/// A bounded snapshot of one worker, replacing previous progress rather than accumulating events.
pub struct JobProgress {
    /// Current worker lifecycle, with terminal values set only after acknowledgement.
    pub state: JobState,
    /// Current operation path, if the worker has started an entry.
    pub current_path: Option<PathBuf>,
    /// Bytes transferred for the current file so far.
    pub current_file_bytes: u64,
    /// Expected current-file byte length, when available.
    pub current_file_size: Option<u64>,
    /// Accumulated transferred bytes across completed and partial files.
    pub transferred_bytes: u64,
    /// Acknowledged top-level items processed so far.
    pub completed_items: usize,
    /// Known top-level item count, excluding recursively discovered children.
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
    // Only private, non-panicking status transitions hold this lock; observers
    // run after it is released. Retaining bounded status after a worker unwind
    // lets the desktop report failure instead of panicking while reading progress.
    fn lock_progress(&self) -> std::sync::MutexGuard<'_, JobProgress> {
        self.0.progress.lock().unwrap_or_else(|e| e.into_inner())
    }

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
    /// Copy bounded progress without retaining a lock in the caller.
    pub fn snapshot(&self) -> JobProgress {
        self.lock_progress().clone()
    }
    fn update(&self, f: impl FnOnce(&mut JobProgress)) {
        let snapshot = {
            let mut progress = self.lock_progress();
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
    /// Request cancellation and wake a paused worker; completed writes are not rolled back.
    pub fn cancel(&self) {
        self.update(|p| p.state = JobState::Cancelling);
    }
    /// Pause or resume a running worker without changing a terminal state.
    pub fn set_paused(&self, paused: bool) {
        self.update(|p| match (p.state, paused) {
            (JobState::Running, true) => p.state = JobState::Paused,
            (JobState::Paused, false) => p.state = JobState::Running,
            _ => {}
        });
    }
    /// Test for a requested or acknowledged cancellation.
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
        let mut progress = self.lock_progress();
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
        self.update(|p| p.completed_items = p.completed_items.saturating_add(1));
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

impl std::fmt::Debug for JobControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JobControl")
            .field("progress", &self.snapshot())
            .finish_non_exhaustive()
    }
}
