# W17 tasks

All implementation tasks are pending. Execute correctness fixes in reviewable slices and keep existing compatibility tests green. W16 owns theme/layout choices; share component/loading contracts without introducing a correctness dependency on visual completion. The external BeautifulRust checkout is a review source, not an implementation prerequisite.

## 1. Baseline and failing regressions

- [ ] 1.1 Record current workspace checks, compare snapshot/serialization formats, invariant-bearing public fields and existing platform/service verification gaps.
- [ ] 1.2 Add failing regressions for invalid remote-location construction/deserialization and the ZIP-to-local captured-viewer target inconsistency; distinguish API misuse from proven ordinary UI paths.
- [ ] 1.3 Audit captured transfer, metadata and rename plans plus pane/listing/selection relationships; identify mutable evidence-bearing fields and write focused regressions for unsafe transitions.
- [ ] 1.4 Add narrow test injection for destination write/sync/remove operations and a failing sync-before-delete ordering regression without replacing real cross-device tests.

## 2. Validated core state and error contracts

- [ ] 2.1 Encapsulate `RemoteLocation`, route serde through validated conversion, add borrowed accessors/formatting and guarantee valid root/parent navigation; migrate workspace callers and verify saved-data round trips.
- [ ] 2.2 Replace optional cross-dependent viewer capture fields with coherent private variants and read-only inspection; require fresh capture for retargeting and add compile-fail API examples where useful.
- [ ] 2.3 Encapsulate evidence-bearing transfer-plan targets/policies and migrate callers; retain staged SFTP publication, trust checks and current snapshot formats.
- [ ] 2.4 Encapsulate evidence-bearing metadata and batch-rename plan state; retain fresh-preview validation and established partial-result semantics.
- [ ] 2.5 Model mutually exclusive committed provider state with the existing location enum, separate pending navigation, and migrate local/ZIP/SFTP callers together.
- [ ] 2.6 Encapsulate listing/order/selection updates, expose needed read-only inspection and reject or internally derive inconsistent permutations; retain compare adapters and selection snapshots.
- [ ] 2.7 Introduce focused input/stale/cancel/limit error contracts at recovery boundaries, preserving underlying I/O causes and path/operation context; keep appropriate `io::Result` APIs and migrate presentation conversion.
- [ ] 2.8 Run malformed-input and normalisation/round-trip tests plus current core/compare regressions; verify diagnostics redact secrets and no new panic path replaces an error.

## 3. Desktop navigation boundaries and background browsing

- [ ] 3.1 Extract workspace navigation state/actions and a captured browse request/result interface from the root controller; inventory every local/ZIP navigation/refresh trigger, including post-job refreshes and inactive tabs.
- [ ] 3.2 Move canonicalisation, directory enumeration, metadata loading, ZIP catalog preparation and sorting into browse workers; keep production core listing operations usable synchronously outside UI callbacks.
- [ ] 3.3 Implement at most one running and one latest queued browse request per pane, with cooperative cancellation, acknowledgement, replacement and worker ownership through shutdown.
- [ ] 3.4 Implement commit-on-success bound to stable tab, generation, requested location and applicable view options; retain previous listing/history/marks/focus on failure and reapply or supersede changed options.
- [ ] 3.5 Integrate loading/error/waiting state and close handling with explicit presentation data; keep the event loop responsive and unrelated mutation/search/viewer/SFTP workers independent.
- [ ] 3.6 Use delayed/failing providers to verify rapid replacement, stale/queued completion, inactive/closed origin tabs, live Show Hidden/sort changes, failed refresh and bounded worker count.
- [ ] 3.7 Verify successful parent/history navigation, marks/focus restoration and post-job refresh using real local and ZIP fixtures; provide W16 with the resulting component/loading contract.

## 4. Checked move persistence

- [ ] 4.1 Add checked destination `sync_all` after fallback writes and the established metadata step and before source deletion; preserve the source and identify the failed stage on sync/write error.
- [ ] 4.2 Verify cancellation before deletion and deletion failure retain the source; assert write/sync/remove ordering through the narrow test adapter.
- [ ] 4.3 Run real same-filesystem rename and cross-device move regressions, including collisions, direct overwrite, partial destination, links and earlier completed items; retain existing report semantics.
- [ ] 4.4 Document the strengthened file-sync guarantee, existing best-effort metadata/direct-overwrite behaviour and limits on directory/power-loss durability; record staged local overwrite as deferred rather than implemented.

## 5. Desktop decomposition and idioms

- [ ] 5.1 Extract action-confirmation components from `app.rs` with owned dialog state and explicit returned actions; preserve validation and default/cancel behaviour.
- [ ] 5.2 Extract mutation/job coordination with explicit origin/result context; preserve panic containment, acknowledgement and origin-tab unmarking/focus rules.
- [ ] 5.3 Reduce the root UI method to composition/dispatch, replace production glob imports/re-exports and narrow visibility; avoid introducing a general event framework or shared mutable application handle.
- [ ] 5.4 Review panic/poison recovery, domain boolean modes, borrowed parameters and clones in touched code; fix demonstrated weaknesses and document justified invariant/application exceptions.
- [ ] 5.5 Replay desktop keyboard/menu/button, focused-field, viewer clipboard and concurrent job regressions after extraction; reconcile any W16 edits without changing its visual contract.

## 6. Documentation and parser boundaries

- [ ] 6.1 Update the core crate/module overview and document validated location, pane, selection, request/plan, error and job APIs with cancellation, errors, bounds, side effects and relevant panic contracts.
- [ ] 6.2 Document remaining public core APIs and add executable fallible/compile-fail examples that need only disposable fixtures; supply useful standard derives or redacted Debug implementations.
- [ ] 6.3 Add bounded deterministic malformed/truncated/oversized SFTP packet and ZIP path/header regression cases and retain parser limit checks.
- [ ] 6.4 Add documented opt-in fuzz entry points for those private parsers using the narrowest test-only access needed; require neither live servers nor personal filesystem access.
- [ ] 6.5 Resolve public-documentation/debug lint omissions and enable the agreed workspace lints with justified narrow exceptions; retain first-party `unsafe_code = "forbid"` and command-level warning escalation.

## 7. Continuous checks and completion evidence

- [ ] 7.1 Add pull-request and ordinary branch-push CI for formatting, strict all-target/all-feature Clippy, locked all-feature workspace tests, documentation and Python tests.
- [ ] 7.2 Provision unprivileged CI and a real second filesystem for existing move tests; keep native/service credential checks isolated and report unavailable environments accurately.
- [ ] 7.3 Add strict Clippy to the existing release validation path without weakening frozen/offline release builds or current Linux/macOS tests.
- [ ] 7.4 Run full workspace checks and selected original/native workflow regressions; identify intentional loading/durability differences separately from compatibility failures.
- [ ] 7.5 Exercise browsing responsiveness, replacement, shutdown and unchanged interaction contracts on available Linux/macOS hosts; record uninterruptible-I/O and unavailable host/service limitations explicitly.
- [ ] 7.6 Update current status/architecture/API documentation and W17 evidence, record changed workspace APIs and validate the change strictly; leave earlier unfinished W09–W15 gates open.
