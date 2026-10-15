# muCrab implementation plan and session handoff

**Application:** muCrab · **Command:** `mucrab` · **Tagline:** Two claws, two panes.

This is the durable execution plan and progress ledger for migrating selected
muCommander workflows from published Java binaries to Rust. Keep this file in
Git and update it at every implementation checkpoint so another machine or
session can continue without the original conversation.

## Live checkpoint

| Field | Current value |
| --- | --- |
| Last updated | 2026-10-15 |
| Last verified implementation commit | `212d724` — Migrate verified local browsing and name sorting to Rust |
| Current phase | Phase 5: migrate one verified workflow at a time |
| Active workflow | W02 — recover the next selection/filtering/sort-criteria contract; implementation not started |
| W01 status | `verified` within en-US/macOS arm64/APFS scope; representative original GUI evidence |
| Next executable task | Inspect original selection and filtering callers, choose one bounded W02 subworkflow, and freeze its contract before implementation |
| Ported workflows | W01 — local browsing and recovered name ordering |
| Behavioral comparison cases run | 342 configuration/error cases: 187 predefined + 155 reserved; 45,696 pair signs, 336 lists, 54 enumeration/error checks; zero differences |
| Known blocker | None within W01 scope; other locales/platforms and deferred behaviors require separate increments |
| Verification platform | macOS arm64, APFS with case-insensitive/canonically equivalent lookup; other platforms unverified |

The commit above identifies the implementation baseline, not the commit adding
this plan. On resumption, inspect the actual checkout and reconcile this table
with code and evidence before continuing. Historical checks are not evidence
that the new machine has passed them.

## Objective and experiment rules

Produce a convincing native Rust file manager demo from recovered binary
behavior, with reproducible comparisons against muCommander. Implement one
complete, observable workflow before expanding the scope.

- Keep muCrab-authored code and documentation under `GPL-3.0-only`. Preserve
  upstream notices in `LICENSE`, `NOTICE.md`, and `third_party/`. Include the
  corresponding buildable source and applicable notices when distributing.
- Use the pinned official binaries, decompilation, bytecode inspection, and
  runtime observations. Withhold upstream Java implementation source during
  reconstruction. License and attribution material may be consulted.
- Record reused findings from the earlier feasibility investigation. The
  comparator has already been examined; this is not a blind reconstruction.
- Keep the acquired reference tree unchanged. Run the Java application from a
  separate working copy with disposable configuration and fixture directories.
- Require the original binary as the behavioral oracle. A rewritten Java
  comparator or decompiled source recompiled into Java is not that oracle.
- Build success, a rendered window, and REA container coverage do not establish
  behavioral equivalence. Record REA's contribution separately from CFR,
  `javap`, manual investigation, and Rust implementation work.
- Keep the final Rust application independent of Java. Java remains a test
  dependency for reference comparisons.

## Established state and evidence

| Phase | State | Evidence / implementation |
| --- | --- | --- |
| 1. Repository and licensing | Complete | `9225bc9`; `LICENSE`, `NOTICE.md`, preserved notices |
| 2. Binary acquisition | Complete | `15427e7`; `tools/acquire_mucommander.py`, `tools/manifests/mucommander.json`, `evidence/acquisition/mucommander-1.6.2-1.json` |
| 3. Analysis toolchain | Complete for static inspection | `9996a3e`; `tools/analysis_tools.py`, pinned manifests and npm lockfile, `evidence/toolchain/readiness.json` |
| 4. Cargo workspace and preview | Complete | `88baf29`; `Cargo.toml`, `Cargo.lock`, `crates/`, `evidence/workspace/readiness.json` |
| 5. Verified workflows | W01 verified; W02 not started | `evidence/workflows/W01-local-browse-sort/`; W01 gates below |
| 6. Broader comparison and demo | Not started | Final acceptance below |

The workspace contains:

| Package | Current behavior | Intended responsibility |
| --- | --- | --- |
| `mucrab-core` | Local enumeration, pane state, recovered comparator/table sorting, ICU4X collation | Recovered behavior, independent of the GUI |
| `mucrab-desktop` | Native two-pane local browser using eframe/egui 0.36.2 and OpenGL | Navigation and independent sort controls; later selection/operation UI |
| `mucrab-compare` | Original-JAR/Rust W01 adapters and scoped suites; status alone reports `not_run`/zero cases/exit 2 | Run fresh comparisons and report differences |

