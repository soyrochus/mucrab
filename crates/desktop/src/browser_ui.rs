// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

use crate::{
    app::{click_modifiers, listing_id, selection_key},
    appearance::{FILE_ROW_HEIGHT, Icon, Palette, icon_button, paint_icon},
};
use eframe::egui::{self, Align2, Color32, FontId, Rect, Sense, Stroke, Vec2};
use mucrab_core::{BrowserPane, FileEntry, NameSorter, PaneEvent, PaneSide, SortCriterion};
use std::path::PathBuf;

pub fn pane(
    ui: &mut egui::Ui,
    side: PaneSide,
    pane: &mut BrowserPane,
    sorter: &NameSorter,
    active: &mut PaneSide,
    widths: &mut [f32; 3],
    enabled: bool,
) -> Option<crate::opening::Target> {
    let mut open = None;
    ui.push_id(listing_id(side), |ui| {
        let area = ui.max_rect();
        let palette = Palette::get(ui);
        let is_active = *active == side;
        let stroke = Stroke::new(
            1.0,
            if is_active {
                ui.visuals().selection.bg_fill
            } else {
                ui.visuals().widgets.noninteractive.bg_stroke.color
            },
        );
        egui::Frame::new()
            .fill(palette.content)
            .stroke(stroke)
            .corner_radius(8)
            .inner_margin(6.0)
            .show(ui, |ui| {
                ui.set_min_size(area.size() - Vec2::splat(12.0));
                ui.horizontal(|ui| {
                    if icon_button(
                        ui,
                        Icon::Folder,
                        if pane.archive().is_some() {
                            "ZIP archive (read-only)"
                        } else {
                            "Current folder"
                        },
                        enabled,
                    )
                    .clicked()
                    {
                        *active = side;
                        ui.memory_mut(|m| m.request_focus(ui.id().with("path")));
                    }
                    let edit = ui.add_enabled(
                        enabled,
                        egui::TextEdit::singleline(&mut pane.path_input)
                            .hint_text("Folder or ZIP path")
                            .id(ui.id().with("path"))
                            .desired_width((ui.available_width() - 76.0).max(60.0)),
                    );
                    if edit.has_focus() {
                        *active = side;
                    }
                    if edit.lost_focus() && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
                        pane.open(&PathBuf::from(&pane.path_input), sorter);
                        *active = side;
                        ui.memory_mut(|m| m.request_focus(listing_id(side)));
                    }
                    if icon_button(
                        ui,
                        Icon::Parent,
                        "Parent folder (Backspace)",
                        enabled && (pane.location().is_some()||pane.remote().is_some()),
                    )
                    .clicked()
                    {
                        pane.parent(sorter);
                        *active = side;
                    }
                    if icon_button(
                        ui,
                        Icon::Refresh,
                        "Refresh",
                        enabled && (pane.location().is_some()||pane.remote().is_some()),
                    )
                    .clicked()
                    {
                        pane.refresh(sorter);
                        *active = side;
                    }
                });
                if let Some(error) = &pane.error {
                    ui.colored_label(
                        ui.visuals().error_fg_color,
                        format!("Cannot open location: {error}"),
                    );
                    ui.small("The last successfully opened folder is retained.");
                }
                let find_id=ui.id().with("quick-find");
                let mut find_scroll=false;
                let listing_focused=ui.memory(|m|m.has_focus(listing_id(side)));
                let mut started=false;
                if enabled && listing_focused && pane.quick_find.is_none() {
                    let text=ui.input_mut(|input| {
                        let mut text=String::new();
                        if !input.modifiers.command && !input.modifiers.ctrl && !input.modifiers.alt {
                            input.events.retain(|event|match event {
                                egui::Event::Text(value) if value.chars().any(|c|!c.is_control() && c!=' ')=>{text.push_str(value);false},
                                egui::Event::Ime(egui::ImeEvent::Commit(value))=>{text.push_str(value);false},
                                _=>true
                            });
                        }
                        text
                    });
                    if !text.is_empty(){pane.quick_find=Some(text);started=true;}
                }
                if enabled && let Some(mut query)=pane.quick_find.clone() {
                    let before=query.clone();
                    let mut close=ui.memory(|m|m.has_focus(find_id)) && ui.input_mut(|i|i.consume_key(egui::Modifiers::NONE,egui::Key::Escape) || i.consume_key(egui::Modifiers::NONE,egui::Key::Enter));let mut direction=None;
                    ui.horizontal(|ui| {
                        ui.label("Find:");
                        let edit=ui.add(egui::TextEdit::singleline(&mut query).id(find_id).desired_width((ui.available_width()-125.0).max(30.0)));
                        if started || (!ui.memory(|m|m.has_focus(find_id)) && query.is_empty()) {edit.request_focus();}
                        if ui.small_button("Previous").clicked(){direction=Some(false);}
                        if ui.small_button("Next").clicked(){direction=Some(true);}
                        close |= ui.small_button("×").clicked();
                    });
                    if started || before!=query || direction.is_some(){
                        find_scroll=pane.find_prefix(&query,direction);
                    }
                    let matches=query.is_empty() || pane.order().iter().any(|i|pane.entries()[*i].name.to_lowercase().starts_with(&query.to_lowercase()));
                    if !matches {ui.colored_label(ui.visuals().error_fg_color,"No matching filename");}
                    if close{pane.quick_find=None;ui.memory_mut(|m|m.request_focus(listing_id(side)));}
                    else {pane.quick_find=Some(query);}
                }
                let keyboard_focus = ui.memory(|m| m.has_focus(listing_id(side)));
                let mut scroll_focus = find_scroll;
                let mut navigate = None;
                let mut go_parent = false;
                if keyboard_focus && enabled {
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
                    scroll_focus |= !events.is_empty();
                    for event in events {
                        let _ = pane.apply_event(&event, sorter);
                    }
                    if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Backspace))
                    {
                        go_parent = true;
                    }
                    if ui.input(|i| i.events.iter().any(|event| matches!(event, egui::Event::Key { key: egui::Key::Enter, pressed: true, repeat: false, modifiers, .. } if modifiers.is_none())))
                        && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
                        if pane.selection().has_parent && pane.selection().focus_row == Some(0) {
                            go_parent = true;
                        } else if let Some(row) = pane.selection().focus_row {
                            let position =
                                row.saturating_sub(usize::from(pane.selection().has_parent));
                            if let Some(index) = pane.order().get(position) {
                                let entry = &pane.entries()[*index];
                                if browsable(entry, pane) {
                                    navigate = Some(entry.name.clone());
                                } else {
                                    open = Some(crate::opening::target(pane, &entry.name));
                                }
                            }
                        }
                    }
                }
                let footer_height = 36.0;
                let table_height = (ui.available_height() - footer_height).max(60.0);
                let table_width = ui.available_width();
                let table_rect =
                    Rect::from_min_size(ui.cursor().min, Vec2::new(table_width, table_height));
                let focus = ui.interact(table_rect, listing_id(side), Sense::click());
                if enabled && focus.clicked() {
                    *active = side;
                    ui.memory_mut(|memory| memory.request_focus(listing_id(side)));
                }
                if focus.has_focus() {
                    ui.memory_mut(|m| {
                        m.set_focus_lock_filter(
                            focus.id,
                            egui::EventFilter {
                                tab: true,
                                vertical_arrows: true,
                                ..Default::default()
                            },
                        )
                    });
                }
                // Small panes retain all columns through horizontal scrolling.
                let content_width = table_width.max(440.0);
                let sum = widths.iter().sum::<f32>();
                if sum > content_width - 110.0 {
                    let scale = (content_width - 110.0) / sum;
                    for w in widths.iter_mut() {
                        *w *= scale;
                    }
                }
                egui::ScrollArea::horizontal()
                    .id_salt("table-horizontal")
                    .auto_shrink([false, false])
                    .max_height(table_height)
                    .show(ui, |ui| {
                        ui.set_min_width(content_width);
                        let (header, _) =
                            ui.allocate_exact_size(Vec2::new(content_width, 25.0), Sense::hover());
                        ui.painter()
                            .rect_filled(header, 0, palette.field);
                        let name_width = content_width - widths.iter().sum::<f32>();
                        let mut x = header.left();
                        for (column, (label, criterion, width)) in [
                            ("Name", SortCriterion::Name, name_width),
                            ("Ext", SortCriterion::Extension, widths[0]),
                            ("Size", SortCriterion::Size, widths[1]),
                            ("Modified (UTC)", SortCriterion::Date, widths[2]),
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            let rect = Rect::from_min_size(
                                egui::pos2(x, header.top()),
                                Vec2::new(width, header.height()),
                            );
                            let response = ui.interact(
                                rect.shrink2(Vec2::new(4.0, 0.0)),
                                ui.id().with(("sort", column)),
                                Sense::click(),
                            );
                            response.widget_info(|| {
                                egui::WidgetInfo::labeled(
                                    egui::WidgetType::Button,
                                    enabled,
                                    format!("Sort by {label}"),
                                )
                            });
                            let label = if pane.options.criterion == criterion {
                                format!(
                                    "{label} {}",
                                    if pane.options.ascending { "^" } else { "v" }
                                )
                            } else {
                                label.to_owned()
                            };
                            ui.painter().with_clip_rect(rect.shrink(3.0)).text(
                                rect.left_center() + Vec2::new(5.0, 0.0),
                                Align2::LEFT_CENTER,
                                label,
                                FontId::proportional(13.0),
                                ui.visuals().text_color(),
                            );
                            if enabled && response.clicked() {
                                if pane.options.criterion == criterion {
                                    pane.options.ascending = !pane.options.ascending;
                                } else {
                                    pane.options.criterion = criterion;
                                    pane.options.ascending = true;
                                }
                                pane.resort(sorter);
                                scroll_focus = true;
                                *active = side;
                                ui.memory_mut(|m| m.request_focus(listing_id(side)));
                            }
                            x += width;
                            ui.painter().line_segment(
                                [egui::pos2(x, header.top()), egui::pos2(x, header.bottom())],
                                ui.visuals().widgets.noninteractive.bg_stroke,
                            );
                            if column < 3 {
                                let handle = Rect::from_center_size(
                                    egui::pos2(x, header.center().y),
                                    Vec2::new(7.0, 25.0),
                                );
                                let drag = ui
                                    .interact(
                                        handle,
                                        ui.id().with(("resize", column)),
                                        Sense::drag(),
                                    )
                                    .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                                if enabled && drag.dragged() {
                                    let delta = ui.input(|i| i.pointer.delta().x);
                                    if column == 0 {
                                        let delta =
                                            delta.clamp(110.0 - name_width, widths[0] - 45.0);
                                        widths[0] -= delta;
                                    } else {
                                        let delta = delta.clamp(
                                            45.0 - widths[column - 1],
                                            widths[column] - 45.0,
                                        );
                                        widths[column - 1] += delta;
                                        widths[column] -= delta;
                                    }
                                }
                            }
                        }
                        let row_height = FILE_ROW_HEIGHT;
                        let count = pane.order().len() + usize::from(pane.selection().has_parent);
                        let listing_height = (table_height - 42.0).max(35.0);
                        let bg = ui.visuals().extreme_bg_color;
                        let listing_rect = Rect::from_min_size(
                            ui.cursor().min,
                            Vec2::new(content_width, listing_height),
                        );
                        ui.painter().rect_filled(listing_rect, 0, bg);
                        let mut clicked = None;
                        let mut scroll = egui::ScrollArea::vertical()
                            .id_salt("entries")
                            .auto_shrink([false, false])
                            .max_height(listing_height);
                        if scroll_focus && let Some(row) = pane.selection().focus_row {
                            scroll = scroll.vertical_scroll_offset(
                                (row as f32 * row_height - listing_height / 2.0).max(0.0),
                            );
                        }
                        // show_rows includes item spacing in its row stride.
                        ui.spacing_mut().item_spacing.y = 0.0;
                        scroll.show_rows(ui, row_height, count, |ui, range| {
                            for row in range {
                                let parent = pane.selection().has_parent && row == 0;
                                let entry = if parent {
                                    None
                                } else {
                                    Some(
                                        &pane.entries()[pane.order()
                                            [row - usize::from(pane.selection().has_parent)]],
                                    )
                                };
                                let marked = !parent && pane.selection().is_marked(row, pane.order());
                                let focused = pane.selection().focus_row == Some(row);
                                let (rect, response) = ui.allocate_exact_size(
                                    Vec2::new(content_width, row_height),
                                    Sense::click(),
                                );
                                let painter = ui.painter().with_clip_rect(rect);
                                if focused {
                                    painter.rect_filled(
                                        rect,
                                        0,
                                        if is_active {
                                            ui.visuals().selection.bg_fill
                                        } else {
                                            ui.visuals().widgets.inactive.bg_fill
                                        },
                                    );
                                } else if response.hovered() {
                                    painter.rect_filled(
                                        rect,
                                        0,
                                        ui.visuals().widgets.hovered.bg_fill,
                                    );
                                }
                                let ink = if focused && is_active {
                                    Color32::WHITE
                                } else if marked {
                                    palette.warning
                                } else {
                                    ui.visuals().text_color()
                                };
                                let icon = entry.map_or(Icon::Parent, |e| {
                                    if e.is_symlink {
                                        Icon::Link
                                    } else if e.is_directory {
                                        Icon::Folder
                                    } else if e.name.to_ascii_lowercase().ends_with(".zip") {
                                        Icon::Archive
                                    } else {
                                        Icon::File
                                    }
                                });
                                paint_icon(
                                    &painter,
                                    egui::pos2(rect.left() + 12.0, rect.center().y),
                                    icon,
                                    ink,
                                );
                                if marked {
                                    painter.circle_filled(
                                        egui::pos2(rect.left() + 23.0, rect.center().y),
                                        2.0,
                                        ink,
                                    );
                                }
                                let name = entry.map_or("..", |e| e.name.as_str());
                                let name_rect = Rect::from_min_max(
                                    rect.min + Vec2::new(29.0, 0.0),
                                    egui::pos2(rect.left() + name_width - 4.0, rect.bottom()),
                                );
                                painter.with_clip_rect(name_rect).text(
                                    name_rect.left_center(),
                                    Align2::LEFT_CENTER,
                                    name,
                                    FontId::proportional(13.0),
                                    ink,
                                );
                                if let Some(e) = entry {
                                    let metadata = e.metadata.as_ref();
                                    let ext =
                                        metadata.and_then(|m| m.extension.as_deref()).unwrap_or("");
                                    let size = if e.is_directory {
                                        "<DIR>".to_owned()
                                    } else {
                                        metadata.map_or_else(
                                            || "—".to_owned(),
                                            |m| {
                                                if m.size < 0 {
                                                    "—".to_owned()
                                                } else {
                                                    format!("{} B", m.size)
                                                }
                                            },
                                        )
                                    };
                                    let date = metadata
                                        .and_then(|m| {
                                            chrono::DateTime::from_timestamp_millis(m.modified_ms)
                                        })
                                        .map_or_else(
                                            || "—".to_owned(),
                                            |d| d.format("%Y-%m-%d %H:%M").to_string(),
                                        );
                                    let mut x = rect.left() + name_width;
                                    for (index, text) in [ext, &size, &date].into_iter().enumerate()
                                    {
                                        let cell = Rect::from_min_size(
                                            egui::pos2(x, rect.top()),
                                            Vec2::new(widths[index], row_height),
                                        );
                                        let right = index == 1;
                                        painter
                                            .with_clip_rect(cell.shrink2(Vec2::new(4.0, 0.0)))
                                            .text(
                                                if right {
                                                    cell.right_center() - Vec2::new(5.0, 0.0)
                                                } else {
                                                    cell.left_center() + Vec2::new(5.0, 0.0)
                                                },
                                                if right {
                                                    Align2::RIGHT_CENTER
                                                } else {
                                                    Align2::LEFT_CENTER
                                                },
                                                text,
                                                FontId::proportional(13.0),
                                                if focused && is_active { Color32::WHITE } else if marked { ink } else { palette.secondary },
                                            );
                                        x += widths[index];
                                    }
                                }
                                if focused && keyboard_focus {
                                    painter.rect_stroke(
                                        rect.shrink(1.0),
                                        0,
                                        Stroke::new(1.0, ui.visuals().selection.stroke.color),
                                        egui::StrokeKind::Inside,
                                    );
                                }
                                response.widget_info(|| {
                                    egui::WidgetInfo::selected(
                                        egui::WidgetType::SelectableLabel,
                                        enabled,
                                        focused,
                                        name,
                                    )
                                });
                                let modifiers = click_modifiers(ui, response.rect);
                                if enabled
                                    && (response.clicked()
                                        || (response.clicked_by(egui::PointerButton::Secondary)
                                            && modifiers.ctrl))
                                {
                                    clicked = Some((row, modifiers));
                                }
                                if enabled && response.double_clicked() {
                                    if parent {
                                        go_parent = true;
                                    } else if let Some(e) = entry {
                                        if browsable(e, pane) {
                                            navigate = Some(e.name.clone());
                                        } else {
                                            open = Some(crate::opening::target(pane, &e.name));
                                        }
                                    }
                                }
                                response.on_hover_text(entry.map_or_else(
                                    || "Parent folder".to_owned(),
                                    |e| e.name.clone(),
                                ));
                            }
                        });
                        if count == 0 {
                            ui.painter().text(
                                listing_rect.center(),
                                Align2::CENTER_CENTER,
                                if pane.location().is_some() {
                                    "This folder is empty"
                                } else {
                                    "No folder open"
                                },
                                FontId::proportional(13.0),
                                ui.visuals().weak_text_color(),
                            );
                        }
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
                            ui.memory_mut(|m| m.request_focus(listing_id(side)));
                        }
                    });
                if go_parent {
                    pane.parent(sorter);
                }
                if let Some(name) = navigate {
                    if let Some(remote)=pane.remote() {if let Ok(location)=remote.child(&name){pane.request_remote(location,true,None);}}else if let Some(path)=pane.location().map(|p|p.join(name)){pane.open(&path,sorter);}
                }
                let selected = pane.selection().snapshot(pane.entries(), pane.order());
                ui.separator();
                let selected_names: std::collections::HashSet<&str> =
                    selected.selected.iter().map(String::as_str).collect();
                let selected_bytes: u128 = pane
                    .entries()
                    .iter()
                    .filter(|entry| {
                        !entry.is_directory && selected_names.contains(entry.name.as_str())
                    })
                    .filter_map(|entry| entry.metadata.as_ref())
                    .filter(|metadata| metadata.size >= 0)
                    .map(|metadata| metadata.size as u128)
                    .sum();
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "{} of {} selected · {} B",
                        selected.selected.len(),
                        pane.entries().len(),
                        selected_bytes
                    ));
                    if pane.remote().is_some(){ui.label(if pane.remote_disconnected{"SFTP · disconnected"}else{"SFTP"});}
                    if pane.archive().is_some() {
                        ui.label("ZIP · read-only");
                    }
                    if is_active {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label("Active pane");
                        });
                    }
                });
            });
    });
    open
}

