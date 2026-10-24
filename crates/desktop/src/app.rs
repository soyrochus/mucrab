// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

use crate::appearance::Appearance;
use eframe::egui;
use mucrab_core::{
    BatchRenameOptions, BatchRenamePlan, BrowserPane, CopyIssue, CopyIssueKind, CopyOutcome,
    CopyPolicy, CopyRequest, JobControl, JobState, MoveRequest, NameSorter, PaneEvent, PaneSide,
    TabId, TabbedPanes, ZipExtractPlan, apply_batch_rename_with_control, copy_entries_with_control,
    extract_zip_with_control, is_valid_leaf_name, move_entries_with_control,
};
use std::{path::PathBuf, sync::mpsc, time::Duration};

#[path = "everyday_ui.rs"]
mod everyday_ui;
use everyday_ui::Action as EverydayAction;
#[path = "navigation_ui.rs"]
mod navigation_ui;
use navigation_ui::Action as NavigationAction;
#[path = "browse_jobs.rs"]
mod browse_jobs;
#[path = "chrome.rs"]
mod chrome;
#[path = "confirmations.rs"]
mod confirmations;
#[path = "mutation_job.rs"]
mod mutation_job;
#[path = "workspace_ui.rs"]
mod workspace_ui;
use confirmations::{BatchReview, CopyConfirmation, ZipReview};
use mutation_job::{MutationOrigin, PendingCopy};
#[cfg(target_os = "macos")]
#[path = "native_menu.rs"]
mod native_menu;
#[path = "properties_ui.rs"]
mod properties_ui;
#[path = "read_job.rs"]
mod read_job;
#[path = "remote_ui.rs"]
mod remote_ui;
#[path = "search_ui.rs"]
mod search_ui;
#[path = "viewer_ui.rs"]
mod viewer_ui;

pub fn run(paths: Vec<PathBuf>) -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("muCrab")
            .with_inner_size([1120.0, 720.0])
            .with_min_inner_size([820.0, 500.0])
            .with_icon(egui::IconData::default()),
        renderer: eframe::Renderer::Glow,
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native(
        "muCrab",
        options,
        Box::new(move |creation| {
            let appearance = Appearance::load();
            appearance.apply(&creation.egui_ctx);
            let sorter = NameSorter::en_us().map_err(std::io::Error::other)?;
            let mut panes = [BrowserPane::default(), BrowserPane::default()];
            let default = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
            for (i, pane) in panes.iter_mut().enumerate() {
                pane.defer_browsing();
                pane.open(paths.get(i).unwrap_or(&default), &sorter);
            }
            Ok(Box::new(MuCrabApp {
                #[cfg(target_os = "macos")]
                native_menu: Some(native_menu::NativeMenu::new(&creation.egui_ctx)?),
                appearance,
                preference_error: None,
                help: None,
                split: 0.5,
                sorter,
                browse: Default::default(),
                panes: panes.into(),
                active: PaneSide::Left,
                show_hidden: false,
                confirmations: Default::default(),
                navigation: navigation_ui::Navigation::load(),
                search: search_ui::Search::default(),
                remote: remote_ui::RemoteUi::load(),
                properties: properties_ui::Properties::default(),
                viewer: viewer_ui::Viewer::default(),
                everyday: everyday_ui::Everyday::default(),
                opener: crate::opening::Opener::default(),
                copy_job: None,
                close_after_job: false,
                copy_summary: None,
                copy_details: Vec::new(),
                show_copy_details: false,
                restore_copy_focus: true,
            }))
        }),
    )
}

