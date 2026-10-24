## ADDED Requirements

### Requirement: Maintain independent tabs in each pane
The application SHALL provide between one and 32 tabs per pane with stable identities and exactly one active tab. Each tab SHALL retain independent location, history, focus, marks and sort options. New Tab SHALL open Home with default options; Duplicate SHALL copy location, view, focus and independent history while clearing marks and quick-find. Closing the final tab SHALL be disabled. Switching tabs SHALL restore listing focus and clear transient quick-find without reloading or merging marks.

#### Scenario: Switch and duplicate a browsing context
- **GIVEN** two tabs in one pane with different locations, histories and marks
- **WHEN** the user switches tabs and duplicates the active tab
- **THEN** the original tabs retain their state and the duplicate has its own history with no marks
- **AND** the other pane remains unchanged

#### Scenario: Handle limits and failed initialization
- **WHEN** the user closes the only tab, exceeds 32 tabs, or a new/duplicate location cannot be opened
- **THEN** existing tabs and their state remain intact and the unavailable action or failure is visible

### Requirement: Keep jobs bound to their reviewed tab and paths
Reviewed operations SHALL capture originating tab identity and source/destination locations. Switching or closing tabs SHALL NOT retarget or cancel an already started operation. Completion SHALL only unmark the surviving originating tab at its captured location, refresh affected contexts under existing selection rules and retain global results. Closing the application SHALL retain W07 cancellation acknowledgment behavior.

#### Scenario: Close the source tab while copying
- **GIVEN** a running copy from tab A and another tab B at the same location with different marks
- **WHEN** the user closes A before completion
- **THEN** the copy finishes against captured paths and its result remains visible
- **AND** B is not treated as A and its marks are not cleared
