# W03 contract — bounded recursive local copy

Frozen 2026-10-16 after original-only run `20261016T190523Z-c944c320`
(146 cases). No native copy implementation existed. Source: pinned muCommander
1.6.2-1 original CopyJob, AbstractCopyJob, TransferFileJob, FileJob, collision
checker and local-file methods. First host: unprivileged Linux x86_64/ext4,
JDK 25.0.4.1, umask 022. Fixtures/seeds precede native implementation;
reserved seed 991073 is withheld until after implementation.

## Inputs and behavior

Inputs are an existing source folder, an existing destination folder, an ordered
list of distinct direct child names, optional new leaf name for one item, and
one fixed collision policy: SKIP, OVERWRITE, OVERWRITE_IF_OLDER or
OVERWRITE_IF_SIZE_DIFFERS. The UI copies the active pane's effective selection
to the other pane, with a review dialog and SKIP initially selected.

- Regular files copy complete bytes, including empty/binary/large-buffer cases.
  Source opens before destination creation/truncation. No atomic staging or
  rollback is claimed; earlier successful entries survive a later failure.
- Directories are created if absent and merge if already directories, under
  every policy. Existing destination children not in the source remain.
  Hidden children participate in recursion regardless of the listing filter.
- Existing non-directory destinations trigger the fixed policy. Older compares
  raw signed modified milliseconds strictly (`source > destination`); size
  differs compares raw local lengths. Equal date/size skips. Directories do not
  trigger this collision, even when a source file then fails to write over them.
  A source directory over a destination file cannot mkdir over it; the file stays.
- Identical source/destination or destination under source are collision classes.
  Their original SKIP behavior is covered. The native implementation always
  declines these cases, including with overwrite policies, to prevent recursion
  or source damage. Destructive original overwrite choices are outside scope.
- Local relative symlinks are recreated with their literal readlink target;
  directory links are not traversed. Broken links remain broken. Link size/date
  metadata follows targets; the original metadata-copy helpers are best effort.
  Supported link targets stay within the source/destination roots. Absolute or
  escaping links, existing destination links that would be followed, hard-link
  aliases, special files, concurrent replacements and symlink races are outside
  equivalence scope. Native guards reject unsupported paths without following
  them for a write. An existing broken destination link remains unchanged when
  creating a link fails, matching the original observed outcome.
- Files get source modified time truncated to milliseconds, with negative times
  clamped to epoch by original LocalFile.changeDate. File owner rwx permissions
  come from source access bits; destination group/other bits remain the existing
  or umask-created values. Full POSIX mode cloning is not the original behavior.
  Copied directory dates are set after recursion, but directory permissions are
  not imported. Metadata failures are best effort, as in the original.
- Errors automatically skip the failing operation and continue. Directory
  enumeration failure leaves its newly created folder. A directory returns true
  after iterating even if children failed; child failures do not roll back other
  entries. A failed top-level file returns false. Native reports surface child
  errors separately so a true directory result is not presented as full success.

Only stable owned fixture files and ordinary permission modes are covered.
ACLs, extended attributes, ownership, special permission bits, sparse allocation,
creation/access/change times and disk-full/mid-stream fault recovery are not
verified. Resume, interactive retry/rename/ask, cancellation, pause, archive and
remote providers, file watching and full background job orchestration remain
separate workflows. The basic native worker keeps copy off the UI thread but
makes no W07 cancellation/job-lifecycle parity claim.

## Original adapter and measurement

Construct the real original CopyJob and original file objects. Initialize the
original default DesktopManager adapter so the real postCopy call is valid.
Enable original autoSkipErrors and disable optional integrity-check mode. Invoke
original nextFile/processFile for each selected item; recursion, collision and
transfer methods execute unchanged. No process, transfer, error or collision
method is overridden. Actual bytes and metadata are observed independently.

This boundary excludes FileJob.run/start/finish, notifications, auto-unmark,
main-frame refresh, progress and OS dialog dispatch. Adapter state stays RUNNING
because processFile is being tested, not completion of a full background job.
Native UI unmarks top-level true outcomes and refreshes both panes separately;
representative full-original GUI checks supplement this method boundary.

Each Java and Rust case starts from a separate equivalent disposable tree.
Compare ordered top-level booleans and complete destination relative paths,
entry kinds, byte lengths/SHA-256, mode bits, declared modified milliseconds and
literal link targets. Independently verify each source tree remains unchanged.
No path/name/Unicode/content normalization. Root container timestamps, link
inode timestamps, ctime/atime and allocation are unmeasured. A failed newly
created unreadable directory has a nondeterministic creation date: only the
explicit frozen ignore_destination_mtime paths omit that date; its identity,
type, permissions and children are still compared. No dynamically inferred skips.

The independent inspector temporarily adds owner read/search bits to denied
fixture entries outside copy invocation, hashes/enumerates, and restores the
exact original modes immediately. This changes ctime only, which is excluded.
Before invoking copy, confirm denied fixtures actually deny the executing uid;
root execution cannot pass. Runtime inputs and raw before/after snapshots remain
in ignored run directories; compact provenance is committed.

## Acceptance

Run original-only before Rust; freeze named fixtures, generated/reserved seeds
and contract. Require complete nonzero execution, exact ordered outcomes and
full filesystem comparisons. Missing/extra/reordered/malformed output is
unavailable, never a passing case. All differences must be resolved or explicitly
left open. Keep W01/W02/W02b/W02c predefined/reserved regressions passing.
Add meaningful file-integrity/boundary/protocol tests, representative input-driven
native copies both directions, original live-GUI observations, source/binary/JAR
hashes, notices, build/lint checks, evidence, ledger and committed checkpoint.
