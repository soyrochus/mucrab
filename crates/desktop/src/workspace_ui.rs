// SPDX-License-Identifier: GPL-3.0-only
//! Two-pane composition with stable tab ownership and returned file activation.
use super::{MuCrabApp, PaneSide};
use crate::browser_ui::pane;
use eframe::egui;
impl MuCrabApp {
    pub(super) fn render_workspace(&mut self, ui: &mut egui::Ui, browsing: bool) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(ui.visuals().panel_fill)
                    .inner_margin(8.0),
            )
            .show(ui, |ui| {
                let area = ui.available_rect_before_wrap();
                let divider_width = 10.0;
                let minimum = (220.0 / area.width()).min(0.4);
                self.split = self.split.clamp(minimum, 1.0 - minimum);
                let middle = area.left() + (area.width() - divider_width) * self.split;
                let left = egui::Rect::from_min_max(area.min, egui::pos2(middle, area.bottom()));
                let divider = egui::Rect::from_min_size(
                    egui::pos2(middle, area.top()),
                    egui::vec2(divider_width, area.height()),
                );
                let right =
                    egui::Rect::from_min_max(egui::pos2(divider.right(), area.top()), area.max);
                let response = ui
                    .interact(
                        divider,
                        ui.id().with("divider"),
                        egui::Sense::click_and_drag(),
                    )
                    .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                if response.dragged()
                    && let Some(pos) = response.interact_pointer_pos()
                {
                    self.split = ((pos.x - area.left()) / (area.width() - divider_width))
                        .clamp(minimum, 1.0 - minimum);
                }
                if response.double_clicked() {
                    self.split = 0.5;
                }
                ui.painter().line_segment(
                    [divider.center_top(), divider.center_bottom()],
                    ui.visuals().widgets.noninteractive.bg_stroke,
                );
                for (index, (side, rect)) in [(PaneSide::Left, left), (PaneSide::Right, right)]
                    .into_iter()
                    .enumerate()
                {
                    ui.scope_builder(
                        egui::UiBuilder::new()
                            .max_rect(rect)
                            .layout(egui::Layout::top_down(egui::Align::Min)),
                        |ui| {
                            self.tab_strip(ui, side, browsing);
                            let id = self.panes.stack(index).active().id;
                            if let Some(status) = self.browse.status(id) {
                                ui.weak(status);
                            }
                            let tab = self.panes.stack_mut(index).active_mut();
                            let target = ui
                                .push_id(tab.id, |ui| {
                                    pane(
                                        ui,
                                        side,
                                        &mut tab.pane,
                                        &self.sorter,
                                        &mut self.active,
                                        &mut tab.widths,
                                        browsing
                                            && self.confirmations.copy.is_none()
                                            && self.confirmations.zip.is_none(),
                                    )
                                })
                                .inner;
                            if let Some(target) = target {
                                self.dispatch_target(target);
                            }
                        },
                    );
                }
            });
    }
}