Historical checks passed on the baseline: Cargo build, formatting, Clippy with
warnings denied, Rust tests, CLI checks, and native preview capture. The Rust
suite has **one readiness guard and zero migrated-behavior tests**. The Python
helper suite had **nine passing tests**. These counts describe the baseline,
not the eventual acceptance suite.

W01 adds **five passing Rust tests**, **twelve passing Python tests**, the two
passing Java/Rust suites, and original/native GUI observations. The original
comparator and table sorting execute directly from the pinned JARs. Both native
panes browse regular folders and sort independently. File mutations and the
remaining workflows are unimplemented. Historical phase 4 readiness files still
describe the old scaffold; current evidence is under
`evidence/workflows/W01-local-browse-sort/`, including retained GUI captures.

### Pinned inputs

Use `tools/manifests/mucommander.json` and
`tools/manifests/analysis-tools.json` as the owning sources for download URLs
and pins; do not silently switch to newer releases.

| Input | Pin |
| --- | --- |
| muCommander | Version `1.6.2`, release `1.6.2-1`, implementation revision `3bc2118` |
| Official archive | `mucommander-1.6.2-1-portable-x86_64.zip`; 141,934,827 bytes |
| Archive SHA-256 | `e6769537bbb56cc52c1289ebef2e3d051912bb8b10691d320f28b945f15819a4` |
| File-library JAR | `bundle/mucommander-commons-file-1.6.2.jar` |
| File-library JAR SHA-256 | `bb3a456fb42d40b2551b90bc5649b789931ddee9df79c1c2df771a74cafe641e` |
| Comparator class | `com.mucommander.commons.file.util.FileComparator` |
| Comparator class SHA-256 | `d4bc2dfe6884a78916248afef758831efb91863bf5dc86788c5ead29fde54a2a` |
| REA | npm package `rea-agents`, version `6.5.0`; registry integrity in `tools/rea/package-lock.json` |
| CFR | `0.152`; SHA-256 `f686e8f3ded377d7bc87d216a90e9e9512df4156e75b06c655a16648ae8765b2` |
| ICU Java dependency | `bundle/icu4j-78.3.jar` in the verified distribution |
| Other comparator dependency | `bundle/commons-lang3-3.20.0.jar` in the verified distribution |

The complete distribution contains 202 files and 196 JARs, with no bundled JRE.
It includes native components whose runtime compatibility is unverified. CFR's
SHA-256 is a locally established reproducibility pin, not a publisher signature.

REA successfully inventories the JAR and verifies the selected class hash, but
reports its class format as `unknown`. Its coverage is for container entries.
The Android/JADX route explicitly rejects this JAR because it requires an APK.
CFR and `javap` provide the JVM representations used in this experiment. Do not
spend a migration increment trying to feed an ordinary JAR into the APK route.

## Move to another machine and resume

All paths below are relative to the repository root. No original absolute path
is required. Transfer the Git repository, including this plan, rather than this
file alone. The observed `gdrive` remote points to a local filesystem path on the
original machine; its presence does not establish availability elsewhere.

For a self-contained Git handoff, commit the intended checkpoint and create a
bundle on the source machine:

```sh
git status --short --branch
git bundle create /tmp/mucrab-handoff.bundle --all
git bundle verify /tmp/mucrab-handoff.bundle
```

Copy the bundle using your chosen transfer mechanism. On the destination:

```sh
git clone /path/to/mucrab-handoff.bundle mucrab
cd mucrab
git status --short --branch
git log -5 --oneline
```

A bundle includes committed history and refs; it does not include uncommitted
work or ignored artifacts. The clone's remote will refer to the bundle file;
configure a writable remote separately if needed. No push is required to resume.

### Host prerequisites

- Git, Python 3.9 or newer, and `curl`.
- Rust 1.95 or newer with Cargo, rustfmt, and Clippy. The verified compiler was
  Rust/Cargo 1.99.0; the workspace has no exact toolchain pin. Record the compiler
  used on the destination.
- A Node version supported by the pinned REA manifest:
  `^22.19.0 || ^24.11.0 || >=26.0.0`, with npm. Verified: Node 24.12.0 / npm 11.7.0.
