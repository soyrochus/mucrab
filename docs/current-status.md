# Current status

[Back to the README](../README.md).

**W09 — default-application file opening** is implemented for local files and
archive members, with native Linux verification of Enter, double-click and
File → Open. Archive copies survive application exit and do not update ZIPs.
The macOS dispatch path awaits native verification. See
[W09 evidence](../evidence/workflows/W09-open-files/README.md) for scope and results.

The preceding verified functional milestone is **W07 — background operations,
progress and cancellation**, delivered after the prioritized **W08 classic UI**.
Checks recorded on **2026-10-17** include W07's original full-job comparisons,
all eight earlier component suites, Rust/Python checks and W07/W08 native replays.
The preceding [Mac acceptance](../evidence/platforms/macos-arm64/README.md) also passes all
18 suite executions, 47 Rust tests, 51 Python tests and native W07/W08 workflows.
See the [implementation ledger](../mucrab-implementation-plan.md).

## Verified workflows

**W01 is implemented:** the native application opens local folders in two independent panes, navigates into regular subfolders and back, and sorts names using recovered muCommander behavior. The Java/Rust suite passes **187 predefined and 155 reserved cases**, including **45,696 pairwise signs**, **336 complete ordered lists**, and **54 enumeration/error checks**, with no differences in the declared scope. Counts include option combinations and repeated real-folder checks; they are not counts of distinct user journeys.

**W02a adds selection and marking:** separate focused rows and marked identities,
Ctrl/Shift clicks, Space with advance/repeat handling, arrow/Home/End navigation,
Shift+arrows, All/None/Invert, and selection persistence through sorting. The
original-JAR/Rust comparison passes **60 sequences and 4,022 state snapshots**,
with zero differences. Native checks cover both panes. [The W02 evidence](../evidence/workflows/W02-selection-marking/README.md)
explains the isolated runtime boundary and representative original GUI coverage.

**W02b adds hidden-file filtering:** one shared **Show hidden files** option (in View since W08),
initially off, refreshes both panes. Surviving marks/focus remain; hidden marks
are cleared and missing focus falls back to a valid row. The original-JAR/Rust
suite passes **56 sequences, 2,520 global snapshots and 5,040 pane snapshots**,
with zero differences. [W02b evidence](../evidence/workflows/W02b-hidden-filtering/README.md)
records the contract, original reload-worker boundary and GUI checks.

**W02c adds metadata sorting:** each pane can sort by Name, Size, Modified or
Extension, with independent direction and folders-first settings. Focus and
marks survive sort changes. The original-JAR/Rust suites pass **74 sequences,
1,454 snapshots, 223,432 pair signs, 1,386 ordered lists and eight metadata
enumeration checks**, with zero differences. Four extreme-value sequences cover
comparison signs only: the original sorter can stall on overflowing synthetic
timestamps. [W02c evidence](../evidence/workflows/W02c-metadata-sorting/README.md)
records that limitation and the supported range. W08 now displays size, extension and minute-resolution UTC dates in simultaneous
columns, with sorting through column headers; the underlying timestamp and
sorting semantics retain their original precision.

**W03 adds recursive local copy:** press **F5** or **Copy to other pane** to
review the selection and collision policy before copying. Existing folders
merge; files can be skipped, overwritten, or replaced only when newer/different
in size. Empty folders, hidden descendants, file bytes, timestamps and contained
relative links are covered. The original-JAR/Rust comparison passes **242 cases**
(146 predefined + 96 reserved), with zero differences in top-level outcomes and
complete destination trees. Source contents and declared metadata remain unchanged. Partial copies are kept
and errors are shown in the result details. [W03 evidence](../evidence/workflows/W03-recursive-copy/README.md)
records the method boundary and unsupported cases.

**W04 adds local move and focused rename:** **F6** reviews a move to the other
pane; **F2** reviews a new name for the focused item, preserving unrelated marks.
Moves try a filesystem rename first and fall back to copy then delete. Partial
failures retain the remaining sources and expose details. The original-JAR/Rust
comparison passes **479 cases** (319 predefined + 160 reserved), including
**216 real cross-filesystem cases**, with zero differences in outcomes, source/
destination trees and inode-identity relations. Native code declines fallback
through symlinks because the original can consume their targets.
[W04 evidence](../evidence/workflows/W04-move-rename/README.md) records the boundary.

**W05 adds batch rename:** **Batch rename…** opens a live old/new-name preview
for the active pane's selection. Masks support names/extensions, UTF-16 ranges,
independent counters, parent names and UTC dates, plus literal search/replace
and ASCII case changes. Duplicate names and conflicts with selected originals
block application. Unchanged rows are skipped; existing files stay intact and
partial results remain after errors. A stale reviewed plan is rejected before
mutation. **86 fixture sets** (54 predefined + 32 reserved) pass **346 previews,
1,721 row previews and 315 ordered outcomes**, with zero differences in scope.
Manual per-row overrides and Unicode case conversion remain outside this increment.
[W05 evidence](../evidence/workflows/W05-batch-rename/README.md) records the limits.

