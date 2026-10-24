// SPDX-License-Identifier: GPL-3.0-only
//! Bounded directory preparation and publication tied to stable originating tabs.
use super::{MuCrabApp, read_job::ReadJob};
use eframe::egui;
use mucrab_core::{PreparedBrowse, TabId};
use std::{path::PathBuf, time::Duration};

struct Completion {
    tab: TabId,
    generation: u64,
    result: Result<PreparedBrowse, String>,
}

#[derive(Default)]
struct Slot {
    job: ReadJob<Completion>,
    target: Option<(TabId, u64, PathBuf)>,
}

/// One running worker and one latest replacement for each pane.
#[derive(Default)]
pub(super) struct BrowseJobs {
    slots: [Slot; 2],
}

impl BrowseJobs {
    pub(super) fn busy(&self) -> bool {
        self.slots.iter().any(|s| s.job.busy())
    }
    pub(super) fn cancel(&mut self) {
        for slot in &mut self.slots {
            slot.job.discard();
        }
    }
    pub(super) fn status(&self, id: TabId) -> Option<&str> {
        self.slots
            .iter()
            .find(|s| s.job.busy() && s.target.as_ref().is_some_and(|t| t.0 == id))
            .map(|_| "Loading folder…")
    }
}

impl MuCrabApp {
    pub(super) fn poll_browsing(&mut self, ctx: &egui::Context) {
        for side in 0..2 {
            // Prefer the visible tab when a post-job refresh requests several tabs.
            let active = self.panes.stack(side).active().id;
            let mut requests = Vec::new();
            for tab in self.panes.stack_mut(side).tabs_mut() {
                if let Some(request) = tab.pane.take_browse_request()
                    && !self.close_after_job
                {
                    requests.push((tab.id, request));
                }
            }
            requests.sort_by_key(|(id, _)| *id == active);
            for (tab, request) in requests {
                let generation = request.generation();
                let path = request.path().to_owned();
                let slot = &mut self.browse.slots[side];
                slot.target = Some((tab, generation, path));
                if let Err(error) = slot.job.start(move |control| {
                    Ok(Completion {
                        tab,
                        generation,
                        result: request.prepare(&control).map_err(|e| e.to_string()),
                    })
                }) && let Some(pane) = self.panes.find_mut(tab)
                {
                    pane.error = Some(error);
                }
            }
            let slot = &mut self.browse.slots[side];
            if let Some(result) = slot.job.poll() {
                match result {
                    Ok(done) => {
                        if let Some(pane) = self.panes.find_mut(done.tab)
                            && pane.browse_generation() == done.generation
                        {
                            match done.result {
                                Ok(prepared) => {
                                    pane.commit_browse(prepared);
                                }
                                Err(error) => {
                                    let path = slot
                                        .target
                                        .as_ref()
                                        .map(|t| t.2.display().to_string())
                                        .unwrap_or_default();
                                    pane.error = Some(format!("Cannot load {path}: {error}"));
                                }
                            }
                        }
                    }
                    Err(error) => {
                        if let Some((id, generation, _)) = &slot.target
                            && let Some(pane) = self.panes.find_mut(*id)
                            && pane.browse_generation() == *generation
                        {
                            pane.error = Some(error);
                        }
                    }
                }
            }
        }
        if self.browse.busy() {
            ctx.request_repaint_after(Duration::from_millis(30));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::mpsc, time::Instant};

    #[test]
    fn delayed_browse_replacement_discards_old_content_and_close_waits_for_ack() {
        let mut app = super::super::job_tests::app();
        let root = std::env::temp_dir().join(format!("mucrab-browse-ui-{}", std::process::id()));
        std::fs::create_dir_all(root.join("child")).unwrap();
        app.panes[0].open(&root, &app.sorter);
        app.panes[0].defer_browsing();
        app.panes[0].open(&root.join("child"), &app.sorter);
        let request = app.panes[0].take_browse_request().unwrap();
        let generation = request.generation();
        let tab = app.panes.stack(0).active().id;
        let prepared = request
            .prepare(&mucrab_core::JobControl::default())
            .unwrap();
        let (release, wait) = mpsc::channel();
        app.browse.slots[0].target = Some((tab, generation, root.join("child")));
        app.browse.slots[0]
            .job
            .start(move |_| {
                wait.recv().unwrap();
                Ok(Completion {
                    tab,
                    generation,
                    result: Ok(prepared),
                })
            })
            .unwrap();
        // Starting another request must return while the old provider is blocked.
        app.panes[0].open(&root, &app.sorter);
        let ctx = egui::Context::default();
        app.poll_browsing(&ctx);
        assert!(app.browse.busy());
        assert_eq!(app.panes[0].location(), Some(&root));
        release.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while app.browse.busy() {
            assert!(
                Instant::now() < deadline,
                "browse replacement must acknowledge"
            );
            app.poll_browsing(&ctx);
            std::thread::yield_now();
        }
        assert_eq!(
            app.panes[0].location(),
            Some(&root),
            "old child result must not replace latest navigation"
        );
        assert_eq!(app.panes[0].history_len(), 1);

        let (release, wait) = mpsc::channel();
        app.browse.slots[0]
            .job
            .start(move |_| {
                wait.recv().unwrap();
                Err("discarded fixture".into())
            })
            .unwrap();
        app.request_close(&ctx);
        assert!(app.close_after_job && app.browse.busy());
        release.send(()).unwrap();
        while app.browse.busy() {
            assert!(Instant::now() < deadline);
            app.poll_browsing(&ctx);
            std::thread::yield_now();
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
