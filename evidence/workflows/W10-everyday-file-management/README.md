# W10 — Everyday file management

All five features are implemented. Linux X11 verification passes; macOS adapters
are present but native verification remains pending. This is not full muCommander
parity or closure of all OpenSpec host/evidence tasks.

## Behavior

- **F7 / File → New Folder:** review one direct child name. Creation never replaces
  an entry, revalidates the destination, refreshes panes and focuses the visible
  new folder. Cancelling makes no change; hidden filtering is preserved.
- **F8 or Delete / Move to Trash:** review the effective local selection and use
  the real OS Trash. **Shift+F8 or Shift+Delete** separately reviews permanent
  deletion and explicitly states that it bypasses Trash. Native Trash failure
  never falls back to permanent deletion.
- **Ctrl/Cmd+C and V:** with listing focus, publish a native file selection or
  review Paste into the active local folder. Fields retain text editing shortcuts.
  Multi-parent sources, stable deduplication and collision policies are supported.
  Paste copies rather than moves and does not clear unrelated destination marks.
- **Type a prefix / View → Find by name:** case-insensitive Unicode lowercase
  prefix matching in visible order, with previous/next, wrapping and no-match
  feedback. Marks/filter/sort remain unchanged. Enter dismisses find first;
  another listing Enter activates the item. Escape dismisses. Pane changes and
  successful navigation clear the query; refresh reconciles it.
- **Alt+Left/Right / Go → Back/Forward / toolbar arrows:** independent per-pane
  history of at most 100 structured local/ZIP locations. Only successful explicit
  navigation records entries. Failures retain the cursor/listing, new navigation
  after Back truncates the forward branch, and history remains session-only.

Mutations share W07's one-worker gate, progress, cooperative Pause/Cancel,
partial-result reporting and deferred close. A native Trash call must return
before cancellation can be acknowledged. Symlinks are deleted/trashed themselves,
never traversed. Permanent deletion includes hidden descendants. Reviewed entries
are revalidated; protected pane locations/ancestors and filesystem roots are
rejected. Linux mount-table entries (including bind mounts) and cross-device
directory roots are retained. Guards assume stable trees and do not establish
race-free filesystem security or rollback.

## Native adapters and limits

Linux X11 publishes text/uri-list, UTF-8 text and GNOME copied-files through the
already locked x11rb 0.13.2. Both file and text representations are needed because
egui-winit generates its Paste event only when it can fetch clipboard text.
`xclip` imports file URI lists; `gio trash` supplies native Trash behavior.
Missing tools fail visibly. Clipboard ownership is in-process: retention after
muCrab exits depends on the desktop clipboard manager. Clipboard payloads are
bounded to 1 MiB, 10,000 entries and the X11 server's single-request limit;
INCR export is not implemented. Remote/malformed/mixed URLs, NULs, queries and
fragments are rejected before mutation. Original entry/link identity is preserved.

macOS uses Foundation/AppKit via built-in `osascript` JavaScript for Automation.
These adapters have not been run on this Linux host. Pure Wayland clipboard
publication is not implemented; Xwayland clipboard bridging and the wl-paste
import fallback are unverified. Windows remains unsupported.

New File, clipboard Cut, Trash restore/emptying UI, recursive search, persistent
history, archive-member clipboard export, remote filesystems and ZIP mutation are
outside W10. The native Trash verification restores a disposable file externally;
it does not add restoration UI to muCrab. Real IME composition input remains
unverified, although committed text events are handled separately from preedit.

## Original recovery and comparisons

[Recovery](recovery.json) fingerprints 19 classes from the pinned distribution,
including actions, dialogs, jobs, clipboard/history/search and Trash providers.
Upstream Java implementation source was not fetched.

[Default run](comparison.json) and [reserved-seed run](comparison-reserved.json)
each execute **80 cases**: 24 MkdirJob, 24 permanent DeleteJob and 24 CopyJob
processFile requests, five QuickSearch.matches observations and three history
addition/capacity/branching cases. Complete original/native fixture trees, bytes
and links match. Each run has **zero unexpected differences** and one explicitly
recorded difference: the original matches `xAlpha.txt` for `al`, while W10's
prefix-only contract does not. History comparisons use the original addition
methods and field-initialized branch cursor, not original asynchronous window
navigation. UI/native Trash parity is not claimed by those reports.

The [contract](contract.md) lists other intended differences: success-only history
cursor, reviewed Skip-default clipboard Paste, single-leaf folder creation and
no automatic Trash-to-permanent-delete fallback. Original QuickFindAction is a
separate search-URL command; W10's workflow corresponds to QuickSearch typing.
Shortcut defaults and exhaustive original dialog dispatch are not fully recovered.

The contract categories and seeds preceded Rust implementation. The executable
generator and exact GUI trace were written afterwards. Reserved-seed cases are
reproducible independent fixtures, not a blind holdout. This sequencing limit is
retained in the OpenSpec task list.

## Native verification and regressions

[Native services](native-services.json) verifies **both directions** between
muCrab's native clipboard and the original ClipboardSupport/AWT implementation,
including exact Unicode/space/percent/hash filenames. It also verifies real GIO
Trash contents/origin metadata and native restoration of exact file bytes. Tests
use isolated Xvfb/DBus and disposable XDG data; personal clipboard/Trash are not used.
A denied FUSE mount warning did not prevent the asserted GIO restore from passing.

[GUI receipt](native.json) records the complete both-pane input trace, actual
folder creation, reviewed/cancelled Paste/Trash/permanent deletion, history and
quick-find transitions, exact resulting trees and normal exit. Captures live in
`artifacts/gui/W10-everyday/native/`. [Regressions](regressions.json) records W07's
27-action/20,000-source pause/cancel/marks/close replay, W08's operations/themes/
layout replay, and W09's three real local/archive dispatches with retained copies.
Old replay coordinates were updated for the expanded File/View menus.

The Linux workspace passes **66 Rust tests**, with the isolated native-service
test additionally invoked explicitly (it is ignored in the ordinary suite),
**51 Python tests**, formatting, locked/offline build and denied-warning Clippy.
Mac native input/interoperability/Trash and real IME tests remain open. Earlier
W01–W07 comparison receipts remain historical; they were not all rerun for W10.

## Reproduce

```
python3 -B tools/w10_inspect.py
cargo build --workspace --locked --offline
python3 -B tools/w10.py
python3 -B tools/w10.py --reserved
cargo test --workspace --all-features --locked --offline
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
cargo fmt --all -- --check
python3 -B -m unittest discover -s tools -p 'test_*.py'
xvfb-run -a dbus-run-session -- python3 -B tools/w10_native.py
python3 -B tools/w10_gui.py prepare
xvfb-run -a python3 -B tools/linux_gui.py artifacts/gui/W10-everyday/launch.json --replay fixtures/W10-everyday-file-management/native-gui-actions.json
python3 -B tools/w10_gui.py verify
```

The GUI helper refuses to overwrite an existing fixture. Preserve an old run
before preparing another. Native checks need GIO/GVFS, xclip, X11/XTest, Xvfb,
DBus, a JDK and Pillow. Never replay these inputs on the personal desktop.
