// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

use eframe::egui;
use mucrab_core::{
    BrowserPane, FileEntry, NameSorter, PaneEvent, PaneSide, SortCriterion, SortMode,
};
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
            Ok(Box::new(MuCrabApp {
                sorter,
                panes,
                active: PaneSide::Left,
                show_hidden: false,
            }))
        }),
    )
}

struct MuCrabApp {
    sorter: NameSorter,
    panes: [BrowserPane; 2],
    active: PaneSide,
    show_hidden: bool,
}

impl eframe::App for MuCrabApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
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
        egui::CentralPanel::default().show(ui, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("muCrab").size(30.0));
                ui.add_space(12.0);
                ui.label("Two claws, two panes.");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.colored_label(egui::Color32::from_rgb(115, 211, 165), "Two-pane file browser");
                });
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label("File sorting · en-US · Times UTC");
                ui.add_space(12.0);
                if ui.checkbox(&mut self.show_hidden, "Show hidden files").changed() {
                    for pane in &mut self.panes {
                        pane.set_show_hidden(self.show_hidden, &self.sorter);
                    }
                    ui.memory_mut(|memory| memory.request_focus(listing_id(self.active)));
                }
            });
            ui.small("Click to focus · Ctrl-click to mark · Shift-click for a range · Space to mark and advance · Tab to switch panes");
            ui.add_space(14.0);
            let height = (ui.available_height() - 40.0).max(180.0);
            ui.columns(2, |columns| {
                pane(&mut columns[0], PaneSide::Left, &mut self.panes[0], &self.sorter, height, &mut self.active);
                pane(&mut columns[1], PaneSide::Right, &mut self.panes[1], &self.sorter, height, &mut self.active);
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
    active: &mut PaneSide,
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
                let keyboard_focus = ui.memory(|memory| memory.has_focus(listing_id(side)));
                ui.strong(if keyboard_focus {
                    format!("{title} · active")
                } else {
                    title.to_owned()
                });
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
                    egui::ComboBox::from_id_salt("criterion")
                        .width(90.0)
                        .selected_text(match pane.options.criterion {
                            SortCriterion::Name => "Name",
                            SortCriterion::Size => "Size",
                            SortCriterion::Date => "Modified",
                            SortCriterion::Extension => "Extension",
                        })
                        .show_ui(ui, |ui| {
                            for (value, label) in [
                                (SortCriterion::Name, "Name"),
                                (SortCriterion::Size, "Size"),
                                (SortCriterion::Date, "Modified"),
                                (SortCriterion::Extension, "Extension"),
                            ] {
                                ui.selectable_value(&mut pane.options.criterion, value, label);
                            }
                        });
                    egui::ComboBox::from_id_salt("mode")
                        .width(100.0)
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
                ui.horizontal(|ui| {
                    for (label, event) in [
                        ("All", PaneEvent::All),
                        ("None", PaneEvent::None),
                        ("Invert", PaneEvent::Invert),
                    ] {
                        if ui.button(label).clicked() {
                            let _ = pane.apply_event(&event, sorter);
                            *active = side;
                            ui.memory_mut(|memory| memory.request_focus(listing_id(side)));
                        }
                    }
                    ui.weak(format!("{} marked", pane.selection.marked_count()));
                });
                let mut scroll_focus = false;
                if keyboard_focus {
                    let events = ui.input_mut(|input| {
                        let mut commands = Vec::new();
                        input.events.retain(|event| {
                            if let Some(command) = selection_key(event) {
                                commands.push(command);
                                false
                            } else {
                                true
                            }
                        });
                        commands
                    });
                    scroll_focus = !events.is_empty();
                    for event in events {
                        let _ = pane.apply_event(&event, sorter);
                    }
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
                let mut clicked = None;
                let mut go_parent = false;
                let list = ui.interact(
                    ui.available_rect_before_wrap(),
                    listing_id(side),
                    egui::Sense::focusable_noninteractive(),
                );
                if list.has_focus() {
                    ui.memory_mut(|memory| {
                        memory.set_focus_lock_filter(
                            list.id,
                            egui::EventFilter {
                                tab: true,
                                vertical_arrows: true,
                                ..Default::default()
                            },
                        )
                    });
                }
                egui::ScrollArea::vertical()
                    .id_salt("entries")
                    .max_height((height - 220.0).max(100.0))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if pane.selection.has_parent {
                            let response = ui.selectable_label(
                                pane.selection.focus_row == Some(0),
                                "[parent]  ..",
                            );
                            let modifiers = click_modifiers(ui, response.rect);
                            if response.clicked()
                                || (response.clicked_by(egui::PointerButton::Secondary)
                                    && modifiers.ctrl)
                            {
                                clicked = Some((0, modifiers));
                            }
                            if response.double_clicked() {
                                go_parent = true;
                            }
                            if scroll_focus && pane.selection.focus_row == Some(0) {
                                response.scroll_to_me(Some(egui::Align::Center));
                            }
                        }
                        if pane.entries.is_empty() {
                            ui.add_space(18.0);
                            ui.weak(if pane.location.is_some() {
                                "This folder is empty"
                            } else {
                                "No folder open"
                            });
                        }
                        for (position, index) in pane.order.iter().enumerate() {
                            let row = position + usize::from(pane.selection.has_parent);
                            let entry = &pane.entries[*index];
                            let prefix = if entry.is_symlink {
                                "[link]"
                            } else if entry.is_directory {
                                "[folder]"
                            } else {
                                "[file]"
                            };
                            let marked = pane.selection.is_marked(row, &pane.order);
                            let text = format!(
                                "{} {prefix}  {}{}",
                                if marked { "[x]" } else { "[ ]" },
                                entry.name,
                                metadata_label(entry, pane.options.criterion)
                            );
                            let label = egui::RichText::new(text).color(if marked {
                                egui::Color32::from_rgb(245, 210, 110)
                            } else if entry.is_directory {
                                egui::Color32::from_rgb(150, 195, 245)
                            } else {
                                ui.visuals().text_color()
                            });
                            let response =
                                ui.selectable_label(pane.selection.focus_row == Some(row), label);
                            let modifiers = click_modifiers(ui, response.rect);
                            if response.clicked()
                                || (response.clicked_by(egui::PointerButton::Secondary)
                                    && modifiers.ctrl)
                            {
                                clicked = Some((row, modifiers));
                            }
                            if response.double_clicked() && entry.is_directory && !entry.is_symlink
                            {
                                navigate =
                                    pane.location.as_ref().map(|path| path.join(&entry.name));
                            }
                            if scroll_focus && pane.selection.focus_row == Some(row) {
                                response.scroll_to_me(Some(egui::Align::Center));
                            }
                        }
                    });
                if let Some((row, modifiers)) = clicked {
                    let _ = pane.apply_event(
                        &PaneEvent::Click {
                            row,
                            ctrl: modifiers.ctrl,
                            shift: modifiers.shift,
                        },
                        sorter,
                    );
                    *active = side;
                    ui.memory_mut(|memory| memory.request_focus(listing_id(side)));
                }
                if go_parent {
                    pane.parent(sorter);
                }
                if let Some(path) = navigate {
                    pane.open(&path, sorter);
                }
                let selected = pane.selection.snapshot(&pane.entries, &pane.order);
                ui.small(format!(
                    "Focus: {} · {} selected",
                    selected.focused.as_deref().unwrap_or("none"),
                    selected.selected.len()
                ));
            });
    });
}

