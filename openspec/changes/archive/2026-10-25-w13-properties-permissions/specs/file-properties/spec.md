## ADDED Requirements

### Requirement: Inspect the captured focused entry
Properties SHALL inspect the focused child independently of unrelated marks and remain bound to its captured identity and origin. It SHALL display available local metadata including kind, path, sizes, timestamps, UID/GID, mode and link/hard-link information, with explicit unavailable fields. Link properties SHALL describe the link itself and its target text without following the target for edits. ZIP properties SHALL show only available member metadata and disable mutations.

#### Scenario: Inspect a link while another item is marked
- **GIVEN** a focused symbolic link and marks on unrelated files
- **WHEN** Properties is opened and the active tab later changes
- **THEN** the view continues to describe the captured link and does not change marks or target metadata

#### Scenario: Inspect unavailable ZIP or owner information
- **WHEN** a ZIP member lacks ownership data or local account-name lookup fails
- **THEN** unsupported fields are marked unavailable and available numeric IDs/member metadata remain accurate

### Requirement: Calculate scoped folder contents asynchronously
Folder size SHALL be a cancellable background calculation of logical bytes and entry counts, including hidden descendants but excluding link targets and descendant mount roots. Hard-linked entries SHALL count once per directory entry and SHALL NOT be presented as unique allocated storage. Errors, cancellation and depth 256 or one-million-entry limits SHALL distinguish partial from complete totals without blocking existing file jobs.

#### Scenario: Cancel a partially readable folder calculation
- **WHEN** calculation encounters an unreadable subtree or is cancelled
- **THEN** known counts and diagnostics remain visible as partial
- **AND** no source metadata or W07 mutation controls are changed

### Requirement: Review and apply bounded metadata edits
The application SHALL review and apply only ordinary rwx bits and modified time for one captured local non-link entry, using stale-plan validation and the single mutation worker. Permission changes SHALL retain special bits; time changes SHALL preserve access time and respect displayed filesystem precision. Changes SHALL require no privilege elevation. Partial success and cancellation between field changes SHALL be reported without rollback claims, and unrelated marks SHALL be preserved.

#### Scenario: Reject stale metadata edits
- **WHEN** the reviewed entry is replaced or its relevant mode/time assumptions change before Apply
- **THEN** the request is rejected before changing the replacement

#### Scenario: Retain and report a partial update
- **WHEN** one reviewed field change succeeds and a later change fails or is cancelled
- **THEN** the successful change remains and actual per-field outcomes are displayed and refreshed from disk

#### Scenario: Preserve hard-link semantics
- **GIVEN** a file with multiple hard links
- **WHEN** a metadata edit is reviewed
- **THEN** the link count and effect on all names of that inode are visible before Apply
