# W12 design

## Query and matching

Capture a canonical local root and immutable options when Search starts. The
filename pattern uses only `*` and `?` over Unicode scalar values, with a literal
escape convention frozen in the recovered contract. Default pattern is `*`.
Default matching is case-insensitive Unicode lowercase, with an explicit
case-sensitive option and no implicit canonical-equivalence claim. Extension
uses the existing last-extension rule; a blank extension criterion adds no filter.

Support regular-file/folder/link type selection. Size bounds are inclusive bytes
and apply only to regular files; entries without an applicable size do not match
an enabled size filter. Modified-time bounds are UTC [from, to), with the UI
showing that precision and handling date-only end values as the next UTC midnight.
Enabled metadata criteria exclude unknown values with a diagnostic count. Freeze
these intended semantics against original observations, recording differences.

Hidden inclusion is captured from Show Hidden initially, then explicitly editable
in the search form. When excluded, prune hidden folders and descendants. List
links as links, match their own name/lstat metadata, and never traverse directory
links or resolve a link to search an outside tree. The parent row is not a result.
Initial traversal does not cross descendant mount roots and reports that scope.

## Job and result lifecycle

Use a separate cancellable read-only worker with a search generation ID. Only
one query runs per search view; replacement cancels the prior generation and
waits for acknowledgment before starting the next. Late batches cannot mix
queries or navigate a newly active tab. Capture the destination tab identity when
Open Containing Folder is invoked, not when the search originally started.

Bound traversal to depth 256, one million visited entries and 10,000 results.
Use bounded worker-to-UI batches and virtualized rows; stop with an explicit
limit/partial reason rather than reporting complete. Show visited/matched counts,
current location and errors without an invented percentage when total is unknown.
Cancel retains existing hits as partial. Permission/I/O failures retain earlier
hits and report skipped subtrees. A blocked filesystem call must return before
cooperative cancellation can acknowledge. Do not alter marks or mutation progress.

Keep a deterministic displayed relative-path order independent of batch arrival.
Results capture root-relative names, full structured location, kind and observed
metadata. They are observations, not a promise that entries still exist. Opening
a result revalidates its containing folder and focuses its name only if still
present; disappearance reports an error without selecting another entry by index.
Closing results cancels their worker; application close waits for active workers
under the existing close lifecycle. No source files are modified by search.

## Evidence

Compare the scoped matcher/traversal/result boundaries with original binaries;
use native checks for responsiveness and result navigation. Include simultaneous
copy/search, replacement/cancellation, permission errors and real Unicode paths.
Do not claim a blind holdout unless the executable generator and reserved inputs
were actually frozen before Rust implementation. Native Linux and Mac receipts
are distinct gates.
