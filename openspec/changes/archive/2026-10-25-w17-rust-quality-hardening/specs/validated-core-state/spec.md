## ADDED Requirements

### Requirement: Validate remote values through every construction boundary
Remote locations SHALL expose only invariant-preserving construction, parsing, deserialization and navigation operations. Callers SHALL NOT mutate validated location fields directly. Invalid serialized values SHALL return an error, and valid serialized shapes SHALL remain compatible with existing saved bookmarks and connection data. Parent navigation for a valid value SHALL NOT panic because of invalid user-supplied fields.

#### Scenario: Deserialize an invalid remote location
- **WHEN** input contains an invalid profile identifier, relative path, control character, excessive path length or forbidden separator
- **THEN** deserialization returns a structured error instead of constructing an invalid location
- **AND** the error omits credentials and other secret data

#### Scenario: Round-trip a valid normalized location
- **WHEN** a valid location is parsed, displayed, serialized and deserialized, and its parent is requested repeatedly
- **THEN** its normalized identity is preserved and parent navigation terminates at the root without panicking

### Requirement: Keep captured targets and evidence coherent
Captured viewer requests and reviewed mutation plans SHALL retain their target and validation evidence as a coherent value. Changing a target or a policy that invalidates captured evidence SHALL require a validated transition or fresh capture. Public APIs SHALL NOT expose mutation that bypasses those relationships. Known stale filesystem identities SHALL continue to produce errors before an operation uses invalid captured assumptions.

#### Scenario: Attempt to retarget a captured viewer request
- **GIVEN** a request was captured for a ZIP member with archive identity evidence
- **WHEN** a caller needs to view a local file instead
- **THEN** direct target reassignment is unavailable through the public API and a new request must be captured
- **AND** validating either well-formed request does not rely on an unrelated optional field being populated

#### Scenario: Apply a stale reviewed plan
- **GIVEN** a transfer, metadata or rename plan captured filesystem evidence
- **WHEN** the target changes externally before application
- **THEN** application rejects the stale assumptions through an error or the established explicit partial-result contract
- **AND** callers cannot change a captured target to bypass the evidence check

### Requirement: Encapsulate committed pane and listing invariants
Committed local, ZIP and remote locations SHALL use mutually exclusive provider state, distinct from pending navigation. Listing entries, their ordering and their selection SHALL be updated through operations that preserve index and identity consistency. Read-only inspection and existing comparison snapshots SHALL remain available without requiring callers to mutate invariant-bearing fields.

#### Scenario: Commit a provider transition
- **WHEN** a pane successfully transitions from a local directory to ZIP or SFTP content
- **THEN** one coherent committed provider/location is visible with matching listing, order and selection state
- **AND** failed or superseded preparation cannot leave a mixed provider state

#### Scenario: Reject inconsistent listing input
- **WHEN** externally supplied listing/order data contains an invalid index or duplicate permutation entry
- **THEN** a checked boundary rejects it or derives a valid order internally
- **AND** selection operations do not index outside the owned listing or mark storage

### Requirement: Expose actionable errors at recovery boundaries
Core operations requiring different caller recovery actions SHALL distinguish invalid input, stale capture, cancellation, limits and operational failure through typed errors or an existing precise standard error contract. Errors SHALL implement `std::error::Error` and `Display`, preserve underlying causes where applicable and include relevant operation/path context without secrets. Presentation code SHALL convert errors to user-facing text at its boundary.

#### Scenario: Distinguish stale input from an I/O failure
- **WHEN** a captured request is stale or the underlying filesystem independently denies access
- **THEN** the caller can distinguish refresh/review recovery from an operational failure without matching message text
- **AND** wrapped failures retain their underlying cause where available
