# W05 recovery and implementation decisions

- Started from clean `cd25ffe` (Updated README), main synchronized with origin,
  on the same Linux x86_64/ext4 host with JDK 25.0.4.1 and cached pinned binaries.
  No missing installation, upstream implementation-source fetch, global tool
  update or immutable-reference mutation was needed. No agents were delegated.
- Static recovery pins 19 classes under
  `W05-recovery-20261016T203531Z-144d8105`. The parser belongs to the original
  BatchRenameDialog with nested tokens, not a separate utility. BatchRenameJob
  sets the indexed new name then invokes inherited MoveJob. Confirmation and
  collision dialogs, action and manual-edit/block model behavior were recovered.
- Original-only runs `W05-20261016T203924Z-c7a7eac7` and
  `W05-20261016T203943Z-ae8c407c` each execute 54 sets before Rust. The second
  calls original PathUtils for constructor-equivalent old-name map keys. No
  application methods are overridden. The top-level Swing constructor alone is
  bypassed with field-only headless wiring; full GUI construction is separate.
- Froze 30 named + 24 generated sets (755051), 32 withheld reserved sets (991075),
  and the bounded contract before native code. Native predefined and reserved
  runs passed on first execution, respectively
  `W05-20261016T204235Z-71d896f4` and `W05-20261016T204321Z-edc00569`.
  Later protocol hardening requires exact boolean/integer types without changing
  the accepted inputs or expected behavior; final runs re-execute the originals.
- Counters advance independently per token and wrap signed i32, rather than
  saturating. Range positions count UTF-16 units, not Rust characters/bytes.
  Last-dot splitting also applies to directory/dot names. Search is literal,
  then one trailing period is removed, then case conversion runs. A first
  selected-original conflict can take precedence over a later duplicate.
- Preserved a deliberate subset: ASCII case conversion, unchanged Unicode names,
  pair-preserving ranges, UTC date tokens for 1970–9999 and bounded padding.
  Manual overrides, block/unblock, row removal and range-helper dialogs are not
  exposed as native features. Native errors reject unsupported text/ranges
  before mutation. Broader locale/timezone/Unicode case semantics need new cases.
- Native plans capture selected metadata/identity and regenerate before apply;
  stale plans fail before writing. Skip is fixed for batch application. Reuse
  W04's local rename/fallback/error handling, including partial folder merges,
  instead of introducing a second mutation engine. Unchanged rows have no job
  outcome. Native blocks wholly unchanged/invalid-leaf application as an extra
  guard; original preview button semantics are still measured independently.
- An initial Rust assertion expected one hyphen around an empty `[]` token;
  the literal mask contains two. Corrected the assertion to the recovered
  empty-token behavior (also observed in the predefined original literal case),
  without changing implementation. All 26 Rust / 33 Python tests then pass.
- Core reuses the already installed/pinned chrono 0.4.45 with its existing std
  feature set for UTC date tokens. Cargo.lock changes only the local core's
  dependency edge; no package/version refresh or new external download.
- Native dialog uses live mask/replacement/counter options and reviewed Apply.
  Updated CLI help, which still said file operations were deferred. Replaced a
  missing arrow glyph in the preview with ASCII. Extended the isolated XTest
  helper with actual bracket/comma keysyms so mask strings replay correctly.
  Earlier passes remain retained; final comparisons/UI checks pin final sources.
- Exploratory GUI run `artifacts/gui/W05-20261016T204537Z-d64357a5` and final
  `artifacts/gui/W05-20261016T204917Z-c3334d15` are isolated. The final 41-action
  native replay passes; original actual BatchRenameDialog/confirmation/Skip
  controls execute two full jobs in both panes. Ten original stages/twenty pane
  states and four live previews are retained. Final trees and native partial
  merge match independent original expectations. Existing W03/W04 GUI replays
  also pass with the final binary and updated input helper.
- Final comparisons cover 86 sets, 346 previews/1,721 name pairs, 69 method-boundary
  apply jobs, 315 outcomes, 86 tree comparisons, 404 identity relations and
  172 selected-content checks. Zero differences/omitted cases. All fourteen
  current suites pass. Validation, fingerprint audit, notices, README and ledger
  retain reproducibility. Physical/Wayland desktop, full scheduling/cancellation
  and release packaging remain open. W06 ZIP browsing/extraction is next.
