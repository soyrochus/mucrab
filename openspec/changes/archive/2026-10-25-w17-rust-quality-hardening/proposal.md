# W17 — Rust quality hardening

## Why

muCrab has substantial behavioural tests and file-operation safeguards, but publicly mutable validated state can violate captured invariants, local browsing performs blocking work on the UI thread, and move fallback deletes sources without a checked destination synchronisation step. Addressing these findings alongside desktop decomposition, clearer error contracts and routine quality gates will make the application safer to extend.

## What Changes

- **BREAKING (unpublished workspace APIs):** Encapsulate validated remote locations and captured requests/plans; use validated deserialization and coherent state variants. Replace affected field access with explicit accessors and validated operations, migrating all workspace callers together.
- Move local/ZIP listing preparation out of UI callbacks into bounded, generation-tagged workers with cancellation, stale-result isolation and commit-on-success behaviour.
- Check destination synchronisation before deleting a move source, report failures while preserving the source, and document direct-overwrite versus staged-publication semantics.
- Split the desktop controller into components with explicit actions and ownership; preserve existing user-visible workflow semantics.
- Introduce typed domain errors where callers need recovery decisions, improve Rust API documentation and add behavioural documentation examples.
- Add pull-request quality checks, apply missing Clippy enforcement to releases, and add targeted malformed-input/property coverage for validated values and parser boundaries.

## Capabilities

### New Capabilities

- `validated-core-state`: Invariant-preserving values, captured requests/plans and structured recoverable errors.
- `background-directory-browsing`: Responsive local/ZIP listing and result lifecycle bound to tab and location identity.
- `durable-local-move`: Checked destination persistence before copy-fallback source removal and explicit partial-result semantics.
- `rust-maintainability-gates`: Desktop component boundaries, public API documentation, regression protection and continuous workspace checks.

### Modified Capabilities

None. No archived baseline specs exist in `openspec/specs/`. These cross-cutting additions extend the implemented W09–W15 workflows without editing their outstanding tasks or claiming full parity.

## Impact

Touches `crates/core` state, request, error and move code; desktop browsing, job lifecycle and controller modules; workspace lint configuration; tests, documentation and `.github/workflows`. Preserve existing serializers used by comparison fixtures unless an explicitly versioned migration is required. Keep first-party unsafe forbidden and retain justified application-oriented threads and concrete types.

Use BeautifulRust's agent-independent Rust guidance as a checklist, recorded in this change's design; implementation must not require Claude Code, a particular agent, or the external BeautifulRust checkout. Dependency pins, compatibility algorithms, panic boundaries and threading choices require contextual review, not mechanical replacement. W16 owns visual design and consumes the resulting component/loading contracts. This proposal adds planning artifacts only; it does not implement Rust changes.
