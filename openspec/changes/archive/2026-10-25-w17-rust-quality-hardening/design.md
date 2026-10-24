# W17 design

## Context

The workspace separates `core`, `desktop` and `compare`, and forbids first-party unsafe code. The 2026-10-24 review passed formatting, strict all-target/all-feature Clippy, documentation generation, 107 Rust tests and 51 Python tests; four environment-dependent Rust tests were ignored. These are baseline observations, not future completion evidence. Existing original-binary compatibility algorithms, partial-result rules and open platform gates remain important constraints.

Two public-API invariant violations were reproduced: constructing an invalid `RemoteLocation` can make `parent()` panic; capturing a ZIP `ViewRequest` then changing its public target to the archive's local path can make `validate()` panic. These are safe-code API failures, not established ordinary UI crash paths. Folder loading performs synchronous local/ZIP filesystem work from UI callbacks. Move fallback drops its output and removes the source without a checked destination synchronisation; no data-loss incident was reproduced.

The review used BeautifulRust's `rust-best-practices` guidance. The resulting requirements are recorded here so any agent can implement them without a Claude plugin or an external skill checkout.

## Goals / Non-Goals

**Goals:** Preserve captured-state invariants; keep browsing responsive; strengthen copy-fallback move durability; make the desktop easier to change; expose actionable error contracts; document APIs and enforce checks routinely.

**Non-Goals:** Rewrite the application, add an async runtime, publish a general-purpose library, remove dependency pins without cause, replace compatibility algorithms mechanically, make all filesystem operations race-proof, promise transactional multi-file rollback or full power-loss recovery, or redesign W16's visuals. Staged local overwrite publication is deferred because it changes the existing partial-destination contract and needs its own compatibility decision.

## Decisions

### 1. Encode validated and captured state in types

Make `RemoteLocation` fields private. Route constructors, parsing and deserialization through the same validation/normalisation boundary, using a private wire DTO and `TryFrom`/custom Deserialize. Preserve the valid persisted shape. Provide borrowed accessors, `Display` where appropriate and fallible child operations. A valid value's parent must be constructible without a user-input-dependent panic. Validate incoming remote values at the public boundary rather than relying on callers to remember a constructor.

Represent captured viewer requests with variants that keep local identity/resolution data or ZIP identity/member data together. Expose read-only target inspection; retargeting requires fresh capture. Audit other captured plans, including transfer, metadata and rename plans, for mutable targets/policies paired with private evidence. Make those relationships immutable or validate replacement as a complete new capture. Passive output reports can retain public fields.

Encapsulate the pane's committed location/listing/order/selection relationship. Local, ZIP and remote committed locations use one enum, distinct from pending navigation. Callers get read-only access and explicit validated state transitions; a location cannot simultaneously claim incompatible providers, and permutations must index the owned listing. Adapt the existing `PaneLocation` rather than creating a second competing location model. Keep existing compare snapshots and valid bookmark/SFTP serialization stable.

Write regression tests before fixes. Use compile-fail documentation examples for unsupported mutation where useful, runtime malformed-input tests, and properties such as parse/display round trips and normalisation idempotence. Do not claim memory unsafety: the demonstrated failures are panics in safe APIs.

**Alternative considered:** adding more `expect` messages or revalidation everywhere leaves mutable relationships exposed and shifts the proof obligation to every caller.

### 2. Use bounded browsing workers with explicit commit

Introduce a captured browse request/result for local and ZIP listing work. UI actions schedule preparation and return; canonicalisation, enumeration, metadata loading, ZIP catalog work and sorting run on a worker. A request captures stable tab ID, requested location, view options and generation. Results contain owned data and are committed on the UI thread only if that tab still exists and the request still matches. Preserve the last successful location, history, listing, focus and marks until a successful commit; failures are associated with the requested location and do not masquerade as an empty directory.

Use one running browse worker per pane and at most one queued latest replacement per pane, bounding active preparation across many tabs. Retain worker ownership through cancellation and completion; starting replacements waits for acknowledgement. Inactive-tab results can update only their originating surviving tab. Sorting/filtering changes during preparation must either supersede the captured request or be reapplied before commit, never silently overwritten. Share Show Hidden semantics across tabs as today.

Cancellation is cooperative between blocking operations. Ordinary filesystem calls and ZIP parsing can still stall inside the OS or a decoder: keeping them off the UI thread does not make them interruptible. Keep the event loop responsive and show waiting/cancelling state without launching an unbounded replacement chain. Window close follows the existing worker-acknowledgement policy. Reuse `ReadJob` lifecycle principles, but do not reuse decoder panic wording or force unrelated progress/result types together. UI tests use injected delayed/failing listing providers; core production APIs remain straightforward synchronous operations where useful.

**Alternative considered:** adding async filesystem wrappers introduces a runtime and does not remove underlying blocking filesystem work. A fresh detached thread per navigation has unbounded lifetime and stale-result problems.

### 3. Check move destination persistence before source removal

For regular-file copy fallback, finish writes, apply the established metadata policy, call and check `File::sync_all` on the destination handle before permitting source deletion, then honour cancellation before deleting. A write or synchronisation failure reports a failed outcome and preserves the source. A deletion failure reports both retained copies as today. Expose a narrow internal injectable transfer boundary so a deterministic test can force the sync step to fail and record write/sync/remove ordering without depending on failing hardware.

