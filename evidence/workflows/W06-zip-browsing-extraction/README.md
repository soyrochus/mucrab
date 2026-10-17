# W06 — bounded local ZIP browsing and extraction

muCrab opens local `.zip` files in either pane, including virtual paths such as
`archive.zip/folder`. Folder navigation, Parent, Refresh, hidden filtering, sorting
and selection reuse the earlier pane behavior. Archive panes are read-only.
**Extract ZIP…** reviews a whole local archive or the effective selection inside
one; F5 also opens this review while an archive listing has keyboard focus.
Extraction targets the other local pane and runs off the UI thread. Four fixed
collision policies are available. Skips continue; the first actual transfer
failure stops this archive, retains earlier writes and appears in Details.

The [frozen contract](contract.md) precedes Rust implementation. Its 25 named and
18 generated sets use seed 766061. Twenty-four reserved sets (991086) stayed
withheld until implementation. [Recovery](recovery.json) identifies 27 original
classes, JAR/class hashes and retained CFR/javap output. The actual ZIP format
provider, archive tree and UnpackJob methods supply the oracle. No upstream Java
implementation source was fetched and no behavior is reimplemented in Java.
REA's contribution remains its earlier JAR inventory; CFR and javap provide
the recovered method representations.

| Measurement | Predefined | Reserved | Total |
| --- | ---: | ---: | ---: |
| Fixture/policy cases | 163 | 96 | 259 |
| Complete folder listings | 486 | 328 | 814 |
| Entry observations in those listings | 1,047 | 840 | 1,887 |
| Extraction method jobs | 160 | 96 | 256 |
| Complete resulting tree comparisons | 163 | 96 | 259 |
| Source archive integrity checks, Java + Rust | 326 | 192 | 518 |
| Differences / omitted cases | 0 / 0 | 0 / 0 | 0 / 0 |

Counts include policy combinations and repeated listings; they are not counts
of distinct user journeys. Successful empty archives, empty selections, expected
skips, directory/file conflicts, denied writes, partial failures, invalid/truncated
archives and invalid folder listings are measured cases. The two comparison
receipts retain commands, input/source/binary hashes, normalization and raw-run
locations. [The original-only receipt](oracle-predefined.json) records the
pre-implementation oracle with zero native comparison cases.

The original tree synthesizes missing folders at first encounter, using size
zero and the first descendant's date. A later explicit folder replaces its
metadata while retaining order and children. Extraction follows central-directory
order, strips the current folder depth and includes selected-folder descendants
regardless of the visibility filter. Files preserve bytes and encoded dates;
owner rwx is imported, while destination group/other bits remain existing or
umask-created. Folder dates and permissions are not imported. The original DOS
conversion leaves Calendar's millisecond field intact: comparison truncates
these dates to whole seconds and retains raw values. Conditional-date collisions
within that same second are excluded. Directory execution dates are omitted;
tree shape, modes and every file size/hash/date remain checked.

The method adapter registers the original ZIP provider, uses original entry
objects, constructs UnpackJob with the selected base depth and collision policy,
enables auto-skip errors, disables optional integrity mode and invokes its
original nextFile/processFile boundary in RUNNING state. It overrides no
application methods. Start/run/finish, notifications, progress, password prompts
and auto-unmark/refresh are excluded from this method boundary. GUI checks use
actual original folder-change workers, CopyDialog/UnpackDialog controls and full
jobs; CopyDialog's recovered archive branch constructs UnpackJob.

Native code additionally rejects unsafe/absolute/dot/backslash/drive paths,
archive links/special files, ambiguous duplicates, and destination links or
hard-link/source aliases. Review snapshots reject replaced/changed archives and
replaced destination roots. Entry sizes and CRC are checked during transfer.
Limits are 512 MiB compressed archive, 512 MiB per expanded entry, 2 GiB total
expanded bytes, 20,000 tree nodes, 64 path components, and 16 MiB central-directory
metadata. Guard tests are separate from equivalence counts. Earlier writes,
including partial corrupt-file data, can remain after a read/CRC failure. These
checks do not establish race-free or exhaustive adversarial filesystem handling.

Only ordinary ZIP32 stored/deflate, ASCII/UTF-8 names and UTC DOS timestamps are
verified. ZIP64, encryption, other methods/encodings, extended timestamps, nested
archives, archive writing, arbitrary interactive policies, ownership/ACLs/xattrs,
special permission bits, concurrent replacement, disk-full/mid-stream faults,
other platforms and W07 lifecycle/cancellation remain outside this increment.
Current component and GUI checks cover unprivileged Linux x86_64/ext4, OpenJDK
25.0.4.1, en-US/UTC and umask 022. Physical/Wayland and current macOS GUI behavior
remain unverified. [Decisions](decisions.md) records precision and harness issues.

Reproduce with cached dependencies/reference inputs and a full JDK:

```sh
python3 -B tools/w06_inspect.py
python3 -B tools/w06.py oracle
cargo build --workspace --locked --offline
target/debug/mucrab-compare --suite w06
target/debug/mucrab-compare --suite w06 --reserved
python3 -B tools/w06_gui.py
# Replace RUN with the printed directory. Use fresh trees for each mutating replay.
xvfb-run -a -s '-screen 0 1120x720x24' python3 -B tools/linux_gui.py RUN/native-launch.json --replay fixtures/W06-zip-browsing-extraction/native-gui-actions.json
python3 -B tools/w06_gui.py --verify-native RUN
xvfb-run -a -s '-screen 0 1280x900x24' python3 -B tools/w06_gui.py --original-probe RUN
```

GUI checks require Xvfb, X11/XTest and Pillow. The helper removes inherited
Wayland state for the original process and explicitly sets UTC. It copies and
verifies the reference runtime and uses a disposable profile. The reference tree
remains unchanged. [Regressions](regressions.json) records all fourteen earlier
predefined/reserved executions. Eight new Rust integrity/navigation tests and
three Python protocol tests bring the suites to 34 Rust / 46 Python tests. The
dependency update adds zip 9.0.4, nix 0.31.3 and typed-path 0.12.3 without changing
any previous external pins; [ZIP provenance](../../../third_party/zip/README.md)
and the release workflow preserve their notices and corresponding source.

[GUI observations](gui-observations.json) retain 54 artifacts: a 43-action native
input replay plus its initial capture (44 logged actions), and twelve original
stages/twenty-four live pane states. Three original full jobs cover selected
extraction with SKIP, whole-archive OVERWRITE, and reverse-direction extraction.
Native checks additionally cover Cancel, partial-error details and retaining the
last listing after invalid ZIP input. Both complete final trees and a native
partial tree are independently checked. The previous W05 batch-rename GUI replay
also passes on the final executable. Raw X images may be black; direct original
Swing renders are retained. No pixel-equivalence or exhaustive OS dispatch is claimed.

[Validation](validation.json) records formatting, Clippy with warnings denied,
34 Rust/46 Python tests, all sixteen suite executions, native CLI/linkage without
Java, and a frozen release build from a corresponding-source export with an
empty registry cache. The source archive vendors all 387 external crates; the Linux
binary bundle includes their notices. Both pass checksum/CLI checks. The hosted
release workflow and current macOS build/signing remain separately unverified;
no tag or release is published by this increment. W07 is next.