- A full JDK 17 or newer, including `java`, `javac`, `javap`, and `jar`. Verified
  static analysis used OpenJDK 21.0.10. Prefer the same JDK version for the first
  behavioral baseline; record any difference. `java` and `javap` must use the
  same JDK major version.
- The host's native build tools and desktop libraries required by the existing
  Cargo dependencies, plus a graphical desktop for UI checks. Only macOS arm64
  has been verified; establish other hosts separately.

### Rehydrate and check the baseline

Review repository instructions, this plan, `README.md`, and `NOTICE.md` first.
Check the live state against Git before editing. Then, from the repository root:

```sh
python3 -B tools/acquire_mucommander.py
python3 -B tools/acquire_mucommander.py --verify-only
python3 -B tools/analysis_tools.py setup
python3 -B tools/analysis_tools.py probe
python3 -B -m unittest discover -s tools -p 'test_*.py' -v
cargo build --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo run --locked -p mucrab-desktop -- --version
```

Run the comparison status separately because its baseline exit code is
intentionally nonzero:

```sh
cargo run --locked -p mucrab-compare -- --status
```

At the current W01 checkpoint, expect exit code **2**, `status: "not_run"`, and
`cases_run: 0`. Status alone does not execute comparisons or reuse historical
passes. The earlier phase 4 baseline used `not_ready`. Run fresh W01 comparisons:

```sh
cargo run --locked -p mucrab-compare -- --suite w01
cargo run --locked -p mucrab-compare -- --suite w01 --reserved
```

Expect exit 0 and scoped passing reports when all required cases run. Exit 1
means differences and exit 2 means unavailable/incomplete execution. Raw runs
are under `artifacts/runs/`; compact reports update the W01 evidence directory.
Run `python3 -B tools/w01_inspect.py` to regenerate scoped class recovery.
`python3 -B tools/w01_gui.py` prepares isolated GUI launch commands; optional
`--mac-apps` creates local wrappers for macOS UI automation. See the W01 evidence
README for the complete procedure.

For a GUI check, run `cargo run --locked -p mucrab-desktop`. The existing optional
capture command renders the window, writes a PNG, and exits automatically:

```sh
mkdir -p artifacts/workspace
EFRAME_SCREENSHOT_TO="$PWD/artifacts/workspace/mucrab-preview.png" \
  cargo run --locked -p mucrab-desktop --features screenshot
```

The acquisition and probe scripts write generated evidence, including tracked
receipts. Inspect `git diff` after running them. Preserve prior committed
evidence in history, record the new host/run, and explain changed outputs. A new
JDK, absolute path, or platform may change raw output without changing inputs.
Do not reuse an old GUI capture or a prior host's test result as a fresh pass.

Ignored directories (`artifacts/`, `target/`, `tools/.cache/`,
`tools/rea/node_modules/`, `evidence/local/`) are absent from a normal clone. The
scripts regenerate the reference, inventory, decompilation, and tools. For an
offline handoff, separately transfer the verified archive, tool caches, and
Cargo dependency cache/vendor material needed by the destination, then verify
them. `setup --offline` and Cargo `--offline` require populated caches; they do
not bootstrap a blank machine. Do not transfer credentials or global agent
configuration. Rebuild native output for the destination architecture.

## Phase 5: W01 — local browsing with verified name sorting

The first workflow is: **open a local folder, display its entries in the
recovered name order, enter a subfolder, and return to its parent in either
pane**. A comparator-only milestone is useful but does not complete W01.

Start with `NAME`, ascending/descending, directories-first on/off, natural and
lexicographic modes, one explicit locale, and one explicitly recorded OS and
filesystem. `en-US` is the selected initial locale, confirmed in the disposable
original GUI profile. Other criteria, filtering, file mutations, archive
browsing, and cross-platform equivalence belong to later increments.

Specify hidden-file, symbolic-link, parent-entry, and error behavior in the
contract. If an item is deferred, make that boundary explicit in the UI and
evidence. An unknown behavior is not an implicitly passing case.

### G1 — prove the original Java adapter can run

1. Inspect the verified JAR inventory and `javap` output to locate concrete
   local-file implementations and required protocol initialization. An earlier
   guess at `com.mucommander.commons.file.impl.local.LocalFile` was not found;
   discover the actual classes rather than relying on that package name.
