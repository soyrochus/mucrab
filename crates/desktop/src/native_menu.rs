// SPDX-License-Identifier: GPL-3.0-only
// macOS presentation of the existing application commands. All AppKit objects
// stay on the UI thread; the event callback only queues an ID and wakes eframe.
use super::{
    Appearance, Command, EverydayAction, FileAction, MuCrabApp, NavigationAction, PaneEvent,
    PaneSide, egui, listing_id, mpsc,
};
use mucrab_core::{SortCriterion, SortMode};
use muda::{
    CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, MenuItemKind, PredefinedMenuItem, Submenu,
    accelerator::{Accelerator, Code, Modifiers},
};

#[derive(Clone, Copy)]
pub(super) enum Action {
    Hidden,
    FoldersFirst,
    Ascending,
    Sort(SortCriterion),
    Ordering(SortMode),
    Appearance(Appearance),
    SplitEqually,
    Help(bool),
    Close,
    Cut,
    Copy,
    Paste,
    SelectAll,
}

#[derive(Clone, Copy)]
enum Gate {
    Always,
    Browse,
    Copy,
    Move,
    Rename,
    Batch,
    Extract,
    Local,
    SelectedLocal,
    View,
    Properties,
    Back,
    Forward,
    Search,
    AddBookmark,
    NewTab,
    CloseTab,
    EditCopy,
    EditCut,
    EditPaste,
    SelectAll,
}

struct State {
    browsing: bool,
    idle: bool,
    local: bool,
    selected: bool,
    listing: bool,
    text: bool,
    viewer: bool,
    copy: bool,
    move_: bool,
    rename: bool,
    extract: bool,
    view: bool,
    properties: bool,
    back: bool,
    forward: bool,
    remote: bool,
    tabs: usize,
    bookmarks_available: bool,
    hidden: bool,
    folders_first: bool,
    ascending: bool,
    criterion: SortCriterion,
    ordering: SortMode,
    appearance: Appearance,
    bookmarks: Vec<(u64, String)>,
}

