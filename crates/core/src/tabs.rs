// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
use crate::{BrowserPane, NameSorter, PaneLocation};
use std::{
    io,
    ops::{Index, IndexMut},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct TabId(u64);
impl TabId {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        loop {
            let current = NEXT.load(Ordering::Relaxed);
            let next = current.checked_add(1).expect("tab IDs exhausted");
            if NEXT
                .compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                return Self(current);
            }
        }
    }
}
pub struct BrowserTab {
    pub id: TabId,
    pub pane: BrowserPane,
    pub widths: [f32; 3],
}
impl BrowserTab {
    fn new(pane: BrowserPane) -> Self {
        Self {
            id: TabId::new(),
            pane,
            widths: [65.0, 84.0, 145.0],
        }
    }
}
pub struct TabStack {
    tabs: Vec<BrowserTab>,
    active: usize,
}
impl TabStack {
    pub fn new(pane: BrowserPane) -> Self {
        Self {
            tabs: vec![BrowserTab::new(pane)],
            active: 0,
        }
    }
    pub fn tabs(&self) -> &[BrowserTab] {
        &self.tabs
    }
    pub fn active(&self) -> &BrowserTab {
        &self.tabs[self.active]
    }
    pub fn active_mut(&mut self) -> &mut BrowserTab {
        &mut self.tabs[self.active]
    }
    pub fn len(&self) -> usize {
        self.tabs.len()
    }
    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }
    pub fn select(&mut self, id: TabId) -> bool {
        if let Some(i) = self.tabs.iter().position(|t| t.id == id) {
            self.active_mut().pane.quick_find = None;
            self.active = i;
            self.active_mut().pane.quick_find = None;
            true
        } else {
            false
        }
    }
    pub fn cycle(&mut self, forward: bool) {
        let len = self.tabs.len();
        let i = if forward {
            (self.active + 1) % len
        } else {
            (self.active + len - 1) % len
        };
        self.select(self.tabs[i].id);
    }
    pub fn close(&mut self, id: TabId) -> bool {
        if self.tabs.len() == 1 {
            return false;
        }
        if let Some(i) = self.tabs.iter().position(|t| t.id == id) {
            self.tabs.remove(i);
            if i < self.active {
                self.active -= 1;
            } else if i == self.active {
                self.active = self.active.min(self.tabs.len() - 1);
                self.active_mut().pane.quick_find = None;
            }
            true
        } else {
            false
        }
    }
    pub fn add_home(
        &mut self,
        home: &Path,
        hidden: bool,
        sorter: &NameSorter,
    ) -> io::Result<TabId> {
        self.check_limit()?;
        let mut pane = BrowserPane {
            show_hidden: hidden,
            ..Default::default()
        };
        if !pane.open(home, sorter) {
            return Err(io::Error::other(
                pane.error.unwrap_or_else(|| "Cannot open Home".into()),
            ));
        }
        Ok(self.push(BrowserTab::new(pane)))
    }
    pub fn duplicate(&mut self, sorter: &NameSorter) -> io::Result<TabId> {
        self.check_limit()?;
        let source = self.active();
        let location = source
            .pane
            .current_location()
            .ok_or_else(|| io::Error::other("No successful tab location"))?;
        let mut pane = BrowserPane {
            show_hidden: source.pane.show_hidden,
            options: source.pane.options,
            ..Default::default()
        };
        if !pane.open_location(&location, sorter) {
            return Err(io::Error::other(
                pane.error
                    .unwrap_or_else(|| "Cannot duplicate location".into()),
            ));
        }
        let snapshot = source
            .pane
            .selection
            .snapshot(&source.pane.entries, &source.pane.order);
        if let Some(name) = snapshot.focused {
            pane.focus_name(&name);
        }
        pane.history = source.pane.history.clone();
        let mut tab = BrowserTab::new(pane);
        tab.widths = source.widths;
        Ok(self.push(tab))
    }
    fn check_limit(&self) -> io::Result<()> {
        if self.len() >= 32 {
            Err(io::Error::other("Maximum 32 tabs per pane"))
        } else {
            Ok(())
        }
    }
    fn push(&mut self, tab: BrowserTab) -> TabId {
        self.active_mut().pane.quick_find = None;
        let id = tab.id;
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
        id
    }
}
pub struct TabbedPanes {
    stacks: [TabStack; 2],
}
impl From<[BrowserPane; 2]> for TabbedPanes {
    fn from([left, right]: [BrowserPane; 2]) -> Self {
        Self {
            stacks: [TabStack::new(left), TabStack::new(right)],
        }
    }
}
impl Index<usize> for TabbedPanes {
    type Output = BrowserPane;
    fn index(&self, i: usize) -> &BrowserPane {
        &self.stacks[i].active().pane
    }
}
impl IndexMut<usize> for TabbedPanes {
    fn index_mut(&mut self, i: usize) -> &mut BrowserPane {
        &mut self.stacks[i].active_mut().pane
    }
}
impl TabbedPanes {
    pub fn stack(&self, i: usize) -> &TabStack {
        &self.stacks[i]
    }
    pub fn stack_mut(&mut self, i: usize) -> &mut TabStack {
        &mut self.stacks[i]
    }
    pub fn iter(&self) -> impl Iterator<Item = &BrowserPane> {
        self.stacks.iter().map(|s| &s.active().pane)
    }
    pub fn all(&self) -> impl Iterator<Item = &BrowserTab> {
        self.stacks.iter().flat_map(|s| s.tabs.iter())
    }
    pub fn all_mut(&mut self) -> impl Iterator<Item = &mut BrowserTab> {
        self.stacks.iter_mut().flat_map(|s| s.tabs.iter_mut())
    }
    pub fn find_mut(&mut self, id: TabId) -> Option<&mut BrowserPane> {
        self.all_mut().find(|t| t.id == id).map(|t| &mut t.pane)
    }
    pub fn contains(&self, id: TabId) -> bool {
        self.all().any(|t| t.id == id)
    }
}
impl BrowserPane {
    pub fn open_location(&mut self, location: &PaneLocation, sorter: &NameSorter) -> bool {
        if !self.load_location(location, sorter) {
            return false;
        }
        self.quick_find = None;
        self.record_location();
        true
    }
    pub(crate) fn load_location(&mut self, location: &PaneLocation, sorter: &NameSorter) -> bool {
        match location {
            PaneLocation::Local(path) => self.open_unrecorded(path, sorter),
            PaneLocation::Zip(location) => match crate::ZipCatalog::open(&location.archive)
                .and_then(|c| c.list(&location.prefix))
            {
                Ok(entries) => {
                    let path = location.display_path();
                    let parent = path.parent().is_some();
                    self.load_listing(path, entries, parent, sorter);
                    self.archive = Some(location.clone());
                    true
                }
                Err(error) => {
                    self.error = Some(error.to_string());
                    false
                }
            },
        }
    }
}
