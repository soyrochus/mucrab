# recursive-file-search Specification

## Purpose
Search local directory trees with bounded background work, explicit results and cooperative cancellation.
## Requirements
### Requirement: Search a captured local subtree by explicit criteria
The application SHALL search a captured local directory using immutable filename wildcard, extension, entry-type, size and UTC modified-time criteria with explicit case and hidden-entry options. Quick-find SHALL retain its current-listing prefix behavior. Search SHALL NOT traverse directory links, archive contents, remote providers or descendant mount roots. Criteria, supported Unicode matching and unknown metadata handling SHALL be documented and validated before traversal.

#### Scenario: Find matching descendants
- **GIVEN** a local tree with matching names in several subfolders and excluded hidden entries
- **WHEN** a search is started with filename, size and UTC date bounds
- **THEN** results contain only entries satisfying all applicable criteria within the captured scope
- **AND** searching does not change marks, sorting or the current pane location

#### Scenario: Retain link and metadata boundaries
- **WHEN** a directory link points outside the root or an entry lacks metadata needed by an enabled filter
- **THEN** the link is not traversed and the unknown-metadata entry does not silently match
- **AND** scope exclusions and diagnostic counts are available

### Requirement: Deliver bounded cancellable search results
Search SHALL execute off the UI thread with bounded batches and a generation ID, at most 10,000 hits, one million visited entries and depth 256. Cancellation, traversal failure or a resource limit SHALL preserve earlier hits and visibly distinguish partial results from completion. Replacing a query SHALL wait for prior acknowledgment and reject stale batches. Search SHALL NOT monopolize or overwrite W07 mutation progress.

#### Scenario: Replace a running query
- **WHEN** a user replaces a query while batches from the previous generation are pending
- **THEN** the old worker is cancelled and acknowledged before the next starts
- **AND** old hits cannot appear in the new result set

#### Scenario: Cancel or reach a limit while copying
- **GIVEN** a copy and a search are both running
- **WHEN** search is cancelled or reaches its result limit
- **THEN** existing hits remain visibly partial and copy retains its controls and results

### Requirement: Navigate a result using current filesystem state
Results SHALL expose their containing location and observed metadata in deterministic relative-path order. Open Containing Folder SHALL target the explicitly chosen current tab, navigate successfully before focusing the named result and preserve unrelated tabs. A vanished result or failed navigation SHALL report an error without selecting another entry by row index.

#### Scenario: Open a stale result
- **WHEN** a result is removed or its parent becomes unreadable before activation
- **THEN** the failure is visible and no unrelated entry is activated or modified

