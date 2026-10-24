# Rust quality and desktop ownership

W17 applies the Rust guidance from the BeautifulRust review without depending on
a particular agent, plugin or external skill checkout. The core remains independent
of egui; the workspace continues to use its pinned dependencies and safe Rust.

## Validated state

`RemoteLocation` constructors, URL parsing and deserialization share validation.
Its fields are private and the valid JSON shape is unchanged. Property/viewer
captures, reviewed rename/metadata/transfer plans, SSH host identities and
authenticated session settings expose read-only inspection. Retargeting requires
fresh capture. `BrowserPane` stores one committed `PaneLocation` and exposes
read-only entry/order/selection projections; marking uses the pane's own order.

The viewer capture uses separate local and ZIP variants, so resolved local identity
cannot be missing from a well-formed local request. Compile-fail examples protect
the public mutation boundary. A stale filesystem capture is an `io::Error` wrapping
`CaptureChanged`, allowing recovery without matching diagnostic text. Ordinary I/O
errors retain their standard contract; presentation turns them into readable text.

## Browsing and worker lifetime

Headless core callers can still browse synchronously. Desktop panes enable deferred
browsing; startup, paths, parent/history, tabs, refresh and post-operation refresh
queue owned requests. Each pane owns at most one running read worker and one latest
replacement. Results carry the originating tab and generation. A successful current
result commits a complete listing; failed or superseded preparation does not replace
the previous location, history, focus or marks. Changed ordering/visibility options
cause fresh preparation before commit.

Revisiting a tab refreshes deferred content. Closing cancels and tracks workers until
acknowledgement. Blocking filesystem calls remain OS-controlled: cancellation cannot
forcibly interrupt a stalled call, although the event loop stays responsive. The
existing SFTP, viewer, search and mutation lifetimes remain independent.

`browse_jobs.rs` coordinates listing workers. `mutation_job.rs` owns mutation worker
startup, panic containment and acknowledgement. `confirmations.rs` owns confirmation
state and returns decisions without starting workers. `chrome.rs` and
`workspace_ui.rs` separate rendering from the root composition/dispatch method.

## Move persistence

Atomic filesystem rename remains preferred. Copy/delete fallback completes writes,
applies the existing best-effort metadata policy and checks destination `sync_all`
before source removal. Sync failure or cancellation before deletion retains the
source. Source-removal failure reports that both copies remain. Failure injection
tests verify write/sync/remove ordering alongside the real two-filesystem tests.

Direct local overwrites still write into the destination: an error can leave it
partial. This change does not provide staged local overwrite, multi-file rollback,
directory-entry persistence or a guarantee against every power-loss/device failure.
SFTP staged publication is unchanged.

## Checks and parser tooling

Workspace lints enforce public documentation, useful Debug implementations and
forbidden first-party unsafe. Secret/transport internals are omitted from session
Debug output. Production glob imports and exports have been replaced with explicit
dependencies. Job observers run outside internal progress locks; the bounded
progress snapshot deliberately remains readable after a worker unwind so the
application can report failure.

Pull requests and branch pushes run formatting, strict Clippy, all-feature Rust
tests, documentation and Python tests. Releases also run strict Clippy. Move tests
require an unprivileged user and two real filesystems. Service/credential tests
require the documented disposable environments and are not automatically redirected
to personal credential stores.

Run the ordinary checks:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps --locked
python3 -B -m unittest discover -s tools -p 'test_*.py' -v
```

The additive `test-util` feature exposes bounded, side-effect-free SFTP packet and
ZIP path/directory parser entry points. A deterministic smoke runner needs no
network, personal files, nightly compiler or additional dependency:

```sh
cargo run -p mucrab-core --example fuzz_boundaries --features test-util --locked -- 10000
```

These entry points can also be driven by a coverage-guided fuzzer. The smoke runner
is not a substitute for a long fuzz campaign or a dependency vulnerability audit.

See [W17 verification](../evidence/workflows/W17-rust-quality-hardening/README.md)
for exercised checks and explicit host/service limitations.