struct MuCrabApp {
    #[cfg(target_os = "macos")]
    native_menu: Option<native_menu::NativeMenu>,
    appearance: Appearance,
    preference_error: Option<String>,
    help: Option<bool>,
    split: f32,
    sorter: NameSorter,
    browse: browse_jobs::BrowseJobs,
    panes: TabbedPanes,
    active: PaneSide,
    show_hidden: bool,
    confirmations: confirmations::Confirmations,
    navigation: navigation_ui::Navigation,
    search: search_ui::Search,
    remote: remote_ui::RemoteUi,
    properties: properties_ui::Properties,
    viewer: viewer_ui::Viewer,
    everyday: everyday_ui::Everyday,
    opener: crate::opening::Opener,
    copy_job: Option<PendingCopy>,
    close_after_job: bool,
    copy_summary: Option<String>,
    copy_details: Vec<String>,
    show_copy_details: bool,
    restore_copy_focus: bool,
}
#[derive(Clone, Copy, PartialEq)]
enum FileAction {
    RemoteCopy,
    Metadata,
    NewFolder,
    Trash,
    Delete,
    Paste,
    Copy,
    Move,
    Rename,
    BatchRename,
    Extract,
}
impl FileAction {
    fn label(self) -> &'static str {
        match self {
            Self::RemoteCopy => "SFTP copy",
            Self::Metadata => "Metadata changes",
            Self::NewFolder => "New folder",
            Self::Trash => "Move to Trash",
            Self::Delete => "Permanent delete",
            Self::Paste => "Paste",
            Self::Copy => "Copy",
            Self::Move => "Move",
            Self::Rename => "Rename",
            Self::BatchRename => "Batch rename",
            Self::Extract => "Extract ZIP",
        }
    }
}
struct JobResult {
    cancelled: bool,
    outcomes: Vec<CopyOutcome>,
    issues: Vec<CopyIssue>,
    summary: String,
}
impl MuCrabApp {
    fn can_extract(&self) -> bool {
        let index = if self.active == PaneSide::Left { 0 } else { 1 };
        let source = &self.panes[index];
        let target = &self.panes[1 - index];
        if target.archive().is_some() || target.location().is_none() {
            return false;
        }
        let names = source
            .selection()
            .snapshot(source.entries(), source.order())
            .selected;
        !names.is_empty()
            && (source.archive().is_some()
                || (names.len() == 1
                    && source.entries().iter().any(|entry| {
                        entry.name == names[0]
                            && !entry.is_directory
                            && !entry.is_symlink
                            && entry.name.to_ascii_lowercase().ends_with(".zip")
                    })))
    }
    fn prepare_extract(&self) -> Option<ZipReview> {
        if !self.can_extract() {
            return None;
        }
        let index = if self.active == PaneSide::Left { 0 } else { 1 };
        let source = &self.panes[index];
        let names = source
            .selection()
            .snapshot(source.entries(), source.order())
            .selected;
        let source_directory = source.location().cloned()?;
        let destination = self.panes[1 - index].location()?;
        let plan = if let Some(location) = source.archive() {
            ZipExtractPlan::new(
                &location.archive,
                &location.prefix,
                Some(&names),
                destination,
                CopyPolicy::Skip,
            )
        } else {
            ZipExtractPlan::new(
                &source_directory.join(&names[0]),
                "",
                None,
                destination,
                CopyPolicy::Skip,
            )
        }
        .map_err(|error| error.to_string());
        Some(ZipReview {
            source_side: self.active,
            source_tab: self
                .panes
                .stack(usize::from(self.active == PaneSide::Right))
                .active()
                .id,
            source_directory,
            source_names: names,
            policy: CopyPolicy::Skip,
            plan,
        })
    }
    fn zip_dialog(&mut self, ctx: &egui::Context) {
        let decision = self.confirmations.draw_zip_dialog(ctx);
        if decision == confirmations::Decision::Cancel {
            self.confirmations.zip = None;
            self.restore_copy_focus = true;
            ctx.request_repaint();
        } else if decision == confirmations::Decision::Apply
            && let Some(review) = self.confirmations.zip.take()
            && let Ok(mut plan) = review.plan
        {
            plan.policy = review.policy;
            self.restore_copy_focus = true;
            let names = review.source_names;
            self.start_job(
                FileAction::Extract,
                review.source_side,
                review.source_tab,
                review.source_directory,
                None,
                move |control| {
                    extract_zip_with_control(&plan, &control)
                        .map(|report| JobResult {
                            cancelled: report.cancelled,
                            summary: format!(
                                "ZIP extraction {}: {} files · {} folders",
                                if report.success {
                                    "finished"
                                } else if report.cancelled {
                                    "cancelled"
                                } else {
                                    "stopped after an error"
                                },
                                report.files_extracted,
                                report.directories_processed
                            ),
                            outcomes: names
                                .into_iter()
                                .map(|name| CopyOutcome {
                                    name,
                                    success: report.success,
                                })
                                .collect(),
                            issues: report.issues,
                        })
                        .map_err(|error| error.to_string())
                },
            );
            ctx.request_repaint();
        }
    }
    fn prepare_batch(&self) -> Option<BatchReview> {
        let index = if self.active == PaneSide::Left { 0 } else { 1 };
        let pane = &self.panes[index];
        if pane.archive().is_some() {
            return None;
        }
        let names = pane
            .selection()
            .snapshot(pane.entries(), pane.order())
            .selected;
        if names.is_empty() {
            return None;
        }
        let directory = pane.location().cloned()?;
        let options = BatchRenameOptions::default();
        let plan =
            BatchRenamePlan::new(&directory, &names, &options).map_err(|error| error.to_string());
        Some(BatchReview {
            source_side: self.active,
            source_tab: self
                .panes
                .stack(usize::from(self.active == PaneSide::Right))
                .active()
                .id,
            directory,
            names,
            options,
            plan,
        })
    }
    fn batch_dialog(&mut self, ctx: &egui::Context) {
        let decision = self.confirmations.draw_batch_dialog(ctx);
        if decision == confirmations::Decision::Cancel {
            self.confirmations.batch = None;
            self.restore_copy_focus = true;
            ctx.request_repaint();
        } else if decision == confirmations::Decision::Apply
            && let Some(review) = self.confirmations.batch.take()
            && let Ok(plan) = review.plan
        {
            self.restore_copy_focus = true;
            self.start_job(
                FileAction::BatchRename,
                review.source_side,
                review.source_tab,
                review.directory,
                None,
                move |control| {
                    apply_batch_rename_with_control(&plan, &control)
                        .map(|report| JobResult {
                            cancelled: report.cancelled,
                            summary: format!(
                                "Batch rename finished: {} items completed",
                                report
                                    .outcomes
                                    .iter()
                                    .filter(|outcome| outcome.success)
                                    .count()
                            ),
                            outcomes: report
                                .outcomes
                                .into_iter()
                                .map(|outcome| CopyOutcome {
                                    name: outcome.name,
                                    success: outcome.success,
                                })
                                .collect(),
                            issues: report.issues,
                        })
                        .map_err(|error| error.to_string())
                },
            );
            ctx.request_repaint();
        }
    }
    fn prepare_action(&self, action: FileAction) -> Option<CopyConfirmation> {
        let side = if self.active == PaneSide::Left { 0 } else { 1 };
        let source = &self.panes[side];
        if source.archive().is_some()
            || (action != FileAction::Rename && self.panes[1 - side].archive().is_some())
        {
            return None;
        }
        let snapshot = source
            .selection()
            .snapshot(source.entries(), source.order());
        let names = if action == FileAction::Rename {
            vec![snapshot.focused.filter(|name| name != "..")?]
        } else {
            snapshot.selected
        };
        if names.is_empty() {
            return None;
        }
        Some(CopyConfirmation {
            action,
            source_side: self.active,
            source_tab: self
                .panes
                .stack(usize::from(self.active == PaneSide::Right))
                .active()
                .id,
            request: CopyRequest {
                source_directory: source.location().cloned()?,
                destination_directory: if action == FileAction::Rename {
                    source.location().cloned()?
                } else {
                    self.panes[1 - side].location().cloned()?
                },
                new_name: (action == FileAction::Rename).then(|| names[0].clone()),
                names,
                policy: CopyPolicy::Skip,
            },
        })
    }
    fn poll_copy(&mut self, ctx: &egui::Context) {
        let Some(job) = self.copy_job.as_mut() else {
            return;
        };
        let Some(result) = job.poll() else {
            ctx.request_repaint_after(Duration::from_millis(50));
            return;
        };
        let Some(job) = self.copy_job.take() else {
            return;
        };
        self.copy_details.clear();
        let mut rename_succeeded = false;
        match result {
            Ok(report) => {
                rename_succeeded = matches!(job.action, FileAction::Rename | FileAction::NewFolder)
                    && report
                        .outcomes
                        .first()
                        .is_some_and(|outcome| outcome.success);
                if !matches!(job.action, FileAction::Paste | FileAction::Metadata)
                    && let Some(pane) = self.panes.find_mut(job.source_tab)
                    && (job
                        .source_remote
                        .as_ref()
                        .is_some_and(|r| pane.remote() == Some(r))
                        || (job.source_remote.is_none()
                            && pane.remote().is_none()
                            && pane.location() == Some(&job.source_directory)))
                {
                    pane.unmark_copy_outcomes(&report.outcomes);
                }
                let skipped = report
                    .issues
                    .iter()
                    .filter(|issue| issue.kind == CopyIssueKind::Skipped)
                    .count();
                let cancelled = report
                    .issues
                    .iter()
                    .filter(|issue| issue.kind == CopyIssueKind::Cancelled)
                    .count();
                let problems = report.issues.len() - skipped - cancelled;
                self.copy_summary = Some(format!(
                    "{} · {skipped} skipped · {problems} problems",
                    if report.cancelled {
                        format!(
                            "{} cancelled; completed and partial results retained",
                            job.action.label()
                        )
                    } else {
                        report.summary
                    }
                ));
                self.copy_details = report
                    .issues
                    .iter()
                    .map(|issue| format!("{}: {}", issue.path.display(), issue.message))
                    .collect();
            }
            Err(message) => {
                self.copy_summary = Some(format!(
                    "{} failed; inspect partial results",
                    job.action.label()
                ));
                self.copy_details.push(message);
            }
        }
        for tab in self.panes.all_mut() {
            tab.pane.refresh(&self.sorter);
        }
        if let Some(name) = job.renamed_to
            && rename_succeeded
            && let Some(pane) = self.panes.find_mut(job.source_tab)
            && (job
                .source_remote
                .as_ref()
                .is_some_and(|r| pane.remote() == Some(r))
                || (job.source_remote.is_none()
                    && pane.remote().is_none()
                    && pane.location() == Some(&job.source_directory)))
        {
            pane.focus_after_browse(&name);
        }
        if job.action == FileAction::Metadata {
            self.refresh_properties_after_metadata();
        }
        self.restore_copy_focus = true;
        if self.close_after_job
            && !self.browse.busy()
            && !self.opener.busy()
            && !self.search.busy()
            && !self.remote.busy()
            && !self.properties.busy()
            && !self.viewer.busy()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
    fn copy_dialogs(&mut self, ctx: &egui::Context) {
        let decision = self.confirmations.draw_copy_dialogs(ctx);
        if decision == confirmations::Decision::Cancel {
            self.restore_copy_focus = true;
            ctx.request_repaint();
            self.confirmations.copy = None;
        } else if decision == confirmations::Decision::Apply
            && let Some(confirmation) = self.confirmations.copy.take()
        {
            self.restore_copy_focus = true;
            ctx.request_repaint();
            let source_directory = confirmation.request.source_directory.clone();
            let action = confirmation.action;
            let renamed_to = if action == FileAction::Rename {
                confirmation.request.new_name.clone()
            } else {
                None
            };
            self.start_job(action, confirmation.source_side, confirmation.source_tab, source_directory, renamed_to, move |control| {
                if action == FileAction::Copy {
                    copy_entries_with_control(&confirmation.request, &control).map(|report| JobResult {
                        cancelled: report.cancelled,
                        summary: format!("Copy finished: {} files · {} folders processed · {} links", report.files_copied, report.directories_processed, report.links_copied),
                        outcomes: report.outcomes, issues: report.issues,
                    })
                } else {
                    move_entries_with_control(&MoveRequest { transfer: confirmation.request, rename_mode: action == FileAction::Rename }, &control).map(|report| JobResult {
                        cancelled: report.cancelled,
                        summary: format!("{} finished: {} items completed · {} renamed entries · {} files transferred", action.label(), report.outcomes.iter().filter(|outcome| outcome.success).count(), report.renamed_entries, report.files_transferred),
                        outcomes: report.outcomes, issues: report.issues,
                    })
                }.map_err(|error| error.to_string())
            });
        }
        if self.show_copy_details {
            let response = egui::Modal::new(egui::Id::new("copy-details")).show(ctx, |ui| {
                ui.set_width(600.0);
                ui.heading(self.copy_summary.as_deref().unwrap_or("Copy details"));
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        for detail in &self.copy_details {
                            ui.label(detail);
                            ui.separator();
                        }
                    });
                ui.button("Close").clicked()
            });
            if response.inner || response.should_close() {
                self.show_copy_details = false;
                self.restore_copy_focus = true;
                ctx.request_repaint();
            }
        }
    }
}
fn copy_policy_label(policy: CopyPolicy) -> &'static str {
    match policy {
        CopyPolicy::Skip => "Skip existing files",
        CopyPolicy::Overwrite => "Overwrite files",
        CopyPolicy::OverwriteIfOlder => "Overwrite if source is newer",
        CopyPolicy::OverwriteIfSizeDiffers => "Overwrite if size differs",
    }
}
fn batch_case_label(mode: usize) -> &'static str {
    match mode {
        1 => "Lowercase (ASCII)",
        2 => "Uppercase (ASCII)",
        3 => "First uppercase (ASCII)",
        4 => "Word initials uppercase (ASCII)",
        _ => "Unchanged",
    }
}

