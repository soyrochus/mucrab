// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

use eframe::egui;
use mucrab_core::{BrowserPane, NameSorter, PaneSide, SortMode};
use std::path::PathBuf;

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
            creation.egui_ctx.set_visuals(egui::Visuals::dark());
            let sorter = NameSorter::en_us().map_err(std::io::Error::other)?;
            let mut panes = [BrowserPane::default(), BrowserPane::default()];
            let default = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
            for (i, pane) in panes.iter_mut().enumerate() {
                pane.open(paths.get(i).unwrap_or(&default), &sorter);
            }
            Ok(Box::new(MuCrabApp { sorter, panes }))
        }),
    )
}

struct MuCrabApp {
    sorter: NameSorter,
    panes: [BrowserPane; 2],
}

impl eframe::App for MuCrabApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("muCrab").size(30.0));
                ui.add_space(12.0);
                ui.label("Two claws, two panes.");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.colored_label(egui::Color32::from_rgb(115, 211, 165), "W01 · Local browsing");
                });
            });
            ui.add_space(6.0);
            ui.label("Name sorting · en-US · All entries shown, including hidden files");
            ui.small("Open folders with a double-click. File operations and opening symbolic links are deferred.");
            ui.add_space(14.0);
            let height = (ui.available_height() - 40.0).max(180.0);
            ui.columns(2, |columns| {
                pane(&mut columns[0], PaneSide::Left, &mut self.panes[0], &self.sorter, height);
                pane(&mut columns[1], PaneSide::Right, &mut self.panes[1], &self.sorter, height);
            });
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.small(format!("muCrab {}", env!("CARGO_PKG_VERSION")));
                ui.separator(); ui.small("GNU GPL version 3");
                ui.separator(); ui.small("Based on muCommander");
            });
        });
    }
}

fn pane(
    ui: &mut egui::Ui,
    side: PaneSide,
    pane: &mut BrowserPane,
    sorter: &NameSorter,
    height: f32,
) {
    let title = match side {
        PaneSide::Left => "Left pane",
        PaneSide::Right => "Right pane",
    };
    ui.push_id(title, |ui| {
        egui::Frame::group(ui.style())
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.set_min_height(height - 24.0);
                ui.strong(title);
                ui.add_space(6.0);
                let mut open = false;
                ui.horizontal(|ui| {
                    let edit = ui.add(
                        egui::TextEdit::singleline(&mut pane.path_input)
                            .hint_text("Folder path")
                            .desired_width((ui.available_width() - 56.0).max(140.0)),
                    );
                    open = ui.button("Open").clicked()
                        || (edit.lost_focus()
                            && ui.input(|input| input.key_pressed(egui::Key::Enter)));
                });
                if open {
                    pane.open(&PathBuf::from(&pane.path_input), sorter);
                }
                ui.horizontal(|ui| {
                    let has_parent = pane
                        .location
                        .as_deref()
                        .and_then(std::path::Path::parent)
                        .is_some();
                    if ui
                        .add_enabled(has_parent, egui::Button::new("Parent"))
                        .clicked()
                    {
                        pane.parent(sorter);
                    }
                    if ui
                        .add_enabled(pane.location.is_some(), egui::Button::new("Refresh"))
                        .clicked()
                    {
                        pane.refresh(sorter);
                    }
                    ui.weak(format!("{} entries", pane.entries.len()));
                });
                let before = pane.options;
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("mode")
                        .selected_text(match pane.options.mode {
                            SortMode::Natural => "Natural",
                            SortMode::Lexicographic => "Lexicographic",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut pane.options.mode,
                                SortMode::Natural,
                                "Natural",
                            );
                            ui.selectable_value(
                                &mut pane.options.mode,
                                SortMode::Lexicographic,
                                "Lexicographic",
                            );
                        });
                    ui.checkbox(&mut pane.options.ascending, "Ascending");
                    ui.checkbox(&mut pane.options.directories_first, "Folders first");
                });
                if before != pane.options {
                    pane.resort(sorter);
                }
                if let Some(error) = &pane.error {
                    ui.colored_label(
                        egui::Color32::from_rgb(245, 155, 135),
                        format!("Cannot open folder: {error}"),
                    );
                    ui.small("The last successfully opened folder is retained.");
                }
                ui.separator();
                let mut navigate = None;
                egui::ScrollArea::vertical()
                    .id_salt("entries")
                    .max_height((height - 170.0).max(100.0))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if pane.entries.is_empty() {
                            ui.add_space(18.0);
                            ui.weak(if pane.location.is_some() {
                                "This folder is empty"
                            } else {
                                "No folder open"
                            });
                        }
                        for index in &pane.order {
                            let entry = &pane.entries[*index];
                            let prefix = if entry.is_symlink {
                                "[link]"
                            } else if entry.is_directory {
                                "[folder]"
                            } else {
                                "[file]"
                            };
                            let text = format!("{prefix}  {}", entry.name);
                            let label = egui::RichText::new(text).color(if entry.is_directory {
                                egui::Color32::from_rgb(150, 195, 245)
                            } else {
                                ui.visuals().text_color()
                            });
                            if ui.selectable_label(false, label).double_clicked()
                                && entry.is_directory
                                && !entry.is_symlink
                            {
                                navigate =
                                    pane.location.as_ref().map(|path| path.join(&entry.name));
                            }
                        }
                    });
                if let Some(path) = navigate {
                    pane.open(&path, sorter);
                }
            });
    });
}
