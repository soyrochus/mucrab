// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use std::path::PathBuf;

/// Toolbar target size in logical points; file rows deliberately stay compact.
pub(crate) const ACTION_SIZE: f32 = 32.0;
/// Compact listing row height, independent of navigation control padding.
pub(crate) const FILE_ROW_HEIGHT: f32 = 23.0;
const ICON_GRID: f32 = 16.0;

/// Semantic colours shared by egui widgets and custom-painted file tables.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Palette {
    pub chrome: Color32,
    pub content: Color32,
    pub raised: Color32,
    pub field: Color32,
    pub text: Color32,
    pub secondary: Color32,
    pub border: Color32,
    pub accent: Color32,
    pub selection: Color32,
    pub hover: Color32,
    pub pressed: Color32,
    pub warning: Color32,
    pub error: Color32,
}
impl Palette {
    pub fn get(ui: &egui::Ui) -> Self {
        Self::for_theme(if ui.visuals().dark_mode {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        })
    }
    pub fn for_theme(theme: egui::Theme) -> Self {
        let c = Color32::from_rgb;
        if theme == egui::Theme::Dark {
            Self {
                chrome: c(28, 32, 38),
                content: c(34, 40, 49),
                raised: c(43, 51, 62),
                field: c(24, 29, 36),
                text: c(237, 241, 247),
                secondary: c(190, 202, 217),
                border: c(67, 78, 93),
                accent: c(128, 184, 255),
                selection: c(40, 87, 135),
                hover: c(53, 65, 81),
                pressed: c(65, 82, 104),
                warning: c(255, 211, 131),
                error: c(255, 166, 157),
            }
        } else {
            Self {
                chrome: c(244, 246, 248),
                content: Color32::WHITE,
                raised: Color32::WHITE,
                field: c(232, 236, 241),
                text: c(32, 41, 54),
                secondary: c(82, 97, 116),
                border: c(211, 218, 227),
                accent: c(36, 95, 168),
                selection: c(36, 95, 168),
                hover: c(229, 237, 247),
                pressed: c(208, 223, 242),
                warning: c(139, 72, 0),
                error: c(179, 38, 30),
            }
        }
    }
}

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
            let t = Palette::for_theme(theme);
            style.spacing.item_spacing = Vec2::new(6.0, 4.0);
            style.spacing.button_padding = Vec2::new(9.0, 5.0);
            style.spacing.interact_size.y = 28.0;
            style.spacing.window_margin = egui::Margin::same(16);
            style.visuals = theme.default_visuals();
            style.visuals.panel_fill = t.chrome;
            style.visuals.window_fill = t.raised;
            style.visuals.extreme_bg_color = t.content;
            style.visuals.text_edit_bg_color = Some(t.field);
            style.visuals.override_text_color = Some(t.text);
            style.visuals.weak_text_color = Some(t.secondary);
            style.visuals.hyperlink_color = t.accent;
            style.visuals.error_fg_color = t.error;
            style.visuals.warn_fg_color = t.warning;
            style.visuals.faint_bg_color = t.hover;
            style.visuals.window_stroke = Stroke::new(1.0, t.border);
            style.visuals.window_corner_radius = 8.into();
            style.visuals.menu_corner_radius = 8.into();
            style.visuals.window_shadow = egui::epaint::Shadow {
                offset: [0, 4],
                blur: 18,
                spread: 0,
                color: Color32::from_black_alpha(if theme == egui::Theme::Dark { 80 } else { 24 }),
            };
            style.visuals.popup_shadow = style.visuals.window_shadow;
            for widget in [
                &mut style.visuals.widgets.noninteractive,
                &mut style.visuals.widgets.inactive,
                &mut style.visuals.widgets.hovered,
                &mut style.visuals.widgets.active,
                &mut style.visuals.widgets.open,
            ] {
                widget.corner_radius = 6.into();
                widget.fg_stroke = Stroke::new(1.2, t.text);
                widget.bg_stroke = Stroke::new(1.0, t.border);
            }
            style.visuals.widgets.inactive.weak_bg_fill = t.raised;
            style.visuals.widgets.inactive.bg_fill = t.field;
            style.visuals.widgets.hovered.weak_bg_fill = t.hover;
            style.visuals.widgets.hovered.bg_fill = t.hover;
            style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, t.accent);
            style.visuals.widgets.active.weak_bg_fill = t.pressed;
            style.visuals.widgets.active.bg_fill = t.pressed;
            style.visuals.widgets.active.bg_stroke = Stroke::new(1.5, t.accent);
            style.visuals.widgets.open.weak_bg_fill = t.hover;
            style.visuals.widgets.open.bg_fill = t.hover;
            style.visuals.selection.bg_fill = t.selection;
            style.visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);
            style
                .text_styles
                .insert(egui::TextStyle::Body, egui::FontId::proportional(13.0));
            style
                .text_styles
                .insert(egui::TextStyle::Button, egui::FontId::proportional(13.0));
            style
                .text_styles
                .insert(egui::TextStyle::Small, egui::FontId::proportional(11.0));
            style
                .text_styles
                .insert(egui::TextStyle::Heading, egui::FontId::proportional(16.0));
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

pub(crate) fn settings_path() -> Option<PathBuf> {
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
    Back,
    Forward,
    Refresh,
    Copy,
    Move,
    Rename,
    Link,
    Home,
}

