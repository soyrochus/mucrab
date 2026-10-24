# file-clipboard Specification

## Purpose
Exchange native local file selections and execute reviewed Paste requests through existing copy guards.
## Requirements
### Requirement: Exchange native file clipboard selections
Copy SHALL publish absolute paths for the active local pane's effective selection
in platform-native file clipboard formats consumable by other file managers.
Paste SHALL read native local file selections published by other applications.
The synthetic parent SHALL never be copied. Copy SHALL preserve source files,
entry/link identity and marks. File clipboard commands SHALL yield to normal text
editing when a text field or dialog owns keyboard focus.

#### Scenario: Copy files for another application
- **GIVEN** selected local entries whose names contain spaces, Unicode, percent signs or hashes
- **WHEN** the user copies them and a native file manager reads the clipboard
- **THEN** the native application obtains the exact local entry paths
- **AND** no file is moved and no marks change

#### Scenario: Copy while editing a location
- **WHEN** a text-editing field owns focus and the user invokes the standard Copy shortcut
- **THEN** text is copied rather than the pane's file selection

### Requirement: Review and execute a clipboard Paste as a copy
Paste SHALL target the active local directory and snapshot the clipboard for
review. It SHALL validate the complete payload, deduplicate identical entries in
stable order, and reject malformed or unsupported non-local URLs before mutation.
It SHALL support multiple source parents and reuse W03 copy collision/overlap/link
guards and W07 progress/cancellation under one mutation job. ZIP destinations and
archive-member clipboard export SHALL remain unsupported. Paste SHALL NOT clear
the clipboard, infer a move, or unmark unrelated pane selections.

#### Scenario: Paste from multiple source directories
- **GIVEN** a native file clipboard containing local entries from different parents
- **WHEN** the user reviews the destination and collision policy and confirms Paste
- **THEN** one copy job applies the captured request with ordered per-entry outcomes
- **AND** changing the clipboard after review does not change that job

#### Scenario: Unsupported payload or stale source
- **WHEN** the clipboard payload is malformed or contains unsupported remote URLs
- **THEN** the request is rejected before copying any entry
- **WHEN** a reviewed source is missing or its identity assumptions have changed
- **THEN** the application reports the validation failure before applying the stale request

#### Scenario: Collision or cancellation
- **WHEN** a destination collision or cancellation occurs during Paste
- **THEN** existing copy policy and partial-result rules apply without overwriting unrelated paths or silently moving sources

