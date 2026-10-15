# W02a decisions and discrepancy history

- Selection/marking was chosen as the bounded W02 increment before filtering or
  additional criteria. Parent is now an explicit unmarkable row, as well as the
  existing navigation button. Focus and marks remain separate in both panes.
- Static recovery: 19 original FileTable/model/SortInfo and action classes were
  inspected with pinned CFR and javap. No upstream implementation source was
  obtained. Action accelerator labels observed in the original menu corroborate
  Space, Cmd+A and Cmd+D; shortcut dispatch itself is outside the runtime adapter.
- An initial full Swing mouse dispatch in headless mode failed with
  HeadlessException in BasicGraphicsUtils. The successful adapter explicitly
  normalizes focus before calling original mousePressed; it does not silently
  replace the original marking handler. The contract documents this boundary.
- Original FileTable construction requires full desktop wiring. A serialization
  constructor executes JTable's constructor and bypasses only FileTable's wiring.
  A field-only MainFrame supplies activeTable; AbstractAction constructors run
  for each original action. Translator and CustomDateFormat initialization are
  required. All behavioral methods remain original, unmodified JAR methods.
- Runtime preparation first failed on package-private SortInfo setters, then on
  passing Ctrl directly as JTable's toggle parameter (which can deselect the
  focused row). Reflection invokes the original setters; normalized focus uses
  changeSelection(row,0,false,false). This clarifies the original contract's
  post-Swing boundary. Raw failed runs remain under artifacts/runs/W02-*.
- The first Rust comparison, run `20261015T200718Z-a4fd266f`, reported 20 snapshot
  differences. Initial sorting incorrectly preserved the first unsorted entry
  when no parent existed. Initial load now sorts before selecting row zero;
  subsequent sorts preserve identity. The rerun and reserved suites pass.
- Subsequent sorting must mutate the existing permutation; resetting it before
  each sort can change ties. W01's sorter now exposes an in-place entry point;
  its original initial-order comparisons remain unchanged and all 342 pass.
- Native testing exposed modifier-click handling using the frame's final
  modifier state. Quick Ctrl/Shift release could occur before rendering. The
  UI now reads the pointer event's modifiers; fresh native Ctrl/Shift checks
  demonstrate the intended transitions. Space uses key events including release.
- One native UI inventory took several minutes; a replaced local .app was later
  rejected by LaunchServices. A fresh owned bundle launched successfully. No
  failed/unchanged input attempt was counted as verification. Test wrappers are
  ignored local artifacts, not published packages.
- Original representative captures retain marks/focus through reverse sorting.
  Its AX/status text did not update reliably enough to establish key dispatch or
  exact modifier-click timing. The headless oracle and native UI checks are
  reported separately, with no whole-application or exhaustive GUI parity claim.
- Refresh and return-to-parent restoration follow recovered FileTable callers
  and FolderChangeThread. Rust tests cover surviving marks, vanished entries,
  failed navigation, pane isolation and parent focus. These are integration
  checks, not original-runtime transition comparisons.
