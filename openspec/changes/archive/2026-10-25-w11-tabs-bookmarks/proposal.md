# W11 — Tabs and bookmarks

## Why

W10 history revisits locations within one pane, but each pane still holds only
one browsing context. Frequent folder changes lose the immediately available
context, and useful locations cannot be saved as favorites across sessions.

## What Changes

- Add multiple independently navigable tabs to each pane, with New, Duplicate,
  Close and Next/Previous actions and a visible active tab.
- Keep location, local/ZIP history, focus, marks and sort options per tab. Keep
  the existing Show Hidden setting shared and appearance application-wide.
- Add shared, ordered bookmarks with Add Current Location, Open, Rename and
  Remove actions, persisted atomically in the existing configuration area.
- Bind background job outcomes to stable originating tab identities so switching
  or closing tabs cannot clear another tab's marks or cancel an unrelated job.

Tabs and bookmarks share location navigation and fit one change. They do not
include persistent tab sessions, multiple windows, locked/reorderable tabs,
remote connections, credential storage or a bookmark import/export format.

## Dependencies and Impact

Build on W10's structured local/ZIP locations and history. No W12–W15 dependency.
Refactor the desktop's fixed pair of BrowserPane values into two tab collections;
retain the two-pane layout and current operations. Persist bookmarks only, not
session tabs/history or marked selections. W15 may later add profile-backed
remote bookmark locations without placing secrets in bookmark files.

## Verification

Recover the original tab/bookmark actions and callers from the pinned binaries.
Freeze the generator, predefined/reserved cases and state protocol before Rust
implementation. Compare observable state boundaries and exercise both panes on
Linux and macOS; record chosen differences and unavailable host checks. This
proposal introduces no application behavior and does not close W10's pending gates.
