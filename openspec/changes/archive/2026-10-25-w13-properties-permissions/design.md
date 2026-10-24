# W13 design

## Inspection and identity

Capture the focused entry, its origin tab and concrete location when Properties
opens. Use lstat metadata for a link itself, display its target text separately,
and never substitute host-archive metadata for ZIP member ownership or modes.
Properties follows the captured entry rather than a later pane selection.
Refresh reloads observed values without applying any edits. Unknown fields show
Unavailable, not zero values or guessed ownership. UID/GID remain visible even
when account-name lookup fails. Render all paths/targets as plain text.

Use an identity snapshot including kind, device/inode where available, current
permission mode and relevant timestamps. Revalidate immediately before mutations;
reject a replaced entry or changed assumptions before applying stale drafts.
This is the existing stable-filesystem contract, not a race-free guarantee.
Permission/time editing is disabled for symbolic links and ZIP members. Show a
hard-link count and explain that metadata changes affect all names of that inode.

## Folder size

Calculate logical file bytes, file/folder/link counts and unreadable/skipped
counts on a cancellable read-only worker. Count hard-linked regular entries once
per directory entry, not as unique allocated storage; label that rule. Do not
follow links, cross descendant mount roots or count a symlink target's bytes.
Bound depth 256 and one million visited entries. Never call a partial result a
complete folder size. Do not block or overwrite mutation progress. Closing the
properties view cancels its read worker; preserve application close acknowledgment.

## Editing

Edit only nine ordinary rwx bits and modified time for one local entry at a time.
Retain existing special bits and access time when applying the respective change;
if the platform cannot preserve required fields, fail before writing. Show UTC
and the supported filesystem timestamp precision. No elevated helper, password
prompt, recursive propagation or ownership change is introduced.

Apply opens a review identifying the captured entry and each old/new value.
A worker enters W07's single-mutation gate. Permission and time changes are
separate reported steps: an error after one successful step retains that result,
shows which fields changed and refreshes from disk without implying rollback.
Cancellation is acknowledged between steps, not during an atomic metadata call.
Refresh affected tabs without clearing unrelated marks.

## Evidence

Compare normalized original/native observations without hiding permission or
timestamp differences. Include actual local files, folders, links, hard links,
unknown owners, read-only cases and filesystem precision boundaries. Native
platform checks must verify fields read back from disk, not merely dialog text.
