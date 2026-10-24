## ADDED Requirements

### Requirement: Save and navigate structured bookmarks
The application SHALL support adding the current successful location and opening, renaming, removing and ordering shared local/ZIP bookmarks. Bookmarks SHALL persist as validated, versioned structured locations with stable IDs and plain-text labels. Opening SHALL use the active tab, recording history only on success. Secrets SHALL NOT be stored in bookmarks.

#### Scenario: Reopen an archived location after restart
- **GIVEN** a saved bookmark for a ZIP subfolder
- **WHEN** the application restarts and the bookmark is opened
- **THEN** the active tab navigates to that exact archive and prefix
- **AND** other tabs retain their own state

#### Scenario: Handle a missing target
- **WHEN** a bookmarked folder or archive no longer exists
- **THEN** opening reports an error and preserves current location, listing and history cursor
- **AND** the bookmark remains available for rename or removal

### Requirement: Preserve valid bookmark data on failure
Bookmark saves SHALL use atomic replacement and preserve the previous valid file on failure. Invalid versions, malformed entries, duplicate IDs or size/count violations SHALL be reported without crashing startup or silently overwriting the input. Save failures SHALL state that edits were not persisted.

#### Scenario: Fail a save or load
- **WHEN** a bookmark write is denied or stored data is malformed
- **THEN** the error is visible, existing valid persisted data is preserved and the application remains usable
