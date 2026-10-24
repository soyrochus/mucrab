# sftp-connections Specification

## Purpose
Manage validated SFTP profiles, host trust, authentication and bounded remote browsing without exposing secrets.
## Requirements
### Requirement: Represent SFTP profiles and locations without secrets
The application SHALL support versioned SFTP profiles and structured profile-backed remote locations in panes/history/bookmarks. Profiles SHALL contain only non-secret connection settings and optional key-file paths. Password-bearing URLs SHALL be rejected; secrets SHALL NOT appear in profile/bookmark files, clipboard exports, logs or process arguments. Missing profiles and failed connects SHALL preserve the last successful pane location/history.

#### Scenario: Reopen a saved remote location
- **GIVEN** a bookmark referencing an existing SFTP profile and remote path
- **WHEN** the user opens it
- **THEN** the application connects through that profile and commits navigation only after successful listing

#### Scenario: Fail safely with a missing profile or secret-bearing URL
- **WHEN** a bookmark profile is missing or input embeds a password
- **THEN** the error is visible, no secret is persisted/logged and the pane remains at its last successful location

### Requirement: Verify server identity before authentication
SFTP connections SHALL verify host keys before sending authentication credentials. First-use trust SHALL display endpoint, algorithm and SHA-256 fingerprint for an explicit decision; cancellation SHALL store no trust. A changed key SHALL fail without automatic replacement and require a separate deliberate trust-management action. Known matching keys SHALL continue through the verified path.

#### Scenario: Encounter an unknown server
- **WHEN** an unrecognized host key is received
- **THEN** authentication waits for an explicit trust decision and declining leaves profiles/trust unchanged

#### Scenario: Encounter a changed key
- **WHEN** a known endpoint presents a different host key
- **THEN** the connection is refused before authentication without overwriting trusted identity

### Requirement: Keep credential persistence optional and native
Initial authentication SHALL support passwords and private-key files with session passphrases. Persistence SHALL occur only after explicit Remember Secret selection through an available macOS Keychain or Linux Secret Service adapter. If storage is unavailable, remembering SHALL be disabled or visibly fail while session-only authentication remains possible. There SHALL be no plaintext fallback or modification of user private-key files.

#### Scenario: Use an unavailable credential store
- **WHEN** the native credential service is unavailable
- **THEN** the user may authenticate for the session without saving secrets to configuration or logs

#### Scenario: Remove a saved profile
- **WHEN** a profile with an owned stored credential is removed
- **THEN** its credential reference is removed or an error is reported, and user key files remain unchanged

### Requirement: Keep sessions and listings independent of active UI state
Connection/listing I/O SHALL run off the UI thread with finite timeouts, cancellation and generation-tagged results. Tabs and jobs SHALL hold explicit session/location identities. Closing a tab SHALL NOT retarget or cancel its already reviewed transfer; explicit Disconnect SHALL cancel associated work, await acknowledgment and make remaining remote views disconnected. Unsupported remote actions SHALL be disabled explicitly.

#### Scenario: Switch tabs during a slow listing
- **WHEN** an earlier remote listing arrives after the tab navigates elsewhere
- **THEN** the stale result is discarded and cannot replace the newer location

#### Scenario: Disconnect with a transfer active
- **WHEN** Disconnect is requested during transfer
- **THEN** the job is cancelled and acknowledged, partial outcomes remain visible and stale remote listings cannot accept new mutations

