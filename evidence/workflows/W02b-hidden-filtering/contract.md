# W02b contract — hidden-file filtering and selection

Status: frozen 2026-10-16 after the original-only run
`20261016T123431Z-28bef7df` (32 sequences, 960 complete two-pane snapshots),
before Rust implementation. Inputs are the pinned muCommander 1.6.2-1 JARs. First host:
Linux x86_64/ext4, OpenJDK 25.0.4.1; locale en-US. macOS GUI observations from
earlier increments remain separate historical evidence.

## Recovered behavior

- `ConfigurableFolderFilter` follows the global `SHOW_HIDDEN_FILES` preference,
  whose original default is false. `ToggleHiddenFilesAction` flips it and asks
  `WindowManager` to refresh current folders. Both native panes must share one
  control; their locations, direction, folders-first and selection stay separate.
- With show-hidden off, the original inverted HIDDEN attribute filter excludes
  files for which `LocalFile.isHidden()` / `java.io.File.isHidden()` is true.
  This contract covers Unix dot-prefixed names only. Hidden directories, hidden
  symlinks and broken hidden symlinks are filtered by their own names. A visible
  symlink to a hidden target remains visible. `.DS_Store` participates as a
  dotfile; independent DS_Store/system exclusion preferences are fixed to show.
  Windows hidden attributes and macOS Finder flags are outside this contract.
- Filtering is applied before model loading and W01 sorting. The parent row is
  constructed separately, stays first, and cannot be marked. Each sequence
  freezes one shared natural/lexicographic mode; direction and folders-first
  may differ between panes. Per-pane mixed sort-mode reload equivalence is not
  claimed because the original mode preference is global.
- Same-folder refresh captures the previously focused file and marked files.
  The original `FileTable$FolderChangeThread.run` replaces/sorts the listing,
  restores surviving focus by identity, and restores only surviving marks.
  A hidden mark is discarded when its entry disappears; showing it again does
  not resurrect the mark. Effective selection follows W02a: visible marked
  children in current order, otherwise the focused child, never the parent.
- When focused identity vanishes, keep its old row index if possible, otherwise
  clamp to the final row. Empty listings have no focused/selected entry. An
  existing parent focus survives. Reload assigns the resulting row as the
  next range anchor; Space repeat/end-of-list guards survive the reload until
  release, as they do in the original table object.
- The native application starts with hidden entries off and applies the shared
  setting to subsequent navigation and refresh. Failed enumeration retains the
  previous listing/selection and reports the error, following W01. Failure and
  navigation integration are Rust-tested/native-observed, not part of the
  same-folder original-worker comparison claim.

## Original oracle boundary

`tools/java/W02bOracle.java` executes the original `ConfigurableFolderFilter`,
original configuration listener, original table/model/selection methods and the
complete original `FolderChangeThread.run` on the Swing event thread. Adapter
glue supplies the same-folder focused/marked inputs recovered from the outer
`setCurrentFolder`; it does not reimplement sorting, focus fallback or marks.
Original model/column constructors and JTable run normally. ReflectionFactory
bypasses full desktop construction, as in W02a; field-only MainFrame,
FolderPanel and LocationManager fixtures provide UI wiring and a monitored
folder. No behavioral methods are overridden. A reload-worker exception,
including an internally caught exception, makes execution unavailable.

The two-table adapter applies the shared preference then reloads both tables.
It does not test `WindowManager` dispatch, asynchronous application refresh,
shortcut wiring, races, filesystem watching or a full desktop constructor.
Parent presence is explicit fixture input, including an empty listing without
parent. Unicode paths and input enumeration order are identical on both sides;
there is no case/Unicode normalization. Selection events retain W02a's normalized
post-Swing mouse and explicit key-release boundary. Reserved seeds are frozen
before Rust implementation and executed only afterward.

## Acceptance

Fixture specification: `fixtures/W02b-hidden-filtering/cases.json`. Compare the
shared show-hidden setting and both complete pane states after **every** event:
visible order, focus row/identity, per-row marks, ordered marks, marked count,
and effective selection. Validate expected visible sets independently, including
empty listings/parent handling. Reject missing, reordered or extra transitions,
malformed state, zero toggles/cases, unavailable adapters and swallowed worker
exceptions. Compare predefined and reserved sequences; retain discrepancies and
rerun W01/W02a regression suites. GUI observations supplement this isolated
method suite; they do not establish exhaustive GUI parity.

Recover → run original → freeze contract/fixtures → implement Rust → compare →
check both native panes → record evidence. The Rust runtime remains independent
of Java, and pinned upstream notices remain intact.
