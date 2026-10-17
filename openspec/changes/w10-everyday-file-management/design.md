# W10 design

## Existing foundations

`BrowserPane` owns local/ZIP location, entries, ordering, focus and marks.
`FileSelection` already supplies marked entries or the focused child, excluding
the parent row. W03 provides guarded reviewed copy; W07 provides cooperative
mutation jobs, progress, cancellation and deferred close. W09's external-app
requests are independent of mutation jobs. Retain these boundaries.

## Decisions

### Delivery and evidence

Implement New Folder, Delete/Trash, Copy/Paste, Quick-find and History in that
order. Each stage must close its recovery, fixtures, implementation, UI and
verification tasks before its completion is recorded. W10 is complete only when
all five stages and required host checks are complete; unavailable host checks
remain explicitly pending.

Inspect MkdirAction, DeleteAction/PermanentDeleteAction, CopyFilesToClipboardAction,
PasteClipboardFilesAction, QuickFindAction, GoBackAction/GoForwardAction and their
actual callers/providers in the pinned distribution. Resolve keyboard mappings,
confirmation defaults, matching policy and platform boundaries from observations.
The requirements in this change are intended muCrab behavior; they are not claims
that unobserved muCommander behavior is identical.

### File mutations

Keep core requests independent of widgets. Review operations against a captured
source selection and destination. Reject changed source/destination assumptions
before applying a reviewed request. Reuse existing path/name guards and outcome
reporting; these checks do not establish race-free filesystem mutation.

New Folder creates one leaf atomically without replacing an existing entry.
Deletion operates on directory entries: links are removed or trashed themselves,
never traversed. Reject filesystem roots, the synthetic parent and deletion of
an open pane location or its ancestors. Descendant directories may be deleted.
Do not relax existing W03 link-copy restrictions for clipboard imports.

Trash and permanent deletion are different actions. Use native platform Trash
semantics, including restorable origin metadata where the platform provides it.
No shell interpolation and no private substitute trash directory. An unsupported
volume or adapter failure is an error, never permission to delete permanently.
Select the adapter/dependency after testing actual Linux/macOS behavior and
checking licensing; this proposal does not prescribe an unverified library.

Mutations use the W07 single-job gate. Cancellation is checked between entries
and, for recursive permanent deletion, traversal steps. A native Trash call may
finish before cancellation can be acknowledged. Preserve completed results,
report partially processed roots and retain unfinished marks. No rollback promise.

### Clipboard boundary

Add a platform service capable of publishing and reading file selections, not
just egui text clipboard contents. Use interoperable file URI lists on Linux
and native file URL pasteboard representations on macOS, with formats finalized
from interoperability probes. Do not fake integration with an internal clipboard.

Copy snapshots absolute local entry paths without modifying files or marks.
Paste snapshots the native clipboard once for review, imports only supported
local file URLs, and copies into the active local pane. Support source entries
from multiple parent directories through one reviewed job. Decode file URLs
correctly; do not treat URL text as shell syntax or silently follow remote URLs.
Preserve symbolic-link entry identity for existing copy guards.

Validate the complete imported list before mutation; reject malformed or
unsupported mixed payloads. Deduplicate identical entries in stable order and
apply existing overlap/collision guards. Clipboard contents are external input.
Successful Paste does not clear the clipboard, imply a move, or unmark unrelated
entries in either pane. Clipboard Cut and archive-member clipboard export are
separate future increments.

### Quick-find and focus

Maintain a transient query for the active pane's visible ordered listing. A
printable text event in a listing starts a small find field; IME composition must
not move focus until committed text arrives. Existing command shortcuts and
location/dialog editing take precedence. Space still marks when find is closed;
inside the find field it is query text.

Use filename-prefix matching, with a documented case-insensitive policy frozen
before implementation. Include Unicode, composed/decomposed names and repeated
letters in recovery fixtures; explicitly record any unsupported equivalence.
Search the visible order without changing sorting, hidden filtering or marks.
Escape dismisses; Enter dismisses and retains the match without opening it.
A subsequent ordinary listing Enter retains W09 activation. Provide next/previous
match controls, wrapping and visible no-match feedback. Clear on pane switch or
successful navigation; reconcile safely on refresh.

### History boundary

Represent a location structurally: local directory or ZIP archive plus internal
prefix. Do not infer a virtual ZIP location from an ambiguous display string.
Keep at most 100 entries per pane in memory for the session. Seed with the first
successful open. Record successful explicit location changes, including folder,
parent and Home navigation; do not record refresh, sorting, filtering or file
opening. Suppress consecutive identical locations.

Back/Forward navigate using the same guarded open path with history recording
suppressed. Advance the cursor only on success. On failure, retain the current
location, cursor and listing and expose the error. A new successful explicit
navigation after Back discards the forward branch. Reuse current navigation
selection rules: history restores location, not snapshots of old marks/options.

## Verification boundaries

Use disposable, equivalent Java/Rust trees and independent reserved fixtures.
Exercise native input in both panes and prove filesystem outcomes, rather than
relying on screenshots alone. Never write the user's personal clipboard, change
file associations, or trash personal files during automated checks: use isolated
clipboard sessions/profiles and disposable platform Trash fixtures. Test native
Trash restoration through the platform to verify recoverability without adding
a restore UI to muCrab. Record unavailable platform isolation as a missing check,
not a passing mock.

Broaden tests when implementation exposes a new dependency or semantic gap.
Run existing workspace checks and W07/W08/W09 regressions before closure.