impl eframe::App for MuCrabApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.restore_copy_focus {
            // egui retains the previous modal layer for one pass. Requesting
            // listing focus before that layer clears is immediately rejected.
            if ui.memory(|memory| memory.top_modal_layer().is_none()) {
                ui.memory_mut(|memory| memory.request_focus(listing_id(self.active)));
                self.restore_copy_focus = false;
            } else {
                ui.ctx().request_repaint();
            }
        }
        if ui.input(|input| input.viewport().close_requested()) {
            self.request_close(ui.ctx());
        }
        self.poll_browsing(ui.ctx());
        self.poll_remote(ui.ctx());
        self.properties.poll(ui.ctx());
        self.viewer.poll(ui.ctx());
        self.search.poll(ui.ctx());
        self.poll_everyday(ui.ctx());
        self.opener.poll();
        if self.close_after_job
            && self.copy_job.is_none()
            && !self.browse.busy()
            && !self.opener.busy()
            && !self.search.busy()
            && !self.remote.busy()
            && !self.properties.busy()
            && !self.viewer.busy()
        {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if self.opener.watching() {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }
        self.poll_copy(ui.ctx());
        #[cfg(target_os = "macos")]
        self.poll_native_menu(ui.ctx());
        let idle = !self.remote.modal()
            && self.properties.review.is_none()
            && !self.navigation.modal()
            && self.everyday.review.is_none()
            && !self.everyday.busy()
            && self.copy_job.is_none()
            && self.confirmations.copy.is_none()
            && self.confirmations.batch.is_none()
            && self.confirmations.zip.is_none()
            && !self.show_copy_details
            && self.help.is_none();
        let browsing = !self.remote.modal()
            && self.properties.review.is_none()
            && !self.navigation.modal()
            && self.everyday.review.is_none()
            && self.confirmations.copy.is_none()
            && self.confirmations.batch.is_none()
            && self.confirmations.zip.is_none()
            && !self.show_copy_details
            && self.help.is_none();
        let can_copy =
            idle && (self.prepare_action(FileAction::Copy).is_some() || self.can_remote_copy());
        let can_rename = idle && self.prepare_action(FileAction::Rename).is_some();
        let can_extract = idle && self.can_extract();
        let active_index = if self.active == PaneSide::Left { 0 } else { 1 };
        let active_pane = &self.panes[active_index];
        let can_batch = idle
            && active_pane.location().is_some()
            && active_pane.archive().is_none()
            && !active_pane
                .selection()
                .snapshot(active_pane.entries(), active_pane.order())
                .selected
                .is_empty();
        let can_move = idle && self.prepare_action(FileAction::Move).is_some();
        let archive_active = self.panes[active_index].archive().is_some();
        let can_copy = can_copy || (archive_active && can_extract);
        let old_active = self.active;
        if browsing && ui.memory(|m| m.has_focus(listing_id(self.active))) {
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::F)) {
                self.open_search();
            }
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F3)) {
                self.open_viewer(ui.ctx());
            }
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::ALT, egui::Key::Enter)) {
                self.open_properties();
            }
            let nav = ui.input_mut(|i| {
                if i.consume_key(
                    egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                    egui::Key::T,
                ) {
                    Some(NavigationAction::Duplicate)
                } else if i.consume_key(egui::Modifiers::COMMAND, egui::Key::T) {
                    Some(NavigationAction::New)
                } else if i.consume_key(egui::Modifiers::COMMAND, egui::Key::W) {
                    Some(NavigationAction::Close)
                } else if i.consume_key(
                    egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
                    egui::Key::Tab,
                ) {
                    Some(NavigationAction::Previous)
                } else if i.consume_key(egui::Modifiers::CTRL, egui::Key::Tab) {
                    Some(NavigationAction::Next)
                } else if i.consume_key(egui::Modifiers::COMMAND, egui::Key::B) {
                    Some(NavigationAction::AddBookmark)
                } else {
                    None
                }
            });
            if let Some(action) = nav {
                self.navigation_action(action, ui.ctx());
            }
        }

        if browsing && ui.memory(|m| m.has_focus(listing_id(self.active))) {
            let next = ui.input_mut(|input| {
                let copy = input.events.iter().any(|e| matches!(e, egui::Event::Copy));
                let paste = input
                    .events
                    .iter()
                    .any(|e| matches!(e, egui::Event::Paste(_)));
                input
                    .events
                    .retain(|e| !matches!(e, egui::Event::Copy | egui::Event::Paste(_)));
                if idle && (copy || input.consume_key(egui::Modifiers::COMMAND, egui::Key::C)) {
                    Some(EverydayAction::CopyClipboard)
                } else if idle
                    && (paste || input.consume_key(egui::Modifiers::COMMAND, egui::Key::V))
                {
                    Some(EverydayAction::Paste)
                } else if idle && input.consume_key(egui::Modifiers::NONE, egui::Key::F7) {
                    Some(EverydayAction::NewFolder)
                } else if idle
                    && (input.consume_key(egui::Modifiers::SHIFT, egui::Key::F8)
                        || input.consume_key(egui::Modifiers::SHIFT, egui::Key::Delete))
                {
                    Some(EverydayAction::Delete)
                } else if idle
                    && (input.consume_key(egui::Modifiers::NONE, egui::Key::F8)
                        || input.consume_key(egui::Modifiers::NONE, egui::Key::Delete))
                {
                    Some(EverydayAction::Trash)
                } else if input.consume_key(egui::Modifiers::ALT, egui::Key::ArrowLeft) {
                    Some(EverydayAction::Back)
                } else if input.consume_key(egui::Modifiers::ALT, egui::Key::ArrowRight) {
                    Some(EverydayAction::Forward)
                } else {
                    None
                }
            });
            if let Some(action) = next {
                self.everyday_action(action, ui.ctx());
            }
        }
        if idle && ui.memory(|memory| memory.has_focus(listing_id(self.active))) {
            for (key, action) in [
                (egui::Key::F5, FileAction::Copy),
                (egui::Key::F6, FileAction::Move),
                (egui::Key::F2, FileAction::Rename),
            ] {
                if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, key)) {
                    self.execute(Command::Action(action), ui.ctx());
                    break;
                }
            }
        }

        if ui.memory(|memory| memory.has_focus(listing_id(self.active)))
            && ui.input_mut(|input| {
                input.consume_key(egui::Modifiers::NONE, egui::Key::Tab)
                    || input.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab)
            })
        {
            self.active = if self.active == PaneSide::Left {
                PaneSide::Right
            } else {
                PaneSide::Left
            };
            ui.memory_mut(|memory| memory.request_focus(listing_id(self.active)));
        }
        let browsing = browsing
            && self.properties.review.is_none()
            && self.confirmations.copy.is_none()
            && self.confirmations.zip.is_none()
            && self.everyday.review.is_none()
            && !self.navigation.modal();
        let mut command;
        let old_appearance = self.appearance;
        let state = chrome::FrameState {
            idle,
            browsing,
            can_copy,
            can_move,
            can_rename,
            can_extract,
            can_batch,
            archive_active,
            active_index,
        };
        command = self.render_top(ui, state);
        if let Some(bottom) = self.render_commands(ui, state) {
            command = Some(bottom);
        }
        self.render_workspace(ui, browsing);
        if self.appearance != old_appearance {
            self.appearance.apply(ui.ctx());
            self.preference_error = self
                .appearance
                .save()
                .err()
                .map(|e| format!("Could not save appearance: {e}"));
        }
        if let Some(command) = command {
            self.execute(command, ui.ctx());
        }
        if let Some(about) = self.help {
            let response = egui::Modal::new(egui::Id::new("help")).show(ui.ctx(), |ui| {
                ui.set_width(420.0);
                if about {
                    ui.heading(format!("muCrab {}", env!("CARGO_PKG_VERSION")));
                    ui.label("Two claws, two panes.");
                    ui.label("Based on muCommander · GNU GPL version 3");
                } else {
                    ui.heading("Keyboard shortcuts");
                    for text in [
                        "Tab / Shift+Tab — switch panes",
                        "Enter — open file, folder or ZIP",
                        "F7 — New Folder · F8 / Delete — Trash · Shift+F8 — permanent delete",
                        "Ctrl/Cmd+T / Shift+T / W — new / duplicate / close tab",
                        "Ctrl+Tab / Ctrl+Shift+Tab — next / previous tab",
                        "Ctrl/Cmd+B — bookmark location · Ctrl/Cmd+F — search files",
                        "Ctrl/Cmd+C / V — copy/paste files with listing focus",
                        "Type a prefix — find a name · Alt+Left / Right — history",
                        "Backspace — parent folder",
                        "Arrow keys / Home / End — move focus",
                        "Space — mark and advance",
                        "Ctrl-click — toggle mark; Shift-click — mark range",
                        "Shift+Up / Down — mark range",
                        "Ctrl+A / Ctrl+D (⌘ on macOS) — mark / unmark all",
                        "F5 — copy (extract inside ZIP); F6 — move",
                        "F2 — rename focused item · F3 — view focused file · Alt+Enter — properties",
                        "Click a column header to sort; click again to reverse",
                        "Drag column borders or the divider to resize",
                    ] {
                        ui.label(text);
                    }
                }
                ui.add_space(8.0);
                ui.button("Close").clicked()
            });
            if response.inner || response.should_close() {
                self.help = None;
                self.restore_copy_focus = true;
            }
        }
        if old_active != self.active {
            for tab in self.panes.all_mut() {
                tab.pane.quick_find = None;
            }
        }
        if self.remote.busy() {
            ui.ctx().request_repaint_after(Duration::from_millis(30));
        }
        self.search_window(ui.ctx());
        self.properties_window(ui.ctx());
        self.viewer_window(ui.ctx());
        self.remote_window(ui.ctx());
        self.bookmark_dialogs(ui.ctx());
        self.everyday_dialog(ui.ctx());
        self.copy_dialogs(ui.ctx());
        self.properties_review(ui.ctx());
        self.batch_dialog(ui.ctx());
        self.zip_dialog(ui.ctx());
        #[cfg(target_os = "macos")]
        self.sync_native_menu(ui.ctx());
    }
}

