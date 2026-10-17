// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use std::path::PathBuf;

#[derive(Clone, Copy, Default, PartialEq)]
pub enum Appearance {
    Light,
    Dark,
    #[default]
    System,
}

impl Appearance {
    pub fn load() -> Self {
        match settings_path()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .as_deref()
            .map(str::trim)
        {
            Some("light") => Self::Light,
            Some("dark") => Self::Dark,
            _ => Self::System,
        }
    }

    pub fn save(self) -> std::io::Result<()> {
        let path = settings_path()
            .ok_or_else(|| std::io::Error::other("No user configuration directory available"))?;
        std::fs::create_dir_all(path.parent().expect("settings parent"))?;
        std::fs::write(
            path,
            match self {
                Self::Light => "light\n",
                Self::Dark => "dark\n",
                Self::System => "system\n",
            },
        )
    }

    pub fn apply(self, ctx: &egui::Context) {
        // Keep both palettes styled: System can change without restarting.
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            let mut style = (*ctx.style_of(theme)).clone();
            style.spacing.item_spacing = Vec2::new(4.0, 2.0);
            style.spacing.button_padding = Vec2::new(7.0, 3.0);
            style.spacing.interact_size.y = 24.0;
            style.visuals = theme.default_visuals();
            style.visuals.override_text_color = Some(if theme == egui::Theme::Dark {
                Color32::from_gray(215)
            } else {
                Color32::from_gray(35)
            });
            style.spacing.window_margin = egui::Margin::same(12);
            style.visuals.window_corner_radius = 3.into();
            style.visuals.menu_corner_radius = 2.into();
            for widget in [
                &mut style.visuals.widgets.noninteractive,
                &mut style.visuals.widgets.inactive,
                &mut style.visuals.widgets.hovered,
                &mut style.visuals.widgets.active,
                &mut style.visuals.widgets.open,
            ] {
                widget.corner_radius = 2.into();
            }
            style.visuals.selection.bg_fill = if theme == egui::Theme::Dark {
                Color32::from_rgb(43, 83, 120)
            } else {
                Color32::from_rgb(49, 102, 166)
            };
            style.visuals.selection.stroke.color = Color32::WHITE;
            ctx.set_style_of(theme, style);
        }
        ctx.options_mut(|options| options.fallback_theme = egui::Theme::Light);
        ctx.set_theme(match self {
            Self::Light => egui::ThemePreference::Light,
            Self::Dark => egui::ThemePreference::Dark,
            Self::System => egui::ThemePreference::System,
        });
    }
}

fn settings_path() -> Option<PathBuf> {
    // XDG also makes isolated GUI checks independent of the user's preferences.
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| {
                let home = PathBuf::from(home);
                if cfg!(target_os = "macos") {
                    home.join("Library/Application Support")
                } else {
                    home.join(".config")
                }
            })
        })?;
    Some(base.join("mucrab/appearance"))
}

#[derive(Clone, Copy)]
pub enum Icon {
    Folder,
    File,
    Archive,
    Parent,
    Refresh,
    Copy,
    Move,
    Rename,
    Link,
    Home,
}

pub fn icon_button(ui: &mut egui::Ui, icon: Icon, tooltip: &str, enabled: bool) -> egui::Response {
    let response = ui.add_enabled(enabled, egui::Button::new(" ").min_size(Vec2::splat(27.0)));
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, tooltip));
    let mut painter = ui.painter().clone();
    painter.multiply_opacity(if enabled { 1.0 } else { 0.35 });
    paint_icon(
        &painter,
        response.rect.center(),
        icon,
        ui.visuals().text_color(),
    );
    response.on_hover_text(tooltip)
}

