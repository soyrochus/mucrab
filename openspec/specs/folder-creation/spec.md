# folder-creation Specification

## Purpose
Create one validated local folder without replacement and refresh the originating workspace coherently.
## Requirements
### Requirement: Create one local folder
The application SHALL provide New Folder in the active local pane, accepting one
valid leaf name and creating it without replacing any existing entry. It SHALL
reject empty names, parent/self names, separators and invalid platform names.
Creation SHALL respect the single-mutation-job gate and be unavailable in ZIP panes.

#### Scenario: Create and focus a folder
- **GIVEN** an idle application with an active writable local pane
- **WHEN** the user submits an unused valid folder name
- **THEN** exactly one directory is created in that pane's reviewed location
- **AND** affected panes refresh, the new visible entry receives focus and unrelated marks survive
- **AND** a hidden new folder is not forced visible when hidden filtering excludes it

#### Scenario: Reject a collision or changed destination
- **WHEN** the name already exists, creation is denied, or the reviewed destination has changed
- **THEN** the application reports the failure without replacing or modifying an existing entry

#### Scenario: Cancel before creation
- **WHEN** the user cancels the New Folder dialog
- **THEN** no filesystem mutation occurs

