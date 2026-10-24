## ADDED Requirements

### Requirement: Distinguish Trash from permanent deletion
The ordinary Delete action SHALL offer moving the effective local selection to
the platform Trash. Permanent deletion SHALL be a separate explicitly named
action with a review stating that it bypasses Trash. Both reviews SHALL identify
the captured source location and entries before mutation. Neither action SHALL
operate on the parent row, filesystem roots, an open pane location or its ancestors,
or entries inside ZIP panes.

#### Scenario: Trash a marked selection
- **GIVEN** marked local entries and a different focused child
- **WHEN** the user confirms Move to Trash
- **THEN** only the captured marked entries are sent to the native Trash
- **AND** platform-provided restoration information is retained

#### Scenario: Trash is unavailable or fails
- **WHEN** native Trash fails or is unsupported for a selected entry
- **THEN** the failure is reported and the application does not fall back to permanent deletion

#### Scenario: Review permanent deletion
- **WHEN** the user invokes permanent deletion
- **THEN** the review clearly states the action bypasses Trash
- **AND** cancelling the review leaves every source unchanged

### Requirement: Preserve deletion boundaries and partial results
Deletion SHALL operate on links themselves without following their targets,
including broken links. Recursive permanent deletion SHALL include hidden
children and use cooperative cancellation through the W07 mutation worker.
Reviewed identity assumptions SHALL be revalidated before mutation. Successful
work SHALL remain committed, with failed, partial and unprocessed roots reported
and unfinished marks retained when the originating pane location still matches.

#### Scenario: Delete a folder containing an external link
- **WHEN** a selected folder containing hidden files and a link to an outside tree is permanently deleted
- **THEN** its contained entries and link are removed without traversing or changing the outside target

#### Scenario: Cancellation or failure during a multi-entry job
- **WHEN** cancellation or an error occurs after earlier work completed
- **THEN** the application exposes per-root outcomes and any partially deleted tree
- **AND** it retains unfinished marks without altering unrelated marks at a different pane location
- **AND** closing the window waits for the worker to acknowledge cancellation
