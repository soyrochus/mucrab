# Current status

[Back to the README](../README.md).

The current verified milestone is **W06 — bounded local ZIP browsing and extraction**
(see the [implementation ledger](../mucrab-implementation-plan.md)). The latest full verification was recorded on
**2026-10-17**. **W07 — background operations and cancellation** is next.

## Verified workflows

**W01 is implemented:** the native application opens local folders in two independent panes, navigates into regular subfolders and back, and sorts names using recovered muCommander behavior. The Java/Rust suite passes **187 predefined and 155 reserved cases**, including **45,696 pairwise signs**, **336 complete ordered lists**, and **54 enumeration/error checks**, with no differences in the declared scope. Counts include option combinations and repeated real-folder checks; they are not counts of distinct user journeys.

**W02a adds selection and marking:** separate focused rows and marked identities,
Ctrl/Shift clicks, Space with advance/repeat handling, arrow/Home/End navigation,
Shift+arrows, All/None/Invert, and selection persistence through sorting. The
original-JAR/Rust comparison passes **60 sequences and 4,022 state snapshots**,
with zero differences. Native checks cover both panes. [The W02 evidence](../evidence/workflows/W02-selection-marking/README.md)
explains the isolated runtime boundary and representative original GUI coverage.

**W02b adds hidden-file filtering:** one shared **Show hidden files** checkbox,
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
records that limitation and the supported range. The native display shows bytes,
extensions and millisecond UTC dates for the selected criterion.

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

## Platform scope and limitations

The latest recorded verification passes all eight component suites on
**Linux x86_64/ext4**, OpenJDK **25.0.4.1**, with en-US ordering and a separate
`/tmp` filesystem for cross-device moves. Native input checks run under X11/Xvfb; representative
original menu-action checks retain live-table states and Swing renders.
The macOS badge refers to earlier W01/W02a checks on macOS arm64; W03/W04/W05/W06
validation covers Linux. Wayland/physical-desktop
behavior, Windows hidden attributes and exhaustive OS shortcut parity remain
unverified. Opening symbolic links, other filters, additional archive formats, cancellation
and other locales remain for later increments. Copy rejects overlapping source/
destination paths, outside link targets and destination aliases; race-free
filesystem handling and full background-job parity remain unverified.