pub fn icon_button(ui: &mut egui::Ui, icon: Icon, tooltip: &str, enabled: bool) -> egui::Response {
    let response = ui.add_enabled(
        enabled,
        egui::Button::new(" ")
            .frame(false)
            .min_size(Vec2::splat(ACTION_SIZE)),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, tooltip));
    let t = Palette::get(ui);
    if response.is_pointer_button_down_on() || response.hovered() {
        ui.painter().rect_filled(
            response.rect,
            6,
            if response.is_pointer_button_down_on() {
                t.pressed
            } else {
                t.hover
            },
        );
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect.shrink(1.0),
            6,
            Stroke::new(2.0, t.accent),
            egui::StrokeKind::Inside,
        );
    }
    let mut painter = ui.painter().clone();
    painter.multiply_opacity(if enabled { 1.0 } else { 0.35 });
    paint_icon_at(
        &painter,
        response.rect.center(),
        icon,
        ui.visuals().text_color(),
        18.0,
    );
    response.on_hover_text(tooltip)
}

/// A deliberate primary action; keyboard/default-action policy stays with its caller.
pub(crate) fn primary_button(ui: &mut egui::Ui, label: &str, enabled: bool) -> egui::Response {
    let t = Palette::get(ui);
    ui.add_enabled(
        enabled,
        egui::Button::new(egui::RichText::new(label).strong().color(Color32::WHITE))
            .fill(t.selection)
            .stroke(Stroke::NONE),
    )
}

pub fn paint_icon(p: &egui::Painter, center: Pos2, icon: Icon, ink: Color32) {
    paint_icon_at(p, center, icon, ink, 16.0);
}

fn paint_icon_at(p: &egui::Painter, center: Pos2, icon: Icon, ink: Color32, size: f32) {
    let unit = size / ICON_GRID;
    let origin = center - Vec2::splat(size / 2.0);
    let at = |x, y| origin + Vec2::new(x, y) * unit;
    let line = Stroke::new(1.2 * unit, ink);
    let box_at = |x, y, w, h| Rect::from_min_size(at(x, y), Vec2::new(w, h) * unit);
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
                p.arrow(at(1.0, 14.0), Vec2::new(7.0, -5.0) * unit, line);
            } else {
                for y in [6.0, 9.0, 12.0] {
                    p.line_segment([at(5.0, y), at(11.0, y)], Stroke::new(0.7 * unit, ink));
                }
            }
        }
        Icon::Back => {
            p.arrow(at(14.0, 8.0), Vec2::new(-12.0, 0.0) * unit, line);
        }
        Icon::Forward => {
            p.arrow(at(2.0, 8.0), Vec2::new(12.0, 0.0) * unit, line);
        }
        Icon::Parent => {
            p.arrow(at(8.0, 14.0), Vec2::new(0.0, -12.0) * unit, line);
        }
        Icon::Refresh => {
            let points: Vec<Pos2> = (0..=16)
                .map(|step| {
                    let angle = (45.0 + step as f32 * 17.0).to_radians();
                    center + Vec2::new(angle.cos(), angle.sin()) * (5.5 * unit)
                })
                .collect();
            p.add(egui::Shape::line(points, line));
            p.arrow(at(10.0, 2.0), Vec2::new(4.0, 3.0) * unit, line);
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
            p.arrow(at(1.0, 8.0), Vec2::new(14.0, 0.0) * unit, line);
        }
        Icon::Rename => {
            p.line_segment(
                [at(3.0, 13.0), at(13.0, 3.0)],
                Stroke::new(3.0 * unit, Color32::from_rgb(211, 163, 58)),
            );
            p.line_segment([at(3.0, 15.0), at(14.0, 15.0)], line);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contrast(a: Color32, b: Color32) -> f32 {
        let luminance = |c: Color32| {
            let linear = |v: u8| {
                let s = f32::from(v) / 255.0;
                if s <= 0.04045 {
                    s / 12.92
                } else {
                    ((s + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * linear(c.r()) + 0.7152 * linear(c.g()) + 0.0722 * linear(c.b())
        };
        let a = luminance(a);
        let b = luminance(b);
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    #[test]
    fn applied_text_selection_and_focus_states_have_readable_contrast() {
        let ctx = egui::Context::default();
        Appearance::System.apply(&ctx);
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            let t = Palette::for_theme(theme);
            let style = ctx.style_of(theme);
            for background in [t.chrome, t.content, t.raised, t.field, t.hover, t.pressed] {
                for foreground in [style.visuals.text_color(), t.secondary] {
                    assert!(
                        contrast(foreground, background) >= 4.5,
                        "unreadable {theme:?} text {foreground:?} on {background:?}"
                    );
                }
            }
            assert!(
                contrast(
                    style.visuals.selection.stroke.color,
                    style.visuals.selection.bg_fill
                ) >= 4.5
            );
            assert!(contrast(t.accent, t.content) >= 3.0);
            assert!(contrast(t.error, t.content) >= 4.5);
            assert!(contrast(t.warning, t.content) >= 4.5);
        }
    }

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
                    // Verify an OS change retains the application metrics.
                    assert_eq!(ui.spacing().interact_size.y, 28.0);
                },
            )
            .drop_without_applying_deltas();
        }
    }
}
