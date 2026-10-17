# W06 contract — bounded local ZIP browsing and extraction

Frozen 2026-10-17 after original-only run
`W06-20261017T091304Z-5c35ab2b` (163 cases), before native ZIP implementation.
The 25 named and 18 generated sets (seed 766061) and four fixed collision
policies define 163 cases. Twenty-four reserved sets (seed 991086) stay withheld
until the Rust implementation. Original muCommander 1.6.2-1 JARs, not rewritten
Java, supply the ZIP provider, archive tree and UnpackJob methods.

## Inputs and listing

Ordinary single-disk local ZIPs using stored or deflated content and ASCII or
UTF-8 names, with valid DOS dates in UTC, are supported. Entries are regular
files or folders. An existing owned local destination folder is required. Archive
writing, nested archive browsing, encryption/password prompts, other compression
methods, legacy name encodings, ZIP64 equivalence, extended timestamp fields,
symlinks/special files and malformed ambiguous duplicate paths are excluded.

Archive children are enumerated in first-encounter order. Missing parent folders
are synthesized in the tree with size zero and the first descendant's date.
An explicit folder appearing later replaces its metadata without moving it or
discarding children. Paths are case-sensitive. Folders and files expose their
leaf names, size, date and last-dot extension through the existing listing model.
Native filtering, sorting, marking and both-pane navigation reuse W01–W02c.
Parent at an archive root returns to the containing local folder and focuses the
archive. Parent within it returns to its virtual parent folder. Refresh rebuilds
the listing; a failed open retains the last successful pane state.

Invalid/truncated ZIP structure yields `archive_unavailable`. A requested
nonexistent folder or a file used as a folder yields an explicit failed listing.
Successful empty archives/folders stay distinct from these errors.

The original ZIP provider's DOS conversion uses a Calendar without clearing its
current millisecond field. Raw observations retain this nondeterministic fraction.
Comparison truncates ZIP-derived dates to whole seconds, within DOS's two-second
precision. Native timestamps use the encoded date exactly. Conditional collision
cases within the same second as the source date are excluded; native compares
its deterministic timestamp strictly. Other locales/timezones and date formats
are not claimed.

## Extraction, collisions and partial results

Extract the whole archive or an effective selection of distinct direct children
of the current archive folder. Selected folders include all descendants, including
hidden children. Strip the current folder prefix from destination paths. An empty
selection extracts nothing; a root whole-archive request includes every entry.
The destination itself must already exist; implicit parents and empty folders
are created as needed.

Process entries in central-directory order, rather than sorted UI order. Existing
folders merge. Existing non-directory destinations use the fixed SKIP, OVERWRITE,
OVERWRITE_IF_OLDER or OVERWRITE_IF_SIZE_DIFFERS choice. Older compares strictly;
size-differs compares byte lengths. Type conflicts can fail after a preceding
skip (e.g. an existing file blocks later children of a skipped directory).
The first actual mkdir/open/transfer failure stops this archive. Earlier writes
remain, later entries are not processed, and no rollback is claimed. Skips alone
do not make the archive result false. This differs from W03's recursive copy
continuation and is measured separately.

Files preserve complete bytes and their encoded modified date. On the verified
owned-file Linux/umask-022 boundary, owner rwx comes from the entry; group/other
bits remain existing or umask-created values. Directory permissions and dates
are not imported. Newly created directory dates depend on execution time and
child writes, so those dates are omitted from comparison. Modes, complete tree
shape and all file bytes, sizes, hashes and whole-second dates remain checked.
Metadata application is best effort. The source ZIP bytes must remain unchanged.

Native code additionally rejects unsafe/absolute/dot/backslash/drive paths,
duplicate or conflicting file/folder paths, archive symlinks, unsupported methods,
and extraction through destination symlinks or aliases of the source archive.
It bounds archive entries, declared/actual expanded bytes and nesting depth,
checks entry size and CRC, and rejects stale reviewed archive identity/metadata
before writing. These guards are tested independently, not counted as original
equivalence. They do not claim race-free filesystem handling, full adversarial
format coverage, or equivalence for original unvalidated CRC data.

## Oracle boundary and native UI

The adapter registers the actual original ZIP format provider, uses original
archive listings and selected ArchiveEntry objects, constructs UnpackJob with
the chosen policy/base depth, enables original auto-skip errors, disables optional
integrity mode, sets RUNNING, and invokes original nextFile/processFile. It
overrides no application behavior. This boundary excludes FileJob start/run/
finish, notifications, progress, password dialogs and automatic refresh/unmark.
Representative actual original GUI construction and jobs supplement it.

Native ZIPs open by path or double-click in either pane. Extraction has an
explicit review with the source, selected names, destination and collision policy,
plus Apply/Cancel. It runs off the UI thread, retains the archive and exposes
skipped/failed paths and partial results. Archive panes are read-only: move,
rename, batch rename, and writes into archives are disabled. Ordinary ZIP files
can still be copied/moved/renamed from a local folder as files. W07 pause,
cancellation and full background lifecycle remain separate work.

Each case uses separate equivalent original/native trees. Compare every requested
listing (including encounter order and declared metadata), archive-wide boolean
result, complete destination tree, root mode and source archive SHA-256. Zero
cases, missing/reordered responses, incomplete metadata and malformed booleans
cannot produce a pass. Raw commands, class/JAR/source/fixture hashes, host,
normalization and failure history accompany compact receipts. Runtime boundary:
unprivileged Linux x86_64/ext4, OpenJDK 25.0.4.1, en-US/UTC, umask 022. Native
macOS, Wayland, races, ACLs/xattrs/ownership/special bits and disk-full/mid-stream
fault recovery are not verified by this increment.
