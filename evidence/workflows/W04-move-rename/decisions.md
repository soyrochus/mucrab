# W04 investigation and implementation decisions

- Started at clean `027078f` on Linux x86_64, OpenJDK 25.0.4.1, with the pinned
  local reference/tool caches. Read the plan and repository before proceeding.
  No new dependency, global installation, upstream implementation-source fetch,
  or change to the immutable reference was needed.
- Recovered 21 original classes via CFR/javap. The original-only 319-case run
  `W04-20261016T194020Z-450a05b5` preceded native implementation. A second run
  `W04-20261016T194307Z-85242245` validates finalized observers and conservation
  without changing application behavior or fixtures. The contract froze before
  Rust; reserved seed 991074 was exercised only after implementation.
- Actual local rename is attempted before fallback for any IOException,
  including source-parent permission failures on the same filesystem. Read-only
  source/destination file bits alone do not prevent directory-authorized rename.
  An empty destination folder can be replaced; nonempty folders force merging.
  Same filesystem does not imply that every directory rename succeeds.
- Rename retains inode/full ordinary metadata, including pre-epoch dates.
  Copy/delete fallback uses W03's date/owner-permission rules. A source parent
  with mode 555 permits copying but denies deletion: false outcome, identical
  bytes in both places, explicit native details. Source removal never follows
  a failed byte copy. Directory fallback aggregates child success and retains
  parents after errors, unlike original W03 directory-copy true outcomes.
- A real different `/tmp` filesystem is mandatory. Each original/native run gets
  independent fixture trees and a unique external root. The suite refuses to
  count a cross-device case without different st_dev values. No mocked rename
  failure or duplicate Java algorithm supplies evidence.
- Snapshot inode numbers are opaque per fixture: compare only the logical
  source-before/destination-after identity relationship, retaining raw IDs.
  Source directories modified by removing children have nondeterministic dates;
  the date mask is derived statically from input paths and includes same-root
  rename views. Destination dates otherwise remain checked. Unit tests reject
  loss of selected bytes/link text and differences in identity relationships.
- Original-only diagnostic `W04-symlink-fallback-20261016T194137Z-54267cf2`
  demonstrates target consumption through directory-link fallback and file-link
  conversion. Its exit 2 / zero-equivalence status stays separate. Native guard
  declines fallback through links and retains source/target. Safe atomic paths,
  whole-folder included links and broken-link failure cases remain measured.
  Native alias/overlap/escape guards are separate integrity tests, not parity.
- Reused W03 copy metadata/path helpers without changing their logic. Added a
  separate move operation/report, strict W04 adapter and shared UI worker/review
  integration. F2 targets focus rather than effective marked selection, as the
  original filename editor does. Unchanged/invalid names cannot start a job.
  Rename mode is one child in its current folder; batch naming remains W05.
- Initial 319 predefined and 160 reserved comparisons passed on first native
  execution. Final source-pinned reruns of all twelve suites also pass. Strict
  protocols reject incomplete/reordered/nonboolean output; zero work cannot
  become a fresh pass. Expected skipped/failed entries are not omitted.
- Full original GUI observations operate actual MoveDialog controls and inline
  JTextField action listener, without transforming classes or replacing jobs.
  Three moves and focused rename retain ten stages/twenty pane states plus
  verified final trees. Native replays run with no Java on PATH. An initial
  read-only native launch ended on stdin EOF; interactive exploration and the
  final portable replay use fresh fixtures. Exploratory runs remain retained.
- Final native GUI replay additionally marks a failing partial source before
  a real cross-device move, verifying the mark survives while one sibling moves
  and two denied children remain. Screenshots review both permission details.
  The original GUI's broader optional-bundle warnings and physical/Wayland
  dispatch remain unverified. W03's existing 40-action GUI replay still passes.
- F6/F2 are native controls; exhaustive original OS shortcut parity is not
  claimed. A basic worker is not full job scheduling, pause/cancel, retries or
  race-safe mutation. Those limits are recorded rather than hidden by passing
  component counts. GPL attribution is preserved; W05 is the next workflow.