pub(crate) fn listing_id(side: PaneSide) -> egui::Id {
    egui::Id::new(match side {
        PaneSide::Left => "left-file-list",
        PaneSide::Right => "right-file-list",
    })
}

pub(crate) fn click_modifiers(ui: &egui::Ui, rect: egui::Rect) -> egui::Modifiers {
    // A quick modifier-click can release its modifier in the same frame.
    // Read the pointer event, rather than the frame's final keyboard state.
    ui.input(|input| {
        input
            .events
            .iter()
            .rev()
            .find_map(|event| match event {
                egui::Event::PointerButton {
                    pos,
                    pressed: false,
                    modifiers,
                    ..
                } if rect.contains(*pos) => Some(*modifiers),
                _ => None,
            })
            .unwrap_or(input.modifiers)
    })
}

pub(crate) fn selection_key(event: &egui::Event) -> Option<PaneEvent> {
    if let egui::Event::Text(text) = event
        && text == "*"
    {
        return Some(PaneEvent::Invert);
    }
    let egui::Event::Key {
        key,
        pressed,
        modifiers,
        ..
    } = event
    else {
        return None;
    };
    if *key == egui::Key::Space && !pressed {
        return Some(PaneEvent::Release);
    }
    if !pressed {
        return None;
    }
    if modifiers.command && !modifiers.shift && !modifiers.alt {
        return match key {
            egui::Key::A => Some(PaneEvent::All),
            egui::Key::D => Some(PaneEvent::None),
            _ => None,
        };
    }
    if modifiers.ctrl || modifiers.command || modifiers.alt {
        return None;
    }
    match key {
        egui::Key::ArrowDown => Some(if modifiers.shift {
            PaneEvent::MarkNext
        } else {
            PaneEvent::Next
        }),
        egui::Key::ArrowUp => Some(if modifiers.shift {
            PaneEvent::MarkPrevious
        } else {
            PaneEvent::Previous
        }),
        egui::Key::Home if !modifiers.shift => Some(PaneEvent::First),
        egui::Key::End if !modifiers.shift => Some(PaneEvent::Last),
        egui::Key::Space if !modifiers.shift => Some(PaneEvent::Space),
        _ => None,
    }
}

