# internal-file-viewer Specification

## Purpose
View captured local and archive content with bounded text/image handling and independent cancellation.
## Requirements
### Requirement: Preview the focused file without changing normal opening
The application SHALL provide an explicit read-only View action for the focused regular local file, resolved file link or supported ZIP member, independent of unrelated marks. W09 Enter/double-click default-application opening SHALL remain unchanged. View requests SHALL capture source/origin identity, prepare content off the UI thread and reject stale generations or known changed source snapshots.

#### Scenario: View while other files are marked
- **GIVEN** a focused text file and marks on unrelated entries
- **WHEN** the user invokes View
- **THEN** only the focused file is prepared and no marks, source bytes or archive contents change

#### Scenario: Replace or close a preparing viewer
- **WHEN** a new file is requested or the viewer closes during preparation
- **THEN** the previous worker is cancelled and acknowledged and late content cannot replace a newer preview
- **AND** an unrelated mutation job or external application is not cancelled

### Requirement: Display bounded text and images with explicit failures
The viewer SHALL support strict UTF-8 and BOM-marked UTF-16 LE/BE text within 16 MiB, 100,000 lines and 64 KiB per decoded line, with virtual scrolling, find and selected-text copy. It SHALL support PNG/JPEG within 32 MiB encoded, 64 MiB decoded RGBA, 16,384 pixels per dimension and the GPU limit, with fit/100%, zoom/pan and visible transparency. Unsupported, malformed or over-limit content SHALL report the reason without silent data replacement, automatic external launch or unbounded allocation.

#### Scenario: Copy text inside the viewer
- **WHEN** the user selects decoded Unicode text and invokes Copy
- **THEN** the selected text is copied accurately and file clipboard Copy is not invoked

#### Scenario: Reject a dangerous or unsupported image
- **WHEN** headers declare excessive dimensions, decoding fails or the format is unsupported
- **THEN** the viewer reports an error while the application and unrelated jobs remain usable

### Requirement: Manage archive preview and external-copy lifetimes
ZIP preview preparation SHALL reuse guarded extraction and reject failed CRC, unsafe paths or unsupported entries before display. Viewer-only copies SHALL be cleaned up only after their last viewer/preparation lease ends. Explicit Open Externally SHALL hand off a retained copy through W09, preserving availability after viewer/application close and never writing back to the archive.

#### Scenario: Open an archive preview externally
- **WHEN** the user explicitly opens a previewed ZIP member in its default application and closes the viewer
- **THEN** the external copy remains readable and the original ZIP is unchanged
- **AND** cleanup does not delete user paths or unrelated W09 copies

