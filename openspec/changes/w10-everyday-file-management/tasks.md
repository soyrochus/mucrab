# W10 implementation tasks

## 1. Recovery and contracts

- [x] 1.1 Recover the five original action paths and platform providers from verified pinned binaries; record hashes and static/runtime boundaries.
- [ ] 1.2 Observe effective selection, shortcuts, dialogs, partial errors, Trash behavior, clipboard formats, quick-find matching and Back/Forward transitions.
- [ ] 1.3 Freeze scoped contracts, predefined fixtures and independently seeded reserved cases before Rust implementation; record intentional divergences.
- [ ] 1.4 Choose Linux/macOS Trash and clipboard adapters after disposable interoperability probes and license review; document unavailable host checks.

## 2. New Folder

- [x] 2.1 Implement validated single-leaf creation with no replacement, stale-destination checks and actionable errors.
- [x] 2.2 Add File → New Folder and the recovered platform shortcut, respecting dialogs, ZIP read-only state and the single-job gate.
- [ ] 2.3 Verify success/focus, cancellation, Unicode/hidden names, invalid names, collisions, denied creation and changed destinations against original cases and both-pane native input.

## 3. Delete/Trash

- [x] 3.1 Add captured-selection review that clearly distinguishes Trash from permanent deletion; make Trash the ordinary Delete action.
- [x] 3.2 Implement platform Trash dispatch and recursive permanent deletion without following links or deleting protected roots/ancestors.
- [x] 3.3 Integrate W07 progress, cooperative cancellation, deferred close, partial results and successful/unfinished mark handling.
- [x] 3.4 Verify cancellation, stale plans, hidden descendants, broken/external links, permission failures, partial trees and no automatic permanent-delete fallback.
- [ ] 3.5 On Linux and macOS, verify actual Trash entries and platform restoration of disposable files/folders, including unsupported or cross-volume behavior where available.

## 4. Clipboard Copy/Paste

- [x] 4.1 Implement native file clipboard read/write, validated local file URL conversion and stable deduplication; preserve entry/link identity.
- [x] 4.2 Add focus-aware Copy/Paste menu actions and platform shortcuts; retain normal text editing in fields/dialogs.
- [x] 4.3 Build one captured, reviewed Paste plan for multi-parent sources; reuse W03 collision guards and W07 single-job execution without altering unrelated marks.
- [ ] 4.4 Verify muCrab-to-native and native-to-muCrab interoperability on Linux/macOS, escaped Unicode/space/percent/hash filenames, malformed/remote payloads, missing sources, overlaps and cancellation.

## 5. Quick-find

- [x] 5.1 Add transient prefix-query/match state over the current visible order with documented case/Unicode policy, wrap and no-match behavior.
- [x] 5.2 Integrate committed text input, find-field editing, next/previous, Escape/Enter handling and focused-row scrolling without changing marks.
- [ ] 5.3 Verify hidden entries, sorting, repeated letters, Unicode/IME commit, empty/no-match queries, pane switching, refresh and W09 Enter activation.

## 6. Navigation history

- [x] 6.1 Add structured, independent, bounded local/ZIP history with success-only commits, branch truncation and duplicate suppression.
- [x] 6.2 Add Back/Forward menu/toolbar actions and recovered shortcuts with correct enabled state and text-field precedence.
- [x] 6.3 Verify initial/root states, bounds, failure retention, stale/missing ZIPs, parent/Home/location entry, same-location refresh, independent panes and forward-branch changes.

## 7. Integration and closure

- [x] 7.1 Run original/Rust predefined and reserved comparisons for each recoverable boundary; retain explicit differences and native-only assertions.
- [ ] 7.2 Run both-pane Linux and macOS native replays with isolated clipboard/Trash fixtures, exact filesystem checks and captures.
- [x] 7.3 Pass formatting, denied-warning Clippy, Rust/Python checks and W07/W08/W09 regression replays; preserve reference integrity.
- [x] 7.4 Update README, current status, workflow ledger, dependency notices and W10 evidence with exact host results and remaining limits.
- [ ] 7.5 Complete W10 only after all five stages and required host checks pass; leave incomplete checks pending without claiming full muCommander parity.

## Implementation checkpoint — 2026-10-17

All five features are implemented. Linux unit/native checks and bounded original
comparisons are recorded in `evidence/workflows/W10-everyday-file-management/`.
Unchecked verification tasks remain deliberately visible:

- 1.2: original jobs/history/matching/clipboard execute; original full-window
  dialogs and exhaustive platform shortcut/partial-error UI parity are not proved.
- 1.3: the contract, fixture categories and seeds preceded Rust implementation;
  the executable generator and exact native input trace were written afterwards.
  This sequencing limit cannot be retroactively marked as a blind fixture gate.
- 1.4, 3.5, 4.4 and 7.2: Linux X11 native services and both-pane input pass; native
  macOS interoperability/Trash checks require a Mac host. Wayland is unverified.
- 2.3 and 5.3: Linux success/error/Unicode/core and native focus checks pass;
  original full-window error dialogs and real IME composition input remain unverified.
- 7.5: W10 is implemented and Linux-verified, not closed across required hosts.

No future implementation run should replace these limits with a broad parity claim.
