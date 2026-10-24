# W12 — Recursive file search

## Why

W10 quick-find focuses a prefix in the current listing. It cannot find files
under other folders or combine filename, extension, size and modified-date
criteria. Users still need another tool for those searches.

## What Changes

- Add a reviewed, immutable search query rooted at a selected local directory.
- Search descendants by filename pattern, extension, type, size and UTC modified
  date, with explicit case/hidden-entry options.
- Run traversal in the background with progress, cancellation, bounded results
  and clear completion/partial/error status.
- Present virtualized results and navigate to a result's containing folder,
  focusing it only after successful navigation.

Keep W10 quick-find unchanged. Content search, regular expressions, archive
contents, remote providers, indexes, saved searches and direct file mutations
from search results are out of scope. W15 SFTP search is not included implicitly.

## Dependencies and Impact

Reuse local metadata and success-only navigation; integrate W11 tab identity if
it has landed. Search does not require W13–W15. Add a bounded read-only job rather
than acquiring W07's mutation slot, so search and copy can coexist. Shared guarded
traversal may later serve W13 folder size, without merging their user workflows.

## Verification

Recover original Find/search-provider behavior and separate it from QuickSearch.
Freeze query semantics, generator, error/result protocol and reserved seeds before
implementation. Use independent disposable trees, errors, links, boundaries and
both-pane native navigation; record intentional restrictions and each host result.
This is a proposal only and leaves W10's pending verification unchanged.
