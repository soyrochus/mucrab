// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
use super::*;
use mucrab_core::{SearchHit, SearchPlan, SearchQuery, SearchReport, search_time_bound};
use std::sync::mpsc::{Receiver, TrySendError, sync_channel};

struct Worker {
    generation: u64,
    control: JobControl,
    batches: Receiver<(u64, Vec<SearchHit>)>,
    done: Receiver<Result<SearchReport, String>>,
    thread: std::thread::JoinHandle<()>,
}
#[derive(Default)]
pub(super) struct Search {
    pub(super) visible: bool,
    root: String,
    pattern: String,
    extension: String,
    case_sensitive: bool,
    hidden: bool,
    types: [bool; 3],
    sizes: [String; 2],
    dates: [String; 2],
    initialized: bool,
    generation: u64,
    worker: Option<Worker>,
    pending: Option<SearchPlan>,
    hits: Vec<SearchHit>,
    selected: Option<PathBuf>,
    report: Option<SearchReport>,
    error: Option<String>,
}
impl Search {
    pub(super) fn busy(&self) -> bool {
        self.worker.is_some() || self.pending.is_some()
    }
    pub(super) fn cancel(&mut self) {
        self.pending = None;
        if let Some(w) = &self.worker {
            w.control.cancel();
        }
    }
    pub(super) fn start(&mut self, plan: SearchPlan) {
        if let Some(w) = &self.worker {
            w.control.cancel();
            self.pending = Some(plan);
            return;
        }
        self.generation = self
            .generation
            .checked_add(1)
            .expect("search generation exhausted");
        let generation = self.generation;
        let control = JobControl::default();
        let job_control = control.clone();
        let (send, batches) = sync_channel(8);
        let (finish, done) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name(format!("file-search-{generation}"))
            .spawn(move || {
                let result = plan
                    .run(&job_control, |batch| {
                        let mut payload = (generation, batch);
                        loop {
                            if job_control.is_cancelled() {
                                return false;
                            }
                            match send.try_send(payload) {
                                Ok(()) => return true,
                                Err(TrySendError::Disconnected(_)) => return false,
                                Err(TrySendError::Full(p)) => {
                                    payload = p;
                                    std::thread::sleep(Duration::from_millis(5));
                                }
                            }
                        }
                    })
                    .map_err(|e| e.to_string());
                job_control.finish(result.is_err());
                let _ = finish.send(result);
            });
        let thread = match thread {
            Ok(t) => t,
            Err(e) => {
                self.error = Some(format!("Cannot start search worker: {e}"));
                return;
            }
        };
        self.worker = Some(Worker {
            generation,
            control,
            batches,
            done,
            thread,
        });
        self.hits.clear();
        self.selected = None;
        self.report = None;
        self.error = None;
    }
    pub(super) fn poll(&mut self, ctx: &egui::Context) {
        let mut finished = None;
        if let Some(w) = &self.worker {
            // At most eight bounded batches per frame, even while the producer runs.
            for _ in 0..8 {
                let Ok((generation, batch)) = w.batches.try_recv() else {
                    break;
                };
                if self.pending.is_none()
                    && generation == self.generation
                    && generation == w.generation
                {
                    self.hits.extend(batch);
                }
            }
            self.hits
                .sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
            match w.done.try_recv() {
                Ok(result) => finished = Some(result),
                Err(mpsc::TryRecvError::Disconnected) => {
                    finished = Some(Err("Search worker stopped unexpectedly".into()))
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
            ctx.request_repaint_after(Duration::from_millis(30));
        }
        if let Some(result) = finished {
            let w = self.worker.take().expect("finished worker");
            let _ = w.thread.join();
            if self.pending.is_none() {
                match result {
                    Ok(mut report) => {
                        self.hits = std::mem::take(&mut report.hits);
                        self.report = Some(report);
                    }
                    Err(e) => self.error = Some(e),
                }
            }
            if let Some(plan) = self.pending.take() {
                self.start(plan);
            }
        }
    }
    fn query(&self) -> Result<SearchPlan, String> {
        let number = |s: &str| {
            if s.trim().is_empty() {
                Ok(None)
            } else {
                s.parse::<u64>()
                    .map(Some)
                    .map_err(|_| "Size must be a whole number of bytes".to_owned())
            }
        };
        SearchPlan::new(SearchQuery {
            root: PathBuf::from(&self.root),
            pattern: self.pattern.clone(),
            extension: self.extension.clone(),
            case_sensitive: self.case_sensitive,
            files: self.types[0],
            folders: self.types[1],
            links: self.types[2],
            include_hidden: self.hidden,
            min_size: number(&self.sizes[0])?,
            max_size: number(&self.sizes[1])?,
            from_ms: search_time_bound(&self.dates[0], false)?,
            to_ms: search_time_bound(&self.dates[1], true)?,
        })
        .map_err(|e| e.to_string())
    }
}
impl MuCrabApp {
    pub(super) fn open_search(&mut self) {
        if !self.search.initialized {
            let i = usize::from(self.active == PaneSide::Right);
            if self.panes[i].archive.is_some() {
                self.navigation.error =
                    Some("Search requires a local folder; leave the archive first.".into());
                return;
            }
            self.search.root = self.panes[i]
                .location
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            self.search.hidden = self.show_hidden;
            self.search.pattern = "*".into();
            self.search.types = [true; 3];
            self.search.initialized = true;
        }
        self.search.visible = true;
    }
    pub(super) fn search_window(&mut self, ctx: &egui::Context) {
        if !self.search.visible {
            return;
        }
        let mut visible = true;
        let mut start = false;
        let mut cancel = false;
        let mut open = None;
        let s = &mut self.search;
        egui::Window::new("Search files").id(egui::Id::new("file-search")).open(&mut visible)
            .default_width(760.0).default_height(540.0).resizable(true).show(ctx, |ui| {
                egui::Grid::new("search-fields").num_columns(2).show(ui,|ui| {
                    ui.label("Local folder");ui.add(egui::TextEdit::singleline(&mut s.root).desired_width(570.0));ui.end_row();
                    ui.label("Name pattern");ui.horizontal(|ui|{ui.text_edit_singleline(&mut s.pattern).on_hover_text("* = any sequence; ? = one character; backslash escapes the next character");ui.checkbox(&mut s.case_sensitive,"Match case");});ui.end_row();
                    ui.label("Extension");ui.horizontal(|ui|{ui.text_edit_singleline(&mut s.extension);ui.checkbox(&mut s.hidden,"Include hidden");});ui.end_row();
                    ui.label("Types");ui.horizontal(|ui|{ui.checkbox(&mut s.types[0],"Files");ui.checkbox(&mut s.types[1],"Folders");ui.checkbox(&mut s.types[2],"Links (never followed)");});ui.end_row();
                    ui.label("Size in bytes");ui.horizontal(|ui|{ui.label("Min");ui.add(egui::TextEdit::singleline(&mut s.sizes[0]).desired_width(120.0));ui.label("Max");ui.add(egui::TextEdit::singleline(&mut s.sizes[1]).desired_width(120.0));});ui.end_row();
                    ui.label("Modified (UTC)");ui.horizontal(|ui|{ui.label("From");ui.add(egui::TextEdit::singleline(&mut s.dates[0]).desired_width(170.0));ui.label("To");ui.add(egui::TextEdit::singleline(&mut s.dates[1]).desired_width(170.0));});ui.end_row();
                });
                ui.small("Dates: YYYY-MM-DD (To includes that day), or RFC3339 timestamp (To is exclusive). Blank bounds mean no limit.");
                ui.horizontal(|ui| {
                    start=ui.button(if s.busy(){"Replace search"}else{"Search"}).clicked();
                    cancel=ui.add_enabled(s.busy(),egui::Button::new("Cancel search")).clicked();
                    if ui.add_enabled(s.selected.is_some(),egui::Button::new("Open containing folder")).clicked(){open=s.selected.clone();}
                });
                if let Some(w)=&s.worker {
                    let p=w.control.snapshot();
                    ui.label(format!("{:?} — {} visited, {} matches",p.state,p.completed_items,s.hits.len()));
                    if let Some(path)=p.current_path {ui.label(path.display().to_string());}
                } else if let Some(r)=&s.report {
                    ui.label(format!("{} — {} visited, {} matches, {} omissions/errors",if r.cancelled{"Cancelled — partial results"}else if r.partial(){"Partial results"}else{"Complete"},r.visited,s.hits.len(),r.error_count));
                    if !r.issues.is_empty(){ui.collapsing("Details (first 256)",|ui|{egui::ScrollArea::vertical().max_height(110.0).show(ui,|ui|{for e in &r.issues {ui.label(e);}});});}
                }
                if let Some(e)=&s.error {ui.colored_label(ui.visuals().error_fg_color,e);}
                ui.separator();
                ui.small("Relative path · type · bytes · modified UTC");
                egui::ScrollArea::vertical().id_salt("search-results").auto_shrink([false,false]).show_rows(ui,22.0,s.hits.len(),|ui,range| {
                    for row in range {
                        let hit=&s.hits[row];
                        let date=hit.modified_ms.and_then(chrono::DateTime::from_timestamp_millis).map(|t|t.format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or_else(||"Unknown".into());
                        let response=ui.selectable_label(s.selected.as_ref()==Some(&hit.path),format!("{} · {:?} · {} · {date}",hit.relative_path.display(),hit.kind,hit.size));
                        if response.clicked(){s.selected=Some(hit.path.clone());}
                        if response.double_clicked(){open=Some(hit.path.clone());}
                    }
                });
            });
        if !visible {
            self.search.visible = false;
            self.search.cancel();
            self.restore_copy_focus = true;
        }
        if cancel {
            self.search.cancel();
        }
        if start {
            match self.search.query() {
                Ok(p) => self.search.start(p),
                Err(e) => self.search.error = Some(e),
            }
        }
        if let Some(hit) = open
            .and_then(|path| self.search.hits.iter().find(|h| h.path == path))
            .cloned()
        {
            // Capture the destination identity at invocation, never retain a pane index.
            let i = usize::from(self.active == PaneSide::Right);
            let target = self.panes.stack(i).active().id;
            match self
                .panes
                .find_mut(target)
                .ok_or_else(|| "Destination tab was closed".to_owned())
                .and_then(|pane| {
                    hit.open_containing(pane, &self.sorter)
                        .map_err(|e| e.to_string())
                }) {
                Ok(()) => {
                    self.search.visible = false;
                    self.restore_copy_focus = true;
                }
                Err(e) => self.search.error = Some(format!("Cannot open result: {e}")),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tree() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "mucrab-search-worker-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&p).unwrap();
        for i in 0..700 {
            std::fs::write(p.join(format!("file{i}")), b"x").unwrap();
        }
        p
    }
    fn await_done(s: &mut Search) {
        let ctx = egui::Context::default();
        let limit = std::time::Instant::now() + Duration::from_secs(5);
        while s.busy() {
            assert!(
                std::time::Instant::now() < limit,
                "search failed to acknowledge cancellation"
            );
            s.poll(&ctx);
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    #[test]
    fn full_queue_cancel_acknowledges_without_a_consumer() {
        let root = tree();
        let mut s = Search::default();
        s.start(
            SearchPlan::new(SearchQuery {
                root: root.clone(),
                ..Default::default()
            })
            .unwrap(),
        );
        // A stopped consumer fills the eight-batch queue before cancellation.
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while s
            .worker
            .as_ref()
            .unwrap()
            .control
            .snapshot()
            .completed_items
            < 576
        {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(2));
        }
        let control = s.worker.as_ref().unwrap().control.clone();
        s.cancel();
        while !control.snapshot().state.is_terminal() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(
            s.busy(),
            "worker stays owned until acknowledgment is joined"
        );
        await_done(&mut s);
        assert!(s.report.as_ref().unwrap().cancelled);
        assert!(!s.hits.is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn replacement_joins_old_worker_and_rejects_its_pending_batches() {
        let root = tree();
        let mut s = Search::default();
        s.start(
            SearchPlan::new(SearchQuery {
                root: root.clone(),
                ..Default::default()
            })
            .unwrap(),
        );
        let generation = s.generation;
        s.start(
            SearchPlan::new(SearchQuery {
                root: root.clone(),
                pattern: "no-match".into(),
                ..Default::default()
            })
            .unwrap(),
        );
        assert_eq!(s.generation, generation);
        assert!(s.pending.is_some());
        await_done(&mut s);
        assert_eq!(s.generation, generation + 1);
        assert!(s.hits.is_empty());
        assert!(!s.report.as_ref().unwrap().partial());
        std::fs::remove_dir_all(root).unwrap();
    }
}
