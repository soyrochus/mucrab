## ADDED Requirements

### Requirement: Maintain independent session histories
Each pane SHALL maintain its own session-only history of at most 100 structured
local or ZIP locations, initially seeded by its first successful open. Successful
explicit navigation, including parent/Home/location entry, SHALL append a location
unless it duplicates the current entry. Refresh, sorting, filtering, file opening
and history traversal SHALL NOT append entries. Exceeding the bound SHALL evict
the oldest entry while retaining a valid cursor.

#### Scenario: Record local and archive navigation
- **WHEN** a pane enters a folder, a ZIP root and then a ZIP subfolder
- **THEN** its history preserves those distinct structured locations in order
- **AND** the other pane's location and history remain unchanged

#### Scenario: Avoid incidental entries
- **WHEN** the user refreshes, sorts, changes hidden filtering or opens an ordinary file
- **THEN** no location history entry is added

### Requirement: Navigate history with success-only cursor changes
Back/Forward SHALL be available through menu/toolbar actions and focus-aware
platform shortcuts, disabled at their respective bounds. Traversal SHALL reuse
normal guarded location opening and existing folder-change selection rules, not
restore historical marks or options. The history cursor SHALL move only after
successful navigation. A new successful explicit navigation after Back SHALL
discard the forward branch.

#### Scenario: Back and Forward across archive boundaries
- **WHEN** the user traverses between a local folder and previously visited ZIP locations
- **THEN** each location opens with its correct archive identity and prefix
- **AND** no duplicate history entry is created by traversal

#### Scenario: Missing or unreadable historical location
- **WHEN** a historical folder or ZIP can no longer be opened
- **THEN** the application reports the error and retains the current location, listing and history cursor

#### Scenario: Replace a forward branch
- **GIVEN** history A → B → C with the cursor returned to B
- **WHEN** explicit navigation to D succeeds
- **THEN** history becomes A → B → D and Forward is disabled
- **WHEN** explicit navigation instead fails
- **THEN** the original A → B → C history and cursor remain unchanged
