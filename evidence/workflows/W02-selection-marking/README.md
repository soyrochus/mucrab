# W02a — selection and marking

Verified within the [bounded contract](contract.md), on macOS arm64 with the
pinned muCommander 1.6.2-1 binaries and en-US sorting. Filtering and additional
criteria are the next separate increments.

| Suite | Sequences | Full state snapshots | Differences / skips |
| --- | ---: | ---: | ---: |
| Predefined + generated seed 90241 | 28 | 1,430 | 0 / 0 |
| Reserved generated seed 180413 | 32 | 2,592 | 0 / 0 |
| Total | 60 | 4,022 | 0 / 0 |

Every snapshot compares visible order, focused row/name, per-row marks, marked
count, ordered marked names, and effective selected names. Counts include the
initial snapshot per sequence, not 4,022 distinct user journeys. The predefined
suite has 12 named sequences and 16 generated ones; reserved has 32 sequences.
W01's 342 configuration/error cases also pass on the updated implementation.

The oracle calls original binary methods and selection actions. It does **not**
compile recovered Java implementation source. It bypasses full desktop
constructors with ReflectionFactory, initializes JTable/AbstractAction normally,
and supplies a field-only MainFrame fixture. Its mouse events are normalized
post-Swing focus events. Key release resets the two original repeat-state fields;
it does not test ActionManager shortcut dispatch. This boundary is deliberate
and recorded in the contract; it is not full original GUI equivalence.

## Reproduce

From the repository root, with Python 3.9+, a full JDK (verified: 21.0.10), and
the Rust toolchain/dependencies described in the root plan:

```sh
python3 -B tools/acquire_mucommander.py
python3 -B tools/analysis_tools.py setup
python3 -B tools/w02_inspect.py
cargo build --workspace --locked
python3 -B tools/w02.py oracle
target/debug/mucrab-compare --suite w02
target/debug/mucrab-compare --suite w02 --reserved
target/debug/mucrab-compare --suite w01
target/debug/mucrab-compare --suite w01 --reserved
cargo test --workspace --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo fmt --all -- --check
python3 -B -m unittest discover -s tools -p 'test_*.py' -v
```

For native GUI testing, read `raw_run` in the latest predefined comparison JSON.
Launch `target/debug/mucrab` with that directory's
`fixtures/click-focus-and-toggle` path twice, once for each pane. This fixture
contains two folders, ordinary/hidden files, and a symbolic link. Ctrl-click
Alpha.txt, then Shift-click folder2: four children are marked. Tab, Down, Space
marks folder2 in the independent right pane. End and held Space marks the final
row once; another press after release unmarks it. Sorting/Refresh preserve the
marked/focused identities; All/None/Invert exclude the parent row.

For the original full GUI, regenerate the W01 fixtures and run
`python3 -B tools/w01_gui.py --mac-apps` on macOS. Use its saved `launch.json`
commands to launch the disposable reference, then enter zFolder. Mark child2
and child10, focus child2, and reverse name order via the Name header. The
retained original captures show both marked files and child2 retaining focus
after the order changes. Never launch from the immutable reference tree.
The .app wrappers contain local runtime components and are not releases.

## Evidence and practical limits

- `recovery.json`: CFR/javap recovery for 19 classes and original-bytecode hashes.
- `oracle-predefined.json`: the first successful Java run, before Rust selection
  implementation. Later comparison reports identify current source hashes.
- `comparison-*.json`: commands, tool versions, input/JAR/adapter hashes, raw
  output digests, counts and complete differences (empty on passing runs).
- `gui-observations.json`, `gui/`: native observations and representative original
  Swing screenshots. Native two-pane controls, held Space/release, sorting and
  refresh were exercised. Original status/AX did not reliably report marks;
  screenshots show red marked filenames separately from the focused blue row.
  Original shortcut dispatch and exhaustive OS mouse-event parity remain open.
- `validation.json`: final local build, format, Clippy, seven Rust tests, fifteen
  Python tests, independent linkage, source hashes and reference integrity.
- `decisions.md`: adapter setup, corrected differences and remaining boundaries.

Refresh/navigation restoration is statically recovered and Rust-tested, with
native GUI checks. It is excluded from the headless original-method comparison
claim. External renames, automatic refresh, configurable keymaps, middle-button
dragging, page/block selection, other OSes/locales, and file operations remain
outside this increment. No upstream Java implementation source was fetched.

REA remains an inventory tool in this experiment. JVM semantic recovery here
uses CFR/javap, direct Java execution, native observations and Rust work; no
automatic REA Java-to-Rust conversion is claimed.