Keep atomic same-filesystem rename as the preferred path. Keep existing direct-write overwrite and retained-partial-destination rules for compatibility; explicitly document that a failed overwrite can leave a partial destination even though its source is preserved. Do not describe this as transactional replacement or complete power-failure durability: directory persistence, device behaviour and concurrent external changes remain outside that guarantee. SFTP's staged publication remains intact.

**Alternative considered:** a universal staged overwrite is attractive, but changes cancellation, collision, metadata and original-comparison semantics. It is not necessary to close the unchecked-sync-before-delete gap and is deferred explicitly.

### 4. Introduce focused errors and component ownership

Use typed errors at domain boundaries where the caller distinguishes invalid input, stale capture, cancelled work, limits and underlying operation failure. Implement `Error`, `Display`, `source()` and conversions, preserving useful path/operation context and keeping secrets out of diagnostics. Plain `io::Result` stays where it accurately expresses the contract. Convert errors into user text at the presentation boundary; do not replace all errors with one oversized crate-wide enum or force a new error dependency.

Split the desktop root into composition/dispatch, workspace navigation, action confirmations and job coordination, retaining existing feature modules. Components own their state and return explicit action enums to the application. Extract helpers for cohesive jobs, not arbitrary line-count limits. Replace production glob imports/re-exports with explicit names and narrower visibility; group loose arguments into meaningful request/context values when it clarifies ownership. Keep message passing, small immutable clones, application panic boundaries and justified shared sessions. Review poisoned-lock recovery and any remaining panic sites by invariant, with documented reasoning; do not mechanically turn impossible invariants into misleading recoverable errors.

**Alternative considered:** moving methods between files while every module imports all of `MuCrabApp` preserves the same coupling. A generic service/event framework would add unnecessary abstraction.

### 5. Make checks and contracts durable

Document the core crate's actual current scope, its public modules/types/methods, errors, cancellation, side effects, bounds and relevant panic contracts. Add executable examples using fallible APIs; redact secrets in manual `Debug` implementations. Resolve documentation/debug warnings before enabling their workspace lint policy. Keep meaningful standard derives and borrowed parameters; use semantic enums for domain modes where a boolean obscures intent, without replacing obvious UI setters solely for checklist compliance.

Add pull-request and ordinary branch-push validation with formatting, strict Clippy, workspace tests/all features, documentation and Python tests. Add missing strict Clippy to the release validation path. Keep warnings-as-errors in commands, not `deny(warnings)` source attributes. CI must provide unprivileged execution and two real filesystems for existing move tests, as release validation already does. Never run credential integration tests against personal stores; isolate optional service tests and report their status separately. Keep locked dependency resolution; review additional dependencies and supply-chain tools separately rather than claiming a vulnerability audit from Clippy.

Add targeted generated/malformed cases at remote path deserialization, viewer request capture, selection invariants and parser limits. Add a bounded opt-in fuzz harness for the custom SFTP packet decoder and ZIP path/header validation using existing private parsing boundaries; normal CI retains deterministic regression cases without requiring nightly fuzz execution. Test real I/O as well as injected failure paths. No blanket ban on `Arc`, threads, `expect`, exact dependency pins or application-level string presentation is introduced.

## Risks / Trade-offs

- Encapsulation changes workspace APIs → migrate core, desktop, compare and test callers together; retain valid stored/wire shapes and explicitly test round trips.
- Background browsing alters event ordering → bind results to tab/generation/options, preserve previous state on failure, and verify rapid navigation, tab closure and live copy jobs.
- Blocking storage cannot always acknowledge cancellation promptly → keep ownership bounded and UI responsive; document shutdown waiting instead of claiming interruption guarantees.
- Synchronisation increases move latency → limit the added mandatory barrier to the copy-fallback path and measure separately from atomic renames.
- Structural work can hide behaviour changes → extract in small slices with current regression fixtures green; keep intentional durability/loading differences explicit.
- New lint coverage can create noisy work → fix affected contracts and enable lints incrementally, with narrowly justified exceptions rather than broad suppression.

## Migration Plan

1. Record current snapshots, public call sites and tests; add failing invariant and sync-order regressions.
2. Encapsulate core state and introduce focused error contracts, preserving stored data and comparison formats.
3. Extract desktop navigation/job interfaces; implement bounded background browsing and commit rules.
4. Add checked destination synchronisation and fault-injection verification.
5. Finish component decomposition, documentation, parser regression/fuzz entry points and CI enforcement.
6. Run workspace checks, selected original/native regressions and separate Linux/macOS interaction checks; publish evidence and intentional differences.

Each slice is independently reviewable. Rollback uses source changes without a user-data migration; do not remove destination-sync safeguards merely to simplify a later refactor. Existing unfinished W09–W15 tasks and unavailable host/service gates remain open. W16 can build on the component interfaces after step 3; it does not gate correctness fixes.

## Open Questions

No user decision blocks implementation. Confirm platform-specific `sync_all` and metadata-handle ordering through Linux/macOS tests; document unsupported cases rather than silently weakening source preservation. Choose the smallest error and test-adapter implementation that meets the specified recovery and failure-order contracts.
