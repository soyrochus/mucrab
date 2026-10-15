# W01 — local browsing and recovered name ordering

This increment implements a native two-pane local browser, using the same Rust
enumeration and sorting functions exercised by the comparison adapter. It is
scoped to en-US on the observed macOS arm64 host. See `contract.md` for supported
behavior and explicit exclusions.

## Evidence

| Record | Meaning |
| --- | --- |
| `recovery.json` | CFR/javap provenance for eight original classes, including comparator, table sorter, callers, settings, and launcher configuration |
| `oracle-predefined.json` | Successful execution of the original Java classes; zero Rust equivalence cases claimed |
| `comparison-predefined.json` | 187 configuration/error cases; 184 full index permutations; 19,776 pairwise signs; 27 enumeration/error checks |
| `comparison-reserved.json` | 155 reserved configuration/error cases; 152 permutations; 25,920 pairwise signs; 27 enumeration/error checks |
| `gui-observations.json` | Native controls exercised in both panes and representative original GUI observations, with retained captures and their scope |
| `validation.json` | Build, tests, lint, reference integrity, standalone native linkage, and final render checks |

Both comparison runs pass with zero differences and no skipped cases. Totals
include repeated real-folder/error checks and eight sort-option combinations;
342 cases do not mean 342 distinct user journeys. Synthetic label cases test
names that cannot all coexist as physical files on the chosen filesystem.

The harness invokes the **original** `FileComparator` and the private
`FileTableModel.sort` from verified JARs. It compiles adapter glue only. The
table's custom quicksort can permute comparator ties; the Rust implementation
reproduces that permutation instead of substituting a stable sort. ICU4X
collation data is pinned by `Cargo.lock`; Java is unnecessary at application
runtime.

GUI observations supplement the component suite. The original GUI evidence is
representative, including the matched initial listing and child/parent
navigation; it is not an exhaustive automated GUI-equivalence suite. Native
controls were exercised in both panes, including separate sort settings,
navigation, empty folders, a missing path, and recovery via refresh. Selection
restoration and other deferred behaviors are not covered by W01.

## Reproduce

From the repository root, with the prerequisites in the implementation plan:

```sh
python3 -B tools/acquire_mucommander.py
python3 -B tools/analysis_tools.py setup
python3 -B tools/w01_inspect.py
python3 -B tools/w01.py oracle
cargo build --workspace --locked
cargo run --locked -p mucrab-compare -- --suite w01
cargo run --locked -p mucrab-compare -- --suite w01 --reserved
```

Each run gets a unique directory under ignored `artifacts/runs/`. Requests,
original/Rust JSON lines, commands, stdout/stderr, backing fixtures, and a copy
of the report are retained there. The compact report includes source/input
hashes, classpath fingerprints, compiler versions, adapter binary hashes,
matrix/permutation counts, and failures. Rerunning updates the tracked summary;
review the diff and retain meaningful prior evidence in history.

Prepare a disposable original GUI runtime and matching settings:

```sh
python3 -B tools/w01_gui.py
# For macOS UI automation that needs registered app bundles:
python3 -B tools/w01_gui.py --mac-apps
```

Preparation prints and saves `launch.json` with explicit `reference_argv`,
`reference_cwd`, and `native_argv`. Execute the reference command from its stated
working directory, and the native command from the repository. Commands are
stored as argument arrays to preserve spaces in paths. For example, an executor
can load the selected JSON and call `subprocess.run(record["reference_argv"],
cwd=record["reference_cwd"])`, resolving the relative working directory against
the repository root. The settings file comes from the committed
`fixtures/W01-local-browse-sort/reference-preferences.xml`.

The optional `.app` wrappers and bundled JDK runtime are **local test output**
under ignored artifacts. They are not a release/distribution of muCrab or Java.
The helper verifies the reference and copied files, and never changes the
acquired reference or a personal application profile.

On the observed Mac, short sustained clicks and screenshot coordinates worked
for the original Swing table; indexed accessibility clicks were unreliable.
Native egui controls worked with indexed clicks held for about 150 ms, and
double-clicks held about 80 ms. Always inspect fresh UI state and coordinates.

## Limits and next increment

No file mutation, archive operation, selection/filter behavior, alternate
locale, or cross-platform equivalence is established. Large directory loading
and sorting currently run synchronously; background work is a later increment.
Do not turn comparator compatibility on these fixtures into a claim of full
muCommander compatibility or arbitrary automated JAR-to-Rust conversion.

Next: W02, with a fresh recovered contract, original adapter coverage, fixtures,
Rust implementation, UI integration, and comparison evidence. Keep W01 as a
regression gate while adding later workflows.
