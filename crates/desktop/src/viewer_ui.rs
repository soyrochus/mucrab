// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
use super::read_job::ReadJob;
use super::{Duration, MuCrabApp, PaneSide, PathBuf, TabId, egui};
use mucrab_core::{EntryTarget, Preview, PreviewContent, TextDocument, ViewRequest};
use std::ops::Range;
#[derive(Default)]
pub(super) struct Viewer {
    pub(super) visible: bool,
    origin: Option<(PaneSide, TabId)>,
    target: Option<EntryTarget>,
    job: ReadJob<(ViewRequest, Preview)>,
    loaded: Option<(ViewRequest, Preview)>,
    error: Option<String>,
    texture: Option<egui::TextureHandle>,
    fit: bool,
    zoom: f32,
    find: String,
    cursor: TextCursor,
    focus: bool,
    owned_focus: bool,
}
#[derive(Default)]
struct TextCursor {
    anchor: usize,
    head: usize,
    dragging: bool,
    scroll_to: Option<usize>,
}
impl Viewer {
    pub(super) fn busy(&self) -> bool {
        self.job.busy()
    }
    pub(super) fn cancel(&mut self) {
        self.job.discard();
    }
    pub(super) fn poll(&mut self, ctx: &egui::Context) {
        if let Some(result) = self.job.poll()
            && self.visible
        {
            match result {
                Ok((request, preview)) => {
                    if let PreviewContent::Image(image) = &preview.content {
                        let limit = ctx.input(|i| i.max_texture_side) as u32;
                        if image.width > limit || image.height > limit {
                            self.error = Some(
                                "Image exceeds the current GPU texture limit; open externally"
                                    .into(),
                            );
                            return;
                        }
                        let pixels = egui::ColorImage::from_rgba_unmultiplied(
                            [image.width as usize, image.height as usize],
                            &image.rgba,
                        );
                        self.texture = Some(ctx.load_texture(
                            "viewer-image",
                            pixels,
                            egui::TextureOptions::LINEAR,
                        ));
                    }
                    self.loaded = Some((request, preview));
                    self.focus = true;
                    self.cursor.anchor = 0;
                    self.cursor.head = 0;
                    self.cursor.dragging = false;
                    self.cursor.scroll_to = Some(0);
                }
                Err(e) => self.error = Some(e),
            }
        }
        if self.busy() {
            ctx.request_repaint_after(Duration::from_millis(30));
        }
    }
}
fn text_id() -> egui::Id {
    egui::Id::new("viewer-text-canvas")
}
fn selection(a: usize, b: usize) -> Range<usize> {
    a.min(b)..a.max(b)
}
fn copy_selection(doc: &TextDocument, a: usize, b: usize) -> String {
    let range = selection(a, b);
    doc.text.get(range).unwrap_or("").to_owned()
}
fn text_canvas(
    ui: &mut egui::Ui,
    doc: &TextDocument,
    find: &str,
    cursor: &mut TextCursor,
    focus: &mut bool,
) {
    let TextCursor {
        anchor,
        head,
        dragging,
        scroll_to,
    } = cursor;
    let id = text_id();
    ui.interact(
        ui.available_rect_before_wrap(),
        id,
        egui::Sense::focusable_noninteractive(),
    );
    if *focus {
        ui.memory_mut(|m| m.request_focus(id));
        *focus = false;
    }
    if ui.memory(|m| m.has_focus(id)) {
        let (all, copy) = ui.input_mut(|i| {
            let all = i.consume_key(egui::Modifiers::COMMAND, egui::Key::A);
            let copy = i.consume_key(egui::Modifiers::COMMAND, egui::Key::C)
                || i.events.iter().any(|e| matches!(e, egui::Event::Copy));
            i.events.retain(|e| !matches!(e, egui::Event::Copy));
            (all, copy)
        });
        if all {
            *anchor = 0;
            *head = doc.text.len();
        }
        if copy {
            ui.ctx().copy_text(copy_selection(doc, *anchor, *head));
        }
        let next = ui.input_mut(|i| {
            if i.consume_key(egui::Modifiers::SHIFT, egui::Key::F3) {
                Some(false)
            } else if i.consume_key(egui::Modifiers::NONE, egui::Key::F3) {
                Some(true)
            } else {
                None
            }
        });
        if let Some(forward) = next {
            let from = if forward {
                (*anchor).max(*head)
            } else {
                (*anchor).min(*head)
            };
            if let Some(found) = doc.find(find, from, forward) {
                *scroll_to = Some(doc.line_at(found.start));
                *anchor = found.start;
                *head = found.end;
            }
        }
    }
    let font = egui::FontId::monospace(14.);
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace).max(18.);
    let desired_width = (doc.max_chars as f32 * 14.).max(ui.available_width() - 12.);
    let mut scroll = egui::ScrollArea::both()
        .id_salt("viewer-text-scroll")
        .auto_shrink([false, false]);
    if let Some(row) = scroll_to.take() {
        scroll =
            scroll.vertical_scroll_offset(row as f32 * (row_height + ui.spacing().item_spacing.y));
    }
    scroll.show_rows(ui, row_height, doc.lines.len(), |ui, rows| {
        ui.set_min_width(desired_width);
        for row in rows {
            let range = doc.lines[row].clone();
            let text = &doc.text[range.clone()];
            let display = text.trim_end_matches(['\r', '\n']);
            let selected = selection(*anchor, *head);
            let start = selected
                .start
                .saturating_sub(range.start)
                .min(display.len());
            let end = selected.end.saturating_sub(range.start).min(display.len());
            let mut job = egui::text::LayoutJob::default();
            job.wrap.max_width = f32::INFINITY;
            let normal = egui::TextFormat {
                font_id: font.clone(),
                color: ui.visuals().text_color(),
                ..Default::default()
            };
            let mut highlight = normal.clone();
            highlight.background = ui.visuals().selection.bg_fill;
            highlight.color = ui.visuals().selection.stroke.color;
            if start < end && display.is_char_boundary(start) && display.is_char_boundary(end) {
                job.append(&display[..start], 0., normal.clone());
                job.append(&display[start..end], 0., highlight);
                job.append(&display[end..], 0., normal);
            } else {
                job.append(display, 0., normal);
            }
            let galley = ui.painter().layout_job(job);
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(desired_width, row_height),
                egui::Sense::click_and_drag(),
            );
            if *scroll_to == Some(row) {
                ui.scroll_to_rect(rect, Some(egui::Align::Center));
                *scroll_to = None;
            }
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                let index = galley.cursor_from_pos(pos - rect.min).index.0;
                let byte = display
                    .char_indices()
                    .nth(index)
                    .map_or(display.len(), |(i, _)| i)
                    + range.start;
                if response.drag_started() || response.clicked() {
                    if !ui.input(|i| i.modifiers.shift) {
                        let start = ui.input(|i| i.pointer.press_origin()).unwrap_or(pos);
                        let index = galley.cursor_from_pos(start - rect.min).index.0;
                        *anchor = display
                            .char_indices()
                            .nth(index)
                            .map_or(display.len(), |(i, _)| i)
                            + range.start;
                    }
                    *head = byte;
                    *dragging = response.drag_started();
                    response.request_focus();
                    ui.memory_mut(|m| m.request_focus(id));
                }
                if *dragging
                    && ui.input(|i| i.pointer.primary_down())
                    && pos.y >= rect.top()
                    && pos.y < rect.bottom()
                {
                    *head = byte;
                }
            }
            ui.painter()
                .galley(rect.min, galley, ui.visuals().text_color());
        }
    });
    if !ui.input(|i| i.pointer.primary_down()) {
        *dragging = false;
    }
}
impl MuCrabApp {
    pub(super) fn open_viewer(&mut self, ctx: &egui::Context) {
        let i = usize::from(self.active == PaneSide::Right);
        if let Some(location) = self.panes[i].remote().cloned() {
            if let Some(name) = self.panes[i]
                .selection()
                .snapshot(self.panes[i].entries(), self.panes[i].order())
                .focused
                .filter(|n| n != "..")
                && let Ok(child) = location.child(&name)
            {
                self.remote_open(child, true);
            }
            return;
        }
        let Some(target) = self.focused_target() else {
            return;
        };
        let i = usize::from(self.active == PaneSide::Right);
        let origin = (self.active, self.panes.stack(i).active().id);
        self.start_view_target(target, origin, ctx);
    }
    pub(super) fn view_prepared_remote(
        &mut self,
        path: PathBuf,
        side: PaneSide,
        tab: TabId,
        ctx: &egui::Context,
    ) {
        self.start_view_target(EntryTarget::Local(path), (side, tab), ctx);
    }
    fn start_view_target(
        &mut self,
        target: EntryTarget,
        origin: (PaneSide, TabId),
        ctx: &egui::Context,
    ) {
        let v = &mut self.viewer;
        v.visible = true;
        v.origin = Some(origin);
        v.target = Some(target.clone());
        v.loaded = None;
        v.texture = None;
        v.error = None;
        v.focus = true;
        v.fit = true;
        v.zoom = 1.;
        v.find.clear();
        v.cancel();
        let gpu = ctx.input(|i| i.max_texture_side.min(16384)) as u32;
        match ViewRequest::capture(target) {
            Ok(request) => {
                if let Err(e) = v.job.start(move |c| {
                    request
                        .prepare(&c, gpu)
                        .map(|p| (request, p))
                        .map_err(|e| e.to_string())
                }) {
                    v.error = Some(e);
                }
            }
            Err(e) => v.error = Some(e.to_string()),
        }
    }
    fn close_viewer(&mut self) {
        self.viewer.visible = false;
        self.viewer.cancel();
        self.viewer.loaded = None;
        self.viewer.texture = None;
        if let Some((side, id)) = self.viewer.origin {
            let i = usize::from(side == PaneSide::Right);
            if self.panes.stack_mut(i).select(id) {
                self.active = side;
            }
        }
        self.restore_copy_focus = true;
    }
    pub(super) fn viewer_window(&mut self, ctx: &egui::Context) {
        if !self.viewer.visible {
            return;
        }
        let mut visible = true;
        let mut external = false;
        let mut close = false;
        let v = &mut self.viewer;
        let window=egui::Window::new("View file").id(egui::Id::new("internal-viewer")).open(&mut visible).default_width(850.).default_height(560.).resizable(true).show(ctx,|ui|{
   let root_response=ui.interact(ui.available_rect_before_wrap(),egui::Id::new("viewer-root-focus"),egui::Sense::focusable_noninteractive());
   if v.focus&&!matches!(v.loaded.as_ref().map(|(_,p)|&p.content),Some(PreviewContent::Text(_))){root_response.request_focus();v.focus=false;}
   if let Some(target)=&v.target{ui.label(target.display_path().display().to_string());}
   ui.horizontal(|ui|{external=ui.add_enabled(v.target.is_some()&&!self.opener.busy(),egui::Button::new("Open externally")).clicked();close=ui.button("Close viewer (Esc)").clicked();});
   if let Some(progress)=v.job.progress(){ui.spinner();ui.label(format!("{:?} — {} bytes read",progress.state,progress.transferred_bytes));}
   if let Some(error)=&v.error{ui.colored_label(ui.visuals().error_fg_color,error);}
   if let Some((_,preview))=&v.loaded {
    ui.small(format!("Resolved preview: {}",preview.resolved_path.display()));
    match &preview.content {
     PreviewContent::Text(doc)=>{
      ui.horizontal(|ui|{ui.label(format!("{} · {} lines",doc.encoding,doc.lines.len()));ui.label("Find");let response=ui.add(egui::TextEdit::singleline(&mut v.find).desired_width(180.));let mut forward=None;if ui.button("Previous").clicked(){forward=Some(false);}
 if ui.button("Next").clicked()||(response.lost_focus()&&ui.input(|i|i.key_pressed(egui::Key::Enter))){forward=Some(true);}
       if let Some(forward)=forward{let from=if forward{v.cursor.anchor.max(v.cursor.head)}else{v.cursor.anchor.min(v.cursor.head)};if let Some(found)=doc.find(&v.find,from,forward){v.cursor.scroll_to=Some(doc.line_at(found.start));v.cursor.anchor=found.start;v.cursor.head=found.end;v.focus=true;}}
       if ui.button("Select all").clicked(){v.cursor.anchor=0;v.cursor.head=doc.text.len();v.focus=true;}
       if ui.add_enabled(v.cursor.anchor!=v.cursor.head,egui::Button::new("Copy text")).clicked(){ui.ctx().copy_text(copy_selection(doc,v.cursor.anchor,v.cursor.head));}
      });
      ui.small("Read-only · drag or Shift-click to select · Ctrl/Cmd+A/C · F3 / Shift+F3 find next/previous");ui.separator();
      text_canvas(ui,doc,&v.find,&mut v.cursor,&mut v.focus);
     }
     PreviewContent::Image(image)=>{
      ui.horizontal(|ui|{ui.label(format!("{} × {} · {}",image.width,image.height,image.orientation));if ui.button("Fit").clicked(){v.fit=true;}
 if ui.button("100%").clicked(){v.fit=false;v.zoom=1.;}
 if ui.button("−").clicked(){v.fit=false;v.zoom=(v.zoom/1.25).clamp(0.05,16.);}
 if ui.button("+").clicked(){v.fit=false;v.zoom=(v.zoom*1.25).clamp(0.05,16.);}ui.label(format!("{:.0}% · drag to pan",v.zoom*100.));});
      let available=ui.available_size().max(egui::vec2(1.,1.));if v.fit{v.zoom=(available.x/image.width as f32).min(available.y/image.height as f32).clamp(0.0001,16.);}
      if let Some(texture)=&v.texture {
       egui::ScrollArea::both().id_salt("viewer-image-scroll").auto_shrink([false,false]).show(ui,|ui|{
        let size=egui::vec2(image.width as f32*v.zoom,image.height as f32*v.zoom);let (rect,response)=ui.allocate_exact_size(size,egui::Sense::drag());let clip=rect.intersect(ui.clip_rect());
        let checker=12.;let min_x=((clip.left()-rect.left())/checker).floor() as i32;let max_x=((clip.right()-rect.left())/checker).ceil() as i32;let min_y=((clip.top()-rect.top())/checker).floor() as i32;let max_y=((clip.bottom()-rect.top())/checker).ceil() as i32;
        for y in min_y..max_y{for x in min_x..max_x{let cell=egui::Rect::from_min_size(rect.min+egui::vec2(x as f32*checker,y as f32*checker),egui::vec2(checker,checker));let shade=if(x+y)%2==0{150}else{210};ui.painter().rect_filled(cell.intersect(rect),0.,egui::Color32::from_gray(shade));}}
        ui.painter().image(texture.id(),rect,egui::Rect::from_min_max(egui::pos2(0.,0.),egui::pos2(1.,1.)),egui::Color32::WHITE);
        if response.dragged(){ui.scroll_with_delta(ui.input(|i|i.pointer.delta()));}
       });
      }
     }
    }
   }
  });
        // Only the viewer's own layer consumes Escape; another properties/search
        // window or the file listings keep their keyboard ownership.
        let owns_focus = ctx
            .memory(|m| m.focused())
            .and_then(|id| ctx.read_response(id))
            .is_some_and(|r| {
                window
                    .as_ref()
                    .is_some_and(|w| r.layer_id == w.response.layer_id)
            });
        let current_focus = ctx.memory(|m| m.focused());
        let owns_focus = owns_focus
            || window
                .as_ref()
                .is_some_and(|w| w.response.contains_pointer());
        let escape_owner = owns_focus || (current_focus.is_none() && self.viewer.owned_focus);
        self.viewer.owned_focus = owns_focus;
        if escape_owner
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            close = true;
        }
        if external {
            let valid = self
                .viewer
                .loaded
                .as_ref()
                .map(|(r, _)| r.validate())
                .transpose();
            if let Err(e) = valid {
                self.viewer.error = Some(format!("Source changed: {e}"));
            } else if let Some(target) = &self.viewer.target {
                self.opener.start(match target {
                    EntryTarget::Local(p) => crate::opening::Target::Local(p.clone()),
                    EntryTarget::Zip { location, name } => {
                        crate::opening::Target::Archived(location.clone(), name.clone())
                    }
                });
            }
        }
        if !visible || close {
            self.close_viewer();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn text_canvas_virtualizes_large_documents_and_copies_exact_decoded_selection() {
        let doc = TextDocument::decode(("café 中\r\n".repeat(50000)).as_bytes()).unwrap();
        let ctx = egui::Context::default();
        let mut cursor = TextCursor::default();
        let mut focus = true;
        let output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800., 500.),
                )),
                ..Default::default()
            },
            |ui| text_canvas(ui, &doc, "", &mut cursor, &mut focus),
        );
        assert!(
            output.shapes.len() < 150,
            "only visible rows should be laid out"
        );
        output.drop_without_applying_deltas();
        assert!(ctx.memory(|m| m.has_focus(text_id())));
        cursor.anchor = 0;
        cursor.head = "café 中\r\n".len();
        let output = ctx.run_ui(
            egui::RawInput {
                events: vec![egui::Event::Copy],
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800., 500.),
                )),
                ..Default::default()
            },
            |ui| text_canvas(ui, &doc, "", &mut cursor, &mut focus),
        );
        assert!(
            output
                .platform_output
                .commands
                .iter()
                .any(|c| matches!(c,egui::OutputCommand::CopyText(t)if t=="café 中\r\n"))
        );
        output.drop_without_applying_deltas();
        cursor.scroll_to = Some(40000);
        let output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800., 500.),
                )),
                ..Default::default()
            },
            |ui| text_canvas(ui, &doc, "", &mut cursor, &mut focus),
        );
        assert!(cursor.scroll_to.is_none());
        assert!(output.shapes.len() < 150);
        output.drop_without_applying_deltas();
    }
}