2. Implement the smallest adapter that constructs original file objects and
   invokes the original `FileComparator`. Compile adapter glue only; load the
   comparator and its dependencies from the acquired JARs. Resolve any logging,
   protocol-provider, OSGi, or other initialization requirements from evidence.
3. Use real temporary files and directories for the first invocation. Print
   machine-readable comparison signs and stable fixture identifiers, with
   diagnostics kept separate from the protocol output.
4. Record the JAR/class hashes, exact classpath, adapter source, compiler and
   runtime versions, command arguments, exit code, stdout/stderr, and fixture.

**Gate:** a repeatable command invokes the original class successfully and
returns a known file/file and directory/file comparison. No Rust sorting work
is required to reach this gate. If direct invocation needs application startup,
investigate a minimal initialized runtime; document the dependency instead of
substituting a duplicate implementation. Mark G1 blocked if unresolved.

### G2 — recover callers and freeze the behavior contract

Trace comparator construction and listing/sorting call sites in the core JAR
using CFR and `javap`. Establish the actual `nameFunc`, selected locale, option
defaults, enumeration input order, tie handling, directory classification,
hidden-file settings, symlink behavior, and handling of the parent entry.

Run the original GUI from a separate working copy with a disposable profile
against the same fixtures. Verify its startup and configuration isolation
mechanism from the bundled launcher/help or binary inspection before relying
on a guessed flag. Check that the adapter's settings correspond to the GUI.
Record screenshots and observed listings; a comparator adapter alone does not
prove the application's complete listing pipeline.

Maintain the recovered contract at
`evidence/workflows/W01-local-browse-sort/contract.md`. Define input fields,
locale/options, file identity, output order, error outcomes, normalization,
supported platform, and exclusions before implementing the Rust behavior.

**Gate:** every claimed W01 behavior has a binary/runtime evidence reference
and a testable expected outcome. Distinguish static inference from observation.

### Comparator findings and recovery correction

These were first recovered statically and are now tested through the original
Java comparator and table sorter in the declared W01 suite. Regenerate the
probe or scoped recovery on a new machine to inspect:

- `artifacts/analysis/toolchain/decompiled/com/mucommander/commons/file/util/FileComparator.java`
- `artifacts/analysis/toolchain/javap-sorter.stdout`

The public constructor accepts criterion, ascending, directories-first,
`Function<AbstractFile, String>`, `Locale`, and `Mode`.

- Directories-first returns before descending inversion. A directory therefore
  remains before a file even in descending mode when this option is enabled.
- Natural mode considers the **first** numeric group only when both groups
  begin at the same position and their preceding prefixes are exactly equal,
  or both start at position zero. It strips leading ASCII zeroes, compares
  remaining numeric lengths and digits, then falls back to collation on a tie.
- Fallback uses ICU4J `Collator.getInstance(locale)` at strength 2, then strength
  0 if the first comparison is equal. A generic natural-sort crate or Unicode
  string comparison must not be presumed equivalent.
- Other criteria include size, date, extension, permissions, owner, and group,
  with name comparison as a tie-breaker. They are outside the first contract.
  Later work must account for unknown size, nulls, and Java arithmetic behavior.
- Recovery corrected the initial stable-sort assumption: `FileTableModel`
  implements a custom quicksort that can reorder equal entries. W01 invokes
  the original private method and reproduces its index permutation in Rust.

**Resolved within the tested scope:** ICU4X `icu_collator` 2.2.1 with compiled
collation data 2.2.0 matches the original ICU4J 78.3 comparator on both W01 suites.
Other locales/data versions remain unverified. Do not silently replace locale collation with
lowercasing, byte order, or a host locale. Do not shrink the fixture suite to
conceal mismatches. If exact compatibility cannot be achieved, retain a failed
or blocked gate and describe the supported subset explicitly.

### G3 — build fixtures and the comparison protocol

Maintain deterministic, versioned fixture definitions under
`fixtures/W01-local-browse-sort/`. Generate runtime directories in ignored
`artifacts/runs/<run-id>/`, not under the immutable reference tree.

Cover the eight combinations of direction, directories-first, and mode for
each applicable fixture, using the frozen locale. Include:

- Plain names, mixed file/directory entries, empty directories, and ties.
- Numeric names (`2`, `10`), embedded numbers (`file2`, `file10`), leading zeroes,
  all-zero groups, long digit runs, and multiple groups (`x1y2`, `x1y10`).
