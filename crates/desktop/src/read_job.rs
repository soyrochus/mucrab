// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
//! One cancellable read job. Replacement joins its predecessor before spawning.
use mucrab_core::JobControl;
use std::{sync::mpsc, thread};
type Work<T> = Box<dyn FnOnce(JobControl) -> Result<T, String> + Send>;
struct Running<T> {
    generation: u64,
    discard_result: bool,
    control: JobControl,
    receiver: mpsc::Receiver<Result<T, String>>,
    worker: thread::JoinHandle<()>,
}
pub(super) struct ReadJob<T> {
    generation: u64,
    running: Option<Running<T>>,
    replacement: Option<Work<T>>,
}
impl<T> Default for ReadJob<T> {
    fn default() -> Self {
        Self {
            generation: 0,
            running: None,
            replacement: None,
        }
    }
}
impl<T: Send + 'static> ReadJob<T> {
    pub(super) fn busy(&self) -> bool {
        self.running.is_some() || self.replacement.is_some()
    }
    pub(super) fn progress(&self) -> Option<mucrab_core::JobProgress> {
        self.running.as_ref().map(|r| r.control.snapshot())
    }
    pub(super) fn cancel(&mut self) {
        self.replacement = None;
        if let Some(r) = &self.running {
            r.control.cancel();
        }
    }
    /// Abandon a target while retaining worker ownership until acknowledgment.
    /// Ordinary cancellation still publishes partial results for folder totals.
    pub(super) fn discard(&mut self) {
        self.cancel();
        if let Some(r) = &mut self.running {
            r.discard_result = true;
        }
    }
    pub(super) fn start(
        &mut self,
        work: impl FnOnce(JobControl) -> Result<T, String> + Send + 'static,
    ) -> Result<(), String> {
        self.start_box(Box::new(work))
    }
    fn start_box(&mut self, work: Work<T>) -> Result<(), String> {
        if let Some(r) = &self.running {
            r.control.cancel();
            self.replacement = Some(work);
            return Ok(());
        }
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or("Read generations exhausted")?;
        let control = JobControl::default();
        let worker_control = control.clone();
        let (send, receiver) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("mucrab-read".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    work(worker_control.clone())
                }))
                .unwrap_or_else(|_| {
                    Err("Read/decoder worker panicked; preview was discarded".into())
                });
                worker_control.finish(result.is_err() && !worker_control.is_cancelled());
                let _ = send.send(result);
            })
            .map_err(|e| e.to_string())?;
        self.running = Some(Running {
            generation: self.generation,
            discard_result: false,
            control,
            receiver,
            worker,
        });
        Ok(())
    }
    pub(super) fn poll(&mut self) -> Option<Result<T, String>> {
        let running = self.running.as_ref()?;
        let result = match running.receiver.try_recv() {
            Ok(r) => r,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => Err("Read worker stopped unexpectedly".into()),
        };
        let running = self.running.take().expect("ready worker");
        let _ = running.worker.join();
        if let Some(work) = self.replacement.take() {
            return self.start_box(work).err().map(Err);
        }
        if running.discard_result || running.generation != self.generation {
            return None;
        }
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};
    fn await_result<T: Send + 'static>(job: &mut ReadJob<T>) -> Result<T, String> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            assert!(Instant::now() < deadline);
            if let Some(r) = job.poll() {
                return r;
            }
            thread::sleep(Duration::from_millis(2));
        }
    }
    #[test]
    fn replacement_waits_for_ack_and_never_publishes_old_or_intermediate_results() {
        let mut job = ReadJob::default();
        let (send, receive) = mpsc::channel();
        job.start(move |_| {
            receive.recv().unwrap();
            Ok(1)
        })
        .unwrap();
        let generation = job.generation;
        job.start(|_| Ok(2)).unwrap();
        job.start(|_| Ok(3)).unwrap();
        assert_eq!(job.generation, generation);
        assert!(job.poll().is_none());
        assert!(job.busy());
        send.send(()).unwrap();
        assert_eq!(await_result(&mut job).unwrap(), 3);
        assert_eq!(job.generation, generation + 1);
        assert!(!job.busy());
    }
    #[test]
    fn close_keeps_worker_owned_until_ack_and_panics_are_errors() {
        let mut job: ReadJob<()> = ReadJob::default();
        let (send, receive) = mpsc::channel();
        job.start(move |c| {
            receive.recv().unwrap();
            assert!(c.is_cancelled());
            Ok(())
        })
        .unwrap();
        job.cancel();
        assert!(job.busy());
        assert!(job.poll().is_none());
        send.send(()).unwrap();
        await_result(&mut job).unwrap();
        assert!(!job.busy());
        job.start(|_| panic!("injected decoder panic")).unwrap();
        assert!(await_result(&mut job).unwrap_err().contains("panicked"));
    }
    #[test]
    fn abandoned_target_drops_even_a_result_queued_before_cancellation() {
        let mut job = ReadJob::default();
        job.start(|_| Ok(17)).unwrap();
        // Wait for the result to be queued, without polling it into the UI.
        let deadline = Instant::now() + Duration::from_secs(5);
        while !job.running.as_ref().unwrap().worker.is_finished() {
            assert!(Instant::now() < deadline);
            thread::yield_now();
        }
        job.discard();
        assert!(job.busy());
        assert!(job.poll().is_none());
        assert!(!job.busy());
        job.start(|_| Ok(23)).unwrap();
        assert_eq!(await_result(&mut job).unwrap(), 23);
    }
}