fn listing_id(side: PaneSide) -> egui::Id {
    egui::Id::new(match side {
        PaneSide::Left => "left-file-list",
        PaneSide::Right => "right-file-list",
    })
}

fn metadata_label(entry: &FileEntry, criterion: SortCriterion) -> String {
    if criterion == SortCriterion::Name {
        return String::new();
    }
    let Some(metadata) = &entry.metadata else {
        return " · Metadata unavailable".to_owned();
    };
    let value = match criterion {
        SortCriterion::Name => return String::new(),
        SortCriterion::Size if entry.is_directory => "<DIR>".to_owned(),
        SortCriterion::Size if metadata.size == -1 => "Unknown size".to_owned(),
        SortCriterion::Size => format!("{} bytes", metadata.size),
        SortCriterion::Date => chrono::DateTime::from_timestamp_millis(metadata.modified_ms)
            .map_or_else(
                || "Date out of range".to_owned(),
                |date| date.format("%Y-%m-%d %H:%M:%S%.3f UTC").to_string(),
            ),
        SortCriterion::Extension => metadata
            .extension
            .clone()
            .unwrap_or_else(|| "No extension".to_owned()),
    };
    format!(" · {value}")
}

fn click_modifiers(ui: &egui::Ui, rect: egui::Rect) -> egui::Modifiers {
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

fn selection_key(event: &egui::Event) -> Option<PaneEvent> {
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
