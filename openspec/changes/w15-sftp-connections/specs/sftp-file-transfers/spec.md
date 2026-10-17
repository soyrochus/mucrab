## ADDED Requirements

### Requirement: Review bounded local and SFTP copies
The application SHALL support reviewed local-to-SFTP and SFTP-to-local copies of regular files and directory trees with Skip/Overwrite policies, captured endpoints and the existing effective-selection rules. Explicitly selected folders SHALL include hidden descendants. Remote links/special files, remote-to-remote copies and unsupported mutations SHALL be reported or disabled, never silently followed or delegated to local filesystem APIs. Available identity/metadata assumptions SHALL be revalidated and weaker remote guarantees documented.

#### Scenario: Copy a selected directory tree
- **WHEN** a user confirms a local-to-SFTP or SFTP-to-local directory copy
- **THEN** the captured tree is transferred under the selected policy, including hidden descendants
- **AND** unsupported remote links/special files are reported without following their targets

#### Scenario: Navigate away after review
- **WHEN** the user changes or closes the originating tab after starting a reviewed transfer
- **THEN** captured endpoints remain unchanged and unrelated tabs are not unmarked

### Requirement: Publish safely and preserve partial outcomes
Remote-involved writes SHALL use uniquely owned staging paths and publish only after successful stream close and length checks under verified server commit semantics. Skip SHALL preserve an existing destination. Overwrite SHALL fail if safe replace/rename capability is unavailable rather than delete the existing destination first. Transfers SHALL use W07 single-mutation progress/pause/cancel control, retain and identify partial staging paths, preserve sources and avoid automatic ambiguous-commit retries.

#### Scenario: Lose the connection before publication
- **WHEN** a connection fails or cancellation is acknowledged before commit
- **THEN** sources and existing destination files remain intact and owned partial paths/outcomes are visible

#### Scenario: Cannot safely replace a destination
- **WHEN** Overwrite is selected but the server lacks the required verified commit capability
- **THEN** the transfer fails visibly without deleting the prior destination as a fallback

### Requirement: Open remote regular files through retained local copies
Remote folder activation SHALL navigate; remote regular-file activation SHALL prepare a private non-executable local copy bounded to 512 MiB and use W09 default-application dispatch. Failed, cancelled or known-changed downloads SHALL NOT launch. Successful copies SHALL retain the validated filename and survive application exit under the temporary-copy policy, with no automatic server writeback. Remote links and remote archives SHALL NOT be treated as supported downloads/browsable archives implicitly.

#### Scenario: Open a remote document then exit
- **WHEN** a regular remote document is successfully downloaded and dispatched before muCrab exits
- **THEN** the external application can still read the local copy and edits do not change the remote source

#### Scenario: Reject incomplete or changed downloads
- **WHEN** a download exceeds the limit, fails, is cancelled or fails the available before/after checks
- **THEN** no external application is launched and the failure is reported
