# desktop-visual-system Specification

## Purpose
Define consistent light/dark appearance, typography, control states and scale-aware icons for desktop surfaces.
## Requirements
### Requirement: Apply one semantic visual system across desktop surfaces
The desktop SHALL use shared semantic colour and geometry roles for window chrome, content, raised surfaces, fields, text, borders, selection, focus and interaction feedback. The system SHALL cover the main workspace and existing confirmations, bookmarks, search, properties, viewer and SFTP surfaces in Light, Dark and System appearance. Persisted appearance choices SHALL retain their existing meaning.

#### Scenario: Switch appearance with multiple surfaces open
- **GIVEN** the file workspace and a feature dialog are visible
- **WHEN** the user changes between Light and Dark
- **THEN** both surfaces use the corresponding coordinated text, field, control and selection roles without stale theme colours
- **AND** the appearance change preserves tab, location, focus, marks and active jobs

#### Scenario: Follow a live system appearance change
- **GIVEN** appearance is System
- **WHEN** the operating system reports a theme change
- **THEN** the desktop applies the matching styled palette without restarting
- **AND** explicit Light or Dark preference continues to override system changes

### Requirement: Preserve readable and distinguishable interaction states
Normal text SHALL achieve at least 4.5:1 contrast against its displayed background; large text and essential non-text indicators SHALL achieve at least 3:1. Selected, focused, hovered, pressed, disabled, error, paused and cancelling states SHALL remain distinguishable in both themes. Keyboard focus and marked-file state SHALL have visible cues beyond colour alone. Disabled text SHALL remain recognisable but is exempt from the normal-text contrast target.

#### Scenario: Focus a quiet toolbar action
- **WHEN** keyboard navigation focuses an enabled toolbar action with an unboxed idle appearance
- **THEN** a visible focus indicator identifies the action independently of pointer hover
- **AND** its accessible name, activation behaviour and enabled checks match the corresponding command

#### Scenario: Distinguish marks from focus and errors
- **GIVEN** the listing contains marked files and a separately focused row
- **WHEN** either appearance is rendered and an operation reports an error
- **THEN** marks, current focus, active pane and error information each remain identifiable
- **AND** selected-row text and metadata satisfy contrast against the selection fill

### Requirement: Use consistent typography and icon geometry
The desktop SHALL define shared caption, body/button and section-title text styles, with stronger emphasis for active tabs and section labels and secondary treatment for metadata. Code-drawn action icons SHALL share a design grid, stroke treatment and optical sizing across toolbar, pane navigation and dialogs. File, folder, link and archive distinctions SHALL remain recognisable. Any added font or icon asset SHALL have recorded provenance and redistribution notices, and fonts SHALL retain a tested fallback.

#### Scenario: Render long Unicode content at different scales
- **WHEN** long Unicode names and paths are displayed at 100% and 200% scale
- **THEN** text and icons remain aligned and readable without clipping essential control labels
- **AND** adopting a new font does not silently remove glyph coverage that was available in the baseline

#### Scenario: Inspect disabled and active icons
- **WHEN** the same action is displayed enabled, hovered, pressed and disabled
- **THEN** its geometry remains consistent and only the intended state treatment changes
- **AND** the disabled action cannot bypass the underlying command's availability checks

