// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
//! Bounded nonempty two-pane tab ownership with stable operation-origin identities.
use crate::{BrowserPane, NameSorter, PaneLocation};
use std::{
    io,
    ops::{Index, IndexMut},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
/// Stable process-local tab identity, independent of stack position or path.
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
#[derive(Debug)]
/// One independently owned browsing context and its resizable metadata-column widths.
pub struct BrowserTab {
    /// Stable identity used instead of a mutable display position.
    pub id: TabId,
    /// Independently owned browsing context for this tab.
    pub pane: BrowserPane,
    /// Per-tab extension, size and modified-time column widths in logical points.
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
#[derive(Debug)]
/// A bounded nonempty ordered collection of independently identifiable browsing tabs.
pub struct TabStack {
    tabs: Vec<BrowserTab>,
    active: usize,
}
impl TabStack {
    /// Create a nonempty tab stack with one stable tab identity.
    pub fn new(pane: BrowserPane) -> Self {
        Self {
            tabs: vec![BrowserTab::new(pane)],
            active: 0,
        }
    }
    /// Mutable tab access for scheduling owned background requests.
    pub fn tabs_mut(&mut self) -> &mut [BrowserTab] {
        &mut self.tabs
    }
    /// Borrow tabs in display order without granting stack removal.
    pub fn tabs(&self) -> &[BrowserTab] {
        &self.tabs
    }
    /// Borrow the currently selected tab; the stack always retains one tab.
    pub fn active(&self) -> &BrowserTab {
        &self.tabs[self.active]
    }
    /// Mutably borrow the selected tab while retaining stack ownership.
    pub fn active_mut(&mut self) -> &mut BrowserTab {
        &mut self.tabs[self.active]
    }
    /// Count live tabs in this pane, bounded to thirty-two.
    pub fn len(&self) -> usize {
        self.tabs.len()
    }
    /// Report whether no tabs exist; normal stack operations retain at least one.
    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }
    /// Select a surviving stable tab identity and clear stale quick-find state.
    pub fn select(&mut self, id: TabId) -> bool {
        if let Some(i) = self.tabs.iter().position(|t| t.id == id) {
            self.active_mut().pane.quick_find = None;
            self.active = i;
            self.active_mut().pane.quick_find = None;
            if self.active().pane.browsing_deferred()
                && let Some(location) = self.active().pane.current_location()
            {
                self.active_mut()
                    .pane
                    .queue_location(&location, false, None);
            }
            true
        } else {
            false
        }
    }
    /// Select the next tab when `forward` is true, otherwise the previous tab.
    pub fn cycle(&mut self, forward: bool) {
        let len = self.tabs.len();
        let i = if forward {
            (self.active + 1) % len
        } else {
            (self.active + len - 1) % len
        };
        self.select(self.tabs[i].id);
    }
    /// Remove a matching tab while retaining at least one tab and a valid active index.
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
    /// Create a home-location tab, respecting the tab limit and browsing mode.
    ///
    /// # Errors
    /// Returns an error when the tab limit is reached or the requested location cannot be captured.
    pub fn add_home(
        &mut self,
        home: &Path,
        hidden: bool,
        sorter: &NameSorter,
    ) -> io::Result<TabId> {
        self.check_limit()?;
        let mut pane = BrowserPane {
            deferred_browsing: self.active().pane.deferred_browsing,
            show_hidden: hidden,
            ..Default::default()
        };
        if !pane.open(home, sorter) && !pane.deferred_browsing {
            return Err(io::Error::other(
                pane.error.unwrap_or_else(|| "Cannot open Home".into()),
            ));
        }
        Ok(self.push(BrowserTab::new(pane)))
    }
    /// Copy the active navigation context while giving the new tab independent marks and identity.
    ///
    /// # Errors
    /// Returns an error when the tab limit is reached or the requested location cannot be captured.
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
        if source.pane.deferred_browsing {
            pane.defer_browsing();
            if let Some(location) = source.pane.current_location() {
                pane.commit_location(location);
            }
            pane.remote_disconnected = source.pane.remote_disconnected;
            pane.path_input = source.pane.path_input.clone();
            pane.entries = source.pane.entries.clone();
            pane.order = source.pane.order.clone();
            pane.selection =
                crate::FileSelection::new(pane.entries.len(), source.pane.selection.has_parent);
            pane.refresh(sorter);
        } else if let PaneLocation::Sftp(loc) = &location {
            pane.request_remote(loc.clone(), false, None);
            let request = pane.remote_request.clone().expect("requested");
            pane.remote_loaded(&request, source.pane.entries.clone(), sorter);
            pane.remote_disconnected = source.pane.remote_disconnected;
        } else if !pane.open_location(&location, sorter) {
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
#[derive(Debug)]
/// Two tab stacks with fixed left/right ownership and stable origin lookup.
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
    /// Borrow one pane stack by its fixed left/right index.
    ///
    /// # Panics
    /// Panics for an index other than zero or one.
    pub fn stack(&self, i: usize) -> &TabStack {
        &self.stacks[i]
    }
    /// Mutably borrow one pane stack by its fixed left/right index.
    ///
    /// # Panics
    /// Panics for an index other than zero or one.
    pub fn stack_mut(&mut self, i: usize) -> &mut TabStack {
        &mut self.stacks[i]
    }
    /// Borrow the two active panes in left-to-right order.
    pub fn iter(&self) -> impl Iterator<Item = &BrowserPane> {
        self.stacks.iter().map(|s| &s.active().pane)
    }
    /// Borrow all tabs, including inactive tabs, in pane/display order.
    pub fn all(&self) -> impl Iterator<Item = &BrowserTab> {
        self.stacks.iter().flat_map(|s| s.tabs.iter())
    }
    /// Mutably visit all tabs without changing their owning stacks.
    pub fn all_mut(&mut self) -> impl Iterator<Item = &mut BrowserTab> {
        self.stacks.iter_mut().flat_map(|s| s.tabs.iter_mut())
    }
    /// Find the pane owned by a surviving stable tab identity.
    pub fn find_mut(&mut self, id: TabId) -> Option<&mut BrowserPane> {
        self.all_mut().find(|t| t.id == id).map(|t| &mut t.pane)
    }
    /// Test whether either pane stack still owns this tab identity.
    pub fn contains(&self, id: TabId) -> bool {
        self.all().any(|t| t.id == id)
    }
}
impl BrowserPane {
    /// Navigate to a structured local, ZIP or saved-profile remote location.
    pub fn open_location(&mut self, location: &PaneLocation, sorter: &NameSorter) -> bool {
        if let PaneLocation::Sftp(loc) = location {
            self.request_remote(loc.clone(), true, None);
            return true;
        }
        if self.deferred_browsing {
            self.queue_location(location, true, None);
            return true;
        }
        if !self.load_location(location, sorter) {
            return false;
        }
        self.quick_find = None;
        self.record_location();
        true
    }
    pub(crate) fn load_location(&mut self, location: &PaneLocation, sorter: &NameSorter) -> bool {
        match location {
            PaneLocation::Sftp(loc) => {
                self.request_remote(loc.clone(), true, None);
                false
            }
            PaneLocation::Local(path) => self.open_unrecorded(path, sorter),
            PaneLocation::Zip(location) => match crate::ZipCatalog::open(&location.archive)
                .and_then(|c| c.list(&location.prefix))
            {
                Ok(entries) => {
                    let path = location.display_path();
                    let parent = path.parent().is_some();
                    self.load_listing(path, entries, parent, sorter);
                    self.commit_location(PaneLocation::Zip(location.clone()));
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
