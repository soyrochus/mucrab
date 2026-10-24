// SPDX-License-Identifier: GPL-3.0-only
//! Confirmation state and rendering return decisions without starting workers.
use super::{FileAction, batch_case_label, copy_policy_label};
use crate::appearance::primary_button;
use eframe::egui;
use mucrab_core::{
    BatchRenameOptions, BatchRenamePlan, CopyPolicy, CopyRequest, PaneSide, TabId, ZipExtractPlan,
    is_valid_leaf_name,
};
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Decision {
    None,
    Cancel,
    Apply,
}
#[derive(Default)]
pub(super) struct Confirmations {
    pub(super) copy: Option<CopyConfirmation>,
    pub(super) batch: Option<BatchReview>,
    pub(super) zip: Option<ZipReview>,
}
pub(super) struct CopyConfirmation {
    pub(super) action: FileAction,
    pub(super) source_side: PaneSide,
    pub(super) source_tab: TabId,
    pub(super) request: CopyRequest,
}

pub(super) struct BatchReview {
    pub(super) source_side: PaneSide,
    pub(super) source_tab: TabId,
    pub(super) directory: PathBuf,
    pub(super) names: Vec<String>,
    pub(super) options: BatchRenameOptions,
    pub(super) plan: Result<BatchRenamePlan, String>,
}

pub(super) struct ZipReview {
    pub(super) source_side: PaneSide,
    pub(super) source_tab: TabId,
    pub(super) source_directory: PathBuf,
    pub(super) source_names: Vec<String>,
    pub(super) policy: CopyPolicy,
    pub(super) plan: Result<ZipExtractPlan, String>,
}

