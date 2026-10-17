# W07 — background operations, progress and cancellation

Frozen before implementation, 2026-10-17. Binary recovery is recorded in
recovery.json (CFR + javap of pinned original classes, not upstream source).

## Behavior

One mutation job runs on a worker at a time. Copy, move/focused rename, batch
rename and ZIP extraction retain W01–W06 validation/collision rules. Browsing,
selection, sorting, appearance and Help remain usable while a job runs. A
thread-safe snapshot reports current path, current-file transferred/total bytes,
total transferred bytes and completed/total work items without an unbounded
per-buffer event queue. Item totals mean selected roots for copy/move, changed
rows for batch rename, and archive members for extraction; no recursive total
or ETA is invented.

Lifecycle: Running ↔ Paused → Cancelling → Cancelled, or Finished/Failed.
Cancel is a request, not acknowledgement: the UI waits until the worker stops
and returns a report. Completed terminal states cannot subsequently be cancelled.
Pause and cancel are cooperative between entries and bounded 64 KiB stream
chunks. Pause/resume is included because the original exposes it. A blocked OS
call or atomic filesystem rename cannot be interrupted mid-call.

Completed writes remain. The active stream's destination prefix remains (even
when overwriting an existing file); cancellation is not rollback. No successful
completion metadata is applied to an incomplete regular file. Source files stay
unchanged on copy/extraction. A cancelled fallback move retains the source until
the full copy succeeds and cancellation has been checked before deletion. A
same-device rename already committed when Cancel is requested remains committed.
No later entries start after cancellation is observed. Cancellation is distinct
from a skipped collision or transfer failure. Previously completed roots can be
unmarked; the interrupted root and untouched later selections remain marked.
Nested directory copy's original success return does not justify unmarking an
interrupted root. Both current pane locations refresh at worker acknowledgement;
marks in an unrelated location are not cleared. Closing the window requests
cancellation and defers exit until acknowledgement; process kill/power loss is
outside scope. Worker errors/panics/disconnection are surfaced as failures.

## Original evidence / comparison boundaries

Original FileJob.start/run/interrupt/setPaused/nextFile are used, with the real
CopyJob/MoveJob/BatchRenameJob/UnpackJob. A startup agent controls and observes the
unmodified application, without method overrides/class transformation; it uses
an isolated runtime copy/profile and real filesystems. FileJob.run stops before
unmarking the interrupted current root, refreshes tables and notifies JobsManager.
TransferFileJob closes its current input on interrupt; AbstractFile.copyStream
closes output in finally, retaining partial bytes. MoveJob checks interruption
before source deletion. FileJob FINISHED means the queue ended, including
fixed-policy skipped/auto-skipped-error items; native Failed is reserved for
unhandled worker/preflight errors.

Streaming cancellation is scheduled at equivalent byte milestones. Exact partial
length/timing and throughput are not compared: scheduling and read-ahead vary.
Each implementation must retain a valid nonempty proper source prefix, earlier
completed bytes, later untouched entries, and move sources. Dates/modes on
incomplete files and directory clocks are not equivalence targets. Completion
metadata remains covered by prior suites. Native request/acknowledgement states
and panic handling are additional explicit lifecycle guarantees.

## Fixtures and checks

Freeze predefined and separately seeded reserved cases before implementation.
Cover completion, skip, partial errors, cancellation before first mutation,
mid-stream cancellation, pause/resume then cancel, directory recursion, real
cross-device move retention, atomic renames/batch boundaries and ZIP extraction.
Check state transitions, both trees and source/destination contents; no zero-work
or zero-case pass. Deterministic native progress observers cover cancellation
between entries and in chunks, repeated/late cancellation, paused cancellation,
worker failure and terminal-state immutability. Native UI checks must exercise
actual progress, Pause/Resume/Cancel, navigation while busy, terminal details,
selection retention, and deferred window close. Retain W08 and all eight earlier
predefined/reserved component suites as regression checks. No claim of full
muCommander scheduler, parallel jobs, remote providers, retry prompts, throughput
controls, checksum verification or race-free filesystem security is made.