pub fn paint_icon(p: &egui::Painter, center: Pos2, icon: Icon, ink: Color32) {
    let origin = center - Vec2::splat(8.0);
    let at = |x, y| origin + Vec2::new(x, y);
    let line = Stroke::new(1.2, ink);
    let box_at = |x, y, w, h| Rect::from_min_size(at(x, y), Vec2::new(w, h));
    match icon {
        Icon::Folder | Icon::Home => {
            let gold = Color32::from_rgb(224, 172, 61);
            p.rect_filled(box_at(1.0, 3.0, 7.0, 4.0), 1, gold);
            p.rect_filled(box_at(1.0, 6.0, 14.0, 8.0), 1, gold);
            p.line_segment(
                [at(2.0, 7.0), at(14.0, 7.0)],
                Stroke::new(1.0, Color32::from_rgb(255, 224, 145)),
            );
            if matches!(icon, Icon::Home) {
                p.line_segment([at(5.0, 12.0), at(8.0, 9.0)], line);
                p.line_segment([at(8.0, 9.0), at(11.0, 12.0)], line);
            }
        }
        Icon::File | Icon::Link | Icon::Archive => {
            p.rect_stroke(
                box_at(3.0, 1.0, 10.0, 14.0),
                1,
                line,
                egui::StrokeKind::Inside,
            );
            if matches!(icon, Icon::Archive) {
                for y in [3.0, 6.0, 9.0, 12.0] {
                    p.rect_filled(box_at(7.0, y, 3.0, 2.0), 0, Color32::from_rgb(201, 151, 54));
                }
            } else if matches!(icon, Icon::Link) {
                p.arrow(at(1.0, 14.0), Vec2::new(7.0, -5.0), line);
            } else {
                for y in [6.0, 9.0, 12.0] {
                    p.line_segment([at(5.0, y), at(11.0, y)], Stroke::new(0.7, ink));
                }
            }
        }
        Icon::Parent => {
            p.arrow(at(8.0, 14.0), Vec2::new(0.0, -12.0), line);
        }
        Icon::Refresh => {
            let points: Vec<Pos2> = (0..=16)
                .map(|step| {
                    let angle = (45.0 + step as f32 * 17.0).to_radians();
                    center + Vec2::new(angle.cos(), angle.sin()) * 5.5
                })
                .collect();
            p.add(egui::Shape::line(points, line));
            p.arrow(at(10.0, 2.0), Vec2::new(4.0, 3.0), line);
        }
        Icon::Copy => {
            p.rect_stroke(
                box_at(2.0, 1.0, 9.0, 11.0),
                1,
                line,
                egui::StrokeKind::Inside,
            );
            p.rect_filled(
                box_at(6.0, 5.0, 9.0, 11.0),
                1,
                p.ctx().style_of(p.ctx().theme()).visuals.panel_fill,
            );
            p.rect_stroke(
                box_at(6.0, 5.0, 9.0, 11.0),
                1,
                line,
                egui::StrokeKind::Inside,
            );
        }
        Icon::Move => {
            p.arrow(at(1.0, 8.0), Vec2::new(14.0, 0.0), line);
        }
        Icon::Rename => {
            p.line_segment(
                [at(3.0, 13.0), at(13.0, 3.0)],
                Stroke::new(3.0, Color32::from_rgb(211, 163, 58)),
            );
            p.line_segment([at(3.0, 15.0), at(14.0, 15.0)], line);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_tracks_live_changes_and_explicit_modes_override_them() {
        let ctx = egui::Context::default();
        Appearance::System.apply(&ctx);
        for (preference, reported, expected) in [
            (Appearance::System, None, egui::Theme::Light),
            (
                Appearance::System,
                Some(egui::Theme::Dark),
                egui::Theme::Dark,
            ),
            (
                Appearance::System,
                Some(egui::Theme::Light),
                egui::Theme::Light,
            ),
            (
                Appearance::Dark,
                Some(egui::Theme::Light),
                egui::Theme::Dark,
            ),
            (
                Appearance::Light,
                Some(egui::Theme::Dark),
                egui::Theme::Light,
            ),
            (
                Appearance::System,
                Some(egui::Theme::Dark),
                egui::Theme::Dark,
            ),
        ] {
            preference.apply(&ctx);
            ctx.run_ui(
                egui::RawInput {
                    system_theme: reported,
                    ..Default::default()
                },
                |ui| {
                    assert_eq!(ui.ctx().theme(), expected);
                    // Verify the palette retains the classic metrics after an OS change.
                    assert_eq!(ui.spacing().interact_size.y, 24.0);
                },
            )
            .drop_without_applying_deltas();
        }
    }
}
