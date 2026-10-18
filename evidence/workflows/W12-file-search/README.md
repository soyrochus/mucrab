# W12 — File search

**Mac checkpoint — 2026-10-18:** [workspace/comparison/native evidence](../../platforms/macos-arm64/W11-W15.md)
passes both-pane result navigation, composed-name search on APFS, real mounted-child
omission and 64+64 scoped predicate cases. Mac traversal now uses NFC like pane
listings; predicate comparisons preserve the same observed direct-provider inputs.

Implemented with bounded Linux/Mac verification. Broader original traversal/UI
and composite regression gates remain open in [tasks](../../../openspec/changes/w12-file-search/tasks.md).

Go → Search files / Ctrl/Cmd+F opens a nonmodal search window. Each start captures
a canonical local root and immutable criteria. Filename globs use `*`, `?` and
backslash escaping; `?` matches one Unicode scalar, literals compare per-scalar
lowercase unless Match Case is enabled. The matcher keeps exact scalar inputs;
Mac traversal supplies visible NFC names like pane listings while retaining raw
filesystem paths. No cross-platform canonical-equivalence guarantee is made.
Extension uses the existing last-dot rule. Inclusive byte-size filters apply to
regular files only. UTC modified bounds are [From, To); date-only To advances to
the following midnight, while RFC3339 To remains exclusive. Hidden inclusion
starts from Show Hidden, then is independently editable.

Traversal uses lstat, streams directory iterators and never follows directory
links or enters archives, remote providers or descendant mounts. Excluded hidden
folders are pruned. Enabled unknown metadata excludes the entry with diagnostics.
Depth is at most 256 path components, with at most one million visited entries
and 10,000 hits. Errors/limits/cancellation retain partial results; details are
bounded to 256 with a separate total. Batches contain at most 64 hits through an
eight-slot queue, checking cancellation even when full. Only one generation runs;
replacement waits for the prior worker to finish and join. Mutation jobs retain
their own controls and progress. Closing search cancels; window close waits for
active workers. Blocked filesystem calls must return before cancellation can
acknowledge; stable filesystem trees are assumed, without race-free IO claims.

Results are virtualized and sorted by relative path. Selection follows the full
path across batches. Open Containing Folder captures the current tab identity,
checks the observed result and parent before committing navigation, and focuses
by name. Same-folder marks are preserved; normal folder changes clear marks.
Missing/replaced/unreadable or currently hidden results report an error without
replacing the pane or selecting an unrelated row. Results/search state are not
persisted. Content/regex/archive/remote search is outside W12.

## Evidence

- [Recovery](recovery.json): ten pinned action/dialog/builder/provider/job classes,
  JAR/class hashes, CFR/javap output. Original SearchDialog creates a `find:` URL;
  SearchFile retains findings and interrupts its job; SearchJob uses a thread
  pool and depth-based traversal. W12 uses the separately specified bounded worker.
- [Contract](contract.md) / [fixture freeze](fixture-freeze.json): executable
  generator and 64 predefined + 64 reserved inputs preceded Rust changes.
- [Predefined](comparison.json) / [reserved](comparison-reserved.json): all 128
  filenames call the original private SearchBuilder predicate and native matcher.
  No unexpected differences; two reserved cases intentionally treat `\*` as a
  literal star where the original treats backslash literally and expands `*`.
  These compare filenames only; metadata validation is a separate native test.
- Core tests execute all 128 frozen metadata cases, plus Unicode/escape/range,
  UTC midnight/pre-epoch, inclusive size/exclusive time, hidden pruning, link
  cycles/broken links, permission errors (300 total/256 retained), depth/visited/
  hit limits, cancellation, stable order and stale-result navigation.
  A supplied mount root tests the boundary; W10's kernel parser supplies Linux
  discovery. A real bind-mount traversal replay remains pending.
- Worker tests fill the queue without a consumer, cancel and await termination,
  replace a generation and reject its old results. Desktop tests keep a mutation
  paused and its progress unchanged while search finishes.
- [Native](native.json): wildcard results and containing-folder navigation in
  both panes; [concurrent native](concurrent-native.json): a real paused copy
  remains paused with identical path/progress pixels while search returns 10,000
  visibly partial results. Normal close cancels the copy and preserves its partial
  destination. All 20,000 source files remain. No benchmark claim is made from
  this short local-disk replay.
- [W07–W10 regressions](regressions.json) and [workspace checks](validation.json).

Full original traversal/error/result-window comparisons, native close during an
actively traversing search and Linux same-device bind mounts remain pending.
Mac both-pane input and a real different-device mounted child pass. No blind
holdout or full search-job parity claim is made.

## Reproduce

```sh
python3 -B tools/w12_inspect.py
python3 -B tools/w11_w12_fixtures.py
cargo build --workspace --all-features --locked --offline
python3 -B tools/w12.py
python3 -B tools/w12.py --reserved
cargo test --workspace --all-features --locked --offline
```

The shared native replay is documented in W11. After preparing the W07 regression
source tree, run `tools/w11_w12_gui.py concurrent-prepare`, replay the printed
launch with `artifacts/gui/W12-concurrent-final/actions.json` under isolated
`xvfb-run`, then run `concurrent-verify`. Preparation refuses existing runs.
