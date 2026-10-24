## ADDED Requirements

### Requirement: Activate the focused or clicked item
Enter and File → Open SHALL activate the focused item independently of marks.
Double-click SHALL activate the clicked item. Enter auto-repeat SHALL NOT dispatch
additional requests. Parent/folder/ZIP navigation SHALL remain available. Local
links SHALL resolve to their target; missing or cyclic targets SHALL report errors.

#### Scenario: Activate with keyboard or pointer
- **WHEN** Enter or File → Open is invoked
- **THEN** the focused item is activated independently of marks
- **WHEN** an item is double-clicked
- **THEN** the clicked item is activated
- **AND** holding Enter does not dispatch additional requests

#### Scenario: Navigate or resolve a link
- **WHEN** the activated item is a parent, folder or ZIP
- **THEN** existing navigation remains available
- **WHEN** a local link is activated
- **THEN** its target is resolved, with an error for missing or cyclic targets

### Requirement: Dispatch through the registered platform application
Linux SHALL dispatch an absolute filename as one argument to `xdg-open`; macOS
SHALL use `/usr/bin/open`. No shell command SHALL be constructed from filenames.
Launcher creation and nonzero exit errors SHALL be visible. Successful dispatch
SHALL mean only that the request was sent, not proof that a document appeared.

#### Scenario: Dispatch an ordinary file
- **WHEN** an ordinary local file is opened
- **THEN** the platform launcher receives the absolute filename as one argument
- **AND** no shell command is constructed from the filename

#### Scenario: Report a launcher failure
- **WHEN** launcher creation fails or the launcher exits unsuccessfully
- **THEN** the error is visible
- **AND** a successful dispatch is not presented as proof that a document appeared

### Requirement: Prepare retained archive copies safely
Archive member extraction SHALL reuse guarded W06 extraction, including CRC,
traversal, link and resource-limit checks. No launch SHALL occur after an extraction
error or cancellation. Each extracted member SHALL receive its own private
directory and original leaf name, with no executable permissions. Successful
copies SHALL survive muCrab exit until OS temporary-directory policy or explicit
user cleanup removes them. No ZIP writeback SHALL occur.

#### Scenario: Open an archive member
- **WHEN** guarded extraction succeeds
- **THEN** the member is dispatched from its own private directory with its original leaf name and no executable permissions
- **AND** the copy remains available after muCrab exits until later cleanup
- **AND** edits do not write back to the ZIP

#### Scenario: Extraction fails or is cancelled
- **WHEN** extraction encounters an error or cancellation
- **THEN** no application is launched for that member

### Requirement: Retain a bounded independent opening lifecycle
Preparing and launching SHALL NOT block the UI or replace W07 mutation
progress/results. Preparation SHALL support cancellation; already dispatched
applications SHALL remain independent. Window close SHALL wait for cancelled
preparation to acknowledge completion without waiting for external applications.
At most one preparation and 64 outstanding launcher processes SHALL be accepted.

#### Scenario: Prepare while a mutation runs
- **WHEN** a file-opening preparation runs alongside a mutation job
- **THEN** the UI remains responsive and mutation progress/results remain intact
- **AND** preparation can be cancelled independently

#### Scenario: Close with preparation or launchers outstanding
- **WHEN** the window is closed during preparation
- **THEN** close waits for cancellation acknowledgement
- **AND** it does not wait for already dispatched external applications
- **AND** the one-preparation and 64-launcher bounds remain enforced
