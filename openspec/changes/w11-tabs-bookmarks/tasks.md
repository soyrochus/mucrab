# W11 tasks

## 1. Recovery and contract

- [x] 1.1 Recover original tab/bookmark actions, duplicate/close behavior, shortcuts, persistence and job-to-tab relationships from pinned binaries.
- [x] 1.2 Freeze the state protocol, exact fixture generator and predefined/reserved cases before Rust code; record chosen differences, limits and host availability.

## 2. Implementation

- [x] 2.1 Add stable per-pane tab collections with isolated location/history/view/focus/marks, new/duplicate/close/switch actions and bounds.
- [x] 2.2 Capture tab identities in review/job state and preserve global progress, close cancellation, correct unmarking and affected-context refresh.
- [x] 2.3 Add validated, versioned bookmark storage with atomic saves and visible load/save errors.
- [x] 2.4 Add both tab strips and bookmark controls, keyboard focus rules and narrow-window behavior.

## 3. Verification and closure

- [ ] 3.1 Compare frozen original/native state cases including duplicate history independence, failed navigation, last-tab/limit handling and shared hidden filtering.
- [ ] 3.2 Verify same-path tabs with different marks, switching/closing origins during copy/move/Paste/open preparation and unchanged captured destinations.
- [x] 3.3 Verify bookmark restart, missing/renamed targets, duplicate labels/IDs, malformed/unsupported data and denied/failed writes without corrupting the old file.
- [ ] 3.4 Run Linux/macOS both-pane native replays, workspace checks and W07–W10 regressions; record unavailable checks as pending.
- [x] 3.5 Update status, ledger, notices and W11 evidence with exact scope/results; close only completed gates.

Implementation and Linux receipts: [W11 evidence](../../../evidence/workflows/W11-tabs-bookmarks/README.md).
Unchecked original full-trace/native-host gates remain pending; see the evidence index for exact tested boundaries.
