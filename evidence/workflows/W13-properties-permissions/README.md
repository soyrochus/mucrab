# W13 — Properties and permissions

Implemented and Linux-verified on 2026-10-17. Native macOS and the acceptance
checks listed below remain pending. W11/W12 were committed as `675bcd1` first.

Alt+Enter / File → Properties captures the focused child and originating tab,
independently of marks. Local lstat fields include available logical/allocated
size, timestamps with stored precision, numeric/named owner/group, mode, link
text and link count. ZIP members show only available archive metadata. Links
and ZIP members cannot be edited.

Folder totals run separately from mutations, include hidden entries, count
hard links per name and never follow links or descendant mounts. Root entries
are excluded from descendant counts. Limits are depth256, one million visited
entries and256 diagnostic details; cancellation/omissions retain partial totals.

Reviewed changes cover one local file/folder's nine ordinary rwx bits and UTC
RFC3339 modified time. Special bits/access time are preserved. A required time
handle is acquired before any chmod; unreadable entries can therefore fail a
combined plan before writing. Per-field results expose committed, failed and
unattempted work without rollback. W07 retains its single mutation gate;
unrelated marks and stable originating tab identity are preserved.

## Evidence and scope

- [Recovery](recovery.json): nine pinned actions/dialogs/jobs inspected with
  CFR/javap. The original PropertiesJob depends on ShowHidden, includes the root
  and may count link targets. Native counting deliberately follows the contract.
- [Contract](contract.md), [fixture freeze](fixture-freeze.json): executable
  generator and40 predefined/40 reserved recipes preceded Rust code. Native
  traces were recorded afterward; no blind GUI-fixture claim is made.
- [Predefined](comparison.json) and [reserved](comparison-reserved.json):40+40
  actual original ChangeFileAttributesJob/native filesystem changes, zero
  unexpected differences. All40 cases in each run have declared differences:
  the isolated original local provider reports changeable mask0700, while native
  supports all nine bits; native preserves special bits, rejects link changes
  and preflights a preserving time handle. Provider capability is checked against
  its measured mask, not assumed to be general Java chmod parity. Dates compare
  at fixture millisecond precision; native nanoseconds have separate tests.
  Original local lstat owner/name parity and full Properties windows are not
  covered by this runtime adapter.
- [Workspace validation](validation.json):101 Rust tests, one separately invoked
  W10 native-service test ignored here,51 Python tests, locked/offline all-feature
  build, format, Clippy and strict OpenSpec. Tests cover preserved mode/time/bytes,
  hard links, stale targets, cancellation, failure after a successful field,
  denied reads/time handles, ZIP unavailable fields, hidden/link cycles and bounds.
- [Linux native replay](native.json): both-pane editing, unchanged bytes, special
  bits/access time, nanosecond time read-back and hard-linked mode changes; hidden
  folder totals, ZIP read-only properties, normal exit and viewer handoff.
- [Concurrency](concurrent-native.json): totals420000bytes/20000files complete
  alongside a paused real copy; exact paused path remains unchanged, source
  bytes remain intact and earlier/partial/later destination outcomes are checked.
- [Regressions](regressions.json): fresh W07–W12 native replays on the initial
  W13/W14 candidate. The final read-result abandonment fix is covered by the
  deterministic queued-result test and the repeated W13/W14 native replays;
  earlier regression receipts retain their actual candidate provenance.

## Reproduce

From the repository root, with the acquired pinned reference, JDK, cached Cargo
crates, Python Pillow, Xvfb/X11/XTest and xclip installed:

```sh
python3 -B tools/w13_inspect.py
python3 -B tools/w13_w14_fixtures.py
cargo build --workspace --all-features --locked --offline
python3 -B tools/w13.py
python3 -B tools/w13.py --reserved
python3 -B tools/w13_w14_validate.py
python3 -B tools/w13_w14_gui.py prepare --run artifacts/gui/W13-W14-replay
xvfb-run -a python3 -B tools/linux_gui.py artifacts/gui/W13-W14-replay/launch.json --replay artifacts/gui/W13-W14-replay/actions.json
python3 -B tools/w13_w14_gui.py verify --run artifacts/gui/W13-W14-replay
python3 -B tools/w13_w14_concurrent.py prepare --run artifacts/gui/W13-W14-concurrent-replay
xvfb-run -a python3 -B tools/linux_gui.py artifacts/gui/W13-W14-concurrent-replay/launch.json --replay artifacts/gui/W13-W14-concurrent-replay/actions.json
python3 -B tools/w13_w14_concurrent.py verify --run artifacts/gui/W13-W14-concurrent-replay
```

Preparation refuses to overwrite fixtures. GUI actions assume1120×720 geometry;
recorded traces include corrected theme-menu attempts. Native screenshot checks
supplement filesystem/clipboard assertions; they do not prove original UI parity.
The concurrent trace pauses real work, so the copied-child count varies by host.

## Remaining gates

Native macOS both-pane inspection/editing, full original-window interactions,
a real bind-mount traversal boundary and foreign-owner chmod denial remain open.
No ownership/ACL/xattr/special-bit editing, recursion or elevation is implemented.
Stable-entry checks do not guarantee race-free access. Cancellation waits for
blocked filesystem/account-lookup calls; no rollback is promised.
