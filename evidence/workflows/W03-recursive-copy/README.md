# W03 — bounded recursive local copy

The native app copies the active pane's effective selection to the other pane.
F5 or the copy button opens a review of source, destination, names and collision
policy. SKIP is initially selected. Existing folders merge; regular files can
be skipped, overwritten, replaced if newer, or replaced if their size differs.
Hidden descendants, empty folders, binary data and contained relative links are
included. A worker keeps copying off the UI thread. Results distinguish collision
skips from failed children; earlier successful entries remain after an error.

[The frozen contract](contract.md) defines exact bounds and the original-method
adapter. The original-only run preceded Rust and froze 26 named fixtures,
12 generated sets (seed 733031), four policies with two SKIP-only overlap cases,
and a withheld reserved seed (991073, 24 sets). All are real filesystem fixtures.
[Recovery](recovery.json) records 17 original classes inspected through CFR and
javap. The two desktop-adapter classes were added to the recovery receipt during
closure; the original copy behavior was established before implementation.
REA's contribution remains the earlier JAR inventory, not Java-to-Rust conversion.

| Measurement | Predefined | Reserved | Total |
| --- | ---: | ---: | ---: |
| Copy cases (fixture/policy combinations) | 146 | 96 | 242 |
| Ordered top-level outcomes | 175 | 96 | 271 |
| Destination tree comparisons | 146 | 96 | 242 |
| Destination regular-file observations | 426 | 688 | 1,114 |
| Destination link observations | 16 | 0 | 16 |
| Source integrity checks (Java + Rust) | 292 | 192 | 484 |
| Differences / omitted test cases | 0 / 0 | 0 / 0 | 0 / 0 |

[Predefined](comparison-predefined.json) and [reserved](comparison-reserved.json)
reports pin original JAR/class bytes, adapters, fixtures, core/desktop source,
commands, outputs and native binary. These are 62 distinct fixture sets plus
policy combinations, not 242 distinct user journeys. Destination observations
include retained/colliding files; they are not counts of successful transfers.
Expected collision skips and copy failures are measured outcomes, never omitted
cases. [The original-only report](oracle-predefined.json) records 146 oracle
cases and zero Rust comparisons before implementation.

The observer independently compares relative paths, entry kinds, sizes/SHA-256,
mode bits, declared modified milliseconds and literal readlink targets, as well
as source contents and declared metadata before/after each execution. Java and
Rust start from separate equivalent trees. The inspector temporarily grants
read/search access to denied fixture entries outside copy execution, then
restores exact modes. Access-denied fixtures must deny the actual executing uid;
privileged Linux execution cannot pass. Ctime/atime, container-root/link inode
timestamps and two explicit failed-directory creation dates are unmeasured.
These exclusions are frozen and explained in the contract; no values are
normalized dynamically to hide differences.

The adapter constructs the real original CopyJob, initializes the original
DefaultDesktopAdapter, and invokes original nextFile/processFile. Recursion,
collision policy, transfer, error-skip and metadata helpers execute unchanged.
It does not override application behavior. It intentionally excludes full
FileJob.run/start/finish, notifications, UI refresh, auto-unmark and OS dialogs;
its method state remains RUNNING. A separate full-original-GUI probe operates
actual CopyDialog controls and observes three real jobs, auto-unmark and refresh
in both directions, with complete final tree verification.

Important recovered details: existing directories always merge; negative modified
times clamp to epoch on copy; file owner rwx bits transfer while group/other bits
remain existing or umask-created; directories get final dates but not source
permissions. A directory may return true despite failed children. Native details
make those partial errors visible. [Decisions](decisions.md) retain investigation,
failed initialization/focus attempts, metadata findings and native path guards.

Native guards decline overlapping paths, source hard-link aliases, destination
links that would be followed, escaping/absolute link targets and special files.
Those restrictions have separate Rust integrity tests; destructive original
overwrite choices outside the contract are not claimed equivalent. Concurrent
replacement/races, other owners/ACLs/xattrs, special mode bits, mid-stream/disk-full
faults, resume/retry/rename prompts, remote/archive providers, cancellation and
full job orchestration remain unverified. W04 move/rename is next; W07 still
owns cancellation and broader job lifecycle behavior.

[GUI evidence](gui-observations.json) retains a 40-action native XTest replay,
eight original stages/sixteen pane states, dialog renders, actual tree results
and partial-error details. Native PATH excludes Java. Tests run on Linux
x86_64/ext4, en-US, JDK 25.0.4.1 and umask 022 under isolated X11/Xvfb.
Physical desktop, Wayland, other platforms and exhaustive OS dispatch remain
unverified. Optional original OSGi/logging warnings do not establish unrelated
component health. No pixel-level equivalence is claimed.

[Validation](validation.json) records formatting, Clippy, sixteen Rust tests,
twenty-seven Python tests, locked offline build, reference integrity and native
Java-independent CLI/linkage checks. All eight earlier predefined/reserved
comparison runs pass again, including all W01/W02/W02b/W02c coverage.

Reproduce after acquiring the pinned binaries and cached dependencies:

```sh
python3 -B tools/w03_inspect.py
# Run the original only when investigating its boundary:
python3 -B tools/w03.py oracle
cargo build --workspace --locked --offline
target/debug/mucrab-compare --suite w03
target/debug/mucrab-compare --suite w03 --reserved
# Prepare fresh independent GUI trees; replace RUN with the printed path:
python3 -B tools/w03_gui.py
xvfb-run -a -s '-screen 0 1120x720x24' python3 -B tools/linux_gui.py RUN/native-launch.json --replay fixtures/W03-recursive-copy/native-gui-actions.json
python3 -B tools/w03_gui.py --verify-native RUN
xvfb-run -a -s '-screen 0 1280x900x24' python3 -B tools/w03_gui.py --original-probe RUN
```

Never replay a mutating GUI sequence into an already used fixture. Prepare a
fresh run each time. The acquired reference tree is unchanged; GUI runtimes are
verified copies with disposable profiles. Raw artifacts remain ignored and need
regeneration or separate transfer. Release source/notice packaging remains open.
