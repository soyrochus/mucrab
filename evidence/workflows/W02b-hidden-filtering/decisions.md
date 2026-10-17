# W02b decisions and discrepancy history

- Recovered the global hidden-file preference and default false before Rust
  implementation. The desktop has one shared checkbox rather than independent
  pane settings. W01's all-entry enumeration and W02a's explicit loaded-list
  adapter remain unchanged in scope; the native default now matches the original.
- Filtering happens before listing replacement and sorting. Preserve only
  surviving marked identities. Do not retain hidden marks off-screen or revive
  them when showing hidden entries again. Focus fallback and held-Space guards
  were confirmed in the original-only run before implementing Rust.
- Executed the original reload worker instead of copying its focus/mark logic
  into a Java oracle. Original column/model constructors, a monitored folder,
  and field-only desktop fixtures let the complete worker run without method
  overrides. Reject its internally caught exceptions by inspecting stderr.
- The original natural/lexicographic preference is global. Freeze it per
  sequence for both panes; direction/folders-first remain pane-local. Mixed
  modes across a shared filter reload are outside the equivalence claim.
- Predefined and reserved comparisons passed on their first executions. No
  behavioral mismatch was hidden, normalized away or removed from fixtures.
  Final comparisons were rerun after adding integration tests so their source
  inventories identify the final sources, including tests.
- Default sandbox access could not connect the native app to the virtual X
  server; isolated Xvfb checks ran with local display access. The first original
  input-driver attempt focused an unmapped startup window and failed BadMatch;
  the driver now waits for a mapped application window. Unreliable original
  mouse/key dispatch and black AWT screen captures were not counted as GUI
  parity. A separate startup agent invokes the actual menu action in the full
  application, validates both live tables, and retains direct Swing renders.
  Native input checks use real XTest mouse/key events and screen captures.
- The reference portable archive emits optional OSGi/logging warnings on this
  host. The original menu-action state probe completes successfully; full
  reference startup/shutdown health and unrelated components are not verified.
- No upstream Java implementation source was fetched. REA remains an inventory
  tool; CFR/javap, original Java execution, original Swing observations and Rust
  work supplied this increment's semantic evidence. No release or push is part
  of this checkpoint.
