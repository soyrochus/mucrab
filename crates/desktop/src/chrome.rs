// SPDX-License-Identifier: GPL-3.0-only
//! Stateless desktop chrome rendering receives explicit command availability.
#[cfg(not(target_os = "macos"))]
use super::{Appearance, NavigationAction, menu_action};
use super::{Command, EverydayAction, FileAction, JobState, MuCrabApp, PaneSide};
use crate::appearance::{Icon, icon_button};
use eframe::egui;
#[cfg(not(target_os = "macos"))]
use mucrab_core::{PaneEvent, SortCriterion, SortMode};
#[derive(Clone, Copy)]
pub(super) struct FrameState {
    pub(super) idle: bool,
    pub(super) browsing: bool,
    pub(super) can_copy: bool,
    pub(super) can_move: bool,
    pub(super) can_rename: bool,
    pub(super) can_extract: bool,
    pub(super) can_batch: bool,
    pub(super) archive_active: bool,
    pub(super) active_index: usize,
}

impl MuCrabApp {
    pub(super) fn render_top(&mut self, ui: &mut egui::Ui, state: FrameState) -> Option<Command> {
        let FrameState {
            idle,
            browsing,
            can_copy,
            can_move,
            can_rename,
            can_extract,
            can_batch,
            archive_active,
            active_index,
            ..
        } = state;
        let mut command = None;
        #[cfg(target_os = "macos")]
        let _ = (idle, can_batch, archive_active);
        egui::Panel::top("menus").show(ui, |ui| {
            #[cfg(not(target_os = "macos"))]
            let _ = egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    menu_action(ui, "Open", "Enter", browsing, Command::Open, &mut command);
                    menu_action(
                        ui,
                        "Copy…",
                        "F5",
                        can_copy,
                        Command::Action(FileAction::Copy),
                        &mut command,
                    );
                    menu_action(
                        ui,
                        "Move…",
                        "F6",
                        can_move,
                        Command::Action(FileAction::Move),
                        &mut command,
                    );
                    menu_action(
                        ui,
                        "Rename…",
                        "F2",
                        can_rename,
                        Command::Action(FileAction::Rename),
                        &mut command,
                    );
                    menu_action(
                        ui,
                        "Batch rename…",
                        "",
                        can_batch,
                        Command::Batch,
                        &mut command,
                    );
                    menu_action(
                        ui,
                        "Extract ZIP…",
                        "",
                        can_extract,
                        Command::Extract,
                        &mut command,
                    );
                    ui.separator();
                    let local = !archive_active && self.panes[active_index].location().is_some();
                    for (label, shortcut, action, enabled) in [
                        (
                            "New Folder…",
                            "F7",
                            EverydayAction::NewFolder,
                            idle && local,
                        ),
                        (
                            "Move to Trash…",
                            "F8 / Delete",
                            EverydayAction::Trash,
                            can_batch,
                        ),
                        (
                            "Permanently delete…",
                            "Shift+F8",
                            EverydayAction::Delete,
                            can_batch,
                        ),
                        (
                            "Copy files to clipboard",
                            "Ctrl/Cmd+C",
                            EverydayAction::CopyClipboard,
                            idle && local && can_batch,
                        ),
                        (
                            "Paste files…",
                            "Ctrl/Cmd+V",
                            EverydayAction::Paste,
                            idle && local,
                        ),
                    ] {
                        menu_action(
                            ui,
                            label,
                            shortcut,
                            enabled,
                            Command::Everyday(action),
                            &mut command,
                        );
                    }
                    ui.separator();
                    menu_action(
                        ui,
                        "View file…",
                        "F3",
                        browsing
                            && (self.focused_target().is_some()
                                || self.panes[active_index].remote().is_some()),
                        Command::View,
                        &mut command,
                    );
                    menu_action(
                        ui,
                        "Properties…",
                        "Alt+Enter",
                        browsing && self.focused_target().is_some(),
                        Command::Properties,
                        &mut command,
                    );
                    if ui.button("Close window").clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Mark", |ui| {
                    for (label, event) in [
                        ("Mark all", PaneEvent::All),
                        ("Unmark all", PaneEvent::None),
                        ("Invert selection", PaneEvent::Invert),
                    ] {
                        menu_action(ui, label, "", browsing, Command::Mark(event), &mut command);
                    }
                });
                ui.menu_button("View", |ui| {
                    if ui
                        .add_enabled(
                            browsing,
                            egui::Checkbox::new(&mut self.show_hidden, "Show hidden files"),
                        )
                        .changed()
                    {
                        for tab in self.panes.all_mut() {
                            tab.pane.set_show_hidden(self.show_hidden, &self.sorter);
                        }
                    }
                    let pane = &mut self.panes[active_index];
                    let before = pane.options;
                    ui.add_enabled_ui(browsing, |ui| {
                        ui.checkbox(&mut pane.options.directories_first, "Folders first");
                        ui.checkbox(&mut pane.options.ascending, "Ascending order");
                        ui.menu_button("Sort by", |ui| {
                            for (criterion, label) in [
                                (SortCriterion::Name, "Name"),
                                (SortCriterion::Extension, "Extension"),
                                (SortCriterion::Size, "Size"),
                                (SortCriterion::Date, "Modified"),
                            ] {
                                ui.radio_value(&mut pane.options.criterion, criterion, label);
                            }
                        });
                        ui.menu_button("Name ordering", |ui| {
                            ui.radio_value(&mut pane.options.mode, SortMode::Natural, "Natural");
                            ui.radio_value(
                                &mut pane.options.mode,
                                SortMode::Lexicographic,
                                "Lexicographic",
                            );
                        });
                    });
                    if pane.options != before {
                        pane.resort(&self.sorter);
                    }
                    ui.separator();
                    ui.menu_button("Appearance", |ui| {
                        ui.radio_value(&mut self.appearance, Appearance::System, "System");
                        ui.radio_value(&mut self.appearance, Appearance::Light, "Light");
                        ui.radio_value(&mut self.appearance, Appearance::Dark, "Dark");
                    });
                    menu_action(
                        ui,
                        "Find by name…",
                        "Type a prefix",
                        browsing,
                        Command::Everyday(EverydayAction::Find),
                        &mut command,
                    );
                });
                ui.menu_button("Go", |ui| {
                    menu_action(
                        ui,
                        "Back",
                        "Alt+Left",
                        browsing && self.panes[active_index].can_back(),
                        Command::Everyday(EverydayAction::Back),
                        &mut command,
                    );
                    menu_action(
                        ui,
                        "Forward",
                        "Alt+Right",
                        browsing && self.panes[active_index].can_forward(),
                        Command::Everyday(EverydayAction::Forward),
                        &mut command,
                    );
                    menu_action(
                        ui,
                        "Parent folder",
                        "Backspace",
                        browsing,
                        Command::Parent,
                        &mut command,
                    );
                    menu_action(ui, "Home folder", "", browsing, Command::Home, &mut command);
                    menu_action(ui, "Refresh", "", browsing, Command::Refresh, &mut command);
                    menu_action(
                        ui,
                        "Search files…",
                        "Ctrl/Cmd+F",
                        browsing && self.panes[active_index].remote().is_none(),
                        Command::Search,
                        &mut command,
                    );
                    ui.menu_button("Bookmarks", |ui| {
                        menu_action(
                            ui,
                            "Add current location…",
                            "Ctrl/Cmd+B",
                            browsing && self.navigation.store.is_some(),
                            Command::Navigation(NavigationAction::AddBookmark),
                            &mut command,
                        );
                        menu_action(
                            ui,
                            "Manage bookmarks…",
                            "",
                            browsing,
                            Command::Navigation(NavigationAction::ManageBookmarks),
                            &mut command,
                        );
                        menu_action(
                            ui,
                            "Reload bookmarks",
                            "",
                            browsing,
                            Command::Navigation(NavigationAction::ReloadBookmarks),
                            &mut command,
                        );
                        ui.separator();
                        if let Some(store) = &self.navigation.store {
                            for b in store.entries() {
                                menu_action(
                                    ui,
                                    &b.label,
                                    "",
                                    browsing,
                                    Command::Navigation(NavigationAction::OpenBookmark(b.id)),
                                    &mut command,
                                );
                            }
                        }
                    });
                    ui.separator();
                    menu_action(
                        ui,
                        "SFTP connections…",
                        "",
                        browsing,
                        Command::Connections,
                        &mut command,
                    );
                });
                ui.menu_button("Window", |ui| {
                    for (label, shortcut, action, enabled) in [
                        (
                            "New Tab",
                            "Ctrl/Cmd+T",
                            NavigationAction::New,
                            browsing && self.panes.stack(active_index).len() < 32,
                        ),
                        (
                            "Duplicate Tab",
                            "Ctrl/Cmd+Shift+T",
                            NavigationAction::Duplicate,
                            browsing && self.panes.stack(active_index).len() < 32,
                        ),
                        (
                            "Close Tab",
                            "Ctrl/Cmd+W",
                            NavigationAction::Close,
                            browsing && self.panes.stack(active_index).len() > 1,
                        ),
                        ("Next Tab", "Ctrl+Tab", NavigationAction::Next, browsing),
                        (
                            "Previous Tab",
                            "Ctrl+Shift+Tab",
                            NavigationAction::Previous,
                            browsing,
                        ),
                    ] {
                        menu_action(
                            ui,
                            label,
                            shortcut,
                            enabled,
                            Command::Navigation(action),
                            &mut command,
                        );
                    }
                    ui.separator();
                    menu_action(
                        ui,
                        "Switch pane",
                        "Tab",
                        browsing,
                        Command::SwitchPane,
                        &mut command,
                    );
                    if ui.button("Split equally").clicked() {
                        self.split = 0.5;
                        ui.close();
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("Keyboard shortcuts").clicked() {
                        self.help = Some(false);
                        ui.close();
                    }
                    if ui.button("About muCrab").clicked() {
                        self.help = Some(true);
                        ui.close();
                    }
                });
            });
            #[cfg(not(target_os = "macos"))]
            ui.separator();
            ui.horizontal(|ui| {
                for (icon, tip, enabled, action) in [
                    (
                        Icon::Parent,
                        "Parent folder (Backspace)",
                        browsing,
                        Command::Parent,
                    ),
                    (Icon::Home, "Home folder", browsing, Command::Home),
                    (
                        Icon::Refresh,
                        "Refresh active pane",
                        browsing,
                        Command::Refresh,
                    ),
                    (
                        Icon::Copy,
                        "Copy to other pane (F5)",
                        can_copy,
                        Command::Action(FileAction::Copy),
                    ),
                    (
                        Icon::Move,
                        "Move to other pane (F6)",
                        can_move,
                        Command::Action(FileAction::Move),
                    ),
                    (
                        Icon::Rename,
                        "Rename focused item (F2)",
                        can_rename,
                        Command::Action(FileAction::Rename),
                    ),
                    (
                        Icon::Archive,
                        "Extract ZIP to other pane",
                        can_extract,
                        Command::Extract,
                    ),
                ] {
                    if icon_button(ui, icon, tip, enabled).clicked() {
                        command = Some(action);
                    }
                }
                ui.separator();
                if icon_button(
                    ui,
                    Icon::Back,
                    "Back (Alt+Left)",
                    browsing && self.panes[active_index].can_back(),
                )
                .clicked()
                {
                    command = Some(Command::Everyday(EverydayAction::Back));
                }
                if icon_button(
                    ui,
                    Icon::Forward,
                    "Forward (Alt+Right)",
                    browsing && self.panes[active_index].can_forward(),
                )
                .clicked()
                {
                    command = Some(Command::Everyday(EverydayAction::Forward));
                }
                ui.weak(if self.active == PaneSide::Left {
                    "Left > Right"
                } else {
                    "Right > Left"
                });
                if let Some(job) = &self.copy_job {
                    let state = job.control.snapshot().state;
                    if state != JobState::Paused {
                        ui.add(egui::Spinner::new().size(16.0));
                    }
                    ui.label(format!(
                        "{} {}",
                        job.action.label(),
                        if state == JobState::Paused {
                            "paused"
                        } else if state == JobState::Cancelling {
                            "cancelling…"
                        } else {
                            "in progress…"
                        }
                    ));
                }
            });
        });
        command
    }
    pub(super) fn render_commands(
        &mut self,
        ui: &mut egui::Ui,
        state: FrameState,
    ) -> Option<Command> {
        let FrameState {
            browsing,
            can_copy,
            can_move,
            can_rename,
            can_extract,
            can_batch,
            archive_active,
            ..
        } = state;
        let mut command = None;
        egui::Panel::bottom("commands").show(ui, |ui| {
            if let Some(error) = self.navigation.error.clone() {
                ui.horizontal(|ui| {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                    if ui.small_button("Dismiss").clicked() {
                        self.navigation.error = None;
                    }
                });
            }
            if let Some(message) = self.everyday.message.clone() {
                ui.horizontal(|ui| {
                    ui.label(message);
                    if ui.small_button("Dismiss").clicked() {
                        self.everyday.message = None;
                    }
                });
            }
            if let Some(message) = self.remote.message.clone() {
                ui.horizontal_wrapped(|ui| {
                    ui.label(message);
                    if ui.small_button("Connections…").clicked() {
                        self.remote.visible = true;
                    }
                    if ui.small_button("Dismiss").clicked() {
                        self.remote.message = None;
                    }
                });
            }
            if self.remote.preparation_busy() {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("SFTP preparation…");
                    if ui.button("Cancel SFTP preparation").clicked() {
                        self.remote.cancel_preparation();
                    }
                });
            }
            if let Some(message) = self.opener.message.clone() {
                ui.horizontal(|ui| {
                    ui.label(message);
                    if self.opener.busy() && ui.button("Cancel Open").clicked() {
                        self.opener.cancel();
                    }
                    if ui.small_button("Dismiss").clicked() {
                        self.opener.message = None;
                    }
                });
            }
            if let Some(error) = &self.preference_error {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
            ui.horizontal(|ui| {
                if let Some(job) = &self.copy_job {
                    let progress = job.control.snapshot();
                    let fraction = progress
                        .current_file_size
                        .filter(|size| *size > 0)
                        .map_or(0.0, |size| progress.current_file_bytes as f32 / size as f32);
                    ui.add(
                        egui::ProgressBar::new(fraction.clamp(0.0, 1.0))
                            .desired_width(155.0)
                            .text(format!(
                                "{} / {} B",
                                progress.current_file_bytes,
                                progress.current_file_size.unwrap_or(0)
                            )),
                    );
                    ui.label(format!(
                        "{} / {} items",
                        progress.completed_items, progress.total_items
                    ));
                    let stopping = matches!(progress.state, JobState::Cancelling);
                    if ui
                        .add_enabled(
                            !stopping,
                            egui::Button::new(if progress.state == JobState::Paused {
                                "Resume"
                            } else {
                                "Pause"
                            }),
                        )
                        .clicked()
                    {
                        job.control.set_paused(progress.state != JobState::Paused);
                    }
                    if ui
                        .add_enabled(
                            !stopping,
                            egui::Button::new(if stopping { "Cancelling…" } else { "Cancel" }),
                        )
                        .clicked()
                    {
                        job.control.cancel();
                    }
                    let path = progress
                        .current_path
                        .map_or_else(|| "Starting…".to_owned(), |path| path.display().to_string());
                    ui.add(egui::Label::new(path.clone()).truncate())
                        .on_hover_text(format!(
                            "{path}\nTotal transferred: {} B",
                            progress.transferred_bytes
                        ));
                } else if let Some(summary) = &self.copy_summary {
                    ui.label(summary);
                    if !self.copy_details.is_empty() && ui.small_button("Details…").clicked() {
                        self.show_copy_details = true;
                    }
                } else {
                    ui.weak("Ready");
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                let width =
                    ((ui.available_width() - 5.0 * ui.spacing().item_spacing.x) / 6.0).max(60.0);
                for (label, enabled, action) in [
                    (
                        if archive_active {
                            "Extract  F5"
                        } else {
                            "Copy  F5"
                        },
                        can_copy,
                        Command::Action(FileAction::Copy),
                    ),
                    ("Move  F6", can_move, Command::Action(FileAction::Move)),
                    (
                        "Rename  F2",
                        can_rename,
                        Command::Action(FileAction::Rename),
                    ),
                    ("Batch rename…", can_batch, Command::Batch),
                    ("Extract ZIP…", can_extract, Command::Extract),
                    ("Refresh", browsing, Command::Refresh),
                ] {
                    if ui
                        .add_enabled(
                            enabled,
                            egui::Button::new(label).min_size(egui::vec2(width, 32.0)),
                        )
                        .clicked()
                    {
                        command = Some(action);
                    }
                }
            });
        });
        command
    }
}