- Case differences, accents, punctuation, spaces, Unicode names, and differing
  prefix case (`A2`, `a10`) to challenge the numeric-prefix rule.
- Repeated visits and the chosen behavior for hidden entries, symlinks, missing
  paths, and access failures. Confirm access-denied fixtures really deny access
  on the test host; do not call a privileged successful read an error-case pass.

Test invalid/empty names and metadata-only edge cases through a separately
identified synthetic adapter layer where useful; they are not real directory
entry evidence. Filesystem restrictions and Unicode normalization differ by
host. Record names that cannot coexist on the selected filesystem rather than
assuming all synthetic cases can be materialized.

Feed identical entry identities, explicit input order, options, and locale to
both adapters. Compare **pairwise signs** (`-1`, `0`, `1`) and **complete ordered
lists**. Sign comparison avoids requiring identical comparator magnitudes.
Use identical declared input order, then invoke the original table quicksort
and compare its complete index permutation, including ties. Recovery showed
the table does not preserve all comparator ties. Do not substitute a stable
sort or invent a new filename tie-breaker. Real enumeration sets are checked
independently; filesystem enumeration order is not promised across hosts.

**Gate:** the original adapter can produce repeatable results for the frozen
fixtures, and the protocol can represent ties, errors, and unsupported cases
without reporting them as matches.

### G4 — implement Rust behavior and close differences

Implement the recovered data model and sorting in `mucrab-core`. Add a Rust
adapter and extend `mucrab-compare` to run the real Java and Rust implementations
against the fixtures. Keep Java loading and test orchestration out of the core
and desktop runtime.

Add meaningful tests for recovered rules and previous discrepancies. Generate
additional deterministic cases with a recorded seed and reserve fresh cases
for validation after implementation. Investigate each difference against
bytecode, dependencies, and runtime behavior; record the cause and resolution.

The implemented harness contract is exit 0 for a completed passing comparison,
exit 1 for differences, and exit 2 for unavailable adapters or an incomplete
run. A pass requires more than zero executed cases, all required cases run,
and no unresolved differences. A partial match is not a global success. Keep
the existing readiness guarantee, updating its test as capabilities grow.

**Gate:** all required pairwise and list comparisons pass for the declared
scope, including reserved cases. Every exclusion and normalization is visible.
No Java subprocess or JVM library is needed by the compiled Rust application.

### G5 — connect the verified core to both panes

Implement folder entry, child/parent navigation, directory listing, sort mode,
direction, and directories-first controls. Keep each pane's location and
options independent. Use the same core function exercised by the Rust adapter.
Avoid adding unverified file mutation controls during W01.

Run the real GUI against the fixtures. Check both panes, empty folders, the
declared error states, option changes, and repeated navigation. Compare the
visible results with the original application under equivalent settings.
Capture fresh native screenshots or a short demonstration with fixture IDs.

**Gate:** the full W01 action sequence works in both panes and produces the
verified ordering. Comparator evidence, enumeration evidence, and GUI evidence
must all exist; a screenshot alone cannot close this gate.

### G6 — close and checkpoint W01

- Run the appropriate helper and Rust checks listed in the resume section.
- Verify reference integrity again with `acquire_mucommander.py --verify-only`.
- Save compact results and provenance, a discrepancy log, and GUI observations.
- Update this plan, `README.md`, and harness usage to reflect actual behavior.
- Review the diff, commit the completed increment, and record its commit in the
  next checkpoint. Do not expand to W02 while required W01 differences remain.

**Gate:** another session can reproduce the scoped result from the committed
sources, pinned inputs, fixtures, commands, and evidence. Mark W01 complete only
when G1–G6 are verified.

## Remaining workflow queue

Apply the same cycle to every increment: recover → contract → Java oracle →
fixtures → Rust → comparisons → UI → evidence and commit. Split large workflows
further when their acceptance criteria cannot be closed in one increment.

