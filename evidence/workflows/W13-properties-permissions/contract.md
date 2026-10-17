# W13 frozen native contract

Capture focused child and origin tab; lstat links, never edit links/ZIP members.
Display numeric Unix owner/group IDs, optional account names, mode (including
special bits), link count/text and available SystemTime values. Missing fields
remain unavailable. ZIP member fields come only from the W06 catalog. Size counts
include hidden regular entries (hard links per name), folders and links; no link
target bytes or traversal. Root is excluded from descendant counts. Streaming
walk: depth256, visited1million, diagnostics256 plus total; mount exclusions,
errors, cancellation and overflow make totals partial. Independent read worker.

Metadata review captures current device/inode/kind/size/mode/mtime assumptions.
Only rwx0..0777 and modified time are editable; preserve special mode bits and
access time at apply. No owner/access-time/special-bit edits or recursion. UTC
RFC3339 nanosecond input; stat API nanosecond resolution, actual filesystem
precision is not guessed and read-back values are shown. Validate and acquire
required handles before writes; permissions precede modified time. Revalidate
between steps, acknowledge cancel between calls, report every changed/failed/
unattempted field without rollback. All updates use the W07 mutation gate, bound
origin ID and preserve marks. Stable trees, no race-free filesystem guarantee.

Generator tools/w13_w14_fixtures.py and 40 predefined/40 reserved metadata cases
were frozen before Rust implementation. Link mutation rejection and nanosecond
native vs millisecond Java time precision are declared differences. Original
runtime scope and native UI are separate gates, not inferred parity.

Original-runtime observation: the isolated Java local provider reports only an
owner-bit changeable mask; native Unix supports all nine ordinary bits. These
provider-capability differences are recorded in both comparison receipts.
