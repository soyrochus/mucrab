// SPDX-License-Identifier: GPL-3.0-only
//! Owned mutation worker with explicit origin and acknowledgement-before-publication.
use super::{FileAction, JobResult};
use mucrab_core::{JobControl, JobState, TabId, sftp::RemoteLocation};
use std::{io, path::PathBuf, sync::mpsc, thread};

pub(super) struct MutationOrigin {
    pub(super) action: FileAction,
    pub(super) source_tab: TabId,
    pub(super) source_directory: PathBuf,
    pub(super) renamed_to: Option<String>,
}

pub(super) struct PendingCopy {
    pub(super) source_remote: Option<RemoteLocation>,
    pub(super) control: JobControl,
    pub(super) action: FileAction,
    pub(super) renamed_to: Option<String>,
    pub(super) source_tab: TabId,
    pub(super) source_directory: PathBuf,
    worker: Option<thread::JoinHandle<()>>,
    receiver: mpsc::Receiver<Result<JobResult, String>>,
}
impl PendingCopy {
    pub(super) fn start(
        origin: MutationOrigin,
        work: impl FnOnce(JobControl) -> Result<JobResult, String> + Send + 'static,
    ) -> io::Result<Self> {
        let control = JobControl::default();
        let worker_control = control.clone();
        let (sender, receiver) = mpsc::channel();
        let worker = thread::Builder::new()
            .name(format!("mucrab-{}", origin.action.label().to_lowercase()))
            .spawn(move || {
                let mut result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    work(worker_control.clone())
                }))
                .unwrap_or_else(|_| {
                    Err("File worker panicked; inspect both folders for partial results".into())
                });
                worker_control.finish(result.is_err());
                if let Ok(report) = &mut result {
                    report.cancelled = worker_control.snapshot().state == JobState::Cancelled;
                }
                // No filesystem work follows acknowledgement.
                let _ = sender.send(result);
            })?;
        Ok(Self {
            source_remote: None,
            control,
            action: origin.action,
            renamed_to: origin.renamed_to,
            source_tab: origin.source_tab,
            source_directory: origin.source_directory,
            worker: Some(worker),
            receiver,
        })
    }
    pub(super) fn poll(&mut self) -> Option<Result<JobResult, String>> {
        let result = match self.receiver.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => Err(
                "File worker stopped unexpectedly; inspect both folders for partial results".into(),
            ),
        };
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        Some(result)
    }
}
