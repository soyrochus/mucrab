# W02c — metadata sorting

Name, Size, Modified and Extension sorting are implemented in both native panes.
Each pane keeps its own criterion, direction and folders-first setting. Sorts
preserve focused and marked identities. The selected criterion displays bytes,
extensions or signed-millisecond dates in UTC. Refresh, navigation and the shared
hidden-file toggle retain the criterion.

The contract was recovered from original JAR methods and frozen before Rust
implementation. [The contract](contract.md) states the synthetic-data layer,
original-method boundary, supported ranges and exclusions. [Recovery](recovery.json)
retains the ten-class CFR/javap inspection provenance. REA's earlier contribution
remains container inventory; it did not generate the Rust implementation.

| Comparison | Predefined | Reserved | Total |
| --- | ---: | ---: | ---: |
| Sequences | 38 | 36 | 74 |
| State snapshots | 698 | 756 | 1,454 |
| Pair signs | 90,712 | 132,720 | 223,432 |
| Complete ordered lists | 630 | 756 | 1,386 |
| Independent real metadata enumeration checks | 4 | 4 | 8 |
| Pair-only sequences (included above) | 4 | 0 | 4 |
| Differences / skips | 0 / 0 | 0 / 0 | 0 / 0 |

[Predefined](comparison-predefined.json) and [reserved](comparison-reserved.json)
reports identify all source/class/JAR/binary hashes, commands and raw runs.
The original-only [pre-implementation run](oracle-predefined.json) executed
38 sequences/698 snapshots. Generated seeds are 620613 and 991037. Counts include
option combinations and repeated directory enumeration, not distinct journeys.

Four extreme-value sequences explicitly have null order/selection and test only
comparison signs. The original descending DATE sorter was observed repeatedly
swapping without progress when wrapping subtraction breaks comparator ordering.
[The failure receipt](reference-stall.json), portable
[reproducer fixture](../../../fixtures/W02c-metadata-sorting/reference-stall.json)
and [decisions](decisions.md) retain that finding. This diagnostic exits **2**,
claims zero equivalence cases, and is separate from passing scoped comparisons.
Full table ordering supports sizes -1 or nonnegative signed i64 and timestamps
within ±(i64::MAX / 2) milliseconds. Native enumeration rejects dates outside
that range. Nothing is silently skipped in the accepted suites.

Real metadata checks cover files, directories, file/directory/broken symlinks,
zero/tied lengths, pre-epoch milliseconds, dotfiles and Unicode extensions.
Original local getters and native enumeration follow targets for size/date but
use the link's own name for extension. The native extension comparison embeds
1,483 pinned simple-case mappings observed from JDK 25.0.4.1; it needs no JRE.
[Case-data provenance](case-data.json) separates observed facts from upstream code.

The final regression run passes W01's 342 cases, W02a's 60 sequences/4,022
snapshots and W02b's 56 sequences/2,520 global snapshots. [Validation](validation.json)
records format, Clippy, twelve Rust tests, twenty-three Python tests, build,
case-data reproduction, reference integrity and Java-independent CLI checks.

[GUI observations](gui-observations.json) retain a fresh native XTest replay,
five original live-table stages, states, sort options and screenshots. The
original probe invokes real FileTable.sortBy and the hidden-file menu action
inside the unmodified full application. Direct Swing renders supplement raw
AWT captures, which can be black under Xvfb. Native captures were inspected for
readable metadata and preserved selection. Date display is intentionally UTC;
the original uses localized display. No pixel-level equivalence is claimed.

All current comparisons ran on Linux x86_64/ext4, en-US, JDK 25.0.4.1. Physical
desktop/Wayland, other JDK case data/locales, permissions/owner/group criteria,
metadata races, remote/archive providers, and exhaustive shortcut dispatch are
unverified. Native mode switches work, but cross-mode refresh equivalence is not
part of this component contract. W03 recursive copy is the next workflow.

Reproduce from the repository root after rehydrating pinned inputs:

```sh
python3 -B tools/w02c_inspect.py
python3 -B tools/w02c_case_data.py --verify-only
cargo build --workspace --locked --offline
target/debug/mucrab-compare --suite w02c
target/debug/mucrab-compare --suite w02c --reserved
# Diagnostic, deliberately exits 2 on the observed reference stall:
python3 -B tools/w02c_stall_probe.py
# Prepare a fresh disposable GUI run; use the printed path as RUN below:
python3 -B tools/w02c_gui.py
xvfb-run -a -s '-screen 0 1280x900x24' python3 -B tools/linux_gui.py RUN/native-launch.json --replay fixtures/W02c-metadata-sorting/native-gui-actions.json
xvfb-run -a -s '-screen 0 1280x900x24' python3 -B tools/w02c_gui.py --original-probe RUN
```

Raw artifacts remain ignored and must be regenerated or transferred separately.
The acquired reference distribution remains unchanged. Release source/notice
packaging is still open; this checkpoint is a local development milestone.