| ID | Proposed increment | Required observable evidence | State |
| --- | --- | --- | --- |
| W01 | Local folder browsing and name sorting | Entry identities, pairwise signs, ordered listings, both-pane native navigation | `verified` within contract |
| W02 | Selection, filtering, and additional sort criteria | Selected identities, visible entries, metadata ordering and ties | `not_started` |
| W03 | Recursive copy | Paths, byte contents, empty folders, declared metadata, collisions, errors | `not_started` |
| W04 | Move and rename | Destination results, source removal rules, collision and failure outcomes | `not_started` |
| W05 | Batch rename | Generated names, preview/apply consistency, collisions, partial failures | `not_started` |
| W06 | ZIP browsing and extraction | Entry listing, extracted paths and bytes, supported edge/error cases | `not_started` |
| W07 | Background operations and cancellation | Completion, cancellation boundaries, partial results, UI responsiveness | `not_started` |

Order after W01 is provisional and may be changed with a recorded reason.
Mutating tests must use separate equivalent fixture copies for Java and Rust.
Measure results and partial failure states, not just return codes. These
increments exclude full muCommander parity, remote/cloud filesystems, and
additional archive formats unless explicitly added to the scope.

## State and evidence maintenance protocol

This document is the human-readable progress ledger; committed evidence is the
proof. W01's tools, fixtures, and evidence now exist. Create records for later
workflows when their work starts; do not infer results from a planned path.

Use states `not_started`, `in_progress`, `blocked`, and `verified`. For each
workflow, track G1–G6 individually. `verified` requires a reproducible evidence
record; a passing unrelated test suite cannot advance a behavior gate.

At the start of every session:

1. Read this plan and repository instructions; inspect Git HEAD, status, and
   changes. Preserve unrelated work and reconcile a stale checkpoint.
2. Locate the last evidence and its input/source hashes. Determine what is
   historical, reproducible, missing locally, or invalidated by changes.
3. Rehydrate missing ignored inputs and establish the destination environment.
4. Continue the first unfinished gate. Do not repeat completed implementation
   merely because a new conversation has no earlier messages.

At every gate or session end:

1. Update the live checkpoint, gate ledger, open issues, and exact next task.
2. Write a session entry using the template below. Record commands and observed
   outcomes, including failures, skipped cases, and work left uncommitted.
3. Keep generated/raw output under `artifacts/runs/<run-id>/`; commit compact
   workflow evidence under `evidence/workflows/<workflow-id>/`. Store portable
   fixture definitions, adapter sources, and commands in Git. If raw output is
   required to audit a result, provide a portable retained copy or regeneration
   instructions; an ignored path alone is insufficient for handoff.
4. Record date/time, OS/architecture/filesystem, tool versions, locale, input
   JAR/class hashes, fixture digest, adapter source revision/hash, Cargo lock
   digest, options, exact commands, exits, case totals, matches, differences,
   skips, and raw-output digests. Use repository-relative paths or documented
   placeholders for machine-specific roots. Record the source revision tested
   separately from the later commit containing the result.
5. Never overwrite a failed result with an unexplained pass. Keep run IDs and
   discrepancy history; mark superseded evidence and why it was invalidated.
6. Review and commit the intended checkpoint when appropriate. Record the last
   commit containing implementation/evidence; do not attempt to put a commit's
   own hash inside itself. Include every required artifact in the handoff.

Changes to comparator logic, collation data, oracle dependencies, fixture
generation, locale, or normalization invalidate the corresponding comparison
claim until rerun. A new host requires its own environment and GUI verification.

### W01 gate ledger

| Gate | State | Evidence / unresolved work |
| --- | --- | --- |
| G1: original Java adapter | `verified` | `tools/java/W01Oracle.java`; original `FileFactory`, `FileComparator`, and private table sorter execute; translation resources initialized |
| G2: recovered contract | `verified` within scope | `contract.md`, `recovery.json`; local name function, en-US settings, original initial/child listings and parent navigation observed |
| G3: fixtures and protocol | `verified` | Versioned fixture definitions/seeds, explicit synthetic-label layer, real folders/links/errors, full matrix/permutation validation |
| G4: Rust and comparison | `verified` within scope | Both comparison JSON reports pass; 342 configuration/error cases, zero differences or skips |
| G5: native workflow | `verified` within scope | Both native panes navigate and change options independently; empty/error/refresh checks; representative original GUI states match |
| G6: reproducible closure | `verified` for local development | Scripts, source/binary hashes, retained captures, validation, notices, and implementation checkpoint; release packaging remains open |

### Open issues

