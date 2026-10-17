# W02b — hidden-file filtering

Verified within [the frozen contract](contract.md) on Linux x86_64/ext4,
OpenJDK 25.0.4.1, and en-US sorting. The application now starts with Unix
dotfiles hidden. One **Show hidden files** checkbox refreshes both panes.
Surviving focus/marks remain; marks on hidden entries are discarded. A missing
focused identity falls back to its previous row, clamped to the new listing.

| Suite | Sequences | Global snapshots | Pane snapshots | Differences / skips |
| --- | ---: | ---: | ---: | ---: |
| Predefined: 20 named + 12 generated, seed 52061 | 32 | 960 | 1,920 | 0 / 0 |
| Reserved, seed 99017 | 24 | 1,560 | 3,120 | 0 / 0 |
| Total | 56 | 2,520 | 5,040 | 0 / 0 |

Each global snapshot includes the shared preference and both complete pane
states. Counts include the initial state of each sequence; they are not counts
of distinct user journeys. W01's 342 cases and W02a's 60 sequences/4,022 states
also pass on the updated implementation. Nine Rust and nineteen Python tests
cover integration and protocol checks separately from binary comparisons.

## Recovery and oracle

`recovery.json` identifies fourteen original classes, JAR/class hashes,
CFR output and javap commands. The original-only `oracle-predefined.json` run
preceded Rust implementation. Fixtures and reserved seeds were frozen first.

The adapter calls the original `ConfigurableFolderFilter`, its configuration
listener, original table/model/action methods, and the complete original
`FileTable$FolderChangeThread.run` on the Swing event thread. Same-folder inputs
are supplied from the original table. ReflectionFactory bypasses full desktop
construction; field-only UI fixtures provide wiring. No behavioral method is
overridden. An internally caught reload exception is an incomplete run.

The comparator mode is one shared natural/lexicographic setting per sequence;
pane direction and folders-first may differ. Mixed per-pane sort-mode reload
equivalence is excluded because the original mode preference is global.
The parent is explicit fixture input; DS_Store/system exclusions are fixed to
show. Windows hidden attributes, macOS Finder flags, other locales, races,
automatic refresh, and asynchronous refresh scheduling are excluded.

## Reproduce

Use the pinned inputs and prerequisites in the root implementation plan:

```sh
python3 -B tools/acquire_mucommander.py
python3 -B tools/analysis_tools.py setup
python3 -B tools/w02b_inspect.py
python3 -B tools/w02b.py oracle
cargo build --workspace --locked
target/debug/mucrab-compare --suite w02b
target/debug/mucrab-compare --suite w02b --reserved
target/debug/mucrab-compare --suite w01
target/debug/mucrab-compare --suite w01 --reserved
target/debug/mucrab-compare --suite w02
target/debug/mucrab-compare --suite w02 --reserved
```

The scripts retain requests, fixtures, Java/Rust output, adapter classes,
commands and reports under each report's ignored `raw_run`. Compact reports
include source hashes, binary/JAR hashes, runtime versions, seeds, output
digests, differences and scope. A fresh unavailable or incomplete execution
exits 2; a difference exits 1. Status alone remains a zero-case non-pass.

## GUI observations

`gui-observations.json` and `gui/` retain native X11/Xvfb screenshots, portable
input actions, original live-table snapshots and direct original Swing renders.
Native checks exercise shared filtering, visible/hidden marks, focus fallback,
re-showing, keyboard navigation/Tab/Space, independent sorting, both-pane folder
navigation, a filtered-empty directory, a missing path, and refresh recovery.
The native GUI is also launched with PATH excluding Java.

The original application is launched from a checksum-verified disposable copy
with its own profile. A Java startup agent observes the actual application
windows, invokes the actual hidden-file menu item on the event thread, and
captures both live tables after each transition. It replaces no application
code. It sets marks/focus through original methods; OS mouse/shortcut dispatch
is outside this observation. X screen captures can be black without a window
manager, so retained original images use `Window.printAll` on the real Swing
window. Raw screen captures and startup/shutdown logs remain in ignored runs.
Optional bundle/logging warnings are not evidence of full reference health.

On Linux, with Xvfb, xwininfo, libX11/libXtst and Pillow with XCB support:

```sh
python3 -B tools/w02b_gui.py
# Replace RUN with the directory printed by preparation:
xvfb-run -a -s '-screen 0 1280x900x24' python3 -B tools/linux_gui.py \
  RUN/native-launch.json --replay evidence/workflows/W02b-hidden-filtering/gui/native-actions.json
xvfb-run -a -s '-screen 0 1440x1000x24' python3 -B tools/w02b_gui.py --original-probe RUN
```

These are representative virtual X11 checks. A physical desktop/Wayland session,
macOS W02b UI, exhaustive GUI parity, and OS shortcut dispatch remain unverified.
`validation.json` records final checks, reference integrity and native linkage.
[decisions.md](decisions.md) records boundaries and investigation corrections.
Next: W02c additional sort criteria; recover and freeze a separate contract.