pub(crate) fn browsable(entry: &FileEntry, pane: &BrowserPane) -> bool {
    if pane.remote().is_some() {
        return entry.is_directory && !entry.is_symlink;
    }
    if pane.archive().is_some() {
        return entry.is_directory;
    }
    let resolved = pane.location().map(|p| p.join(&entry.name));
    entry.is_directory
        || resolved.as_ref().is_some_and(|path| path.is_dir())
        || resolved
            .as_ref()
            .and_then(|path| std::fs::canonicalize(path).ok())
            .is_some_and(|path| {
                path.extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
            })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mucrab_core::FileMetadata;

    fn frame(
        ctx: &egui::Context,
        pane_state: &mut BrowserPane,
        sorter: &NameSorter,
        events: Vec<egui::Event>,
    ) -> (usize, Vec<String>) {
        let mut active = PaneSide::Left;
        let mut widths = [65.0, 84.0, 145.0];
        let output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(800.0, 600.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                pane(
                    ui,
                    PaneSide::Left,
                    pane_state,
                    sorter,
                    &mut active,
                    &mut widths,
                    true,
                );
            },
        );
        let count = output.shapes.len();
        let texts = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect();
        output.drop_without_applying_deltas();
        (count, texts)
    }

    fn key(key: egui::Key) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }

    fn click(ctx: &egui::Context, pane: &mut BrowserPane, sorter: &NameSorter, pos: egui::Pos2) {
        for pressed in [true, false] {
            frame(
                ctx,
                pane,
                sorter,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
    }

    #[test]
    fn path_enter_queues_only_the_edited_location() {
        let ctx = egui::Context::default();
        crate::appearance::Appearance::Light.apply(&ctx);
        let sorter = NameSorter::en_us().unwrap();
        let root = std::env::temp_dir().join(format!("mucrab-path-enter-{}", std::process::id()));
        std::fs::create_dir_all(root.join("child")).unwrap();
        let mut state = BrowserPane::default();
        assert!(state.open(&root, &sorter));
        state.defer_browsing();
        state.path_input = root.join("child").display().to_string();
        frame(&ctx, &mut state, &sorter, vec![]);
        click(&ctx, &mut state, &sorter, egui::pos2(150.0, 30.0));
        frame(&ctx, &mut state, &sorter, vec![key(egui::Key::Enter)]);
        assert_eq!(
            state.take_browse_request().unwrap().path(),
            root.join("child")
        );
        assert_eq!(
            state.location(),
            Some(&root),
            "queued navigation retains the committed listing"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn virtualized_table_keeps_keyboard_focus_marks_and_header_sorting() {
        let ctx = egui::Context::default();
        crate::appearance::Appearance::Light.apply(&ctx);
        let sorter = NameSorter::en_us().unwrap();
        let mut state = BrowserPane::default();
        let entries = (0..10_000)
            .map(|i| FileEntry {
                name: format!("file{i}.txt"),
                metadata: Some(FileMetadata {
                    size: 10_000 - i,
                    modified_ms: 0,
                    extension: Some("txt".to_owned()),
                }),
                ..Default::default()
            })
            .collect();
        state.load_listing(
            PathBuf::from("/synthetic-ui-fixture"),
            entries,
            true,
            &sorter,
        );
        assert!(
            frame(&ctx, &mut state, &sorter, vec![]).0 < 500,
            "offscreen rows must not all be painted"
        );
        // Click the metadata end of a row: the whole row must receive the click.
        click(&ctx, &mut state, &sorter, egui::pos2(765.0, 105.0));
        assert_eq!(state.selection().focus_row, Some(1));
        frame(&ctx, &mut state, &sorter, vec![key(egui::Key::End)]);
        let (_, visible) = frame(&ctx, &mut state, &sorter, vec![]);
        assert!(
            visible.contains(&"file9999.txt".to_owned()),
            "keyboard focus must scroll the last row into view"
        );
        assert_eq!(state.selection().focus_row, Some(10_000));
        frame(&ctx, &mut state, &sorter, vec![key(egui::Key::Space)]);
        let before = state.selection().snapshot(state.entries(), state.order());
        assert_eq!(before.selected, vec!["file9999.txt"]);
        // Size header changes the order while preserving selected identity.
        click(&ctx, &mut state, &sorter, egui::pos2(610.0, 60.0));
        assert_eq!(state.options.criterion, SortCriterion::Size);
        assert_eq!(
            state
                .selection()
                .snapshot(state.entries(), state.order())
                .selected,
            before.selected
        );
        assert_eq!(state.selection().focus_row, Some(1));
        frame(&ctx, &mut state, &sorter, vec![key(egui::Key::Home)]);
        assert_eq!(state.selection().focus_row, Some(0));
    }
    #[test]
    fn quick_find_enter_returns_to_listing_before_activation() {
        let ctx = egui::Context::default();
        let sorter = NameSorter::en_us().unwrap();
        let mut state = BrowserPane::default();
        state.load_listing(
            PathBuf::from("/synthetic-ui-fixture"),
            vec![FileEntry {
                name: "Alpha.txt".into(),
                ..Default::default()
            }],
            true,
            &sorter,
        );
        frame(&ctx, &mut state, &sorter, vec![]);
        click(&ctx, &mut state, &sorter, egui::pos2(100.0, 90.0));
        frame(
            &ctx,
            &mut state,
            &sorter,
            vec![egui::Event::Text("al".into())],
        );
        assert_eq!(state.quick_find.as_deref(), Some("al"));
        frame(&ctx, &mut state, &sorter, vec![key(egui::Key::Enter)]);
        assert!(state.quick_find.is_none());
        assert!(ctx.memory(|m| m.has_focus(listing_id(PaneSide::Left))));
        assert_eq!(
            state.location().map(|p| p.as_path()),
            Some(std::path::Path::new("/synthetic-ui-fixture"))
        );
        let mut release = key(egui::Key::Enter);
        if let egui::Event::Key { pressed, .. } = &mut release {
            *pressed = false;
        }
        frame(&ctx, &mut state, &sorter, vec![release]);
        let mut requested = None;
        let mut active = PaneSide::Left;
        let mut widths = [65.0, 84.0, 145.0];
        let output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(800.0, 600.0),
                )),
                events: vec![key(egui::Key::Enter)],
                ..Default::default()
            },
            |ui| {
                if let Some(target) = pane(
                    ui,
                    PaneSide::Left,
                    &mut state,
                    &sorter,
                    &mut active,
                    &mut widths,
                    true,
                ) {
                    requested = Some(target);
                }
            },
        );
        output.drop_without_applying_deltas();
        assert!(
            matches!(requested,Some(crate::opening::Target::Local(path)) if path.ends_with("Alpha.txt"))
        );
    }
}
