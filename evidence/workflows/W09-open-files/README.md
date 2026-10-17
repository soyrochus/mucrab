# W09 — default-application file opening

Implemented both local file activation and archive-member temporary extraction.
Enter, double-click and File → Open work in either pane. Folder/ZIP navigation
and marks retain their existing behavior; Enter auto-repeat is ignored.

## Recovery and implementation boundary

[Static recovery](recovery.json) records verified pinned-JAR and class hashes,
CFR 0.152 output and javap logs for OpenAction, TempExecJob and DesktopManager.
OpenAction resolves links, navigates browsable items and delegates local files
to DesktopManager; archive members use TempExecJob. TempExecJob copies first,
then opens the result. Its removal of user-write permission is not reproduced:
muCrab's private copy is owner-readable/writable (0600), with directory mode0700,
and never written back to the ZIP. Broader remote-file execution and application
chooser behavior are outside this workflow. This is static original recovery
plus native implementation verification, not a Java/Rust runtime parity suite.

Linux uses `xdg-open`; macOS uses `/usr/bin/open`. Each receives one absolute
filename argument, without shell interpolation. Missing/broken/cyclic links,
non-regular targets, failed extraction and launcher errors surface in the UI.
Preparation runs separately from W07 mutation jobs and can be cancelled. Window
close cancels and waits for preparation acknowledgment, not application lifetime.
Launcher processes are polled without blocking; nonzero exits report failure.
Successful dispatch only acknowledges sending the request.

The guarded W06 extractor verifies archive paths, resources and CRC before
launch. Copies have unique directories under the OS temporary directory, retain
the original filename, and survive muCrab exit. They are removed on preparation
failure, but accepted copies are retained until OS temporary cleanup or explicit
user cleanup. No automatic archive writeback or muCrab startup purge is provided.

## Verification on Linux, 2026-10-17

[Native receipt](native.json): a real isolated XDG text/plain registration received
three requests from Enter, archive double-click and File/Open. The exact leading
dash, Unicode, spaces and ampersand filename and expected local/archive bytes
were verified. The extracted file remained readable after normal application exit.
Screenshots and input trace are under `artifacts/gui/W09-open-files/native/`.
No personal file associations were changed.

Five additional Rust tests cover extraction/name/permissions, unsafe and missing
members, CRC corruption and cancellation, non-file launch rejection, asynchronous
nonzero launcher exits and the pending-worker guard. All 51 Linux Rust tests,
51 Python tests, locked/offline build, formatting and denied-warning Clippy pass.
W07's 27-action native regression passed with 20,000 sources intact, paused
browsing/marks, partial cancellation and normal deferred close. W08's full native
regression passed its exact actions, filesystem, bytes, ZIP integrity and saved
appearance checks. Runs are retained as `artifacts/gui/W07-W09-regression` and
`artifacts/gui/W08-W09-regression`; older receipts remain historical.

Native macOS dispatch remains unverified on this Linux host. No new original
runtime comparisons, desktop-environment matrix or remote-file support are claimed.

## Reproduce

```
python3 -B tools/w09_inspect.py
cargo build --locked --offline
cargo test --workspace --all-features --locked --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
python3 -B -m unittest discover -s tools -p 'test_*.py'
python3 -B tools/w09_gui.py prepare
xvfb-run -a python3 -B tools/linux_gui.py artifacts/gui/W09-open-files/launch.json --replay artifacts/gui/W09-open-files/actions.json
python3 -B tools/w09_gui.py verify
```

GUI preparation refuses to overwrite an existing run. Preserve an old run before
repeating. Xvfb must be a separate display; never replay against the live desktop.
