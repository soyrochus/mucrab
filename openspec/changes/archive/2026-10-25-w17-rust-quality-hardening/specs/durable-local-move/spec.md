## ADDED Requirements

### Requirement: Check destination synchronisation before deleting a fallback source
A regular-file move using copy-and-delete fallback SHALL complete writes and check destination-file synchronisation before removing its source. Write or synchronisation failure SHALL produce a failed outcome and preserve the source. Cancellation after copying or synchronisation but before source deletion SHALL preserve the source. Same-filesystem atomic rename SHALL retain its existing preferred path.

#### Scenario: Destination synchronisation fails
- **GIVEN** the fallback has copied bytes and an injected synchronisation step returns an error
- **WHEN** the move handles that result
- **THEN** source deletion is not attempted, source bytes remain intact and the report identifies destination synchronisation failure
- **AND** any retained partial or complete destination is described accurately

#### Scenario: Synchronisation succeeds before deletion
- **WHEN** a fallback move completes successfully
- **THEN** observed operation ordering places successful destination synchronisation after writes and before source removal
- **AND** a source-removal failure reports the retained source and destination rather than successful completion

#### Scenario: Cancel before source removal
- **WHEN** cancellation is acknowledged after copying but before source removal
- **THEN** the source remains and the report records cancellation under the existing partial-result policy

### Requirement: State move persistence and overwrite guarantees precisely
Move documentation and reports SHALL distinguish atomic rename, copy fallback, checked file synchronisation, retained partial destinations and source-removal failure. Existing direct-overwrite semantics SHALL remain explicit: a failed or cancelled overwrite can retain a partial destination while preserving the fallback source. The implementation SHALL NOT claim multi-file rollback, atomic staged overwrite, directory-entry persistence or complete power-failure recovery from a destination-file synchronisation check alone.

#### Scenario: Fail while overwriting a destination
- **WHEN** a direct-write fallback overwrite fails or is cancelled
- **THEN** the source remains and reports/documentation identify that the destination can be partial
- **AND** previously completed items remain reported under their existing outcomes

#### Scenario: Validate rename and cross-device paths
- **WHEN** regression fixtures exercise same-filesystem rename and a move across real filesystems
- **THEN** the rename path retains its metadata/identity behaviour and the fallback satisfies checked synchronisation ordering
- **AND** verification does not substitute a mocked cross-device error for the existing real-filesystem coverage
