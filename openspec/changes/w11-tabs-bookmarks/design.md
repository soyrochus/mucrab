# W11 design

## Tab state

Use stable, non-reused TabId values and a collection per pane with one active ID.
Each tab owns BrowserPane, its history and its table/focus state. Retain at least
one tab per pane and cap each pane at 32 tabs; disable Close at one and show the
limit when New/Duplicate cannot be accepted. Do not reload merely on switching.
New Tab opens Home with normal default view options, failing without replacing
an existing context if Home cannot be opened. Duplicate copies the current
location, view options, focus and a separate history value, but clears marks and
transient quick-find. These are proposed muCrab semantics to check against the
original, not asserted observations of muCommander.

Location changes use the existing success-only navigation path. Switching tabs
clears transient quick-find and restores listing focus. Global hidden filtering
must reconcile all tabs without resurrecting excluded marks. Closing a tab drops
its browsing state; it does not cancel a W07 mutation or W09 launch already started.
Closing the window retains current cancellation/acknowledgment rules.

## Jobs and refresh

Capture source tab ID and concrete source/destination locations at review time.
Workers never consult the currently selected tab for paths. Unmark successful
roots only in the still-existing originating tab and only when its location still
matches; never substitute another tab at the same path. Refresh affected open
local/ZIP contexts after completion while retaining their own selection rules.
A closed origin is safe: show the global result, without recreating or mutating
an unrelated tab. Reuse this identity boundary in later search and SFTP work.

## Bookmarks

Store an ordered list of stable IDs, user labels and versioned structured local
or ZIP locations. Add Current Location captures the active tab's successful
location; duplicates are allowed only with an explicit distinct label. Opening
uses the active tab and commits history only on success. A missing target remains
editable/removable in the list; opening it reports an error and retains the pane.

Keep the existing configuration directory policy. Save using a same-directory
temporary file and atomic replacement; save failure retains the previous valid
file and reports that edits are not persisted. Validate version, sizes, IDs and
location variants on load; malformed configuration must not crash startup or be
silently overwritten. Cap bookmarks at 256 and labels at 128 Unicode scalar
values; render labels as plain text. No credentials or passwords are stored.

## Integration and evidence

Place tab strips above each existing location bar and bookmarks in a compact
menu/list accessible from either pane. Recover shortcut conventions and prioritize
text fields/dialogs over tab commands. Horizontal scrolling keeps the active tab
reachable at narrow widths. Test identity, not just tab indexes or screenshots.
Mac native verification remains a separate host gate. Existing pending W09/W10
checks do not become passes when this proposal is created.
