# W17 verification — 2026-10-24

Implemented in the existing Rust/egui workspace. The pre-change review passed 107
Rust tests, 51 Python tests, formatting, strict Clippy and documentation generation;
four service-dependent Rust tests were ignored. The review reproduced remote
deserialization bypass and mutable captured-viewer inconsistency through safe
public APIs, without claiming an ordinary UI crash or a data-loss incident.

## Current verification

- 121 runtime Rust tests and three documentation examples pass, including two
  compile-fail API examples. Four environment-dependent tests remain ignored.
- Formatting, strict all-target/all-feature Clippy and warning-free documentation
  generation pass with the public-documentation/Debug lint policy enabled.
- All 51 Python tool tests pass.
- The deterministic fuzz smoke runner exercised 10,000 inputs across SFTP packet,
  ZIP path and ZIP directory boundaries without live services or filesystem writes.
- Background tests cover superseded preparation, view-option changes, failed
  navigation retaining state, cancellation, remote supersession, local history,
  deferred ZIP focus/mark preservation, bounded replacement and close acknowledgement.
- Move tests include injected sync/removal failures and real distinct tmpfs/ext4
  filesystem coverage. Completed destinations and cancellation semantics remain explicit.

Final Rust, Clippy, documentation, Python and parser-smoke transcripts are retained
in [`checks/`](checks/). See [`docs/rust-quality.md`](../../../docs/rust-quality.md)
for the reproducible commands and fixture requirements.

Selected original Java/native comparisons were rerun with receipts here, preserving
earlier workflow receipts:

| Comparison | Cases | Checked snapshots | Differences |
|---|---:|---:|---:|
| W02 selection/marking | 28 sequences | 1,430 | 0 |
| W02c metadata ordering | 38 sequences | 698 | 0 |
| W05 batch rename | 54 cases | Per-case outcomes | 0 |

The JSON receipts in `w02/`, `w02c/` and `w05/` record the exact binary and source
hashes, host, commands and raw runs. These are scoped compatibility checks, not
full application parity. Later edits do not change the compared ordering/rename algorithms; the final workspace checks cover the complete working tree.

## API audit and intentional differences

Validated locations, committed pane provider/listing relationships and captured
targets can no longer be changed through public field assignment. Property
classification and SSH host/session identity are also read-only. Valid stored
remote/bookmark JSON and comparison snapshot shapes remain compatible. Workspace
callers migrated together to accessors and explicit transitions.

Desktop navigation now requests background preparation and commits only an
applicable successful result. Source identity checks remain bounded safeguards for
stable owned trees, not a race-proof sandbox. Local fallback move now checks file
synchronisation before deleting its source; direct-overwrite partial-destination
behaviour remains intentionally unchanged. No transactional or complete power-loss
guarantee is claimed.

## Host limits

Linux x86_64 with Rust 1.99.1 was exercised. W16 records fresh isolated Xvfb/X11
interaction and scaling evidence. Native macOS checks, physical live theme
notification delivery, Wayland verification, live disposable SFTP and credential
round trips were not rerun in this session. CI workflow changes were inspected and
their constituent commands run locally; hosted Actions execution remains external.
The pre-existing W09–W15 incomplete platform/parity gates remain open.

## OpenSpec validation

Both W16 and W17 pass strict validation. The whole-project validation also reports
a pre-existing failure in unchanged `w09-open-files`: its `file-opening/spec.md`
has no recognised delta section, so the change has no parsed requirements delta.
That earlier proposal was not modified as part of these implementations.
