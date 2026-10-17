# W11 — Tabs and bookmarks

Implemented and Linux-verified; broader original-window and macOS acceptance
remain open in [tasks](../../../openspec/changes/w11-tabs-bookmarks/tasks.md).

Each pane has 1–32 session tabs with stable nonreused IDs, location/history,
marks/focus, sorting and widths. New opens Home with default options; Duplicate
copies history/view/focus independently and clears marks/quick-find. Switching
retains the listing without reloading. Shared hidden filtering reconciles all
tabs. Reviews capture source IDs and concrete paths; completion only unmarks a
surviving matching origin. Closing an origin does not cancel a running job.

Shared ordered bookmarks store local/ZIP locations, stable IDs, version 1,
256-entry and 128-scalar label limits. Saves use a private same-directory temporary
file, sync and atomic replacement. Invalid/unknown/oversized/linked files are not
overwritten; failed or externally conflicted saves preserve the old file and show
errors. Rename drafts remain visible. No lock-based multi-process transaction or
race-free filesystem guarantee is claimed. Tabs/history are not persisted.

## Evidence and limits

- [Recovery](recovery.json): 12 pinned classes, original class/JAR hashes,
  CFR 0.152 and javap receipts. No upstream implementation source was fetched.
- [Contract](contract.md) and [fixture freeze](fixture-freeze.json): executable
  generator and 29 predefined/24 reserved sequences preceded Rust changes.
  The native invariant test executes browsing events; the named job/bookmark
  events use separate targeted tests. This is not a full original state oracle.
- [Original probe](original-probe.json): actual default/cloned tab factory and
  Bookmark methods. Same location, independent history and bookmark clone
  independence agree. W11 intentionally copies existing history (Java factory
  starts empty). The original-only Bookmark.equals observation compares names
  alone; native location+label duplicate validation is tested separately, without
  claiming the manager rejects those original values. This supplemental probe
  followed implementation.
- [Native input/restart](native.json): both-pane result/tab interaction, duplicate
  marks, Home, close, minimum width, bookmark management and persisted reopening.
  Source hashes remain unchanged; configuration and GPU caches are isolated.
- Core tests cover limits, missing/ZIP targets, unknown version, duplicate IDs,
  invalid labels/ZIP prefixes, oversized/symlink data, permission errors, external
  file changes, stable ordering and atomic failure. Desktop tests cover same-path
  marked tabs with both surviving and closed origins.
- [Regressions](regressions.json): fresh W07–W10 native fixtures, traces adjusted
  for the 27-pixel strip and Window menu. All recorded filesystem/dispatch,
  selection, theme and close outcomes pass.
- [Validation](validation.json): actual workspace commands, counts and hashes.

Full original tab-state predefined/reserved comparison, tab switching during
all move/Paste/open preparation variants, a native ZIP-bookmark UI replay and
macOS native checks remain pending. Do not turn those gates into passes from
unit tests or static recovery alone. No Java dependency enters the application.

## Reproduce

```sh
python3 -B tools/w11_inspect.py
python3 -B tools/w11_w12_fixtures.py
cargo test --workspace --all-features --locked --offline
cargo build --workspace --all-features --locked --offline
python3 -B tools/w11.py
python3 -B tools/w11_w12_gui.py prepare
xvfb-run -a python3 -B tools/linux_gui.py artifacts/gui/W11-W12-verified/launch.json --replay artifacts/gui/W11-W12-verified/actions.json
xvfb-run -a python3 -B tools/linux_gui.py artifacts/gui/W11-W12-verified/restart-launch.json --replay artifacts/gui/W11-W12-verified/restart-actions.json
python3 -B tools/w11_w12_gui.py verify
```

Preparation refuses existing directories; retain previous receipts and use fresh
run names when repeating. `tools/w11_w12_regressions.py prepare` prints the four
isolated regression commands; `verify` checks their traces and filesystem results.
