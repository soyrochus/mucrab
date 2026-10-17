# Current status

[Back to the README](../README.md).

Updated **2026-10-18** for W15 commit `5b77e5a`. The latest Linux checkpoint
passes **106 Rust tests** and **51 Python tests**, formatting, locked all-feature
build and Clippy. Counts below for earlier workflows describe their historical
checkpoints. Native macOS acceptance covers W01–W08; W09–W15 checks remain pending.

**W15 — SFTP connections and transfers** is implemented and Linux-verified within
its bounded contract. Saved non-secret profiles back typed remote tabs/history/
bookmarks. Host keys are checked before authentication, first-use trust is
explicit, changed keys fail, and trust/profile changes during authentication
prevent a stale connection from becoming active. Password and key-file/passphrase
authentication run on workers. Save changed profile settings before connecting;
changing an endpoint disconnects its old sessions. Optional native credential
storage has no plaintext fallback. Saved credentials are bound to endpoint, user
and authentication method; changing those settings cannot reuse the old secret. Linux uses bounded Secret Service calls and
refuses locked stores without prompting to unlock; macOS Keychain is implemented
but awaits native verification.

Local↔SFTP copies review endpoints/names/policy, include hidden descendants, reject
links/special files, pin available source/destination metadata and use unique
staging. Publication checks close/length; Overwrite requires advertised POSIX
rename, while Skip preserves existing destinations. Source loss/delete-first
fallback and automatic retry of uncertain commits are prohibited. Progress,
pause/cancel and origin-tab-only outcomes reuse the single mutation worker.
Listings use separate generation-tagged workers and serialized protocol exchanges
on each SSH session, releasing the gate between chunks and before pauses.

Remote Open/View downloads one private non-executable retained file (512 MiB max),
preserving its leaf name and checking before/after metadata. External copies remain
readable after exit; edits never update the server. Viewer limits still apply.
Remote archives, search, clipboard export, remote↔remote copying, move/rename/
delete/New Folder and metadata edits are unsupported. No SSH-agent/MFA/proxy or
custom SSH-config execution is implemented.

Linux verification: **106 Rust tests**, **51 Python tests**, four explicitly
separate W15 service-test executions across three distinct tests, real password/
plain-key/encrypted-key authentication, frozen **32 + 16** scoped original/native
provider listing/stream cases with zero differences, actual bytes/hashes, staged
failure/cancellation/timeout/uncertain-publication checks and isolated native
credential save/read/delete/unavailable behavior. Native checks cover trust,
both transfer directions, text clipboard/view/external opening, bookmarks, tab
duplication/close, disconnect/reconnect and normal exit. Fresh W07–W14 native
regressions pass on the preceding W15 candidate; final trust/credential adjustments
have their own final workspace/service/native receipts. Historical receipts are
not relabeled as final-source execution.

Mac build/Keychain/UI, full original connection/job UI parity, active-tab closure
or app-close during an active remote transfer through native input, and broader
server/IPv6/agent-free key-format interoperability remain open acceptance gates.
The server fixture needed documented root/key corrections and later failure
instrumentation after Rust began; no fully blind server/GUI fixture claim is made.
See [W15 evidence](../evidence/workflows/W15-sftp-connections/README.md).

**W13 — Properties and permissions** and **W14 — Built-in viewer** are implemented
and Linux-verified. Alt+Enter captures focused-entry local/ZIP properties, including
lstat link details, available timestamps, allocated size and numeric/named Unix
owner/group. Folder totals include hidden entries and hard links per name, exclude
link targets/descendant mounts and visibly retain partial results. Reviewed
non-recursive rwx/modified-time edits preserve special bits/access time, revalidate
the captured target, use the existing mutation worker and retain unrelated marks.
ZIP metadata stays read-only with unavailable host fields shown explicitly.

F3 opens strict UTF-8/BOM UTF-16 text or bounded PNG/JPEG images in a separate
cancellable worker. Text is virtualized and preserves exact decoded newlines for
selection/copy and literal find; images support orientation, alpha checkerboard,
fit/100%, zoom and pan. ZIP viewer leases are removed on close; explicit external
opening retains an independent W09 copy. Enter/double-click behavior is unchanged.
Text limits:16 MiB input,100,000 lines,64 KiB decoded bytes per line. Image limits:
32 MiB encoded,64 MiB RGBA/decoder samples,16,384 pixels per dimension and the
current GPU limit. Oversized/unsupported/malformed input reports visible errors.

Verification: **101 Rust tests**, **51 Python tests**, locked/offline build,
format/Clippy, Linux native both-pane properties/metadata changes, exact Unicode
text clipboard/find, image controls, light/dark/narrow layouts and archive external
handoff. Viewer/folder totals completed beside a paused real copy; W07–W12 native
regressions pass. Scoped W13 original/native runs cover **40 + 40 cases**;
W14 covers **68 + 52 cases**, with zero unexpected differences. Declared differences
include the isolated original provider's owner-only permission mask, preserved
special bits/link guards, Swing newline normalization and EXIF orientation.
JPEG pixel comparison has maximum channel difference1 within tolerance2.

Native macOS, full original-window parity, a real bind-mount boundary, foreign-owner
chmod denial and broader GPU/font coverage remain pending. Bundled fonts lack some
CJK glyphs although decoded text/clipboard is exact. Read cancellation is cooperative;
blocked lookup/OS/decoder calls must return before close acknowledgment. Stable-entry
checks do not provide race-free filesystem access. See
[W13 evidence](../evidence/workflows/W13-properties-permissions/README.md) and
[W14 evidence](../evidence/workflows/W14-built-in-viewer/README.md).

