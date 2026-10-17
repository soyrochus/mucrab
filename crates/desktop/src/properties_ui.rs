// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
use super::read_job::ReadJob;
use super::*;
use mucrab_core::{
    EntryKind, EntryTarget, FolderSizeReport, MetadataPlan, PropertyRequest, PropertySnapshot,
    folder_size, parse_modified_time,
};
#[derive(Default)]
pub(super) struct Properties {
    pub(super) visible: bool,
    target: Option<EntryTarget>,
    origin: Option<(PaneSide, TabId)>,
    snapshot: Option<PropertySnapshot>,
    load: ReadJob<PropertySnapshot>,
    size: ReadJob<FolderSizeReport>,
    totals: Option<FolderSizeReport>,
    error: Option<String>,
    edit_mode: bool,
    mode: u32,
    edit_time: bool,
    time: String,
    pub(super) review: Option<MetadataPlan>,
}
impl Properties {
    pub(super) fn busy(&self) -> bool {
        self.load.busy() || self.size.busy()
    }
    pub(super) fn cancel(&mut self) {
        self.load.discard();
        self.size.discard();
    }
    pub(super) fn poll(&mut self, ctx: &egui::Context) {
        if let Some(result) = self.load.poll() {
            match result {
                Ok(s) => {
                    self.mode = s.mode.unwrap_or(0) & 0o777;
                    self.time = timestamp(s.modified);
                    self.snapshot = Some(s);
                    self.error = None;
                    self.edit_mode = false;
                    self.edit_time = false;
                }
                Err(e) => self.error = Some(e),
            }
        }
        if let Some(result) = self.size.poll() {
            match result {
                Ok(r) => self.totals = Some(r),
                Err(e) => self.error = Some(e),
            }
        }
        if self.busy() {
            ctx.request_repaint_after(Duration::from_millis(30));
        }
    }
    fn reload(&mut self) -> Result<(), String> {
        let request = PropertyRequest::capture(self.target.clone().ok_or("No captured entry")?)
            .map_err(|e| e.to_string())?;
        self.load.start(move |c| {
            if c.is_cancelled() {
                return Err("Properties cancelled".into());
            }
            request.inspect().map_err(|e| e.to_string())
        })
    }
}
fn timestamp(time: Option<std::time::SystemTime>) -> String {
    time.map(|t| {
        chrono::DateTime::<chrono::Utc>::from(t)
            .to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true)
    })
    .unwrap_or_else(|| "Unavailable".into())
}
impl MuCrabApp {
    pub(super) fn focused_target(&self) -> Option<EntryTarget> {
        let i = usize::from(self.active == PaneSide::Right);
        let pane = &self.panes[i];
        let name = pane
            .selection
            .snapshot(&pane.entries, &pane.order)
            .focused
            .filter(|n| n != "..")?;
        Some(if let Some(location) = &pane.archive {
            EntryTarget::Zip {
                location: location.clone(),
                name,
            }
        } else {
            EntryTarget::Local(pane.location.as_ref()?.join(name))
        })
    }
    pub(super) fn open_properties(&mut self) {
        let Some(target) = self.focused_target() else {
            return;
        };
        self.properties.cancel();
        self.properties.visible = true;
        self.properties.snapshot = None;
        self.properties.totals = None;
        self.properties.error = None;
        let i = usize::from(self.active == PaneSide::Right);
        self.properties.origin = Some((self.active, self.panes.stack(i).active().id));
        self.properties.target = Some(target);
        if let Err(e) = self.properties.reload() {
            self.properties.error = Some(e);
        }
    }
    pub(super) fn properties_window(&mut self, ctx: &egui::Context) {
        if !self.properties.visible {
            return;
        }
        let p = &mut self.properties;
        let mut visible = true;
        let mut refresh = false;
        let mut calculate = false;
        let mut cancel_size = false;
        let mut review = false;
        egui::Window::new("Properties").id(egui::Id::new("properties")).open(&mut visible).default_width(680.0).default_height(640.0).resizable(true).show(ctx,|ui|{
         egui::ScrollArea::vertical().id_salt("property-scroll").max_height((ctx.content_rect().height()-90.).max(160.)).show(ui,|ui|{
   if let Some(target)=&p.target{ui.label(target.display_path().display().to_string());}
   ui.horizontal(|ui|{refresh=ui.add_enabled(!p.busy(),egui::Button::new("Refresh properties")).clicked();if p.load.busy(){ui.spinner();ui.label("Inspecting captured entry…");}});
   if let Some(s)=&p.snapshot{
    egui::Grid::new("property-values").striped(true).num_columns(2).show(ui,|ui|{
     for (label,value) in [
      ("Type",format!("{:?}",s.kind)),("Entry size (bytes)",s.size.map(|v|v.to_string()).unwrap_or_else(||"Unavailable".into())),
      ("Allocated bytes",s.allocated_size.map(|v|v.to_string()).unwrap_or_else(||"Unavailable".into())),
      ("Modified (UTC)",timestamp(s.modified)),("Accessed (UTC)",timestamp(s.accessed)),("Created (UTC)",timestamp(s.created)),
      ("Owner",s.uid.map(|id|format!("{} ({id})",s.owner.as_deref().unwrap_or("Name unavailable"))).unwrap_or_else(||"Unavailable".into())),
      ("Group",s.gid.map(|id|format!("{} ({id})",s.group.as_deref().unwrap_or("Name unavailable"))).unwrap_or_else(||"Unavailable".into())),
      ("Unix mode",s.mode.map(|m|format!("{m:04o}")).unwrap_or_else(||"Unavailable".into())),("Hard links",s.links.map(|v|v.to_string()).unwrap_or_else(||"Unavailable".into())),
      ("Link target",s.link_target.as_ref().map(|v|v.display().to_string()).unwrap_or_else(||"Unavailable".into()))]{ui.label(label);ui.label(value);ui.end_row();}
    });
    if s.kind==EntryKind::File&&s.links.is_some_and(|n|n>1){ui.label("Metadata changes affect every hard-linked name of this inode.");}
    if s.kind==EntryKind::Folder&&matches!(s.target,EntryTarget::Local(_)){
     ui.separator();ui.horizontal(|ui|{calculate=ui.add_enabled(!p.size.busy()&&!p.load.busy(),egui::Button::new("Calculate folder contents")).clicked();cancel_size=ui.add_enabled(p.size.busy(),egui::Button::new("Cancel calculation")).clicked();});
     ui.small("Logical file bytes, including hidden entries; hard links counted per name. Links are not followed; mount roots are excluded.");
     if let Some(progress)=p.size.progress(){ui.label(format!("{:?} — {} visited",progress.state,progress.completed_items));if let Some(path)=progress.current_path{ui.label(path.display().to_string());}}
     if let Some(r)=&p.totals {ui.label(format!("{} — {} bytes · {} files · {} folders · {} links · {} visited · {} omissions/errors",if r.partial(){"Partial"}else{"Complete"},r.bytes,r.files,r.folders,r.links,r.visited,r.errors));if !r.details.is_empty(){ui.collapsing("Details (first 256)",|ui|{egui::ScrollArea::vertical().max_height(100.).show(ui,|ui|{for d in &r.details{ui.label(d);}});});}}
    }
    ui.separator();
    if s.editable(){
     ui.small("Non-recursive changes. Special bits and access time are preserved. UTC input has nanosecond resolution; filesystem rounding is shown after refresh.");
     ui.checkbox(&mut p.edit_mode,"Change permissions");
     ui.add_enabled_ui(p.edit_mode,|ui|{egui::Grid::new("rwx-bits").show(ui,|ui|{ui.label("");ui.label("Read");ui.label("Write");ui.label("Execute");ui.end_row();for (label,shift) in [("Owner",6),("Group",3),("Other",0)]{ui.label(label);for bit in [4,2,1]{let mask=bit<<shift;let mut on=p.mode&mask!=0;if ui.checkbox(&mut on,"").changed(){if on{p.mode|=mask;}else{p.mode&=!mask;}}}ui.end_row();}});});
     ui.checkbox(&mut p.edit_time,"Change modified time");ui.add_enabled(p.edit_time,egui::TextEdit::singleline(&mut p.time).desired_width(430.));
     review=ui.add_enabled(self.copy_job.is_none()&&!self.everyday.busy()&&!p.load.busy()&&(p.edit_mode||p.edit_time),egui::Button::new("Review changes…")).clicked();
    }else{ui.label("Read-only properties. Permission/date editing is unavailable for links, ZIP members and unsupported kinds.");}
   }
   if let Some(error)=&p.error{ui.colored_label(ui.visuals().error_fg_color,error);}
  });
        });
        if !visible {
            self.properties.visible = false;
            self.properties.cancel();
            self.restore_copy_focus = true;
        }
        if cancel_size {
            self.properties.size.cancel();
        }
        if refresh && let Err(e) = self.properties.reload() {
            self.properties.error = Some(e);
        }
        if calculate
            && let Some(snapshot) = self.properties.snapshot.clone()
            && let EntryTarget::Local(path) = snapshot.target.clone()
        {
            self.properties.totals = None;
            if let Err(e) = self.properties.size.start(move |c| {
                snapshot.validate().map_err(|e| e.to_string())?;
                folder_size(&path, &c).map_err(|e| e.to_string())
            }) {
                self.properties.error = Some(e);
            }
        }
        if review {
            let p = &mut self.properties;
            let result = (|| {
                let time = if p.edit_time {
                    Some(parse_modified_time(&p.time)?)
                } else {
                    None
                };
                p.snapshot
                    .as_ref()
                    .ok_or("No captured properties")?
                    .metadata_plan(p.edit_mode.then_some(p.mode), time)
                    .map_err(|e| e.to_string())
            })();
            match result {
                Ok(plan) => p.review = Some(plan),
                Err(e) => p.error = Some(e),
            }
        }
    }
    pub(super) fn properties_review(&mut self, ctx: &egui::Context) {
        let Some(plan) = self.properties.review.as_ref() else {
            return;
        };
        let mut apply = false;
        let response=egui::Modal::new(egui::Id::new("metadata-review")).show(ctx,|ui|{
   ui.set_width(620.);ui.heading("Apply metadata changes");ui.label(plan.snapshot.target.display_path().display().to_string());ui.label("One entry only; no recursion. Completed changes are retained if a later field fails.");
   if let Some(rwx)=plan.rwx{ui.label(format!("Permissions: {:04o} → {:04o} (special bits retained)",plan.snapshot.mode.unwrap_or(0),(plan.snapshot.mode.unwrap_or(0)&0o7000)|rwx));}
   if let Some(time)=plan.modified{ui.label(format!("Modified UTC: {} → {}",timestamp(plan.snapshot.modified),timestamp(Some(time))));ui.label("Access time is preserved; actual stored precision is read back.");}
   if plan.snapshot.kind==EntryKind::File&&plan.snapshot.links.is_some_and(|n|n>1){ui.label("This inode has multiple hard links. Every name shares these metadata changes.");}
   let mut cancel=false;ui.horizontal(|ui|{cancel=ui.button("Cancel").clicked();apply=ui.add_enabled(self.copy_job.is_none(),egui::Button::new("Apply changes")).clicked();});cancel
  });
        if response.inner || response.should_close() {
            self.properties.review = None;
            self.restore_copy_focus = true;
        } else if apply {
            let plan = self.properties.review.take().expect("reviewed plan");
            let Some((side, tab)) = self.properties.origin else {
                return;
            };
            let directory = plan
                .snapshot
                .target
                .display_path()
                .parent()
                .map(PathBuf::from)
                .unwrap_or_default();
            self.start_job(
                FileAction::Metadata,
                side,
                tab,
                directory,
                None,
                move |control| {
                    let report = plan.apply(&control).map_err(|e| e.to_string())?;
                    let changed = report.steps.iter().filter(|s| s.changed).count();
                    let outcomes = Vec::new();
                    let issues = report
                        .steps
                        .iter()
                        .filter(|step| !step.changed)
                        .map(|step| CopyIssue {
                            path: plan.snapshot.target.display_path(),
                            kind: if report.cancelled {
                                CopyIssueKind::Cancelled
                            } else {
                                CopyIssueKind::Failed
                            },
                            message: format!(
                                "{}: {}",
                                step.field,
                                step.error.as_deref().unwrap_or("Not attempted")
                            ),
                        })
                        .collect();
                    Ok(JobResult {
                        cancelled: report.cancelled,
                        outcomes,
                        issues,
                        summary: format!(
                            "Metadata: {changed}/{} fields changed · {}",
                            report.steps.len(),
                            report
                                .steps
                                .iter()
                                .map(|step| format!(
                                    "{} {}",
                                    step.field,
                                    if step.changed {
                                        "changed"
                                    } else {
                                        "not changed"
                                    }
                                ))
                                .collect::<Vec<_>>()
                                .join(" · ")
                        ),
                    })
                },
            );
        }
    }
    pub(super) fn refresh_properties_after_metadata(&mut self) {
        if self.properties.visible
            && let Err(e) = self.properties.reload()
        {
            self.properties.error = Some(e);
        }
    }
}
