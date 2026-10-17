# W10 — Everyday file management

## Why

muCrab supports browsing, copy/move/rename, ZIP extraction and native opening,
but common daily tasks still require another file manager. Users cannot create
a folder, delete or trash files, exchange file selections through the system
clipboard, find a name by typing, or revisit a pane's previous locations.

## What Changes

Implement the five priorities identified after W09, in independently verifiable
stages:

1. **New Folder:** create one named folder in the active local pane.
2. **Delete/Trash:** move selected local entries to the platform Trash, with a
   separately reviewed permanent-delete action.
3. **Clipboard Copy/Paste:** exchange local file selections with other desktop
   applications through native file clipboard formats and reuse reviewed copy.
4. **Quick-find:** type a filename prefix to focus matching visible entries
   without changing marks or filtering the listing.
5. **Navigation history:** independent, session-only Back/Forward history for
   each pane, covering local and ZIP locations.

Recover the original actions, selection rules and platform behavior from the
pinned muCommander binaries before implementation. Freeze fixtures and reserved
cases first. Record intentional differences explicitly; do not label native
integration or static recovery as Java/Rust runtime parity.

## Scope

Target the existing Linux and Apple Silicon macOS platforms. Reuse W02 selection,
W03 copy, W06 ZIP navigation and W07 job control. Preserve W08 appearance and W09
opening. Native platform behavior requires a separate receipt on each host.

This proposal covers the five priorities above, not the entire unported feature
list. New File, clipboard Cut, Trash restoration/emptying UI, recursive search,
tabs, bookmarks, persistent history, remote/cloud filesystems, extra archive
formats, viewers/editors and terminal integration remain out of scope. Archive
panes remain read-only; copying archive members to the system clipboard is deferred.

## Impact

- Add core folder-creation and deletion planning/execution, quick-find state and
  per-pane history, with explicit partial-result and error behavior.
- Add desktop adapters for Trash and native file clipboard exchange.
- Add menus, reviewed dialogs, navigation controls and focus-aware shortcuts.
- Extend the existing mutation worker; do not start parallel mutation jobs.
- Add original-binary recovery, frozen fixtures, comparison reports, isolated
  native replays and platform receipts under W10 evidence.
- Update user documentation and dependency notices when implementation lands.

This is a proposed change. No application behavior is implemented by these files.
