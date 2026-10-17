# W14 frozen native contract

Explicit F3 / File View acts on focus, not marks. Enter/double-click retain W09.
Capture local/link-target or ZIP source identities and origin ID. One read worker;
generation replacement cancels/joins old preparation, close waits for ownership
release. File reads/decode/extraction are off the UI thread. Reject changed source
before/after preparation. Cooperative cancellation around bounded codec calls;
panics become visible errors. Do not auto-launch on preview errors.

Strict UTF8 (optional BOM), BOM UTF16LE/BE with strict surrogate decoding. Preserve
all decoded characters/newlines; reject controls except TAB/CR/LF as binary.
Input16MiB, 100000 rendered lines (terminal newline does not create an extra
empty line), 64KiB decoded UTF8 bytes per line including newline. Empty text has
one empty line. Find is case-sensitive literal, next/previous wrapping, byte
ranges on UTF8 boundaries. Virtual rows; drag/Shift-click or Select All, Copy
exact selected decoded text. No editing, replacement or regex.

Only PNG/JPEG images; input32MiB, RGBA64MiB, dimensions1..16384 and GPU limit.
Check decoder header/sample allocation before pixels; decode to RGBA8, honor
supported EXIF orientations1..8 in both formats. Metadata/ICC are not applied as
color management. JPEG must include final EOI and use strict entropy decoding;
PNG CRC/truncation errors reject. Decoder sample buffers also capped64MiB,
possibly rejecting larger 16-bit samples before RGBA conversion. One texture,
fit/100%, zoom0.05..16, pan, checkerboard alpha. PNG exact pixels; JPEG comparisons
record per-channel codec differences separately (at most2 in scoped fixtures).
Unsupported GIF/PDF magic returns a reason; no automatic external launch.

ZIP uses W06 path/CRC/link guards and owned viewer-only private temporary leases;
max single-member expansion32MiB checked before extraction. Drop only owned
leases. Explicit external open uses original captured W09 target, making its own
retained copy so a viewer lease is never handed out then removed.

Generator and 40+12 predefined/40 reserved text recipes plus 40 small encoded
image fixtures were frozen before Rust implementation. Source/archive hashes,
original decoder probes and native interaction are separate evidence gates.

Implementation clarification: CR, LF and CRLF are each rendered as line endings
while their original bytes remain in decoded selection/copy. Fit may zoom below
the manual 0.05 floor so the entire bounded image fits a narrow viewport.
