# W04 contract — local move and single-item rename

Frozen 2026-10-16 after the original-only 319-case run
`20261016T194020Z-450a05b5`, before native move implementation. Seeds and fixture
paths/policies precede Rust. Reference: pinned muCommander 1.6.2-1 original
MoveJob, rename/copy/delete, collision and inline-rename callers. First host:
unprivileged Linux x86_64, repository ext4 plus a genuinely different `/tmp`
filesystem, JDK 25.0.4.1, umask 022. Reserved seed 991074 stays withheld until
implementation. A second original-only run validates the final observer and
content-conservation checks without changing cases or application behavior.

## Behavior and inputs

Use existing source/destination folders, distinct direct child names, optional
new leaf name for one item, fixed SKIP/OVERWRITE/OVERWRITE_IF_OLDER/
OVERWRITE_IF_SIZE_DIFFERS policy and explicit rename mode. Rename mode is one
focused child in the same folder; marks on other children do not make it a batch
rename. The native UI reviews move-to-other-pane (F6) or focused rename (F2),
starts with SKIP, and confirms exact paths/names. Unchanged inline names start no
original job; native review likewise disables an unchanged rename.

- Original collision rules precede mutation, including raw date/size policy
  comparisons and existing-directory exemption as in W03. Identical/descendant
  SKIP cases are observed. Native always declines overlaps and source inode
  aliases even with overwrite; destructive original choices are excluded.
- Try original local rename first. A successful rename transfers the entry
  itself, retaining inode identity, bytes, link text and full ordinary mode/date
  metadata. An existing regular file can be replaced despite read-only file
  bits when the directory permits it. An existing empty directory can be replaced
  by rename, whereas a nonempty destination directory normally forces merging.
- If rename fails, original falls back on every IOException, not only EXDEV.
  This includes real cross-filesystem moves and permissions that permit copying
  but deny source removal. Files copy first using W03 byte/date/owner-permission
  rules, then source deletion is attempted. If deletion fails, return false and
  keep both copies. Native details must say that the source remains.
- For directory fallback, create or merge destination, capture source date before
  processing children, recurse and aggregate child success. Set destination date
  best effort even after child failures. Delete the source directory only if all
  children succeeded. Failure/skipped children retain the source directory and
  return false; successful siblings remain moved. There is no rollback.
- Ordinary file fallback truncates source timestamps to milliseconds and clamps
  negative times to epoch, copying owner rwx while retaining destination
  group/other creation/existing bits. Directory fallback does not import source
  permissions. Rename fast paths preserve metadata instead of applying those
  copy rules. Both paths must be measured, never conflated.
- Original broken-link rename fails its existence prerequisite; fallback cannot
  read the broken link, so it stays at source. Supported link moves use a
  successful same-filesystem rename only. Whole-folder rename preserves included
  link entries. Relative link targets are compared literally, even if relocation
  makes a formerly valid link dangling.

## Explicit exclusions and native guards

An original-only diagnostic demonstrates that cross-filesystem file-link
fallback converts it into a regular file, and directory-link fallback moves
children out of the link's target before unlinking the link. Native code must
not reproduce that target-consuming behavior. Fallback through a symlink is
unsupported and leaves that link/target intact. The diagnostic reports zero
equivalence cases and exit 2, separate from accepted cases. Directory fallback
containing links is also outside equivalence scope; native reports unsupported
children and retains their source parents. A direct link may use only a safe
supported rename path, with relative targets contained in selected roots.

Absolute/escaping direct links, writes through destination links, hard-link
transfer semantics, special files, races/concurrent replacements, other owners,
ACLs/xattrs/special mode bits, remote/archive providers, mid-stream/disk-full
faults, retry/resume/cancel, batch rename, file watching and full job scheduling
are unverified. Native guards are separately Rust-tested, not counted as
original equivalence. Existing W03 guards remain intact.

## Adapter, measurement and UI boundary

Construct real original MoveJob and files, initialize original default desktop,
enable autoSkipErrors, disable optional integrity-check mode, set RUNNING and
invoke original nextFile/processFile without method overrides. Recursive rename,
copy, metadata and delete methods are original JAR code. Full FileJob lifecycle,
notifications, refresh and dialogs are excluded from this method suite; actual
original MoveDialog and inline editor are observed separately in a full GUI.

Java and Rust receive separately materialized equivalent trees. Same and cross
arrangements are verified with real st_dev values; no mocked EXDEV or rewritten
rename implementation. Record original inode/device values and compare the
logical source-before/destination-after identity relation, never unrelated raw
inode numbers across fixture copies. Compare ordered outcomes and both complete
remaining source and destination trees: paths, kinds, bytes/SHA-256, ordinary
modes, declared modified milliseconds and literal link targets. Assert that each
selected input's byte stream/link remains at its source or mapped destination.
A false return with duplicate copies is a measured partial outcome.

Container root dates, ctime/atime and symlink inode dates are unmeasured. Dates
of potentially surviving selected source directories are omitted by an explicit
mask derived from frozen input paths: removing children assigns runtime-clock
mtime. Their original dates are retained in raw before snapshots; destination
folder dates remain checked. For same-root renames that mask applies to both
views of the same residual source paths. Frozen unreadable-directory cases also
omit their nondeterministic newly-created destination dates as in W03. No
runtime-derived exclusions or path/name/content normalization.

The observer temporarily lifts owner read/search bits only to inspect denied
fixtures and restores exact modes before mutation. Root execution cannot pass.
Source/destination root modes are also compared. Runtime artifacts, including
cross-filesystem trees under uniquely named `/tmp/mucrab-W04-*`, are isolated
from user data and the immutable reference.

Native successful top-level items are unmarked; skipped/failed items remain
marked if they still exist. Refresh both panes. After a successful rename focus
the new visible identity; other marks and independent sorting remain. Hidden
renames follow the existing listing filter/focus fallback. The worker is a basic
UI integration, with no W07 cancellation or full lifecycle parity claim.

## Acceptance

Freeze contract/fixtures/seeds before Rust; original-only first. Require nonzero
complete execution, actual same/cross-device coverage, complete outcomes/trees,
identity relations and selected-content conservation. Reject malformed/incomplete
output as unavailable. All required differences must close. Keep every earlier
predefined/reserved suite passing. Retain meaningful integrity/boundary/protocol
tests, native both-direction moves and focused rename, actual original GUI
observations, checks/hashes, GPL notices, documentation and committed handoff.