**W06 adds ZIP browsing and extraction:** double-click a local `.zip` or enter
its path in either pane, navigate its folders, and use **Extract ZIP…** (or **F5**
inside an archive) to review extraction into the other local pane. Stored and
deflated files, explicit/implicit folders, UTF-8 names, hidden descendants,
selections and four collision policies are covered. The first transfer error
stops the archive, retaining earlier writes and exposing details. The original
ZIP-provider/UnpackJob comparison passes **259 cases** (163 predefined + 96
reserved), **814 folder listings** and **1,887 entry observations**, with zero
differences in scope. DOS-derived dates are compared at whole-second precision;
the original retains a stray millisecond fraction. Archive panes are read-only.
[W06 evidence](../evidence/workflows/W06-zip-browsing-extraction/README.md) records
the contract, native guards, original GUI boundary and limitations.

## W08 interface

W08 replaces the prototype shell with compact File/Mark/View/Go/Window/Help
menus, a drawn-icon toolbar, pane location bars, virtualized metadata tables,
selection counts/known file bytes, and a fixed bottom command bar. Column
borders and the pane divider resize; names clip with hover text and narrow
panes scroll horizontally. Existing review dialogs and core operations remain.
View → Appearance offers only System, Light and Dark, saves the preference,
and follows reported OS light/dark changes (Light fallback). Help contains
shortcuts and About information. New headless regressions cover live reported
OS changes and 10,000-row keyboard/selection/sort/scroll behavior; a native replay
checks file-operation results, menus, themes and resizing. Menus are in-window;
macOS native menu integration is not implemented. Physical theme notification
delivery is verified on macOS 26.6.2 through a live Dark/Light/Dark change with
the original system preference restored. W07 adds the background-job controls described below.

## W07 background operations

Copy, move/focused rename, batch rename and ZIP extraction run with shared
cooperative control. Progress shows current-file bytes and completed/total work
items. Pause/Resume and Cancel are available while browsing, marking and sorting
remain usable. Only one mutation job runs at a time. Cancel waits for worker
acknowledgement; completed writes and partial destination files remain, unfinished
fallback-move sources are retained, and interrupted/later roots stay marked.
Closing the window cancels a running/paused job and defers exit until it stops.
Unhandled worker errors/panics are visible as failures with partial-result details.

The original's real start/run/interrupt/pause methods and concrete jobs pass
**18 predefined + 12 reserved = 30 cases**, with **zero differences** in declared
lifecycle, mark and filesystem outcomes. Streaming prefixes are independently
validated rather than requiring identical scheduler-dependent lengths. The native
27-action replay checks real Pause/Resume/Cancel, navigation/marks while busy and
normal exit after cancelling a paused copy of 20,000 source files. All 16 earlier
component suite executions and the W08 replay also pass. There are **46 cross-platform Rust tests**, one additional Mac filename test, and
**51 Python tests**. [W07 evidence](../evidence/workflows/W07-background-operations/README.md)
records recovery, comparison reports, source fingerprints, native checks and limits.

## W09 — Default-application file opening

Enter and **File → Open** activate the focused item; double-click activates the
clicked item in either pane. Ordinary local files open in the registered default
application, including files reached through symbolic links. Folders and local
ZIPs retain navigation. Broken or cyclic links and launcher failures report errors.

Archive members are extracted with the existing guarded ZIP extractor before
opening. Each copy retains its filename in a unique private temporary directory
and remains available after muCrab exits, until OS temporary cleanup or explicit
user cleanup. Edits affect the temporary copy and do not update the ZIP.
Preparation runs in the background independently of W07 mutation jobs, supports
cancellation and defers window close until cancellation is acknowledged. Already
launched applications remain independent of muCrab.

Linux uses `xdg-open`; macOS uses `/usr/bin/open`. Native Linux verification
covers Enter, archive double-click and File → Open with an isolated real file
association, exact filenames/contents and temporary-copy retention after exit.
The W09 checkpoint passes **51 Linux Rust tests**, **51 Python tests**, formatting,
denied-warning Clippy, and the W07/W08 native regressions. The earlier Mac
checkpoint's 47 Rust tests predate W09; native macOS dispatch remains pending.
[W09 evidence](../evidence/workflows/W09-open-files/README.md) records static
original-binary recovery and native implementation checks. W09 does not yet have
a Java/Rust runtime parity suite.

## Platform scope and limitations

The recorded original-JAR/Rust verification passes all eight earlier component suites plus W07 full-job comparisons on
**Linux x86_64/ext4**, OpenJDK **25.0.4.1**, with en-US ordering and a separate
`/tmp` filesystem for cross-device moves. Native input checks run under X11/Xvfb; representative
original menu-action checks retain live-table states and Swing renders.
The pre-W09 Mac run covers all nine suites on **macOS 26.6.2 / arm64 / APFS**,
OpenJDK **21.0.10**, with a real second APFS volume for cross-device moves.
Native desktop input verifies the W08 operations and W07 controls, including
closing a paused job. See the [Mac receipts](../evidence/platforms/macos-arm64/README.md)
for Unicode path rules and source hashes. Live OS theme-notification delivery,
restoration of the original preference and the signed Mac release bundle pass. Other macOS versions,
Wayland, Windows hidden attributes and exhaustive OS shortcut parity remain
unverified. W09 file opening is verified on Linux only; its macOS dispatch path
awaits native verification. Other filters, additional archive formats and other
locales remain for later increments. Copy rejects overlapping source/
destination paths, outside link targets and destination aliases; race-free
filesystem handling and full scheduler/parallel-job parity remain unverified. W07 cancellation is
cooperative between entries and 64 KiB chunks; blocked OS calls/atomic renames
must return before acknowledgement, and cancellation does not roll back writes.
