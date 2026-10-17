# W07 decisions and verification boundaries

- Use one shared control/snapshot for execution, with final-result acknowledgement
  separate from a cancellation request. Do not expose Java's immediate
  INTERRUPTED transition as proof that native writes have stopped.
- Retain completed writes and partial destination prefixes. The original closes
  streams without deleting incomplete output, and checks interruption before
  deleting move sources. A native cooperative checkpoint cannot undo an atomic
  rename already committed or interrupt a blocked filesystem call.
- Keep legacy synchronous core entry points as wrappers with an uncancelled
  control. Add controlled variants and preserve the earlier behavior when no
  pause/cancel request occurs. All earlier predefined/reserved comparisons pass.
- Count selected roots (copy/move), changed rows (batch) or archive members
  (extraction); expose current-file bytes separately. Do not label a recursive
  estimate as a known byte total or show a speculative ETA. Snapshot replacement
  avoids an accumulating per-buffer channel queue.
- Finalize only on the worker, after core calls have returned and owned streams
  have dropped. The last channel message follows finalization; no filesystem
  work follows it. The UI joins only after that message/disconnection, refreshes
  current locations, and preserves marks when the source pane has navigated away.
  Fatal worker/preflight failures are distinct from finished queues with issues.
- The original run/start/pause/cancel calls execute inside the full application,
  with reflection for construction/active-table setup and a state listener.
  Protected methods are not replaced. Registration/concurrent scheduling through
  the full JobsManager UI and remote providers are outside the oracle boundary.
- Recovery and the contract, 18 base cases, and reserved seed were recorded before
  native implementation. The full runtime agent and reserved-case generator were
  completed during integration, rather than closing every runtime gate before
  writing Rust. This is a sequencing limitation of this increment; no blind
  holdout or fully staged experiment is claimed. The native file-operation
  semantics did not need tuning to make the original/native suites pass. Initial
  failures were in observer access/window capture; successful runs are the only
  counted comparison cases.
- Compare scheduler-independent consequences. Stream cases must reach a genuine
  nonempty incomplete prefix; each prefix is validated against source contents.
  Actual prefix length is retained in raw results, then normalized for comparison.
  Runtime dates/modes on incomplete files are omitted. Earlier metadata suites
  continue to cover completed-file metadata.
- Use the actual production build and 20,000 tiny regular files for native UI
  testing; no injected sleep, mock filesystem, throughput limit, or test-mode
  application behavior. Native progress observers are used only in core tests
  and the Rust comparison adapter for deterministic boundaries.
- The native replay's final await_exit action must observe code 0 before the
  driver cleanup runs. A screenshot of a disappeared window alone is not used
  as proof that close cancellation completed normally.
