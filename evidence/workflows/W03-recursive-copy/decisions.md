# W03 decisions and investigation record

The increment covers recursive copy into an existing local destination folder,
ordered selected children and four fixed collision policies. Optional new-name
support for a single item is checked in the core/adapter; the native UI reviews
copy into the other pane. Move/rename and general destination resolution remain
W04. Resume, cancellation and full job orchestration remain separate workflows.

Original CopyJob construction succeeded with real files, but the initial smoke
invocation (`artifacts/runs/W03-smoke`) failed after copying bytes because the
original DesktopManager had not been initialized. It also reported the known
CachedFile module-access requirement. This is not counted as a passing copy
case. Adding the required java.io module open and invoking the original
DesktopManager.init(false) initializes its registered DefaultDesktopAdapter;
no postCopy method is replaced. The successful smoke at
`artifacts/runs/W03-smoke-ddc39a0d` copied a binary file and empty directory.

Initial CFR/javap recovery covered fifteen classes, including the constructor,
recursion, collision/error/metadata helpers, local file operations and desktop
initialization. The final receipt also fingerprints DefaultDesktopAdapter and
DesktopAdapter, whose default postCopy hook is used in the isolated runtime.
The full GUI uses its actual initialized desktop environment and copy dialogs.
REA was not used to generate behavior or Rust source; its earlier container
inventory remains separate from this JVM investigation.

Original-only run `20261016T190523Z-c944c320` completed 146 cases before Rust.
Fixtures include empty/binary/Unicode/dotfiles, multi-level/empty folders,
permissions, pre-epoch milliseconds, directory merging, type conflicts, policy
ties, unreadable sources, unwritable destinations, missing inputs, continued
work after failures, partial trees, contained/broken links and SKIP overlaps.
The contract and seeds were then frozen. Reserved seed 991073 was first run
after implementation and passed 96 cases without a discrepancy.

The oracle constructs a real CopyJob and invokes original protected methods via
reflection. Literal fixture inputs and original autoSkipErrors configure the
job, but no process/transfer/collision/error method is overridden. Original
FileJob.run needs a live main frame and lifecycle wiring; it is excluded from
the isolated equivalence boundary and is exercised separately by the actual
CopyDialog/ProgressDialog path in the full GUI. Returning RUNNING in the method
protocol is not a claim that a full background job reached FINISHED.

The original directory method intentionally ignores child return values and
returns true after successful enumeration. That boolean alone would hide
partial failures. Native code preserves it for comparison/auto-unmark behavior
while retaining per-path problems in CopyReport and showing them in details.
Earlier successful children remain; no rollback or atomic-file replacement is
promised. Metadata updates are best effort, with measured owned-file outcomes.

File modes do not clone complete POSIX permissions: source owner rwx access
bits are imported, while destination group/other bits retain creation/existing
values. The comparison runner and GUI launches explicitly use umask 022. An
initial Rust test wrongly assumed that mask for every test host; the host's
normal test umask was 002. The test now checks group/other retention against an
independently created control file. No behavior or fixture was changed to hide
a Java/Rust discrepancy. Negative source dates clamp to epoch in original
LocalFile.changeDate; file/directory timestamp comparisons cover that behavior.

The independent observer hashes actual trees and checks each source after
execution. Denied fixture entries temporarily receive owner read/search bits
only while the observer runs, with exact restoration before copy. Ctime and
atime are deliberately excluded. Two explicitly named newly created directories
whose source enumeration fails have nondeterministic creation dates; only those
frozen date fields are omitted. Byte/path/type/permission observations remain.

Native guards always reject source/destination overlap, inode aliases, writes
through destination links and source link targets escaping selected roots.
Original OVERWRITE on such paths can damage sources or recurse, so only
original SKIP overlap outcomes are in the comparison contract. Guard tests
verify unchanged source/outside bytes. Absolute/escaping links, hard-link
transfer semantics, special files and concurrent replacement races are not
claimed equivalent. This code is not a race-proof filesystem sandbox.

The native copy worker uses a Rust thread/channel so ordinary copying does not
block input. This is not W07's full progress/cancellation/queue behavior. The
review freezes exact source/destination/names; cancel creates no files. Successful
top-level outcomes clear their marks; skipped entries and inactive marks survive.
Both panes refresh while preserving their supported criteria and focus state.

Native GUI checks found that closing a modal did not restore F5 listing focus.
The first direct focus request was rejected because egui retains the previous
modal layer for one pass. The final code waits until that layer clears, requests
listing focus and schedules repaint. Failed attempts remain under
`artifacts/gui/W03-20261016T191400Z-80322705` and
`artifacts/gui/W03-20261016T191528Z-a5d9efcc`. A fresh 40-action replay verifies
cancel/reopen, forward SKIP, details, reverse copy, explicit OVERWRITE, and two
permission errors inside a partially copied directory. Final data is checked
against independent original observations; screenshots supplement those checks.

The full-original probe calls actual selection/active-table methods, constructs
actual CopyDialog instances and operates their real path, policy and Copy
controls on the event thread. Original path resolution, progress/job execution,
auto-unmark and refresh run unchanged. Eight stages/sixteen pane states and
three dialog renders are retained. OS input/shortcut dispatch, notifications,
physical desktop and Wayland remain outside the claim. Optional original bundle
and logging warnings are retained without claiming unrelated component health.

No new Cargo dependency, upstream implementation-source fetch or global tool
installation was needed. Native runtime remains independent of Java. GPL and
upstream attribution are preserved in NOTICE.md. This checkpoint is local
source/evidence work; no release or push is performed.