impl Confirmations {
    pub(super) fn draw_copy_dialogs(&mut self, ctx: &egui::Context) -> Decision {
        let mut start = false;
        let mut cancel = false;
        if let Some(confirmation) = &mut self.copy {
            let response = egui::Modal::new(egui::Id::new("copy-confirmation")).show(ctx, |ui| {
                ui.set_width(510.0);
                let destination = if confirmation.source_side == PaneSide::Left { "right" } else { "left" };
                let count=confirmation.request.names.len();
                let rename = confirmation.action == FileAction::Rename;
                ui.heading(if rename { "Rename focused item".to_owned() } else { format!("{} {count} {} to {destination} pane",confirmation.action.label(),if count==1 {"item"} else {"items"}) });
                ui.add_space(8.0);
                ui.label("From:"); ui.label(confirmation.request.source_directory.display().to_string());
                ui.label("To:"); ui.label(confirmation.request.destination_directory.display().to_string());
                ui.separator();
                if let Some(name) = &mut confirmation.request.new_name {
                    ui.label("New name:");
                    ui.text_edit_singleline(name);
                    if !is_valid_leaf_name(name) { ui.label("Enter a direct child name without path separators."); }
                }
                egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                    for name in &confirmation.request.names { ui.label(name); }
                });
                ui.separator();
                egui::ComboBox::from_id_salt("copy-policy").width(300.0).selected_text(copy_policy_label(confirmation.request.policy)).show_ui(ui,|ui| {
                    for policy in [CopyPolicy::Skip,CopyPolicy::Overwrite,CopyPolicy::OverwriteIfOlder,CopyPolicy::OverwriteIfSizeDiffers] {
                        ui.selectable_value(&mut confirmation.request.policy,policy,copy_policy_label(policy));
                    }
                });
                ui.label(if confirmation.action == FileAction::Copy { "Existing folders are merged. Files copied before an error remain in the destination." } else { "Existing folders are merged. Partial moves remain; skipped or failed sources are retained." });
                ui.add_space(8.0);
                let valid = confirmation.request.new_name.as_ref().is_none_or(|name| is_valid_leaf_name(name) && name != &confirmation.request.names[0]);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { start=primary_button(ui,confirmation.action.label(),valid).clicked();cancel=ui.button("Cancel").clicked(); });
            });
            cancel |= response.should_close();
        }
        if cancel {
            Decision::Cancel
        } else if start {
            Decision::Apply
        } else {
            Decision::None
        }
    }
    pub(super) fn draw_batch_dialog(&mut self, ctx: &egui::Context) -> Decision {
        let mut apply = false;
        let mut cancel = false;
        if let Some(review) = &mut self.batch {
            let response = egui::Modal::new(egui::Id::new("batch-rename")).show(ctx, |ui| {
                ui.set_width(700.0);
                ui.heading(format!("Batch rename {} items", review.names.len()));
                ui.label(review.directory.display().to_string());
                ui.separator();
                let before = review.options.clone();
                egui::Grid::new("batch-options").num_columns(2).show(ui, |ui| {
                    ui.label("Mask"); ui.add(egui::TextEdit::singleline(&mut review.options.mask).desired_width(510.0)); ui.end_row();
                    ui.label("Search"); ui.text_edit_singleline(&mut review.options.search); ui.end_row();
                    ui.label("Replace with"); ui.text_edit_singleline(&mut review.options.replacement); ui.end_row();
                    ui.label("Case");
                    egui::ComboBox::from_id_salt("batch-case").selected_text(batch_case_label(review.options.case_mode)).show_ui(ui, |ui| {
                        for value in 0..5 { ui.selectable_value(&mut review.options.case_mode, value, batch_case_label(value)); }
                    }); ui.end_row();
                    ui.label("Counter"); ui.horizontal(|ui| {
                        ui.label("Start"); ui.add(egui::TextEdit::singleline(&mut review.options.counter_start).desired_width(58.0));
                        ui.label("Step"); ui.add(egui::TextEdit::singleline(&mut review.options.counter_step).desired_width(58.0));
                        ui.label("Digits"); ui.add(egui::DragValue::new(&mut review.options.digits).range(1..=5));
                    }); ui.end_row();
                });
                ui.small("[N] name · [E] extension · [C] counter · [P] parent · [YMD] UTC date · [N2-5] range");
                if before != review.options {
                    review.plan = BatchRenamePlan::new(&review.directory, &review.names, &review.options).map_err(|error| error.to_string());
                }
                ui.separator();
                let enabled = match &review.plan {
                    Ok(plan) => {
                        egui::ScrollArea::vertical().max_height(190.0).show(ui, |ui| {
                            egui::Grid::new("batch-preview").num_columns(3).striped(true).show(ui, |ui| {
                                ui.strong("Original name"); ui.label(""); ui.strong("New name"); ui.end_row();
                                for row in &plan.preview().rows { ui.label(&row.name); ui.label("->"); ui.label(&row.new_name); ui.end_row(); }
                            });
                        });
                        if plan.preview().duplicate_names { ui.colored_label(ui.visuals().error_fg_color, "Duplicate new names: change the mask or counter."); }
                        else if plan.preview().old_names_conflict { ui.colored_label(ui.visuals().error_fg_color, "A new name conflicts with another selected original name."); }
                        else if !plan.preview().rows.iter().all(|row| is_valid_leaf_name(&row.new_name)) { ui.colored_label(ui.visuals().error_fg_color, "Each new name must be a nonempty direct child name."); }
                        ui.label(format!("{} changed · {} unchanged", plan.preview().changed_count, plan.preview().rows.len() - plan.preview().changed_count));
                        plan.preview().can_apply()
                    }
                    Err(message) => { ui.colored_label(ui.visuals().error_fg_color, message); false }
                };
                ui.label("Existing files are skipped; folders merge. Partial renames remain after errors.");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { apply = primary_button(ui,"Apply rename",enabled).clicked(); cancel = ui.button("Cancel").clicked(); });
            });
            cancel |= response.should_close();
        }
        if cancel {
            Decision::Cancel
        } else if apply {
            Decision::Apply
        } else {
            Decision::None
        }
    }
    pub(super) fn draw_zip_dialog(&mut self, ctx: &egui::Context) -> Decision {
        let mut apply = false;
        let mut cancel = false;
        if let Some(review) = &mut self.zip {
            let response = egui::Modal::new(egui::Id::new("zip-extraction")).show(ctx, |ui| {
                ui.set_width(560.0);
                ui.heading("Extract ZIP to other pane");
                match &review.plan {
                    Ok(plan) => {
                        ui.label("Archive:");
                        ui.label(plan.archive().display().to_string());
                        if !plan.prefix().is_empty() {
                            ui.label(format!("Folder: {}", plan.prefix()));
                        }
                        ui.label("Destination:");
                        ui.label(plan.destination().display().to_string());
                        ui.separator();
                        egui::ScrollArea::vertical()
                            .max_height(140.0)
                            .show(ui, |ui| {
                                for name in plan.names() {
                                    ui.label(name);
                                }
                            });
                        ui.label(format!(
                            "{} selected entries · {} expanded bytes",
                            plan.names().len(),
                            plan.expanded_bytes()
                        ));
                    }
                    Err(error) => {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                }
                egui::ComboBox::from_id_salt("zip-policy")
                    .width(300.0)
                    .selected_text(copy_policy_label(review.policy))
                    .show_ui(ui, |ui| {
                        for policy in [
                            CopyPolicy::Skip,
                            CopyPolicy::Overwrite,
                            CopyPolicy::OverwriteIfOlder,
                            CopyPolicy::OverwriteIfSizeDiffers,
                        ] {
                            ui.selectable_value(
                                &mut review.policy,
                                policy,
                                copy_policy_label(policy),
                            );
                        }
                    });
                ui.label(
                    "Folders merge. Extraction stops at the first error; earlier files remain.",
                );
                ui.label("The archive stays unchanged. Unsafe paths and links are rejected.");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    apply = primary_button(ui, "Extract", review.plan.is_ok()).clicked();
                    cancel = ui.button("Cancel").clicked();
                });
            });
            cancel |= response.should_close();
        }
        if cancel {
            Decision::Cancel
        } else if apply {
            Decision::Apply
        } else {
            Decision::None
        }
    }
}
