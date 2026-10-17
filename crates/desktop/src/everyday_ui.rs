// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
use super::*;
use crate::native_files;
use mucrab_core::{DeletePlan, PastePlan, PinnedDirectory};
use std::path::Path;

#[derive(Clone, Copy)]
pub(super) enum Action {
    NewFolder,
    Trash,
    Delete,
    CopyClipboard,
    Paste,
    Back,
    Forward,
    Find,
}
pub(super) enum Review {
    Folder {
        side: PaneSide,
        tab: TabId,
        directory: PinnedDirectory,
        name: String,
    },
    Delete {
        side: PaneSide,
        tab: TabId,
        directory: PathBuf,
        plan: DeletePlan,
        permanent: bool,
    },
    Paste {
        side: PaneSide,
        tab: TabId,
        directory: PathBuf,
        plan: PastePlan,
        policy: CopyPolicy,
    },
}
struct ClipboardWork {
    side: PaneSide,
    tab: TabId,
    destination: Option<PinnedDirectory>,
    receiver: mpsc::Receiver<Result<Option<Vec<PathBuf>>, String>>,
}
#[derive(Default)]
pub(super) struct Everyday {
    pub(super) review: Option<Review>,
    clipboard: Option<ClipboardWork>,
    pub(super) message: Option<String>,
    focus_field: bool,
}
impl Everyday {
    pub(super) fn busy(&self) -> bool {
        self.clipboard.is_some()
    }
}
impl MuCrabApp {
    pub(super) fn everyday_action(&mut self, action: Action, ctx: &egui::Context) {
        let i = usize::from(self.active == PaneSide::Right);
        match action {
            Action::Back | Action::Forward => {
                self.panes[i].traverse_history(matches!(action, Action::Forward), &self.sorter);
                return;
            }
            Action::Find => {
                self.panes[i].quick_find = Some(String::new());
                return;
            }
            _ => {}
        }
        if self.copy_job.is_some() || self.everyday.review.is_some() || self.everyday.busy() {
            return;
        }
        let result = (|| -> Result<(), String> {
            let tab = self.panes.stack(i).active().id;
            let pane = &self.panes[i];
            if pane.archive.is_some() {
                return Err("This action requires a local folder.".into());
            }
            let directory = pane.location.clone().ok_or("No local folder open")?;
            let pin = PinnedDirectory::new(&directory).map_err(|e| e.to_string())?;
            let names = pane.selection.snapshot(&pane.entries, &pane.order).selected;
            let paths: Vec<_> = names.iter().map(|n| directory.join(n)).collect();
            self.everyday.review = match action {
                Action::NewFolder => Some(Review::Folder {
                    side: self.active,
                    tab,
                    directory: pin,
                    name: "New folder".into(),
                }),
                Action::Trash | Action::Delete => {
                    let protected: Vec<_> = self
                        .panes
                        .all()
                        .filter_map(|t| {
                            let p = &t.pane;
                            p.archive
                                .as_ref()
                                .map(|z| z.archive.clone())
                                .or_else(|| p.location.clone())
                        })
                        .collect();
                    let plan = DeletePlan::new(&paths, &protected).map_err(|e| e.to_string())?;
                    Some(Review::Delete {
                        side: self.active,
                        tab,
                        directory,
                        plan,
                        permanent: matches!(action, Action::Delete),
                    })
                }
                Action::CopyClipboard | Action::Paste => {
                    if matches!(action, Action::CopyClipboard) && paths.is_empty() {
                        return Err("Select local files to copy.".into());
                    }
                    let (tx, rx) = mpsc::channel();
                    let paste = matches!(action, Action::Paste);
                    let worker = std::thread::Builder::new()
                        .name("mucrab-file-clipboard".into())
                        .spawn(move || {
                            let r = if paste {
                                native_files::read_clipboard().map(Some)
                            } else {
                                native_files::write_clipboard(&paths).map(|()| None)
                            };
                            let _ = tx.send(r.map_err(|e| e.to_string()));
                        })
                        .map_err(|e| e.to_string())?;
                    drop(worker);
                    self.everyday.clipboard = Some(ClipboardWork {
                        side: self.active,
                        tab,
                        destination: paste.then_some(pin),
                        receiver: rx,
                    });
                    self.everyday.message = Some(
                        if paste {
                            "Reading file clipboard…"
                        } else {
                            "Copying file selection to clipboard…"
                        }
                        .into(),
                    );
                    None
                }
                _ => None,
            };
            self.everyday.focus_field = true;
            Ok(())
        })();
        if let Err(error) = result {
            self.everyday.message = Some(error);
        }
        ctx.request_repaint();
    }
    pub(super) fn poll_everyday(&mut self, ctx: &egui::Context) {
        let Some(work) = &self.everyday.clipboard else {
            return;
        };
        let response = match work.receiver.try_recv() {
            Ok(r) => r,
            Err(mpsc::TryRecvError::Empty) => {
                ctx.request_repaint_after(Duration::from_millis(50));
                return;
            }
            Err(mpsc::TryRecvError::Disconnected) => Err("File clipboard worker stopped".into()),
        };
        let work = self.everyday.clipboard.take().unwrap();
        let result = (|| -> Result<(), String> {
            match response? {
                None => {
                    self.everyday.message =
                        Some("File selection copied to the system clipboard.".into())
                }
                Some(paths) => {
                    let directory = work.destination.ok_or("No captured Paste destination")?;
                    directory.validate().map_err(|e| e.to_string())?;
                    let plan =
                        PastePlan::new(&paths, &directory.path).map_err(|e| e.to_string())?;
                    self.everyday.message = None;
                    self.everyday.review = Some(Review::Paste {
                        side: work.side,
                        tab: work.tab,
                        directory: directory.path,
                        plan,
                        policy: CopyPolicy::Skip,
                    });
                }
            }
            Ok(())
        })();
        if let Err(e) = result {
            self.everyday.message = Some(format!("File clipboard: {e}"));
        }
    }
    pub(super) fn everyday_dialog(&mut self, ctx: &egui::Context) {
        let mut apply = false;
        let mut cancel = false;
        if let Some(review) = &mut self.everyday.review {
            let response=egui::Modal::new(egui::Id::new("everyday-review")).show(ctx,|ui|{
                ui.set_width(520.0);
                match review{
                    Review::Folder{directory,name,..}=>{
                        ui.heading("New Folder");ui.label(directory.path.display().to_string());
                        ui.label("Folder name:");let field=ui.text_edit_singleline(name);
                        if self.everyday.focus_field{field.request_focus();self.everyday.focus_field=false;}
                        if !is_valid_leaf_name(name){ui.label("Enter one folder name without path separators.");}
                        ui.separator();
                        ui.horizontal(|ui|{cancel=ui.button("Cancel").clicked();apply=ui.add_enabled(is_valid_leaf_name(name),egui::Button::new("Create folder")).clicked();});
                    }
                    Review::Delete{plan,permanent,..}=>{
                        ui.heading(if *permanent{"Permanently delete selected items?"}else{"Move selected items to Trash?"});
                        if *permanent{ui.label("This permanently removes these entries and all folder contents, including hidden files. It bypasses Trash.");}else{ui.label("Use the operating system Trash. Files are never permanently deleted if Trash fails.");}
                        paths(ui,&plan.paths());
                        ui.separator();ui.horizontal(|ui|{cancel=ui.button("Cancel").clicked();apply=ui.button(if *permanent{"Permanently delete"}else{"Move to Trash"}).clicked();});
                    }
                    Review::Paste{directory,plan,policy,..}=>{
                        ui.heading("Paste files into this folder");ui.label(directory.display().to_string());paths(ui,&plan.paths());
                        egui::ComboBox::from_id_salt("paste-policy").selected_text(copy_policy_label(*policy)).show_ui(ui,|ui|{for p in [CopyPolicy::Skip,CopyPolicy::Overwrite,CopyPolicy::OverwriteIfOlder,CopyPolicy::OverwriteIfSizeDiffers]{ui.selectable_value(policy,p,copy_policy_label(p));}});
                        ui.label("Copy only. Existing folders merge; completed and partial copies remain after errors or cancellation.");
                        ui.horizontal(|ui|{cancel=ui.button("Cancel").clicked();apply=ui.button("Paste files").clicked();});
                    }
                }
            });
            cancel |= response.should_close();
        }
        if cancel {
            self.everyday.review = None;
            self.restore_copy_focus = true;
            ctx.request_repaint();
        } else if apply && let Some(review) = self.everyday.review.take() {
            self.restore_copy_focus = true;
            match review {
                Review::Folder {
                    side,
                    tab,
                    directory,
                    name,
                } => {
                    self.start_job(
                        FileAction::NewFolder,
                        side,
                        tab,
                        directory.path.clone(),
                        Some(name.clone()),
                        move |control| {
                            let success = directory
                                .create_folder(&name, &control)
                                .map_err(|e| e.to_string())?;
                            Ok(JobResult {
                                cancelled: control.is_cancelled(),
                                outcomes: vec![CopyOutcome { name, success }],
                                issues: Vec::new(),
                                summary: "Folder created".into(),
                            })
                        },
                    );
                }
                Review::Delete {
                    side,
                    tab,
                    directory,
                    plan,
                    permanent,
                } => {
                    self.start_job(
                        if permanent {
                            FileAction::Delete
                        } else {
                            FileAction::Trash
                        },
                        side,
                        tab,
                        directory,
                        None,
                        move |control| {
                            let mut adapter = |path: &Path| native_files::trash(path);
                            let r = plan
                                .apply(&control, if permanent { None } else { Some(&mut adapter) })
                                .map_err(|e| e.to_string())?;
                            Ok(result(
                                r,
                                if permanent {
                                    "Permanent deletion finished"
                                } else {
                                    "Move to Trash finished"
                                },
                            ))
                        },
                    );
                }
                Review::Paste {
                    side,
                    tab,
                    directory,
                    plan,
                    policy,
                } => {
                    self.start_job(
                        FileAction::Paste,
                        side,
                        tab,
                        directory,
                        None,
                        move |control| {
                            plan.apply(policy, &control)
                                .map(|r| result(r, "Paste finished"))
                                .map_err(|e| e.to_string())
                        },
                    );
                }
            }
            ctx.request_repaint();
        }
    }
}
fn paths(ui: &mut egui::Ui, paths: &[PathBuf]) {
    ui.label(format!("{} selected items", paths.len()));
    egui::ScrollArea::vertical()
        .max_height(180.0)
        .show(ui, |ui| {
            for p in paths {
                ui.label(p.display().to_string());
            }
        });
}
fn result(report: mucrab_core::CopyReport, label: &str) -> JobResult {
    JobResult {
        cancelled: report.cancelled,
        summary: format!(
            "{label}: {} items completed",
            report.outcomes.iter().filter(|o| o.success).count()
        ),
        outcomes: report.outcomes,
        issues: report.issues,
    }
}
