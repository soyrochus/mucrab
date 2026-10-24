# background-directory-browsing Specification

## Purpose
Keep local and ZIP browsing responsive with bounded background preparation, origin-bound commits and tracked cancellation.
## Requirements
### Requirement: Prepare local and ZIP listings off the UI thread
Local and ZIP navigation, refresh and listing preparation SHALL perform canonicalisation, enumeration, metadata collection, catalog loading and sorting outside UI callbacks. The event loop SHALL remain available while preparation waits on storage. The committed listing SHALL remain visible until a successful replacement is accepted; loading and failure SHALL be distinguishable from a successfully loaded empty directory.

#### Scenario: Navigate to a slow location
- **GIVEN** an injected listing provider is delayed
- **WHEN** navigation requests that location
- **THEN** the UI can process subsequent events while the request is pending
- **AND** the previously committed listing remains visible with a separate loading state

#### Scenario: Fail navigation or refresh
- **WHEN** preparation fails because of access denial, invalid archive content or unavailable storage
- **THEN** the last successful location, listing, history, focus and marks remain committed
- **AND** the error identifies the failed request without presenting its destination as loaded

### Requirement: Commit only results matching their origin and request
Browse requests/results SHALL carry stable tab identity, requested location, captured view options and a generation. A result SHALL commit only to its surviving originating tab when it is still the latest applicable request. Sorting or shared hidden-file changes during preparation SHALL supersede the request or be reapplied before commit. Successful commits SHALL preserve existing focus, marking, parent-navigation and history semantics.

#### Scenario: Complete an older request after a newer navigation
- **GIVEN** request A is pending and the user requests B for the same tab
- **WHEN** A finishes before or after B is queued
- **THEN** A cannot replace the requested B state or append a superseded navigation to history
- **AND** only the accepted request applies its listing and options

#### Scenario: Switch or close the originating tab
- **WHEN** a request completes after its originating tab becomes inactive or is closed
- **THEN** a surviving origin can receive only its own still-applicable result and a closed origin's result is discarded
- **AND** another tab's listing, marks, sort options and history remain unchanged

#### Scenario: Change view options while loading
- **WHEN** sorting or Show Hidden changes while a listing is being prepared
- **THEN** the accepted listing reflects the current applicable options
- **AND** inactive tabs retain the existing shared hidden-file policy and independent sorting

### Requirement: Bound browse workers and retain cancellation ownership
The desktop SHALL run at most one browse preparation worker per pane and queue at most one latest replacement per pane. Superseding requests SHALL cancel prior work cooperatively and wait for its acknowledgement before starting replacements. Worker ownership SHALL remain tracked through result delivery, cancellation and shutdown; unrelated mutation, search, viewer and SFTP work SHALL retain its existing lifecycle. The UI SHALL NOT claim that an underlying blocking filesystem call has been forcibly interrupted.

#### Scenario: Rapidly navigate across several tabs
- **WHEN** repeated requests arrive while a pane's worker is blocked
- **THEN** the running worker and latest replacement remain bounded and intermediate replacements are discarded
- **AND** the event loop remains responsive without accumulating detached browse threads

#### Scenario: Close during blocked preparation
- **WHEN** the user closes the window with a browse request pending
- **THEN** cancellation is requested and the existing acknowledgement-before-close policy tracks the worker until completion
- **AND** completion cannot publish stale content or silently abandon active filesystem work

