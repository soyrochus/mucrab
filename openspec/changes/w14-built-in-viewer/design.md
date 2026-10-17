# W14 design

## Activation and lifecycle

View acts on the focused regular file even when other entries are marked.
Capture origin tab, location and source identity before starting work. Directory
and parent entries are not file previews. A local file link may resolve using
W09's cycle/error handling; the viewer visibly identifies the resolved file.
Keep W09 Enter/double-click external opening unchanged; recover a separate View
shortcut with proper field/dialog precedence.

Use one viewer with generation-tagged preparation requests. Switching files
cancels/acknowledges old preparation before installing new content; late results
cannot replace a newer preview. Run file reads, archive extraction and decoding
off the UI thread with cancellation and bounded progress. Closing cancels the
viewer worker without cancelling an unrelated copy job or external application.
Revalidate source identity around reading and reject a known changed snapshot.
No automatic reload or disk write is introduced.

## Formats and resource bounds

Initial text supports strict UTF-8 (optional BOM) and BOM-marked UTF-16 LE/BE.
Invalid/unsupported encodings and binary content show a reason rather than
silently replacing bytes. Preserve decoded text/newlines for selection/copy;
render lines virtually with horizontal scrolling. Bound input to 16 MiB, 100,000
lines and 64 KiB per decoded line. Exceeding a limit offers external opening rather
than pretending to show a complete file. Text find uses a literal, case-sensitive term with next/previous wrapping only
within the loaded preview, not the filesystem. Regex and editing/saving are
disabled.

Images support PNG and JPEG only, with encoded input <=32 MiB, decoded RGBA
allocation <=64 MiB and each dimension <=16,384 and the reported GPU limit.
Check headers/decoder bounds before allocating/uploading. Reject truncated,
malformed, oversized or unsupported data with a visible error; a worker panic
must not terminate the application. Fit/100%, bounded zoom and pan reuse one
texture rather than allocating a larger raster per zoom level. Transparent pixels
use a visible checkerboard on both light and dark appearances. Honor supported
orientation metadata without mutating the file and verify its declared policy.

## Archive copies and external opening

Reuse the W06 extractor's CRC/path/link/resource guards for one regular ZIP member.
Use an owned viewer-only temporary lease and remove it when no preparation/view
uses it. Never remove W09 retained copies or paths supplied by users. For Open
Externally, transfer to a retained W09-style copy before dispatch so the external
application can read it after viewer/application close. No archive writeback.

A disabled/failed preview leaves marks, paths and source bytes unchanged. Copy
inside text preview copies selected text; it must not invoke file clipboard Copy.
Return focus to the surviving originating tab on close, otherwise to the current
listing. Unknown formats keep W09 external opening available as an explicit user
choice; do not automatically launch applications on decoder errors.

## Evidence

Use exact text and pixel/dimension fixtures, including BOMs, CRLF, Unicode,
corruption, rotation metadata and decompression/long-line limits. Retain native
light/dark, keyboard, zoom and narrow-window captures separately from binary
comparison reports. Host-specific font/GPU limitations remain explicit.
