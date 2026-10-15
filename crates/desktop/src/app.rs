// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

use eframe::egui;
use mucrab_core::PaneSide;

pub fn run() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("muCrab")
            .with_inner_size([1000.0, 640.0])
            .with_min_inner_size([640.0, 400.0])
            .with_icon(egui::IconData::default()),
        renderer: eframe::Renderer::Glow,
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native(
        "muCrab",
        options,
        Box::new(|creation| {
            creation.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::<MuCrabApp>::default())
        }),
    )
}

#[derive(Default)]
struct MuCrabApp;

impl eframe::App for MuCrabApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("muCrab").size(30.0));
                ui.add_space(12.0);
                ui.label("Two claws, two panes.");
            });
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(
                    "Preview · File browsing and file operations are not available yet.",
                )
                .color(egui::Color32::from_rgb(235, 183, 105)),
            );
            ui.add_space(18.0);

            let pane_height = (ui.available_height() - 42.0).max(180.0);
            ui.columns(2, |panes| {
                pane(&mut panes[0], PaneSide::Left, pane_height);
                pane(&mut panes[1], PaneSide::Right, pane_height);
            });

            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.small(format!("muCrab {}", env!("CARGO_PKG_VERSION")));
                ui.separator();
                ui.small("GNU GPL version 3");
                ui.separator();
                ui.small("Based on muCommander");
            });
        });
    }
}

fn pane(ui: &mut egui::Ui, side: PaneSide, height: f32) {
    let title = match side {
        PaneSide::Left => "Left pane",
        PaneSide::Right => "Right pane",
    };
    egui::Frame::group(ui.style())
        .inner_margin(16.0)
        .show(ui, |ui| {
            ui.set_min_size(egui::vec2(ui.available_width(), (height - 32.0).max(120.0)));
            ui.strong(title);
            ui.add_space(8.0);
            ui.label("No folder open");
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(16.0);
            ui.weak("No files to display");
        });
}
