# W04 — bounded local move and single-item rename

F6 or **Move to other pane** reviews the active pane's effective selection,
exact paths and collision policy. F2 or **Rename** reviews a new name for the
focused child, independent of other marks. Both start with SKIP. Successful
moves unmark their top-level items and refresh both panes. A successful rename
focuses its new visible name while preserving unrelated marks and sorting.
Invalid or unchanged rename names cannot be submitted.

Moves try the filesystem rename first. Failed renames fall back to recursive
copy then source deletion, including actual cross-filesystem moves. Existing
folders merge. Successful siblings stay moved after a partial failure; failed
or skipped children and their parents stay at source. If copying succeeds but
source deletion fails, both copies remain and details explain that outcome.
There is no rollback or cancellation yet.

[The frozen contract](contract.md) follows the original-only 319-case run,
before Rust implementation. [Recovery](recovery.json) pins 21 original classes,
CFR and javap output. These include MoveJob, local rename/copy/delete and
collision callers, MoveDialog, RenameAction and FileTable's actual inline editor.
REA supplies earlier JAR inventory only. No upstream implementation source was
fetched. [Decisions](decisions.md) records important findings and boundaries.

| Measurement | Predefined | Reserved | Total |
| --- | ---: | ---: | ---: |
| Cases (fixture / arrangement / policy combinations) | 319 | 160 | 479 |
| Same-filesystem cases | 183 | 80 | 263 |
| Real cross-filesystem cases | 136 | 80 | 216 |
| Single-item rename-mode cases | 37 | 0 | 37 |
| Ordered top-level outcomes / identity relations | 384 | 160 | 544 |
| Remaining source tree comparisons | 319 | 160 | 479 |
| Destination tree comparisons | 319 | 160 | 479 |
| Selected-content conservation checks (Java + Rust) | 638 | 320 | 958 |
| Differences / omitted cases | 0 / 0 | 0 / 0 | 0 / 0 |

[Predefined](comparison-predefined.json) and [reserved](comparison-reserved.json)
reports pin fixtures, original JARs/classes, commands, adapters, native source
and binary. There are 38 named fixture sets, ten generated sets (seed 744041)
and twenty reserved sets (seed 991074), with arrangement/policy combinations;
these are 68 distinct sets, not 479 distinct user journeys. Expected false
outcomes, collision skips and permission failures remain measured cases.
[The original-only receipt](oracle-predefined.json) claims zero Rust comparisons.

Each implementation starts from separate equivalent owned trees. Observations
compare complete remaining source and destination paths, entry kinds, regular
file sizes/SHA-256, ordinary modes, declared modified milliseconds and literal
link targets. Every selected byte stream/link must remain at its source or
mapped destination. Device/inode observations establish whether source identity
was retained at destination; raw unrelated inode numbers are never equated.
Both arrangements require actual st_dev checks. Repository ext4 and a different
`/tmp` mount supplied this host's two filesystems; EXDEV is never mocked.

A rename retains full ordinary mode/date metadata and inode identity. Fallback
uses recovered W03 copy metadata rules: milliseconds, negative times clamped
to epoch, owner file permissions, and destination group/other bits retained.
Directory fallback applies dates best effort even after failed children, and
returns false if any child fails or source removal fails. This differs from
W03's directory-copy outcome rule. Source directory dates changed by removing
children are omitted by a static mask derived from input paths. Raw before
dates remain recorded. Root/atime/ctime/link-inode dates and frozen unreadable
folder creation dates are unmeasured. No result-derived exclusions are used.

The adapter invokes unchanged original MoveJob constructor/nextFile/processFile,
with original default desktop initialization, auto-skip errors, and optional
integrity verification disabled. Recursive operations remain original JAR code.
The isolated method state stays RUNNING; full FileJob lifecycle, notification,
auto-unmark, refresh and dialogs belong to separate GUI observations. The full
original GUI probe operates three actual MoveDialogs/jobs and the real inline
filename editor, with ten stages/twenty pane states and final-tree validation.

[GUI evidence](gui-observations.json) retains a portable 41-action native replay:
both move directions, cancel/F6 focus recovery, SKIP/OVERWRITE, focused rename
with another item marked, unchanged/invalid-name review, refreshed panes, and a
real cross-filesystem partial move with two permission errors. Final complete
trees match independently recorded expectations; remaining partial source and
destination match the original component result. The earlier 40-action W03 copy
replay also passes after integrating these controls. Native PATH is
`/nonexistent`. Original Swing renders, raw captures and states are retained;
there is no pixel-equivalence or exhaustive OS shortcut-dispatch claim.

[The original-only symlink diagnostic](symlink-fallback.json) observes a file
link becoming a regular file and a directory link's target losing its children
during fallback. It deliberately exits 2 with **zero equivalence cases**. Native
code declines fallback through symlinks, preserving source links and targets.
Supported atomic folder/link renames have separate accepted cases. Direct
broken-link moves stay at source; broken links inside an atomically moved folder
are preserved. Existing destination links, escaping/absolute direct links,
overlap, source inode aliases and special files are guarded separately in Rust.
Those guards are not counted as original equivalence.

Tests use stable owned Linux trees, en-US, JDK 25.0.4.1, umask 022 and isolated
X11/Xvfb. Access-denied fixtures must deny the executing uid; privileged execution
cannot pass. Races/concurrent replacements, other owners/ACLs/xattrs/special
bits, mid-stream/disk-full faults, hard-link semantics, remote/archive providers,
retry/resume/cancel, batch rename, physical desktop/Wayland and other platforms
remain unverified. Optional original bundle/logging warnings remain unrelated
to the scoped result. Full job orchestration stays with W07.

[Validation](validation.json) records formatting, Clippy with warnings denied,
21 Rust / 30 Python tests, locked offline build, reference integrity, Java-absent
native execution and no JVM linkage. [Regression results](regressions.json)
retain all twelve predefined/reserved executions across six suites. Cargo.lock
and dependency pins are unchanged. Release source/notice packaging remains open.

Reproduce with the pinned reference, cached dependencies, full JDK, unprivileged
permissions, and a `/tmp` filesystem distinct from the checkout:

```sh
python3 -B tools/w04_inspect.py
python3 -B tools/w04.py oracle
cargo build --workspace --locked --offline
target/debug/mucrab-compare --suite w04
target/debug/mucrab-compare --suite w04 --reserved
# Original hazard diagnostic, expected exit 2 and zero equivalence cases:
python3 -B tools/w04_symlink_probe.py
# Prepare fresh GUI trees; replace RUN with the printed directory:
python3 -B tools/w04_gui.py
xvfb-run -a -s '-screen 0 1120x720x24' python3 -B tools/linux_gui.py RUN/native-launch.json --replay fixtures/W04-move-rename/native-gui-actions.json
python3 -B tools/w04_gui.py --verify-native RUN
xvfb-run -a -s '-screen 0 1280x900x24' python3 -B tools/w04_gui.py --original-probe RUN
```

GUI reproduction additionally requires Xvfb, X11/XTest tools and Python Pillow.
Prepare a fresh run before every mutating replay. Reference runtimes are verified
copies with disposable profiles. Raw artifacts are ignored: regenerate or
transfer them, including uniquely named `/tmp/mucrab-W04-*` cross-device trees.
Tracked snapshots, hashes and receipts retain measurements even if those trees
are removed. W05 batch rename is next.
