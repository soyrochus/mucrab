// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
use super::{MuCrabApp, PaneSide, PathBuf, TabId, egui};
use mucrab_core::{BookmarkStore, PaneLocation};
use std::collections::HashMap;
#[derive(Clone, Copy)]
pub(super) enum Action {
    New,
    Duplicate,
    Close,
    Next,
    Previous,
    Select(TabId),
    AddBookmark,
    ManageBookmarks,
    ReloadBookmarks,
    OpenBookmark(u64),
}
#[derive(Default)]
pub(super) struct Navigation {
    pub(super) store: Option<BookmarkStore>,
    pub(super) error: Option<String>,
    add: Option<(PaneLocation, String)>,
    manage: bool,
    labels: HashMap<u64, String>,
    focus: [Option<TabId>; 2],
    focus_field: bool,
}
impl Navigation {
    pub(super) fn load() -> Self {
        let result = crate::appearance::settings_path()
            .ok_or_else(|| std::io::Error::other("No configuration directory"))
            .and_then(|p| BookmarkStore::load(p.with_file_name("bookmarks.json")));
        let (store, error) = match result {
            Ok(s) => (Some(s), None),
            Err(e) => (
                None,
                Some(format!(
                    "Bookmarks were not loaded: {e}. Existing configuration was not overwritten."
                )),
            ),
        };
        Self {
            store,
            error,
            add: None,
            manage: false,
            labels: HashMap::new(),
            focus: [None, None],
            focus_field: false,
        }
    }
    pub(super) fn modal(&self) -> bool {
        self.add.is_some() || self.manage
    }
}
impl MuCrabApp {
    pub(super) fn navigation_action(&mut self, action: Action, ctx: &egui::Context) {
        let i = usize::from(self.active == PaneSide::Right);
        let result = (|| -> Result<(), String> {
            match action {
                Action::New => {
                    let home = std::env::var_os("HOME").ok_or("Home is unavailable")?;
                    self.panes
                        .stack_mut(i)
                        .add_home(&PathBuf::from(home), self.show_hidden, &self.sorter)
                        .map_err(|e| e.to_string())?;
                }
                Action::Duplicate => {
                    self.panes
                        .stack_mut(i)
                        .duplicate(&self.sorter)
                        .map_err(|e| e.to_string())?;
                }
                Action::Close => {
                    let id = self.panes.stack(i).active().id;
                    if !self.panes.stack_mut(i).close(id) {
                        return Err("Keep at least one tab in each pane.".into());
                    }
                }
                Action::Select(id) => {
                    if !self.panes.stack_mut(i).select(id) {
                        return Err("Tab no longer exists".into());
                    }
                }
                Action::Next | Action::Previous => self
                    .panes
                    .stack_mut(i)
                    .cycle(matches!(action, Action::Next)),
                Action::AddBookmark => {
                    let location = self.panes[i]
                        .current_location()
                        .ok_or("No successful location")?;
                    let label = self.panes[i]
                        .location()
                        .and_then(|p| p.file_name())
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "Root".into());
                    self.navigation.add = Some((location, label));
                    self.navigation.focus_field = true;
                }
                Action::ManageBookmarks => {
                    self.navigation.manage = true;
                    self.navigation.labels.clear();
                }
                Action::ReloadBookmarks => {
                    let n = Navigation::load();
                    self.navigation.store = n.store;
                    self.navigation.error = n.error;
                }
                Action::OpenBookmark(id) => {
                    let loc = self
                        .navigation
                        .store
                        .as_ref()
                        .and_then(|s| s.entries().iter().find(|b| b.id == id))
                        .ok_or("Bookmark no longer exists")?
                        .location
                        .clone();
                    if !self.panes[i].open_location(&loc, &self.sorter) {
                        return Err(self.panes[i]
                            .error
                            .clone()
                            .unwrap_or_else(|| "Cannot open bookmark".into()));
                    }
                }
            }
            Ok(())
        })();
        if let Err(e) = result {
            self.navigation.error = Some(e);
        }
        self.navigation.focus[i] = Some(self.panes.stack(i).active().id);
        self.restore_copy_focus = true;
        ctx.request_repaint();
    }
    pub(super) fn tab_strip(&mut self, ui: &mut egui::Ui, side: PaneSide, enabled: bool) {
        let i = usize::from(side == PaneSide::Right);
        let active_id = self.panes.stack(i).active().id;
        let mut action = None;
        ui.push_id(("tab-strip", side), |ui| {
            ui.horizontal(|ui| {
                let available = (ui.available_width() - 65.0).max(80.0);
                egui::ScrollArea::horizontal()
                    .id_salt("tabs")
                    .max_width(available)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for tab in self.panes.stack(i).tabs() {
                                let name = tab
                                    .pane
                                    .location()
                                    .and_then(|p| p.file_name())
                                    .map(|n| n.to_string_lossy().into_owned())
                                    .unwrap_or_else(|| "Root".into());
                                let label: String = name.chars().take(24).collect();
                                let response = ui.add_enabled(
                                    enabled,
                                    egui::Button::selectable(
                                        tab.id == active_id,
                                        egui::RichText::new(label).strong().color(
                                            if tab.id == active_id {
                                                egui::Color32::WHITE
                                            } else {
                                                ui.visuals().text_color()
                                            },
                                        ),
                                    )
                                    .min_size(egui::vec2(60.0, 28.0)),
                                );
                                if self.navigation.focus[i] == Some(tab.id) {
                                    response.scroll_to_me(Some(egui::Align::Center));
                                    self.navigation.focus[i] = None;
                                }
                                if response.clicked() {
                                    action = Some(Action::Select(tab.id));
                                }
                                response.on_hover_text(tab.pane.path_input.clone());
                            }
                        })
                    });
                if ui
                    .add_enabled(
                        enabled && self.panes.stack(i).len() < 32,
                        egui::Button::new("+"),
                    )
                    .on_hover_text("New Tab (Ctrl/Cmd+T); maximum 32 per pane")
                    .clicked()
                {
                    action = Some(Action::New);
                }
                if ui
                    .add_enabled(
                        enabled && self.panes.stack(i).len() > 1,
                        egui::Button::new("×"),
                    )
                    .on_hover_text("Close active tab (Ctrl/Cmd+W)")
                    .clicked()
                {
                    action = Some(Action::Close);
                }
            })
        });
        if let Some(action) = action {
            self.active = side;
            self.navigation_action(action, ui.ctx());
        }
    }
    pub(super) fn bookmark_dialogs(&mut self, ctx: &egui::Context) {
        let mut save = false;
        let mut cancel = false;
        if let Some((location, label)) = &mut self.navigation.add {
            let r = egui::Modal::new(egui::Id::new("add-bookmark")).show(ctx, |ui| {
                ui.set_width(470.0);
                ui.heading("Add Bookmark");
                ui.label(location_label(location));
                ui.label("Name:");
                let field = ui.text_edit_singleline(label);
                if self.navigation.focus_field {
                    field.request_focus();
                    self.navigation.focus_field = false;
                }
                ui.horizontal(|ui| {
                    cancel = ui.button("Cancel").clicked();
                    save = ui
                        .add_enabled(
                            self.navigation.store.is_some() && !label.trim().is_empty(),
                            egui::Button::new("Save bookmark"),
                        )
                        .clicked();
                });
                if let Some(error) = &self.navigation.error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
            });
            cancel |= r.should_close();
        }
        if cancel {
            self.navigation.add = None;
            self.restore_copy_focus = true;
        } else if save && let Some((loc, label)) = self.navigation.add.clone() {
            match self
                .navigation
                .store
                .as_mut()
                .ok_or("Bookmarks are not loaded".into())
                .and_then(|s| s.add(&label, loc).map_err(|e| e.to_string()))
            {
                Ok(_) => {
                    self.navigation.add = None;
                    self.navigation.error = None;
                    self.restore_copy_focus = true;
                }
                Err(e) => self.navigation.error = Some(format!("Bookmark not saved: {e}")),
            }
        }
        if self.navigation.manage {
            let mut change = None;
            let mut open = None;
            let r = egui::Modal::new(egui::Id::new("manage-bookmarks")).show(ctx, |ui| {
                ui.set_width(680.0);
                ui.heading("Bookmarks");
                let entries = self
                    .navigation
                    .store
                    .as_ref()
                    .map(|s| s.entries().to_vec())
                    .unwrap_or_default();
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        for b in entries {
                            ui.push_id(b.id, |ui| {
                                ui.horizontal(|ui| {
                                    let label = self
                                        .navigation
                                        .labels
                                        .entry(b.id)
                                        .or_insert(b.label.clone());
                                    ui.add(egui::TextEdit::singleline(label).desired_width(200.0))
                                        .on_hover_text(location_label(&b.location));
                                    if ui.button("Rename").clicked() {
                                        change = Some((b.id, 0, label.clone()));
                                    }
                                    if ui.button("Up").clicked() {
                                        change = Some((b.id, 1, String::new()));
                                    }
                                    if ui.button("Down").clicked() {
                                        change = Some((b.id, 2, String::new()));
                                    }
                                    if ui.button("Remove").clicked() {
                                        change = Some((b.id, 3, String::new()));
                                    }
                                    if ui.button("Open").clicked() {
                                        open = Some(b.id);
                                    }
                                })
                            });
                        }
                    });
                if let Some(e) = &self.navigation.error {
                    ui.colored_label(ui.visuals().error_fg_color, e);
                }
                ui.button("Close").clicked()
            });
            if let Some((id, kind, label)) = change {
                let result = self
                    .navigation
                    .store
                    .as_mut()
                    .ok_or_else(|| "Bookmarks are not loaded".to_owned())
                    .and_then(|s| {
                        match kind {
                            0 => s.rename(id, &label),
                            1 => s.reorder(id, true),
                            2 => s.reorder(id, false),
                            _ => s.remove(id),
                        }
                        .map_err(|e| e.to_string())
                    });
                self.navigation.error = result
                    .err()
                    .map(|e| format!("Bookmark change was not saved: {e}"));
            }
            if r.inner || r.should_close() || open.is_some() {
                self.navigation.manage = false;
                self.restore_copy_focus = true;
            }
            if let Some(id) = open {
                self.navigation_action(Action::OpenBookmark(id), ctx);
            }
        }
    }
}

fn location_label(location: &PaneLocation) -> String {
    match location {
        PaneLocation::Sftp(r) => r.display(),
        PaneLocation::Local(p) => p.display().to_string(),
        PaneLocation::Zip(z) => z.display_path().display().to_string(),
    }
}
