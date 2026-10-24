## ADDED Requirements

### Requirement: Find visible entries by filename prefix
Committed printable text received by a focused listing SHALL start a transient
quick-find field and focus a matching visible entry using case-insensitive prefix
matching under a documented policy frozen from recovery fixtures. The application
SHALL provide next/previous match controls with wrapping and no-match feedback.
Quick-find SHALL NOT alter marks, sorting, filtering or either pane's location,
and SHALL NOT match hidden entries excluded from the listing or the parent row.

#### Scenario: Focus a matching entry without changing marks
- **GIVEN** a listing with existing marks and a mix of matching and nonmatching names
- **WHEN** the user types a prefix and advances through its matches
- **THEN** focus and scrolling follow matching entries in displayed order, wrapping at the ends
- **AND** marks, sort order and hidden filtering remain unchanged

#### Scenario: No match
- **WHEN** the query matches no visible entry
- **THEN** the application shows no-match feedback and retains the prior valid focus and marks

### Requirement: Preserve editing and activation semantics
Quick-find SHALL accept committed Unicode text and allow query editing, including
spaces. Uncommitted IME composition SHALL NOT move listing focus. Existing text
fields, dialogs and command shortcuts SHALL take precedence. Space SHALL retain
marking behavior when quick-find is closed. Escape SHALL dismiss quick-find;
Enter SHALL dismiss it while retaining its match without opening the item.
Switching panes or successfully navigating SHALL clear the query. Refresh SHALL
reconcile the query and focus against the new visible listing without restoring
removed entries or invalid focus rows.

#### Scenario: Return to file activation
- **WHEN** the user presses Enter in quick-find
- **THEN** the field closes without opening the match
- **WHEN** the user subsequently presses Enter with listing focus
- **THEN** normal W09 file/folder activation occurs

#### Scenario: Unicode input and unrelated text editing
- **WHEN** the user commits Unicode text through the find field
- **THEN** it is matched under the documented Unicode/normalization policy
- **WHEN** the user types in a location field or review dialog instead
- **THEN** that text does not start or update listing quick-find
