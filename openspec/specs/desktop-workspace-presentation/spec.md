# desktop-workspace-presentation Specification

## Purpose
Present the compact two-pane workspace and feature dialogs with clear hierarchy while preserving interaction contracts.
## Requirements
### Requirement: Present a coherent compact two-pane workspace
The desktop SHALL retain two independently navigable pane/tab collections, an editable path per pane, resizable file columns, the splitter, status information and existing commands. Navigation, file content, metadata and operations SHALL have distinct visual hierarchy using shared surfaces, alignment and spacing. File rows SHALL retain the baseline compact density at the default scale; improved spacing SHALL primarily apply around navigation and command groups.

#### Scenario: Browse populated panes with different state
- **GIVEN** each pane has its own path, tabs, sorting, column widths and marks
- **WHEN** the workspace is rendered after visual changes
- **THEN** aligned pane headers and a clear active-pane cue organise the interface
- **AND** changing one pane's view preserves the other pane's independent state

### Requirement: Group commands without changing their workflow
The toolbar SHALL visually separate navigation from file operations and SHALL avoid redundant persistent outlines. The six existing bottom commands SHALL retain their order, availability and shortcut labels. Dialog primary, secondary and destructive actions SHALL have deliberate and consistent emphasis without creating a new default destructive action. Existing keyboard routing, Enter/double-click opening, text/file clipboard precedence and native macOS menus SHALL retain their behaviour.

#### Scenario: Invoke an operation from keyboard or button
- **WHEN** the user invokes an existing copy, move, rename, batch rename, extraction or refresh command using its button or shortcut
- **THEN** the same selection, validation, confirmation and dispatch rules apply
- **AND** visual emphasis does not enable an unavailable command or bypass confirmation

#### Scenario: Use viewer text copy while files are marked
- **GIVEN** a text viewer has focused selected text and the workspace contains marked files
- **WHEN** the user invokes Copy
- **THEN** the existing text-copy routing applies without dispatching file Copy

### Requirement: Remain usable at the existing minimum window size
The workspace and existing feature surfaces SHALL remain usable at 820×500 logical points and the normal 1120×720 layout at 100% and 200% display scale. Paths and long metadata SHALL support editing, scrolling, elision or full-value inspection without inaccessible controls or overlapping content. Essential actions, keyboard focus, splitter interaction and table navigation SHALL remain reachable. Status presentation SHALL clearly distinguish idle, loading, empty, error, paused and cancelling states when those states exist.

#### Scenario: Resize with long names and a visible operation state
- **WHEN** a populated workspace is reduced to 820×500 with long names, paths and a paused or cancelling operation
- **THEN** controls do not overlap and the operation state and essential actions remain readable and reachable
- **AND** file tables retain horizontal/vertical navigation and full editable paths remain available

#### Scenario: Present a pending or failed browse
- **GIVEN** a browsing worker has a pending request or has failed under W17's lifecycle rules
- **WHEN** its state is displayed
- **THEN** loading or failure is visibly distinct from a successfully loaded empty directory
- **AND** styling does not change W17's rules for retaining or committing listings

### Requirement: Record repeatable visual and interaction evidence
Visual completion SHALL include inspected native before/after captures using the same disposable fixtures, dimensions, theme, host and scale for each comparison. Evidence SHALL include both themes, normal/minimum layouts, representative feature dialogs, focus/marks, long Unicode content and scale checks. Linux and macOS evidence SHALL be identified separately, unavailable checks SHALL remain pending, and visual acceptance SHALL not be presented as proof of full workflow parity.

#### Scenario: Review the completed visual pass
- **WHEN** W16 implementation is submitted for completion
- **THEN** evidence records host/backend/font/scale and checked hierarchy, contrast, alignment, clipping and state treatment
- **AND** targeted interaction regressions cover keyboard focus, shortcuts, marks, table virtualization and native menu behaviour on the exercised hosts
- **AND** unavailable host checks and pre-existing W09–W15 gaps are explicitly retained