#[derive(Clone)]
enum Command {
    #[cfg(target_os = "macos")]
    Native(native_menu::Action),
    Connections,
    View,
    Properties,
    Navigation(NavigationAction),
    Search,
    Everyday(EverydayAction),
    Open,
    Action(FileAction),
    Batch,
    Extract,
    Parent,
    Home,
    Refresh,
    SwitchPane,
    Mark(PaneEvent),
}

#[cfg(not(target_os = "macos"))]
fn menu_action(
    ui: &mut egui::Ui,
    text: &str,
    shortcut: &str,
    enabled: bool,
    command: Command,
    selected: &mut Option<Command>,
) {
    if ui
        .add_enabled(enabled, egui::Button::new(text).shortcut_text(shortcut))
        .clicked()
    {
        *selected = Some(command);
        ui.close();
    }
}

impl MuCrabApp {
    fn execute(&mut self, command: Command, ctx: &egui::Context) {
        let index = usize::from(self.active == PaneSide::Right);
        match command {
            #[cfg(target_os = "macos")]
            Command::Native(action) => {
                self.native_menu_action(action, ctx);
                return;
            }
            Command::Connections => {
                self.remote.visible = true;
                self.remote
                    .target_active(self.panes.stack(index).active().id);
            }
            Command::Navigation(action) => self.navigation_action(action, ctx),
            Command::Search => self.open_search(),
            Command::View => self.open_viewer(ctx),
            Command::Properties => self.open_properties(),
            Command::Everyday(action) => self.everyday_action(action, ctx),
            Command::Open => {
                let pane = &mut self.panes[index];
                if pane.selection().has_parent && pane.selection().focus_row == Some(0) {
                    pane.parent(&self.sorter);
                } else if let Some(row) = pane.selection().focus_row
                    && let Some(&entry) = pane
                        .order()
                        .get(row.saturating_sub(usize::from(pane.selection().has_parent)))
                {
                    let e = &pane.entries()[entry];
                    if crate::browser_ui::browsable(e, pane) {
                        if let Some(remote) = pane.remote() {
                            if let Ok(loc) = remote.child(&e.name) {
                                pane.request_remote(loc, true, None);
                            }
                        } else if let Some(path) = pane.location().map(|p| p.join(&e.name)) {
                            pane.open(&path, &self.sorter);
                        }
                    } else {
                        let target = crate::opening::target(pane, &e.name);
                        self.dispatch_target(target);
                    }
                }
            }
            Command::Action(FileAction::Copy)
                if self.panes[index].remote().is_some()
                    || self.panes[1 - index].remote().is_some() =>
            {
                self.prepare_remote_copy();
            }
            Command::Action(FileAction::Copy) if self.panes[index].archive().is_some() => {
                self.confirmations.zip = self.prepare_extract();
            }
            Command::Action(action) => {
                self.confirmations.copy = self.prepare_action(action);
            }
            Command::Batch => {
                self.confirmations.batch = self.prepare_batch();
            }
            Command::Extract => {
                self.confirmations.zip = self.prepare_extract();
            }
            Command::Parent => {
                self.panes[index].parent(&self.sorter);
            }
            Command::Refresh => {
                self.panes[index].refresh(&self.sorter);
            }
            Command::Home => {
                if let Some(home) = std::env::var_os("HOME") {
                    self.panes[index].open(&PathBuf::from(home), &self.sorter);
                }
            }
            Command::Mark(event) => {
                let _ = self.panes[index].apply_event(&event, &self.sorter);
            }
            Command::SwitchPane => {
                self.active = if self.active == PaneSide::Left {
                    PaneSide::Right
                } else {
                    PaneSide::Left
                };
            }
        }
        if self.confirmations.copy.is_none()
            && self.confirmations.batch.is_none()
            && self.confirmations.zip.is_none()
        {
            ctx.memory_mut(|m| m.request_focus(listing_id(self.active)));
        }
    }
}

