# rust-maintainability-gates Specification

## Purpose
Enforce explicit component boundaries, documented Rust contracts, routine quality checks and meaningful failure tests.
## Requirements
### Requirement: Keep desktop components behind explicit interfaces
The desktop root SHALL compose and dispatch cohesive workspace-navigation, confirmation and job-coordination components through explicit inputs/actions. Components SHALL own their state without depending on production glob imports from the entire application controller. First-party core behaviour SHALL remain independent of egui. Refactoring SHALL preserve current command availability, focus routing, tab-origin identity, cancellation acknowledgement and serialized comparison outcomes except for the explicitly specified browsing and durability changes.

#### Scenario: Refactor command routing
- **WHEN** an operation moves from the root controller into a component
- **THEN** existing keyboard, menu and button triggers still dispatch the same validated operation
- **AND** focused fields/viewers retain their existing precedence and job completion affects only its originating context

### Requirement: Document and enforce public Rust contracts
The core crate SHALL document its current scope and public modules, types and functions, including failure, cancellation, side-effect and limit contracts where applicable. Applicable public types SHALL provide meaningful standard traits, and diagnostics for secret-bearing types SHALL redact secrets. Workspace lint configuration SHALL retain forbidden first-party unsafe and enforce documented public APIs and useful Debug implementations after existing omissions are resolved. Warning escalation SHALL occur in check commands rather than source-wide `deny(warnings)`.

#### Scenario: Exercise documented APIs
- **WHEN** workspace documentation and documentation tests run
- **THEN** examples compile and exercise fallible construction or operations without requiring personal files, live services or the external BeautifulRust checkout
- **AND** public error contracts describe the recovery distinctions exposed by their types

#### Scenario: Inspect secret-bearing diagnostics
- **WHEN** Debug or Display formats a type or error associated with credentials
- **THEN** secret values are omitted or redacted and useful non-secret context remains available

### Requirement: Run quality checks before release tags
Pull requests and ordinary branch pushes SHALL run formatting checks, strict Clippy across all workspace targets/features, workspace tests with all features, documentation generation and Python tests using locked dependency resolution where applicable. Release validation SHALL include strict Clippy in addition to its existing checks. Filesystem tests SHALL run unprivileged with two real filesystems; service/host-dependent gates SHALL be reported separately from ordinary checks and SHALL NOT use personal credential stores.

#### Scenario: Reject a quality regression on a pull request
- **WHEN** a pull request introduces a formatting, Clippy, Rust test, documentation or Python test failure
- **THEN** the corresponding automated check fails before a release tag is required
- **AND** logs identify which command failed

#### Scenario: Report unavailable integration environments
- **WHEN** the required platform, disposable service or second filesystem is unavailable
- **THEN** the relevant gate is explicitly failed or reported unavailable rather than asserted to pass
- **AND** ordinary passing checks do not mark those service/platform gates complete

### Requirement: Protect malformed-input and failure boundaries with meaningful tests
Tests SHALL cover invalid remote deserialization, captured-request invariants, coherent pane/listing transitions, browse replacement/cancellation and move synchronisation failures. Custom SFTP packet and ZIP path/header parsing SHALL have bounded malformed-input regression cases and opt-in fuzz entry points without opening real network connections or writing outside disposable paths. Existing original/native compatibility and real-I/O tests SHALL remain available; test-only failure injection SHALL NOT become a mutable production bypass.

#### Scenario: Process malformed parser input
- **WHEN** a deterministic regression or fuzz harness supplies truncated, oversized or inconsistent parser input
- **THEN** the exercised boundary returns a bounded error without a panic, live service dependency or uncontrolled allocation

#### Scenario: Preserve prior workflow coverage
- **WHEN** the refactored workspace runs its existing behavioural tests and selected native/original comparisons
- **THEN** retained workflow contracts continue to pass and intentional new differences are identified explicitly
- **AND** unfinished W09–W15 tasks and unavailable host checks remain pending

