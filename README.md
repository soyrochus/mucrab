# muCrab

[![Version 0.1.0](https://img.shields.io/badge/version-0.1.0-blue)](Cargo.toml)
[![Rust 1.95 or newer](https://img.shields.io/badge/Rust-1.95%2B-orange?logo=rust)](#build-and-run-mucrab)
[![License: GPL-3.0-only](https://img.shields.io/badge/license-GPL--3.0--only-blue)](LICENSE)
[![Status: experimental](https://img.shields.io/badge/status-experimental-yellow)](#current-status)
[![Verified on macOS arm64](https://img.shields.io/badge/verified-macOS%20arm64-lightgrey?logo=apple)](#current-status)
[![Verified on Linux x86_64](https://img.shields.io/badge/verified-Linux%20x86__64-lightgrey?logo=linux)](#current-status)

**Two claws, two panes.**

muCrab is an experimental Rust port of selected [muCommander](https://github.com/mucommander/mucommander) file-management workflows, reconstructed from its published Java binaries. The intended application name is **muCrab** and its command name is **`mucrab`**.

The experiment uses [REA](https://github.com/morluto/rea), a JVM decompiler, and observations of the original application to recover behavior, implement it in Rust, and compare the two implementations.

## Current status

The current verified milestone is **W05 — bounded local batch rename**
(see the [implementation ledger](mucrab-implementation-plan.md)). The latest full verification was recorded on
**2026-10-16**. **W06 — ZIP browsing and extraction** is next; full background-job control and
cancellation (W07) remain planned.

**W01 is implemented:** the native application opens local folders in two independent panes, navigates into regular subfolders and back, and sorts names using recovered muCommander behavior. The Java/Rust suite passes **187 predefined and 155 reserved cases**, including **45,696 pairwise signs**, **336 complete ordered lists**, and **54 enumeration/error checks**, with no differences in the declared scope. Counts include option combinations and repeated real-folder checks; they are not counts of distinct user journeys.

**W02a adds selection and marking:** separate focused rows and marked identities,
Ctrl/Shift clicks, Space with advance/repeat handling, arrow/Home/End navigation,
Shift+arrows, All/None/Invert, and selection persistence through sorting. The
original-JAR/Rust comparison passes **60 sequences and 4,022 state snapshots**,
with zero differences. Native checks cover both panes. [The W02 evidence](evidence/workflows/W02-selection-marking/README.md)
explains the isolated runtime boundary and representative original GUI coverage.

**W02b adds hidden-file filtering:** one shared **Show hidden files** checkbox,
initially off, refreshes both panes. Surviving marks/focus remain; hidden marks
are cleared and missing focus falls back to a valid row. The original-JAR/Rust
suite passes **56 sequences, 2,520 global snapshots and 5,040 pane snapshots**,
with zero differences. [W02b evidence](evidence/workflows/W02b-hidden-filtering/README.md)
records the contract, original reload-worker boundary and GUI checks.

**W02c adds metadata sorting:** each pane can sort by Name, Size, Modified or
Extension, with independent direction and folders-first settings. Focus and
marks survive sort changes. The original-JAR/Rust suites pass **74 sequences,
1,454 snapshots, 223,432 pair signs, 1,386 ordered lists and eight metadata
enumeration checks**, with zero differences. Four extreme-value sequences cover
comparison signs only: the original sorter can stall on overflowing synthetic
timestamps. [W02c evidence](evidence/workflows/W02c-metadata-sorting/README.md)
records that limitation and the supported range. The native display shows bytes,
extensions and millisecond UTC dates for the selected criterion.

**W03 adds recursive local copy:** press **F5** or **Copy to other pane** to
review the selection and collision policy before copying. Existing folders
merge; files can be skipped, overwritten, or replaced only when newer/different
in size. Empty folders, hidden descendants, file bytes, timestamps and contained
relative links are covered. The original-JAR/Rust comparison passes **242 cases**
(146 predefined + 96 reserved), with zero differences in top-level outcomes and
complete destination trees. Source contents and declared metadata remain unchanged. Partial copies are kept
and errors are shown in the result details. [W03 evidence](evidence/workflows/W03-recursive-copy/README.md)
records the method boundary and unsupported cases.

**W04 adds local move and focused rename:** **F6** reviews a move to the other
pane; **F2** reviews a new name for the focused item, preserving unrelated marks.
Moves try a filesystem rename first and fall back to copy then delete. Partial
failures retain the remaining sources and expose details. The original-JAR/Rust
comparison passes **479 cases** (319 predefined + 160 reserved), including
**216 real cross-filesystem cases**, with zero differences in outcomes, source/
destination trees and inode-identity relations. Native code declines fallback
through symlinks because the original can consume their targets.
[W04 evidence](evidence/workflows/W04-move-rename/README.md) records the boundary.

**W05 adds batch rename:** **Batch rename…** opens a live old/new-name preview
for the active pane's selection. Masks support names/extensions, UTF-16 ranges,
independent counters, parent names and UTC dates, plus literal search/replace
and ASCII case changes. Duplicate names and conflicts with selected originals
block application. Unchanged rows are skipped; existing files stay intact and
partial results remain after errors. A stale reviewed plan is rejected before
mutation. **86 fixture sets** (54 predefined + 32 reserved) pass **346 previews,
1,721 row previews and 315 ordered outcomes**, with zero differences in scope.
Manual per-row overrides and Unicode case conversion remain outside this increment.
[W05 evidence](evidence/workflows/W05-batch-rename/README.md) records the limits.

The latest recorded verification passes all seven component suites on
**Linux x86_64/ext4**, OpenJDK **25.0.4.1**, with en-US ordering and a separate
`/tmp` filesystem for cross-device moves. Native input checks run under X11/Xvfb; representative
original menu-action checks retain live-table states and Swing renders.
The macOS badge refers to earlier W01/W02a checks on macOS arm64; W03/W04/W05
validation covers Linux. Wayland/physical-desktop
behavior, Windows hidden attributes and exhaustive OS shortcut parity remain
unverified. Opening symbolic links, other filters, archives, cancellation
and other locales remain for later increments. Copy rejects overlapping source/
destination paths, outside link targets and destination aliases; race-free
filesystem handling and full background-job parity remain unverified.

## Migration screenshots

### muCrab — current Rust implementation

The W05 checkpoint provides two independent file panes, local folder browsing,
name/metadata sorting, focus/marking, hidden-file filtering, and reviewed recursive
copy, move, single-item rename and batch rename. The native Rust application runs independently
of Java.

![muCrab's native Rust interface with two file panes, batch rename preview with a mask, search/replacement controls, selected items and generated names](images/mucrab.png)

### muCommander — Java reference application

The original muCommander interface is the reference for recovering and comparing
the selected workflows. The screenshot shows its two-pane browser and broader
set of file-management features; the migration scope and comparison results are
described above.

![Original muCommander Java application showing its two-pane file browser, metadata columns, toolbar, and file-operation controls](images/mucommander.png)

## Build and run muCrab

Requirements: Rust **1.95** or newer, Cargo, and a native desktop session for the GUI. Rust/Cargo **1.99.0** was used on macOS arm64 and Linux x86_64. The native GUI was exercised on macOS and under Linux X11/Xvfb; Wayland remains unverified.

The [Cargo workspace](Cargo.toml) contains:

| Package | Role | Current contents |
| --- | --- | --- |
| `mucrab-core` | Framework-independent behavior | Local enumeration/filtering, focus/marks, name/metadata sorting, recursive copy, move/rename, batch preview/apply and detailed outcomes |
| `mucrab-desktop` | Native `mucrab` executable | Two-pane browser using eframe/egui 0.36.2 with the OpenGL backend |
| `mucrab-compare` | Java/Rust comparison harness | W01, W02a, W02b, W02c, W03, W04 and W05 original-binary oracles, Rust adapters, and difference reports |

All three packages use `GPL-3.0-only` and are marked non-publishable. [Cargo.lock](Cargo.lock) pins the resolved dependencies; it is committed. Build and launch from the repository root:

```sh
cargo build --workspace --locked
cargo run --locked -p mucrab-desktop
# Optionally provide initial locations:
cargo run --locked -p mucrab-desktop -- /path/to/left /path/to/right
```

The application supports browsing, sorting, marking, filtering and recursive local copy/move plus focused and batch rename; archive handling and other mutation controls are deferred. Running it requires no Java, Node, REA, or reference JARs. Hidden dotfiles start hidden; **Show hidden files** controls both panes. Double-click a regular folder to enter it; each pane has Open, Parent, Refresh, and sort controls. Check the native command-line entry points without opening a window:

```sh
cargo run --locked -p mucrab-desktop -- --version
cargo run --locked -p mucrab-compare -- --status
```

Status alone deliberately exits with **code 2**, `status: "not_run"`, and `cases_run: 0`. It never reuses a historical pass as a fresh comparison. Run the scoped suite after acquiring the reference and installing a full JDK:

```sh
cargo run --locked -p mucrab-compare -- --suite w01
cargo run --locked -p mucrab-compare -- --suite w01 --reserved
cargo run --locked -p mucrab-compare -- --suite w02
cargo run --locked -p mucrab-compare -- --suite w02 --reserved
cargo run --locked -p mucrab-compare -- --suite w02b
cargo run --locked -p mucrab-compare -- --suite w02b --reserved
cargo run --locked -p mucrab-compare -- --suite w02c
cargo run --locked -p mucrab-compare -- --suite w02c --reserved
cargo run --locked -p mucrab-compare -- --suite w03
cargo run --locked -p mucrab-compare -- --suite w03 --reserved
cargo run --locked -p mucrab-compare -- --suite w04
cargo run --locked -p mucrab-compare -- --suite w04 --reserved
cargo run --locked -p mucrab-compare -- --suite w05
cargo run --locked -p mucrab-compare -- --suite w05 --reserved
```

W04 comparisons and Rust move tests require unprivileged execution and two real
filesystems: the verified host uses repository ext4 and a different `/tmp` mount.
The comparison suite fails as unavailable if that arrangement cannot be
established; EXDEV is never mocked. Linux GUI checks also require Xvfb,
X11/XTest tools and Python Pillow.

The runner exits 0 for a complete scoped pass, 1 for differences, and 2 for unavailable/incomplete execution. Raw results are kept under ignored `artifacts/runs/`; compact reports are in each directory under `evidence/workflows/`. W01 requires a real access-denied fixture, so running with privileges that can read it cannot produce a passing W01 result.

Workspace checks:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
python3 -B -m unittest discover -s tools -p 'test_*.py' -v
```

The Rust suite has twenty-six tests covering recovered rules, navigation/error state,
filtering, metadata/timestamp rules, selection persistence and pane isolation, link enumeration, and the zero-case
readiness guard, plus copy/move integrity, real cross-device fallback, failed source deletion, focused rename, batch preview/apply, stale-plan rejection and boundary checks. The Python helper/protocol suite has thirty-three tests. The separate
Java/Rust comparison suites supply the behavioral evidence. Once dependencies
are cached, Cargo build, test, and Clippy commands also work with `--offline`.

Click a row to give its listing keyboard focus. Ctrl-click toggles a mark and
Shift-click toggles a range; Space toggles and advances. Up/Down, Home/End, and
Shift+Up/Down move/select; Tab switches panes. Cmd+A/Cmd+D (Ctrl on non-macOS)
mark/unmark all; All/None/Invert buttons are also available. F5 reviews a copy
to the other pane; F6 reviews a move and F2 renames the focused child, even when
other children are marked. **Batch rename…** reviews the effective selection
with live generated names and an explicit Apply button. Batch operations skip
existing files; copy/move/single rename initially select the same policy. Successful
top-level items are unmarked, skipped items stay marked, and both panes refresh.
The result details report failed children even when their parent folder was
processed. The worker has no pause/cancel controls yet. A focused row is
highlighted and marks use `[x]`. With no marks, the focused child is the effective
selection. The parent row can be focused but is never included in file selection.

Capture the native browser and exit automatically (verified on macOS):

```sh
mkdir -p artifacts/workspace
EFRAME_SCREENSHOT_TO="$PWD/artifacts/workspace/mucrab-preview.png" \
  cargo run --locked -p mucrab-desktop --features screenshot
```

The optional `screenshot` feature is for development captures.
[Workspace readiness evidence](evidence/workspace/readiness.json) is the historical
phase 4 scaffold snapshot. [W05 evidence](evidence/workflows/W05-batch-rename/README.md),
[validation](evidence/workflows/W05-batch-rename/validation.json) and
[regression results](evidence/workflows/W05-batch-rename/regressions.json) record
the latest implementation checks. A screenshot supplements runtime comparisons.

## Reference application

The reference is muCommander **1.6.2**, release tag **`1.6.2-1`**:

- [Official release](https://github.com/mucommander/mucommander/releases/tag/1.6.2-1).
- [Binary distribution](https://github.com/mucommander/mucommander/releases/download/1.6.2-1/mucommander-1.6.2-1-portable-x86_64.zip).
- Published archive SHA-256: `e6769537bbb56cc52c1289ebef2e3d051912bb8b10691d320f28b945f15819a4`.

The input is the complete distribution, including the launcher, application JARs,
dependencies, resources, and notices. This portable archive includes a Windows
executable launcher and a shell launcher, with no bundled JRE. Original classes
and isolated GUI copies have been exercised on macOS arm64 for earlier workflows
and Linux x86_64 through W05. Those checks do not verify unrelated bundled
components. Acquisition preserves the distribution layout and does not execute
its contents.

## Acquire or verify the reference

Requirements: Python 3.9 or newer, `curl`, and network access for the first download. From the repository root:

```sh
python3 -B tools/acquire_mucommander.py
```

The [pinned manifest](tools/manifests/mucommander.json) supplies the official release URL, archive size, and SHA-256. The script verifies the archive before extraction, checks all extracted file contents, and confirms the launcher version, implementation revision, and preserved license notice. Repeating the command verifies the cached archive and distribution before reusing them. A mismatch stops acquisition and reports the affected input.

- Download: `artifacts/downloads/mucommander-1.6.2-1-portable-x86_64.zip`.
- Extracted reference: `artifacts/reference/mucommander-1.6.2-1/`.
- Launcher JAR: `artifacts/reference/mucommander-1.6.2-1/mucommander-1.6.2.jar`.
- Core JAR: `artifacts/reference/mucommander-1.6.2-1/app/mucommander-core-1.6.2.jar`.
- Full file inventory: `artifacts/inventories/mucommander-1.6.2-1.json` (ignored).
- [Acquisition evidence](evidence/acquisition/mucommander-1.6.2-1.json) (tracked).

The acquired distribution contains **202 files and 196 JARs**. The compact evidence records the input digest, distribution counts, and selected file digests; the local inventory contains the digest of every extracted file. These records establish acquisition and byte integrity only.

Verify the local reference and its evidence without network access or writes:

```sh
python3 -B tools/acquire_mucommander.py --verify-only
```

Check acquisition failure handling and extraction boundaries:

```sh
python3 -B -m unittest discover -s tools -p 'test_*.py' -v
```

Keep the extracted tree unchanged as the binary reference. Runtime tests use
separate working copies, disposable configuration and isolated fixture trees.

## Analysis tools

The [analysis-tool manifest](tools/manifests/analysis-tools.json) pins **REA 6.5.0** and **CFR 0.152**. REA's dependencies are pinned in [the npm lockfile](tools/rea/package-lock.json). The latest probe ran on Linux x86_64 with Node **24.15.0**, npm **12.0.2**, and OpenJDK / `javap` **25.0.4.1**. Setup requires a Node version supported by REA and a full JDK 17 or newer; it uses the installed runtimes rather than installing them.

Install the tools locally, with npm lifecycle scripts disabled:

```sh
python3 -B tools/analysis_tools.py setup
```

Repeat installation from the populated local cache without network access:

```sh
python3 -B tools/analysis_tools.py setup --offline
```

Run the static capability probe against the verified muCommander reference:

```sh
python3 -B tools/analysis_tools.py probe
```

The probe does not download tools, execute muCommander classes, or modify global agent configuration. It writes raw command arguments, stdout, stderr, bytecode, and decompiler output under ignored `artifacts/analysis/toolchain/`. Its compact [readiness evidence](evidence/toolchain/readiness.json) is tracked in Git and records each producer, version, command, output digest, and limitation.

| Producer | Observed capability | Boundary |
| --- | --- | --- |
| REA `inspect-artifact` | Inventoried the file-library JAR and verified the selected class fingerprint | Coverage concerns ZIP entries; the class format is reported as `unknown`, with no JVM method semantics |
| REA `inspect-android-package` | Explicitly rejected the JAR because it requires a standalone APK | This is an expected format refusal, not a successful Java decompilation |
| CFR | Decompiled `FileComparator` into 163 lines of Java with its dependencies resolved | One static smoke test; output correctness is not established |
| JDK `javap` | Returned method signatures and bytecode instructions for the same class | Disassembly supports investigation; it does not establish equivalence |

The selected class was previously examined during the feasibility investigation. This repeatable probe establishes tool readiness and must not be presented as a blind reconstruction or a new behavioral discovery. The input class has JVM class-file major version **55**.

REA is installed under ignored `tools/rea/node_modules/`; CFR is stored under ignored `tools/.cache/cfr/`. The npm lockfile carries REA's registry integrity value. CFR's SHA-256 was computed from the official author download and pinned locally; the author's published MD5 also matches. That SHA-256 is a reproducibility pin, not a publisher-supplied signature. Preserved tool licenses and provenance are listed in [NOTICE.md](NOTICE.md).

No JADX, Hopper, Ghidra, or other native analysis engine is needed for these probes.
Native, browser, and instrumentation providers are outside the static probe's
verified capabilities. Original-Java execution, scoped comparisons and GUI
observations are recorded separately for each implemented workflow.

## Initial demo scope

| Workflow | Intended comparison |
| --- | --- |
| Two-pane navigation and selection | Locations, listings, and selection behavior |
| Sorting and filtering | Ordering, numeric names, locale behavior, and ties |
| Recursive copy, move, and rename | Resulting paths, contents, and selected metadata |
| Batch renaming | Generated names, collision handling, and errors |
| ZIP browsing and extraction | Entry names and extracted contents |
| Background file operations | Completion, cancellation, and failure outcomes |

This scope is an initial demonstration target. Complete muCommander feature parity, remote and cloud filesystems, additional archive formats, and support across operating systems require further increments.

## Execution sequence

The [implementation plan and progress ledger](mucrab-implementation-plan.md) records the current checkpoint, machine handoff instructions, workflow gates, and evidence requirements. Update it during implementation so work can resume in another session or checkout.

1. **Complete:** establish and commit this repository baseline.
2. **Complete:** add reproducible acquisition scripts and a checksum manifest; download and extract the reference into ignored local storage.
3. **Complete for static inspection:** pin REA and a JVM decompiler, verify JAR inventory, the APK-only refusal, class decompilation, and bytecode inspection against the reference, and record tool provenance and limits.
4. **Complete:** create a GPLv3 Cargo workspace separating the Rust core, native desktop interface, and comparison-harness entry point; commit `Cargo.lock`, verify the build and unavailable-harness guard, and capture the native preview.
5. **In progress:** W01 browsing/name sorting, W02a selection/marking, W02b hidden-file filtering, W02c size/date/extension sorting, bounded W03 recursive copy, W04 move/rename and W05 batch rename are verified within their contracts. Next is W06 ZIP browsing/extraction, applying the same recovery and comparison gates. Permissions/owner/group sorting and additional filters remain deferred.
6. Run the agreed comparison suite and demonstrate fresh scenarios with the compiled Rust application running independently of Java.

Upstream Java implementation source will be withheld during reconstruction; license and attribution material remains available. Decompiled representations, binary metadata, and observations of the original are permitted inputs. Findings from the earlier feasibility investigation must be identified as prior evidence if reused.

## Verification approach

Each workflow will have explicit inputs and acceptance criteria established before its Rust implementation. A harness will exercise the original JARs and the Rust implementation against equivalent fixtures, comparing observable results and recording discrepancies. Any normalization of paths, timestamps, or platform details must be stated explicitly.

W01 through W05 have scoped original-JAR/Rust comparisons, reserved cases,
integrity checks and representative GUI evidence. The broader demo still needs
ZIP handling and cancellation coverage; the existing case counts
do not close those acceptance targets.

Current fixtures include numeric and Unicode names, existing destinations,
symbolic links, unreadable files, and partial copy/move failures. File operations
run inside disposable fixture directories. Cancellation fixtures remain for W07.
GUI demonstrations supplement the behavioral comparisons. Build success alone
does not establish equivalence.

The experiment will distinguish evidence that binary-to-Rust reconstruction works from evidence that REA improves the process. Tool provenance, human interventions, and remaining gaps will be recorded with the results.

## Repository layout

- `artifacts/`: ignored downloads, extracted distributions, and generated decompiler output.
- `tools/`: reproducible acquisition, inspection, and comparison helpers.
- `crates/`: Rust application and comparison components.
- `fixtures/`: reproducible test inputs.
- `evidence/`: selected provenance records and comparison results suitable for Git.
- `third_party/`: preserved upstream notices and their provenance.

The repository includes acquisition/analysis helpers, pinned manifests and
lockfiles, the native workspace, W01/W02a/W02b/W02c/W03/W04/W05 Java/Rust adapters
and fixtures, scoped results, GUI evidence, and the maintained implementation
plan. W06 and W07 remain unimplemented; W05 manual row overrides remain deferred.

## License and attribution

muCrab-authored material is licensed under the **GNU General Public License, version 3** (`GPL-3.0-only`); see [LICENSE](LICENSE). Upstream material retains its original copyright notices and applicable terms. See [NOTICE.md](NOTICE.md) and the [preserved muCommander license file](third_party/mucommander/license.txt).

Distribution of the Rust executable will include the corresponding buildable source, build instructions, the GPL text, and applicable notices. Additional dependencies and copied material must retain their own notices and be compatible with the application's distribution terms.