impl MuCrabApp {
    /// Window close and the native Quit menu share cancellation/acknowledgment.
    fn request_close(&mut self, ctx: &egui::Context) {
        let busy = self.browse.busy()
            || self.copy_job.is_some()
            || self.opener.busy()
            || self.search.busy()
            || self.properties.busy()
            || self.viewer.busy()
            || self.remote.busy();
        if !busy {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        self.close_after_job = true;
        self.browse.cancel();
        if let Some(job) = &self.copy_job {
            job.control.cancel();
        }
        if self.opener.busy() {
            self.opener.cancel();
        }
        if self.search.busy() {
            self.search.cancel();
        }
        if self.properties.busy() {
            self.properties.cancel();
        }
        if self.viewer.busy() {
            self.viewer.cancel();
        }
        if self.remote.busy() {
            self.remote.cancel();
        }
    }

    fn start_job(
        &mut self,
        action: FileAction,
        _source_side: PaneSide,
        source_tab: TabId,
        source_directory: PathBuf,
        renamed_to: Option<String>,
        work: impl FnOnce(JobControl) -> Result<JobResult, String> + Send + 'static,
    ) {
        // The UI disables every mutation trigger while this worker exists.
        if self.copy_job.is_some() {
            return;
        }
        let origin = MutationOrigin {
            action,
            source_tab,
            source_directory,
            renamed_to,
        };
        match PendingCopy::start(origin, work) {
            Ok(job) => {
                self.copy_summary = None;
                self.copy_details.clear();
                self.copy_job = Some(job);
            }
            Err(error) => {
                self.copy_summary = Some(format!("{} could not start", action.label()));
                self.copy_details = vec![error.to_string()];
            }
        }
    }
}

#[cfg(test)]
mod job_tests {
    use super::*;
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::Instant,
    };

