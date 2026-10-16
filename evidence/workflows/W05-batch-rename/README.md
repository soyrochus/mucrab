# W05 — bounded local batch rename

**Batch rename…** opens a live old/new-name preview for the active pane's effective
selection. Edit a mask, literal search/replacement, ASCII case mode and counter
controls, then explicitly Apply or Cancel. Masks support names/extensions,
UTF-16 ranges, independent counters, parent folder names and UTC modified dates.
Duplicate targets or targets matching other selected originals block application.
Unchanged rows do not enter the job. Existing files are skipped; folders merge.
Earlier successes remain after a partial failure, with per-path details.

The [frozen contract](contract.md) predates Rust and defines the exact subset.
[Recovery](recovery.json) pins 19 original classes, CFR/javap output and inputs.
The actual BatchRenameDialog parser/token/preview methods and BatchRenameJob
supply the oracle; no rewritten or recompiled Java behavior is used. REA's role
remains the earlier JAR inventory. No upstream implementation source was fetched.
[Decisions](decisions.md) records findings and the native guards.

| Measurement | Predefined | Reserved | Total |
| --- | ---: | ---: | ---: |
| Fixture sets | 54 | 32 | 86 |
| Complete preview snapshots | 218 | 128 | 346 |
| Ordered name-pair observations | 1,077 | 644 | 1,721 |
| Apply job instances (method boundary) | 37 | 32 | 69 |
| Ordered changed-row outcomes | 154 | 161 | 315 |
| Complete resulting tree comparisons | 54 | 32 | 86 |
| Selected-content conservation checks (Java + Rust) | 108 | 64 | 172 |
| Identity-relation checks | 243 | 161 | 404 |
| Differences / omitted cases | 0 / 0 | 0 / 0 | 0 / 0 |

The 54 predefined sets comprise 30 named and 24 generated sets (seed 755051).
The 32 reserved sets (seed 991075) stayed withheld until native implementation.
Each set can contain several complete option configurations, always including
an initial default preview. Preview counts are configurations/rows, not distinct
user journeys. Expected skips, blocked names, unchanged plans and failed jobs
remain measured cases. [Predefined](comparison-predefined.json) and
[reserved](comparison-reserved.json) reports pin source, fixtures, commands,
JAR/class bytes and native binary. [The original-only receipt](oracle-predefined.json)
records 54 original sets and zero Rust comparisons before implementation.

Important recovered details include last-dot splitting for folders/dotfiles,
removing just one trailing period after replacement, literal all-occurrence
replacement, one-based UTF-16 ranges, signed 32-bit counter parsing/wraparound,
per-token counter state, and counter reset after option changes. Word-initial
uppercase preserves the remaining characters. Conflict detection stops at the
first duplicate or selected-original conflict in row order. It does not create
temporary names to resolve swaps/cycles. Original Apply can remain enabled for
an unchanged or invalid-leaf preview; native additionally disables those cases.

The headless adapter bypasses the top-level window constructor only, supplies
real Swing fields/collections and FileSet values, and calls unchanged original
preview methods. Original removeUnchangedFiles prunes the job input; actual
BatchRenameJob chooses its original indexed new-name list before invoking
MoveJob. Its inherited collision field is conditioned to the original fixed
apply-to-all SKIP choice, and auto-skip errors is enabled. This method suite
excludes FileJob start/run/finish, notification and UI refresh. A separate full
GUI uses actual dialog construction, controls, confirmation and collision prompts.

Each implementation starts from a separate equivalent owned tree. Compare every
preview name pair, conflict/enable/count state, unchanged list, ordered outcome,
complete resulting tree, ordinary mode/date metadata, bytes/SHA-256 and literal
link targets. Preserve every selected byte stream/link at its old or mapped
path. Blocked/preview-only cases leave the entire tree unchanged. Record raw
inode/device IDs and compare only the logical old-entry/new-target identity
relationship. W04's static input-derived source-directory date mask covers
partial child removal; root/atime/ctime/link-inode dates remain unmeasured.
Inspection temporarily lifts denied fixture modes outside execution and restores
exact modes. Root/privileged execution cannot produce a pass.

The native plan snapshots selected-entry identity/metadata and regenerates names
before mutation; a stale plan fails before any rename. This is an extra integrity
guard, not a race-safety claim. Native application uses the reviewed names in
selection order, reuses W04's safe move implementation and skips unchanged rows.
Successful changed rows unmark, skipped/failed/unchanged identities retain marks
when still present, and both panes refresh without replacing their sort settings.

[GUI observations](gui-observations.json) retain a 41-action native XTest replay,
ten original stages/twenty pane states, four original preview snapshots, actual
confirmation and collision-Skip renders, two original full jobs, complete final
trees and a native partial folder merge checked against the original component
result. Native checks cover both panes, live replacement/counters, duplicate and
invalid-path prevention, Cancel, marks, refresh and partial details. Native PATH
is `/nonexistent`. The existing 40-action W03 copy and 41-action W04 move/rename
GUI replays also pass on the final binary. No pixel-equivalence is claimed.

The verified subset uses ASCII case conversion and UTC date tokens in years
1970–9999. Unicode names support unchanged case and ranges that preserve valid
UTF-16 pairs. Native rejects split-surrogate ranges, empty first-uppercase
results and padding above 12 digits. Manual per-row overrides, block/unblock,
removing preview rows and range-helper dialogs are recovered but unimplemented.
Other locales/timezones/calendar eras, arbitrary collision/overwrite prompts,
remote/archive providers, races, ACLs/xattrs/other owners/special bits,
mid-stream faults and full scheduling/cancellation remain outside this scope.
W04's symlink/overlap/alias guards remain; their guard tests are not equivalence
cases. Optional original bundle/logging warnings and physical/Wayland desktop
behavior remain unverified. Exact original focus/OS shortcut parity is not claimed.

[Validation](validation.json) records format, Clippy with warnings denied,
26 Rust / 33 Python tests, locked offline build, reference integrity and native
CLI/linkage without Java. [Regressions](regressions.json) retain all fourteen
predefined/reserved executions across seven suites. The core reuses already
pinned chrono 0.4.45 for UTC tokens; Cargo.lock adds only that local dependency
edge, with no package version refresh. Release source/notice packaging remains open.

Reproduce with cached inputs/dependencies, full JDK and unprivileged Linux:

```sh
python3 -B tools/w05_inspect.py
python3 -B tools/w05.py oracle
cargo build --workspace --locked --offline
target/debug/mucrab-compare --suite w05
target/debug/mucrab-compare --suite w05 --reserved
# Fresh independent GUI trees; replace RUN with the printed directory:
python3 -B tools/w05_gui.py
xvfb-run -a -s '-screen 0 1120x720x24' python3 -B tools/linux_gui.py RUN/native-launch.json --replay fixtures/W05-batch-rename/native-gui-actions.json
python3 -B tools/w05_gui.py --verify-native RUN
xvfb-run -a -s '-screen 0 1280x900x24' python3 -B tools/w05_gui.py --original-probe RUN
```

GUI checks require Xvfb, X11/XTest tools and Python Pillow. W04 regressions and
its Rust tests also require a genuinely different `/tmp` filesystem. Prepare a
fresh run before every mutating replay. Reference GUI runtimes are verified
copies with disposable profiles. Ignored raw runs must be regenerated or
transferred separately; tracked snapshots and hashes retain selected measurements.
The next workflow is W06 ZIP browsing/extraction.