**W11 — Tabs and bookmarks** and **W12 — File search** are implemented and
Linux-verified. Each pane supports 1–32 independent session tabs, including
New Home/Duplicate/Close/switch commands and per-tab history, marks and column
widths. Reviewed jobs retain their original tab identity and paths even when
tabs switch or close. Shared local/ZIP bookmarks have ordered stable IDs,
validated versioned storage, atomic saves and visible load/save failures. W15 adds
profile-backed SFTP tabs, history and bookmarks without storing credentials in
locations. Tabs remain session-only; profiles, trust and bookmarks persist.

Search runs in a separate read-only worker alongside copy/move operations.
Its filters cover Unicode filename wildcards, extension, file/folder/link type,
inclusive byte sizes and UTC [From, To) modification times. Hidden pruning,
link/mount boundaries, bounded traversal, 64-hit batches, an eight-batch queue,
replacement acknowledgment and partial results keep scope and progress explicit.
Results are sorted by relative path and open their containing folder in the
current tab after revalidation. Changed/missing results retain the current pane.

Verification: **84 Rust tests**, **51 Python tests**, locked/offline all-feature
build and Clippy, both-pane native input, bookmark restart and W07–W10 native
regressions. W12 compares **64 predefined + 64 reserved filename cases** against
the pinned Java predicate: zero unexpected differences; two reserved escape
cases intentionally differ. W11 includes a supplemental original factory/value
probe, with a deliberate copied-history difference; native bookmark validation is tested separately.
Full original tab/search UI parity, native macOS verification and real mount
integration remain acceptance gates. See [W11 evidence](../evidence/workflows/W11-tabs-bookmarks/README.md)
and [W12 evidence](../evidence/workflows/W12-file-search/README.md).

**W10 — Everyday file management** now implements New Folder, reviewed
Delete/Trash, system clipboard Copy/Paste, type-to-find and independent Back/Forward
history. Linux X11 checks pass: 66 Rust tests, the separately invoked native
clipboard/Trash restoration check, 51 Python tests, both-pane W10 input and
W07/W08/W09 native regressions. Two bounded 80-case original/native runs have
zero unexpected differences and one declared matching difference each.
Native macOS verification and remaining original UI/IME checks are pending;
W10 is not yet closed across all hosts. See
[W10 evidence](../evidence/workflows/W10-everyday-file-management/README.md).

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
component suite executions and the W08 replay also pass. The W07 checkpoint had **46 cross-platform Rust tests**, one additional Mac filename test, and
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

## W10 — Everyday file management

**F7** creates a named folder in the active local pane. **F8/Delete** reviews a
move to the real OS Trash; **Shift+F8/Shift+Delete** separately reviews permanent
deletion. Trash failure never triggers permanent deletion. Links are removed as
links, hidden descendants are included, and partial results remain visible.
Creation/deletion use the existing single mutation worker and cancellation rules.

With listing focus, **Ctrl/Cmd+C/V** copies a system file selection and reviews
Paste into the active local pane, including sources from multiple directories.
Text fields retain text clipboard editing. Paste reuses copy collision/overlap
and link guards and preserves unrelated marks. Linux uses an X11 file/text
selection owner and requires `xclip` for import and `gio` for Trash. Pure Wayland
clipboard publication is not implemented; Xwayland bridging is unverified.

Typing starts a case-insensitive filename-prefix find field without changing
marks, order or hidden filtering. Previous/Next wraps through visible matches;
Escape or Enter dismisses before normal listing activation. **Alt+Left/Right**,
Go menu actions and toolbar arrows navigate each tab's independent, session-only
100-location history. W15 extends local/ZIP history to profile-backed SFTP
locations. Failed navigation preserves its cursor/listing.

[W10 evidence](../evidence/workflows/W10-everyday-file-management/README.md)
records actual original jobs/history/matching comparisons, bidirectional original
Java/native clipboard exchange, native Trash restoration and both-pane input.
Prefix-only matching intentionally differs from original substring matching.
Clipboard Cut, New File and Trash restoration/emptying UI remain deferred.
Recursive local search is implemented in W12. Native macOS adapters are implemented
but unverified; real IME and exhaustive original window/shortcut parity remain open,
as does full W10 closure.

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
unverified. W09–W15 have bounded Linux verification; their native macOS acceptance
remains pending, including default-application dispatch, clipboard/Trash,
tabs/bookmarks/search, properties/viewer and SFTP build/Keychain/UI. The earlier
signed Mac release bundle predates these additions and does not verify the W15
vendored OpenSSL build. Linux SFTP builds require OpenSSL and DBus development
packages; macOS enables locked vendored OpenSSL.

Additional remote protocols, archive formats and other locales remain for later
increments. Copy rejects overlapping source/
destination paths, outside link targets and destination aliases; race-free
filesystem handling and full scheduler/parallel-job parity remain unverified. W07 cancellation is
cooperative between entries and 64 KiB chunks; blocked OS calls/atomic renames
must return before acknowledgement, and cancellation does not roll back writes.
W15 remote transfers use 32 KiB chunks and finite SSH timeouts; remote metadata
checks have weaker identity guarantees than local files. Native active-transfer
tab/app closure and broader disconnect races remain acceptance gates.