    pub(super) fn app() -> MuCrabApp {
        MuCrabApp {
            #[cfg(target_os = "macos")]
            native_menu: None,
            appearance: Appearance::System,
            preference_error: None,
            help: None,
            split: 0.5,
            sorter: NameSorter::en_us().unwrap(),
            browse: Default::default(),
            panes: [BrowserPane::default(), BrowserPane::default()].into(),
            active: PaneSide::Left,
            show_hidden: false,
            confirmations: Default::default(),
            navigation: navigation_ui::Navigation::default(),
            search: search_ui::Search::default(),
            remote: remote_ui::RemoteUi::load(),
            properties: properties_ui::Properties::default(),
            viewer: viewer_ui::Viewer::default(),
            everyday: everyday_ui::Everyday::default(),
            opener: crate::opening::Opener::default(),
            copy_job: None,
            close_after_job: false,
            copy_summary: None,
            copy_details: Vec::new(),
            show_copy_details: false,
            restore_copy_focus: false,
        }
    }
    fn report() -> JobResult {
        JobResult {
            cancelled: false,
            outcomes: vec![],
            issues: vec![],
            summary: "Finished".into(),
        }
    }
    fn await_job(app: &mut MuCrabApp, ctx: &egui::Context) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while app.copy_job.is_some() {
            assert!(
                Instant::now() < deadline,
                "worker acknowledgement timed out"
            );
            app.poll_copy(ctx);
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    #[test]
    fn closing_waits_for_a_cancelled_worker_to_acknowledge() {
        let mut app = app();
        let ctx = egui::Context::default();
        let (release, wait) = mpsc::channel();
        app.start_job(
            FileAction::Copy,
            PaneSide::Left,
            app.panes.stack(0).active().id,
            PathBuf::from("/unused"),
            None,
            move |_| {
                wait.recv().unwrap();
                Ok(report())
            },
        );
        let control = app.copy_job.as_ref().unwrap().control.clone();
        let mut output = ctx.run_ui(Default::default(), |_| {
            #[cfg(target_os = "macos")]
            app.execute(Command::Native(native_menu::Action::Close), &ctx);
            #[cfg(not(target_os = "macos"))]
            app.request_close(&ctx);
            app.poll_copy(&ctx);
        });
        output.textures_delta.clear();
        let commands = &output.viewport_output[&egui::ViewportId::ROOT].commands;
        assert!(commands.contains(&egui::ViewportCommand::CancelClose));
        assert!(!commands.contains(&egui::ViewportCommand::Close));
        assert!(app.close_after_job);
        assert!(app.copy_job.is_some());
        assert_eq!(control.snapshot().state, JobState::Cancelling);

        release.send(()).unwrap();
        let mut output = ctx.run_ui(Default::default(), |_| await_job(&mut app, &ctx));
        output.textures_delta.clear();
        assert!(app.copy_job.is_none());
        assert_eq!(control.snapshot().state, JobState::Cancelled);
        assert!(
            output.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .contains(&egui::ViewportCommand::Close)
        );
    }

    #[test]
    fn cancelling_waits_for_worker_acknowledgement_and_blocks_a_second_job() {
        let mut app = app();
        let ctx = egui::Context::default();
        let (release, wait) = mpsc::channel();
        app.start_job(
            FileAction::Copy,
            PaneSide::Left,
            app.panes.stack(0).active().id,
            PathBuf::from("/unused"),
            None,
            move |_| {
                wait.recv().unwrap();
                Ok(report())
            },
        );
        let control = app.copy_job.as_ref().unwrap().control.clone();
        control.cancel();
        app.poll_copy(&ctx);
        assert_eq!(control.snapshot().state, JobState::Cancelling);
        assert!(app.copy_job.is_some());
        assert!(app.copy_summary.is_none());
        let invoked = Arc::new(AtomicBool::new(false));
        let second = invoked.clone();
        app.start_job(
            FileAction::Move,
            PaneSide::Right,
            app.panes.stack(1).active().id,
            PathBuf::from("/unused"),
            None,
            move |_| {
                second.store(true, Ordering::SeqCst);
                Ok(report())
            },
        );
        release.send(()).unwrap();
        await_job(&mut app, &ctx);
        assert!(!invoked.load(Ordering::SeqCst));
        assert_eq!(control.snapshot().state, JobState::Cancelled);
        assert!(app.copy_summary.as_ref().unwrap().contains("cancelled"));
    }
    #[test]
    fn worker_errors_and_panics_become_visible_terminal_failures() {
        let mut app = app();
        let ctx = egui::Context::default();
        for panic in [false, true] {
            app.start_job(
                FileAction::Copy,
                PaneSide::Left,
                app.panes.stack(0).active().id,
                PathBuf::from("/unused"),
                None,
                move |_| {
                    if panic {
                        panic!("injected worker fault");
                    }
                    Err("injected I/O failure".into())
                },
            );
            let control = app.copy_job.as_ref().unwrap().control.clone();
            await_job(&mut app, &ctx);
            assert_eq!(control.snapshot().state, JobState::Failed);
            assert!(app.copy_summary.as_ref().unwrap().contains("failed"));
            assert!(app.copy_details[0].contains(if panic {
                "panicked"
            } else {
                "injected I/O failure"
            }));
        }
    }
    #[test]
    fn completion_does_not_clear_marks_in_an_unrelated_browsed_location() {
        let mut app = app();
        let ctx = egui::Context::default();
        let path = std::env::temp_dir().join(format!(
            "mucrab-worker-navigation-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("same-name"), b"unrelated").unwrap();
        app.panes[0].open(&path, &app.sorter);
        app.panes[0]
            .apply_event(&PaneEvent::All, &app.sorter)
            .unwrap();
        app.start_job(
            FileAction::Copy,
            PaneSide::Left,
            app.panes.stack(0).active().id,
            path.join("elsewhere"),
            None,
            move |_| {
                let mut report = report();
                report.outcomes.push(CopyOutcome {
                    name: "same-name".into(),
                    success: true,
                });
                Ok(report)
            },
        );
        await_job(&mut app, &ctx);
        assert_eq!(app.panes[0].selection().marked_count(), 1);
        assert_eq!(std::fs::read(path.join("same-name")).unwrap(), b"unrelated");
        app.start_job(
            FileAction::Paste,
            PaneSide::Left,
            app.panes.stack(0).active().id,
            path.clone(),
            None,
            move |_| {
                let mut result = report();
                result.outcomes.push(CopyOutcome {
                    name: "same-name".into(),
                    success: true,
                });
                Ok(result)
            },
        );
        await_job(&mut app, &ctx);
        assert_eq!(
            app.panes[0].selection().marked_count(),
            1,
            "Paste must not unmark destination selections"
        );
        std::fs::remove_dir_all(path).unwrap();
    }
    #[test]
    fn jobs_unmark_only_surviving_origin_tab_even_at_the_same_path() {
        for close_origin in [false, true] {
            let mut app = app();
            let ctx = egui::Context::default();
            let root = std::env::temp_dir().join(format!(
                "mucrab-tab-job-{}-{}-{close_origin}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir(&root).unwrap();
            std::fs::write(root.join("same-name"), b"source").unwrap();
            app.panes[0].open(&root, &app.sorter);
            app.panes[0]
                .apply_event(&PaneEvent::All, &app.sorter)
                .unwrap();
            let origin = app.panes.stack(0).active().id;
            let (send, receive) = mpsc::channel();
            app.start_job(
                FileAction::Copy,
                PaneSide::Left,
                origin,
                root.clone(),
                None,
                move |_| {
                    receive.recv().unwrap();
                    let mut r = report();
                    r.outcomes.push(CopyOutcome {
                        name: "same-name".into(),
                        success: true,
                    });
                    Ok(r)
                },
            );
            let duplicate = app.panes.stack_mut(0).duplicate(&app.sorter).unwrap();
            app.panes[0]
                .apply_event(&PaneEvent::All, &app.sorter)
                .unwrap();
            if close_origin {
                assert!(app.panes.stack_mut(0).close(origin));
            }
            send.send(()).unwrap();
            await_job(&mut app, &ctx);
            assert_eq!(
                app.panes
                    .find_mut(duplicate)
                    .unwrap()
                    .selection()
                    .marked_count(),
                1
            );
            if !close_origin {
                assert_eq!(
                    app.panes
                        .find_mut(origin)
                        .unwrap()
                        .selection()
                        .marked_count(),
                    0
                );
            }
            std::fs::remove_dir_all(root).unwrap();
        }
    }
    #[test]
    fn search_finishes_without_changing_paused_mutation_or_its_progress() {
        let mut app = app();
        let ctx = egui::Context::default();
        let root = std::env::temp_dir().join(format!(
            "mucrab-concurrent-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("hit.txt"), b"x").unwrap();
        let (send, recv) = mpsc::channel();
        app.start_job(
            FileAction::Copy,
            PaneSide::Left,
            app.panes.stack(0).active().id,
            root.clone(),
            None,
            move |_| {
                recv.recv().unwrap();
                Ok(report())
            },
        );
        let control = app.copy_job.as_ref().unwrap().control.clone();
        control.set_paused(true);
        app.search.start(
            mucrab_core::SearchPlan::new(mucrab_core::SearchQuery {
                root: root.clone(),
                ..Default::default()
            })
            .unwrap(),
        );
        let limit = Instant::now() + Duration::from_secs(5);
        while app.search.busy() {
            assert!(Instant::now() < limit);
            app.search.poll(&ctx);
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(control.snapshot().state, JobState::Paused);
        assert_eq!(control.snapshot().completed_items, 0);
        assert!(app.copy_job.is_some());
        assert!(app.copy_summary.is_none());
        send.send(()).unwrap();
        await_job(&mut app, &ctx);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn viewer_escape_closes_focused_window_and_metadata_preserves_other_marks() {
        let mut app = app();
        let ctx = egui::Context::default();
        let root = std::env::temp_dir().join(format!(
            "mucrab-properties-focus-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("focused.txt"), b"hello\n").unwrap();
        std::fs::write(root.join("marked.txt"), b"marked").unwrap();
        app.panes[0].open(&root, &app.sorter);
        app.panes[0].focus_name("marked.txt");
        app.panes[0]
            .apply_event(&PaneEvent::Space, &app.sorter)
            .unwrap();
        app.panes[0].focus_name("focused.txt");
        let marks = app.panes[0].selection().marked_count();
        assert_eq!(
            app.focused_target().unwrap().display_path(),
            root.join("focused.txt")
        );
        app.open_viewer(&ctx);
        let deadline = Instant::now() + Duration::from_secs(5);
        while app.viewer.busy() {
            assert!(Instant::now() < deadline);
            app.viewer.poll(&ctx);
            std::thread::sleep(Duration::from_millis(2));
        }
        for _ in 0..3 {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1120., 720.),
                    )),
                    ..Default::default()
                },
                |_| app.viewer_window(&ctx),
            )
            .drop_without_applying_deltas();
        }
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1120., 720.),
                )),
                events: vec![egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            },
            |_| app.viewer_window(&ctx),
        )
        .drop_without_applying_deltas();
        assert!(!app.viewer.visible);
        assert_eq!(app.panes[0].selection().marked_count(), marks);
        let snapshot =
            mucrab_core::PropertySnapshot::capture(app.focused_target().unwrap()).unwrap();
        let plan = snapshot.metadata_plan(Some(0o640), None).unwrap();
        let origin = app.panes.stack(0).active().id;
        app.start_job(
            FileAction::Metadata,
            PaneSide::Left,
            origin,
            root.clone(),
            None,
            move |c| {
                let r = plan.apply(&c).unwrap();
                assert!(r.steps[0].changed);
                Ok(report())
            },
        );
        await_job(&mut app, &ctx);
        assert_eq!(app.panes[0].selection().marked_count(), marks);
        std::fs::remove_dir_all(root).unwrap();
    }
}