impl State {
    fn capture(app: &MuCrabApp, ctx: &egui::Context) -> Self {
        let index = usize::from(app.active == PaneSide::Right);
        let pane = &app.panes[index];
        let browsing = !app.close_after_job
            && !app.remote.modal()
            && app.properties.review.is_none()
            && !app.navigation.modal()
            && app.everyday.review.is_none()
            && app.confirmations.copy.is_none()
            && app.confirmations.batch.is_none()
            && app.confirmations.zip.is_none()
            && !app.show_copy_details
            && app.help.is_none();
        let idle = browsing && !app.everyday.busy() && app.copy_job.is_none();
        let focused = ctx.memory(|m| m.focused());
        Self {
            browsing,
            idle,
            local: pane.location().is_some() && pane.archive().is_none() && pane.remote().is_none(),
            selected: !pane
                .selection()
                .snapshot(pane.entries(), pane.order())
                .selected
                .is_empty(),
            listing: focused == Some(listing_id(app.active)),
            text: focused.is_some_and(|id| egui::text_edit::TextEditState::load(ctx, id).is_some()),
            viewer: focused == Some(egui::Id::new("viewer-text-canvas")),
            copy: idle
                && (app.prepare_action(FileAction::Copy).is_some()
                    || app.can_remote_copy()
                    || (pane.archive().is_some() && app.can_extract())),
            move_: idle && app.prepare_action(FileAction::Move).is_some(),
            rename: idle && app.prepare_action(FileAction::Rename).is_some(),
            extract: idle && app.can_extract(),
            view: browsing && (app.focused_target().is_some() || pane.remote().is_some()),
            properties: browsing && app.focused_target().is_some() && pane.remote().is_none(),
            back: pane.can_back(),
            forward: pane.can_forward(),
            remote: pane.remote().is_some(),
            tabs: app.panes.stack(index).len(),
            bookmarks_available: app.navigation.store.is_some(),
            hidden: app.show_hidden,
            folders_first: pane.options.directories_first,
            ascending: pane.options.ascending,
            criterion: pane.options.criterion,
            ordering: pane.options.mode,
            appearance: app.appearance,
            bookmarks: app
                .navigation
                .store
                .as_ref()
                .map(|s| {
                    s.entries()
                        .iter()
                        .map(|b| (b.id, b.label.clone()))
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    fn enabled(&self, gate: Gate) -> bool {
        match gate {
            Gate::Always => true,
            Gate::Browse => self.browsing,
            Gate::Copy => self.copy,
            Gate::Move => self.move_,
            Gate::Rename => self.rename,
            Gate::Batch | Gate::SelectedLocal => self.idle && self.local && self.selected,
            Gate::Extract => self.extract,
            Gate::Local => self.idle && self.local,
            Gate::View => self.view,
            Gate::Properties => self.properties,
            Gate::Back => self.browsing && self.back,
            Gate::Forward => self.browsing && self.forward,
            Gate::Search => self.browsing && !self.remote,
            Gate::AddBookmark => self.browsing && self.bookmarks_available,
            Gate::NewTab => self.browsing && self.tabs < 32,
            Gate::CloseTab => self.browsing && self.tabs > 1,
            Gate::EditCopy => {
                (self.listing && self.idle && self.local && self.selected)
                    || self.text
                    || self.viewer
            }
            Gate::EditCut => self.text,
            Gate::EditPaste => (self.listing && self.idle && self.local) || self.text,
            Gate::SelectAll => (self.listing && self.browsing) || self.text || self.viewer,
        }
    }

    fn checked(&self, command: &Command) -> Option<bool> {
        match command {
            Command::Native(Action::Hidden) => Some(self.hidden),
            Command::Native(Action::FoldersFirst) => Some(self.folders_first),
            Command::Native(Action::Ascending) => Some(self.ascending),
            Command::Native(Action::Sort(value)) => Some(self.criterion == *value),
            Command::Native(Action::Ordering(value)) => Some(self.ordering == *value),
            Command::Native(Action::Appearance(value)) => Some(self.appearance == *value),
            _ => None,
        }
    }
}

struct Entry {
    item: MenuItemKind,
    command: Command,
    gate: Gate,
    bookmark: bool,
    listing_accelerator: Option<Accelerator>,
    accelerator_active: bool,
}

pub(super) struct NativeMenu {
    menu: Menu,
    bookmarks: Submenu,
    entries: Vec<Entry>,
    saved_bookmarks: Vec<(u64, String)>,
    events: mpsc::Receiver<MenuId>,
}

fn shortcut(modifiers: Modifiers, key: Code) -> Option<Accelerator> {
    Some(Accelerator::new(modifiers, key))
}

impl NativeMenu {
    pub(super) fn new(ctx: &egui::Context) -> muda::Result<Self> {
        let menu = Menu::new();
        let bookmarks = Submenu::new("Bookmarks", true);
        let (send, events) = mpsc::channel();
        let mut bar = Self {
            menu,
            bookmarks,
            entries: vec![],
            saved_bookmarks: vec![],
            events,
        };
        let application = Submenu::new("muCrab", true);
        bar.add(
            &application,
            "About muCrab",
            Command::Native(Action::Help(true)),
            Gate::Browse,
            None,
            false,
        )?;
        application.append(&PredefinedMenuItem::separator())?;
        application.append(&PredefinedMenuItem::services(None))?;
        application.append(&PredefinedMenuItem::separator())?;
        application.append(&PredefinedMenuItem::hide(Some("Hide muCrab")))?;
        application.append(&PredefinedMenuItem::hide_others(None))?;
        application.append(&PredefinedMenuItem::show_all(None))?;
        application.append(&PredefinedMenuItem::separator())?;
        bar.add(
            &application,
            "Quit muCrab",
            Command::Native(Action::Close),
            Gate::Always,
            shortcut(Modifiers::META, Code::KeyQ),
            false,
        )?;
        bar.menu.append(&application)?;

        let file = Submenu::new("File", true);
        bar.add(&file, "Open", Command::Open, Gate::Browse, None, false)?;
        for (label, action, gate, key) in [
            ("Copy…", FileAction::Copy, Gate::Copy, Code::F5),
            ("Move…", FileAction::Move, Gate::Move, Code::F6),
            ("Rename…", FileAction::Rename, Gate::Rename, Code::F2),
        ] {
            bar.add(
                &file,
                label,
                Command::Action(action),
                gate,
                shortcut(Modifiers::empty(), key),
                true,
            )?;
        }
        bar.add(
            &file,
            "Batch rename…",
            Command::Batch,
            Gate::Batch,
            None,
            false,
        )?;
        bar.add(
            &file,
            "Extract ZIP…",
            Command::Extract,
            Gate::Extract,
            None,
            false,
        )?;
        file.append(&PredefinedMenuItem::separator())?;
        for (label, action, gate, key, modifiers) in [
            (
                "New Folder…",
                EverydayAction::NewFolder,
                Gate::Local,
                Code::F7,
                Modifiers::empty(),
            ),
            (
                "Move to Trash…",
                EverydayAction::Trash,
                Gate::SelectedLocal,
                Code::F8,
                Modifiers::empty(),
            ),
            (
                "Permanently delete…",
                EverydayAction::Delete,
                Gate::SelectedLocal,
                Code::F8,
                Modifiers::SHIFT,
            ),
        ] {
            bar.add(
                &file,
                label,
                Command::Everyday(action),
                gate,
                shortcut(modifiers, key),
                true,
            )?;
        }
        bar.add(
            &file,
            "Copy files to clipboard",
            Command::Everyday(EverydayAction::CopyClipboard),
            Gate::SelectedLocal,
            None,
            false,
        )?;
        bar.add(
            &file,
            "Paste files…",
            Command::Everyday(EverydayAction::Paste),
            Gate::Local,
            None,
            false,
        )?;
        file.append(&PredefinedMenuItem::separator())?;
        bar.add(
            &file,
            "View file…",
            Command::View,
            Gate::View,
            shortcut(Modifiers::empty(), Code::F3),
            true,
        )?;
        bar.add(
            &file,
            "Properties…",
            Command::Properties,
            Gate::Properties,
            shortcut(Modifiers::ALT, Code::Enter),
            true,
        )?;
        bar.add(
            &file,
            "Close window",
            Command::Native(Action::Close),
            Gate::Always,
            shortcut(Modifiers::META | Modifiers::SHIFT, Code::KeyW),
            false,
        )?;
        bar.menu.append(&file)?;

        // Route through egui's clipboard requests, not AppKit responder selectors:
        // the text controls and file listing are egui widgets, not NSTextViews.
        let edit = Submenu::new("Edit", true);
        for (label, action, gate, key) in [
            ("Cut", Action::Cut, Gate::EditCut, Code::KeyX),
            ("Copy", Action::Copy, Gate::EditCopy, Code::KeyC),
            ("Paste", Action::Paste, Gate::EditPaste, Code::KeyV),
            ("Select All", Action::SelectAll, Gate::SelectAll, Code::KeyA),
        ] {
            bar.add(
                &edit,
                label,
                Command::Native(action),
                gate,
                shortcut(Modifiers::META, key),
                false,
            )?;
        }
        bar.menu.append(&edit)?;

        let mark = Submenu::new("Mark", true);
        for (label, event) in [
            ("Mark all", PaneEvent::All),
            ("Unmark all", PaneEvent::None),
            ("Invert selection", PaneEvent::Invert),
        ] {
            bar.add(
                &mark,
                label,
                Command::Mark(event),
                Gate::Browse,
                None,
                false,
            )?;
        }
        bar.menu.append(&mark)?;

        let view = Submenu::new("View", true);
        for (label, action) in [
            ("Show hidden files", Action::Hidden),
            ("Folders first", Action::FoldersFirst),
            ("Ascending order", Action::Ascending),
        ] {
            bar.check(&view, label, action, Gate::Browse)?;
        }
        let sort = Submenu::new("Sort by", true);
        for (criterion, label) in [
            (SortCriterion::Name, "Name"),
            (SortCriterion::Extension, "Extension"),
            (SortCriterion::Size, "Size"),
            (SortCriterion::Date, "Modified"),
        ] {
            bar.check(&sort, label, Action::Sort(criterion), Gate::Browse)?;
        }
        view.append(&sort)?;
        let ordering = Submenu::new("Name ordering", true);
        for (mode, label) in [
            (SortMode::Natural, "Natural"),
            (SortMode::Lexicographic, "Lexicographic"),
        ] {
            bar.check(&ordering, label, Action::Ordering(mode), Gate::Browse)?;
        }
        view.append(&ordering)?;
        let appearance = Submenu::new("Appearance", true);
        for (value, label) in [
            (Appearance::System, "System"),
            (Appearance::Light, "Light"),
            (Appearance::Dark, "Dark"),
        ] {
            bar.check(&appearance, label, Action::Appearance(value), Gate::Always)?;
        }
        view.append(&appearance)?;
        bar.add(
            &view,
            "Find by name…",
            Command::Everyday(EverydayAction::Find),
            Gate::Browse,
            None,
            false,
        )?;
        bar.menu.append(&view)?;

        let go = Submenu::new("Go", true);
        for (label, action, gate, key) in [
            ("Back", EverydayAction::Back, Gate::Back, Code::ArrowLeft),
            (
                "Forward",
                EverydayAction::Forward,
                Gate::Forward,
                Code::ArrowRight,
            ),
        ] {
            bar.add(
                &go,
                label,
                Command::Everyday(action),
                gate,
                shortcut(Modifiers::ALT, key),
                true,
            )?;
        }
        for (label, command) in [
            ("Parent folder", Command::Parent),
            ("Home folder", Command::Home),
            ("Refresh", Command::Refresh),
        ] {
            bar.add(&go, label, command, Gate::Browse, None, false)?;
        }
        bar.add(
            &go,
            "Search files…",
            Command::Search,
            Gate::Search,
            shortcut(Modifiers::META, Code::KeyF),
            true,
        )?;
        let bookmarks = bar.bookmarks.clone();
        bar.add(
            &bookmarks,
            "Add current location…",
            Command::Navigation(NavigationAction::AddBookmark),
            Gate::AddBookmark,
            shortcut(Modifiers::META, Code::KeyB),
            true,
        )?;
        bar.add(
            &bookmarks,
            "Manage bookmarks…",
            Command::Navigation(NavigationAction::ManageBookmarks),
            Gate::Browse,
            None,
            false,
        )?;
        bar.add(
            &bookmarks,
            "Reload bookmarks",
            Command::Navigation(NavigationAction::ReloadBookmarks),
            Gate::Browse,
            None,
            false,
        )?;
        bookmarks.append(&PredefinedMenuItem::separator())?;
        go.append(&bookmarks)?;
        bar.add(
            &go,
            "SFTP connections…",
            Command::Connections,
            Gate::Browse,
            None,
            false,
        )?;
        bar.menu.append(&go)?;

        let window = Submenu::new("Window", true);
        for (label, action, gate, key, modifiers) in [
            (
                "New Tab",
                NavigationAction::New,
                Gate::NewTab,
                Code::KeyT,
                Modifiers::META,
            ),
            (
                "Duplicate Tab",
                NavigationAction::Duplicate,
                Gate::NewTab,
                Code::KeyT,
                Modifiers::META | Modifiers::SHIFT,
            ),
            (
                "Close Tab",
                NavigationAction::Close,
                Gate::CloseTab,
                Code::KeyW,
                Modifiers::META,
            ),
            (
                "Next Tab",
                NavigationAction::Next,
                Gate::Browse,
                Code::Tab,
                Modifiers::CONTROL,
            ),
            (
                "Previous Tab",
                NavigationAction::Previous,
                Gate::Browse,
                Code::Tab,
                Modifiers::CONTROL | Modifiers::SHIFT,
            ),
        ] {
            bar.add(
                &window,
                label,
                Command::Navigation(action),
                gate,
                shortcut(modifiers, key),
                true,
            )?;
        }
        bar.add(
            &window,
            "Switch pane",
            Command::SwitchPane,
            Gate::Browse,
            None,
            false,
        )?;
        bar.add(
            &window,
            "Split equally",
            Command::Native(Action::SplitEqually),
            Gate::Browse,
            None,
            false,
        )?;
        window.append(&PredefinedMenuItem::separator())?;
        window.append(&PredefinedMenuItem::minimize(None))?;
        window.set_as_windows_menu_for_nsapp();
        bar.menu.append(&window)?;
        let help = Submenu::new("Help", true);
        bar.add(
            &help,
            "Keyboard shortcuts",
            Command::Native(Action::Help(false)),
            Gate::Browse,
            None,
            false,
        )?;
        help.set_as_help_menu_for_nsapp();
        bar.menu.append(&help)?;

        let ctx = ctx.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            if send.send(event.id).is_ok() {
                ctx.request_repaint();
            }
        }));
        bar.menu.init_for_nsapp();
        Ok(bar)
    }

    fn add(
        &mut self,
        menu: &Submenu,
        label: &str,
        command: Command,
        gate: Gate,
        accelerator: Option<Accelerator>,
        listing_only: bool,
    ) -> muda::Result<()> {
        let item = MenuItem::new(label, true, if listing_only { None } else { accelerator });
        menu.append(&item)?;
        self.entries.push(Entry {
            item: MenuItemKind::MenuItem(item),
            command,
            gate,
            bookmark: false,
            listing_accelerator: if listing_only { accelerator } else { None },
            accelerator_active: false,
        });
        Ok(())
    }

    fn check(
        &mut self,
        menu: &Submenu,
        label: &str,
        action: Action,
        gate: Gate,
    ) -> muda::Result<()> {
        let item = CheckMenuItem::new(label, true, false, None);
        menu.append(&item)?;
        self.entries.push(Entry {
            item: MenuItemKind::Check(item),
            command: Command::Native(action),
            gate,
            bookmark: false,
            listing_accelerator: None,
            accelerator_active: false,
        });
        Ok(())
    }

    fn sync(&mut self, state: &State) -> muda::Result<()> {
        if self.saved_bookmarks != state.bookmarks {
            while self.bookmarks.remove_at(4).is_some() {}
            self.entries.retain(|entry| !entry.bookmark);
            for (id, label) in &state.bookmarks {
                self.add(
                    &self.bookmarks.clone(),
                    label,
                    Command::Navigation(NavigationAction::OpenBookmark(*id)),
                    Gate::Browse,
                    None,
                    false,
                )?;
                self.entries.last_mut().expect("just inserted").bookmark = true;
            }
            self.saved_bookmarks = state.bookmarks.clone();
        }
        for entry in &mut self.entries {
            match &entry.item {
                MenuItemKind::MenuItem(item) => {
                    item.set_enabled(state.enabled(entry.gate));
                    if let Some(accelerator) = entry.listing_accelerator
                        && entry.accelerator_active != state.listing
                    {
                        item.set_accelerator(state.listing.then_some(accelerator))?;
                        entry.accelerator_active = state.listing;
                    }
                }
                MenuItemKind::Check(item) => {
                    item.set_enabled(state.enabled(entry.gate));
                    if let Some(checked) = state.checked(&entry.command) {
                        item.set_checked(checked);
                    }
                }
                _ => unreachable!("only actionable items are stored"),
            }
        }
        Ok(())
    }

    fn next(&self, state: &State, ctx: &egui::Context) -> Option<Command> {
        while let Ok(id) = self.events.try_recv() {
            if let Some(entry) = self.entries.iter().find(|e| e.item.id() == &id)
                && state.enabled(entry.gate)
            {
                // One command per pass: reevaluate availability before the next.
                ctx.request_repaint();
                return Some(entry.command.clone());
            }
        }
        None
    }
}

impl Drop for NativeMenu {
    fn drop(&mut self) {
        self.menu.remove_for_nsapp();
    }
}

impl MuCrabApp {
    pub(super) fn poll_native_menu(&mut self, ctx: &egui::Context) {
        let state = State::capture(self, ctx);
        let command = self
            .native_menu
            .as_ref()
            .and_then(|menu| menu.next(&state, ctx));
        if let Some(command) = command {
            self.execute(command, ctx);
        }
    }