| ID | Issue | Next investigation |
| --- | --- | --- |
| R01 | Resolved: original comparator/table sorter execute | Keep original JAR/class pins and adapter initialization in regression checks |
| R02 | Scoped resolution: ICU4X matches en-US fixtures | New locales/data versions need new contract and comparisons |
| R03 | Scoped resolution: local callers and representative GUI states recovered | Selection/filtering and additional criteria belong to W02; exhaustive original GUI automation is not claimed |
| R04 | Original GUI launched on macOS arm64; unrelated native components/other hosts unverified | Establish each additional platform/provider separately |
| R05 | Ignored raw artifacts still need rehydration | Use W01 scripts and retained selected GUI evidence; regenerate or transfer required caches/raw runs |

### Session entry template

Copy this block for each future session and replace every placeholder:

```text
Date/time and run ID:
Host, architecture, filesystem, locale:
Starting Git revision and dirty state:
Workflow and gate:
Work completed / files changed:
Evidence paths and source/input hashes:
Commands run and observed results (including case counts):
Differences, failures, exclusions, and decisions:
Gate state after work:
Implementation/evidence commit, or uncommitted files:
Required local artifacts / regeneration commands:
Exact next executable action:
```

### Recorded sessions

- **2026-10-15 — implementation handoff:** created this plan from the current
  repository, committed readiness records, and recovered comparator output.
  Implementation baseline is `88baf29`. Phase 5 has not started; no new Java
  execution or Rust behavior verification was performed while writing this
  document. Next action is W01/G1.

- **2026-10-15 — W01 implementation:** started at `d80f776` on macOS arm64/APFS.
  Added original Java adapters, scoped CFR/javap recovery, fixture definitions,
  native Rust ICU4X comparison/table sorting, local enumeration, and both-pane
  navigation/sort controls. Corrected the stable-sort assumption and a Parent
  path-resolution edge case before closure. Predefined and reserved suites pass
  342 configuration/error cases with 45,696 pair signs, 336 lists, 54 enumeration
  checks, zero differences, and zero skips. Five Rust and twelve Python tests,
  Clippy, format, build, reference integrity, standalone linkage, and final
  render checks pass. Retained original/native screenshots and accessibility
  states are under the W01 evidence directory. Original GUI comparison is
  representative; original right-pane navigation and exhaustive GUI parity are
  not claimed. Implementation checkpoint: `212d724`.
  Raw comparison runs and generated wrappers remain ignored; use `tools/w01.py`,
  `tools/w01_inspect.py`, and `tools/w01_gui.py` to regenerate them. No upstream
  Java implementation source was fetched, no global agent configuration was
  changed, and no release or remote push was performed. Next executable action:
  recover and freeze one bounded W02 selection/filtering contract. Preserve W01
  as a regression gate.

## Final demonstration acceptance

After the selected workflows are verified, run an agreed suite targeting
approximately 100 predefined cases plus reserved fresh cases. Count distinct
scenarios and option combinations transparently; case volume is not a substitute
for coverage. Require every critical file-integrity case to pass and explain
every discrepancy or excluded behavior.

Demonstrate both panes, the supported sort/filter behavior, representative file
operations, and ZIP handling using fresh disposable inputs. Build the Rust
application in release mode, run it independently of Java, and publish the
commands, host constraints, binary/source provenance, notices, and comparison
results. Recheck the release build and applicable source/notice packaging.

Claims must remain scoped: passing this demo demonstrates reconstruction of
the verified workflows from these binary inputs. It does not establish full
muCommander parity or automated conversion of arbitrary Java applications.

## Prompt for a new execution session

> Continue muCrab using `mucrab-implementation-plan.md` as the progress ledger.
> Inspect Git, repository instructions, code, and committed evidence first;
> reconcile the live checkpoint before acting. Rehydrate ignored inputs on this
> machine and record fresh verification. Resume the first unfinished workflow
> gate. W01 is verified within its declared en-US/macOS scope; the next action
> is W02 contract recovery for one bounded selection/filtering subworkflow.
> Keep the W01 suites passing. Continue through each workflow's gates before expanding
> scope. Preserve GPL notices and the immutable reference, withhold upstream
> Java implementation source, and keep the native application independent of
> Java. Update the plan and evidence at each checkpoint, including exact next
> actions, unresolved differences, and committed/uncommitted state. Never
> report behavioral equivalence from the scaffold, static inspection, zero
> comparisons, or historical results alone.