    pub(super) fn sync_native_menu(&mut self, ctx: &egui::Context) {
        let state = State::capture(self, ctx);
        if let Some(menu) = &mut self.native_menu
            && let Err(error) = menu.sync(&state)
        {
            self.preference_error = Some(format!("Could not update native menu: {error}"));
        }
    }

    pub(super) fn native_menu_action(&mut self, action: Action, ctx: &egui::Context) {
        let index = usize::from(self.active == PaneSide::Right);
        match action {
            Action::Close => self.request_close(ctx),
            Action::Cut => ctx.send_viewport_cmd(egui::ViewportCommand::RequestCut),
            Action::Copy | Action::Paste => {
                if ctx.memory(|m| m.has_focus(listing_id(self.active))) {
                    self.everyday_action(
                        if matches!(action, Action::Copy) {
                            EverydayAction::CopyClipboard
                        } else {
                            EverydayAction::Paste
                        },
                        ctx,
                    );
                } else {
                    ctx.send_viewport_cmd(if matches!(action, Action::Copy) {
                        egui::ViewportCommand::RequestCopy
                    } else {
                        egui::ViewportCommand::RequestPaste
                    });
                }
            }
            Action::SelectAll => ctx.input_mut(|i| {
                i.events.push(egui::Event::Key {
                    key: egui::Key::A,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::COMMAND,
                })
            }),
            Action::Hidden => {
                self.show_hidden = !self.show_hidden;
                for tab in self.panes.all_mut() {
                    tab.pane.set_show_hidden(self.show_hidden, &self.sorter);
                }
            }
            Action::FoldersFirst | Action::Ascending | Action::Sort(_) | Action::Ordering(_) => {
                let pane = &mut self.panes[index];
                match action {
                    Action::FoldersFirst => {
                        pane.options.directories_first = !pane.options.directories_first
                    }
                    Action::Ascending => pane.options.ascending = !pane.options.ascending,
                    Action::Sort(value) => pane.options.criterion = value,
                    Action::Ordering(value) => pane.options.mode = value,
                    _ => unreachable!(),
                }
                pane.resort(&self.sorter);
            }
            Action::Appearance(value) => {
                self.appearance = value;
                value.apply(ctx);
                self.preference_error = value
                    .save()
                    .err()
                    .map(|e| format!("Could not save appearance: {e}"));
            }
            Action::SplitEqually => self.split = 0.5,
            Action::Help(about) => self.help = Some(about),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_routing_preserves_text_focus_and_rejects_remote_file_export() {
        let app = super::super::job_tests::app();
        let ctx = egui::Context::default();
        let mut state = State::capture(&app, &ctx);
        state.listing = true;
        state.local = true;
        state.selected = true;
        assert!(state.enabled(Gate::EditCopy));
        assert!(state.enabled(Gate::EditPaste));
        assert!(!state.enabled(Gate::EditCut));
        state.local = false;
        state.remote = true;
        assert!(!state.enabled(Gate::EditCopy));
        assert!(!state.enabled(Gate::EditPaste));
        state.listing = false;
        state.text = true;
        state.browsing = false; // Text editing remains usable inside a modal.
        assert!(state.enabled(Gate::EditCopy));
        assert!(state.enabled(Gate::EditPaste));
        assert!(state.enabled(Gate::EditCut));
        assert!(!state.enabled(Gate::SelectedLocal));
    }

    #[test]
    fn menus_revalidate_modal_and_tab_limits_but_allow_guarded_quit() {
        let mut app = super::super::job_tests::app();
        let ctx = egui::Context::default();
        assert!(!State::capture(&app, &ctx).enabled(Gate::CloseTab));
        app.help = Some(true);
        let state = State::capture(&app, &ctx);
        assert!(!state.enabled(Gate::Browse));
        assert!(!state.enabled(Gate::NewTab));
        assert!(!state.enabled(Gate::Copy));
        assert!(state.enabled(Gate::Always));
        let mut state = state;
        state.browsing = true;
        state.tabs = 32;
        assert!(!state.enabled(Gate::NewTab));
        assert!(state.enabled(Gate::CloseTab));
    }

    #[test]
    fn native_checkmarks_follow_pane_and_application_state() {
        let mut app = super::super::job_tests::app();
        let ctx = egui::Context::default();
        app.native_menu_action(Action::Sort(SortCriterion::Size), &ctx);
        app.native_menu_action(Action::Ascending, &ctx);
        app.native_menu_action(Action::Hidden, &ctx);
        let state = State::capture(&app, &ctx);
        assert_eq!(
            state.checked(&Command::Native(Action::Sort(SortCriterion::Size))),
            Some(true)
        );
        assert_eq!(
            state.checked(&Command::Native(Action::Sort(SortCriterion::Name))),
            Some(false)
        );
        assert_eq!(
            state.checked(&Command::Native(Action::Ascending)),
            Some(false)
        );
        assert_eq!(state.checked(&Command::Native(Action::Hidden)), Some(true));
        assert_eq!(app.panes[1].options.criterion, SortCriterion::Name);
        assert!(app.panes[1].options.ascending);
    }

    #[test]
    fn select_all_enters_the_existing_widget_input_path_without_stealing_focus() {
        let mut app = super::super::job_tests::app();
        let ctx = egui::Context::default();
        let field = egui::Id::new("native-menu-test-text");
        ctx.memory_mut(|m| m.request_focus(field));
        app.execute(Command::Native(Action::SelectAll), &ctx);
        assert!(ctx.memory(|m| m.has_focus(field)));
        assert!(ctx.input(|i| i.events.iter().any(|e| matches!(e,
            egui::Event::Key { key: egui::Key::A, pressed: true, modifiers, .. } if modifiers.command))));
    }
}
