# W15 — SFTP connections and transfers

Implemented with bounded Linux verification on 2026-10-17/18. W13/W14 were
committed as `e44dbb0` before W15 work. W15 is SFTP, correcting the old ledger's
checksums wording. Native macOS build/Keychain/UI and the gates below remain open.

Go → SFTP connections edits versioned non-secret profiles. Save the profile before
connecting; changed connection settings require saving and disconnect old sessions.
Typed locations are `Sftp(profile ID, absolute remote path)` in panes/history/
bookmarks. URI escapes are validated; password-bearing URLs are rejected and
removed from the location field without including their contents in errors.
Failed connections/listings retain the last successful location/history.

Host keys are inspected before authentication. First use displays endpoint,
algorithm and SHA256 fingerprint for explicit trust; cancellation saves no trust.
Changed keys fail until separate deliberate trust removal. Trust/profile identity
is rechecked before a completed connection becomes active. Password/private-key
(passphrase) authentication and optional native credentials run on workers.
Linux directly uses Secret Service with finite DBus calls, a five-second prompt
maximum, and explicit locked-store refusal; macOS uses Keychain through keyring.
There is no plaintext fallback. Saved credentials bind to endpoint/user/authentication
context; changed settings cannot send an old saved secret to another connection.
Profile deletion removes owned credentials or
reports failure, retaining the profile; user key files are never removed.
Owned secret strings are zeroized where possible; GUI/C-library copies are not
claimed completely erased.

Mixed local↔SFTP Copy reviews captured endpoints/names and Skip/Overwrite. Trees
include hidden descendants and reject links/special files. Source/destination pins
are revalidated; remote checks use kind/size/whole-second time, not inode identity.
Files stage exclusively, close and check length before publication. Skip never
replaces an existing file; Overwrite requires advertised POSIX rename, with no
client delete-first fallback. Cancellation/failure preserves sources, existing
destinations before publication and owned partial paths, without rollback or
automatic retry of uncertain commits. Directory merges and completed work remain.

SSH calls have five-second timeouts. Read/listing workers are generation-tagged;
per-session protocol exchanges serialize without holding their gate across pause
or between chunks. Explicit Disconnect cancels associated work and shuts down the
socket. Ordinary tab closure does not cancel captured transfers. Only one mutation
job runs, preserving origin-tab-only outcomes and unrelated marks.

Remote Open/View prepares a private owner-only non-executable retained copy within
512MiB, checks metadata before/after, preserves its name and never writes back.
W14 limits still apply to viewing. These prepared copies remain until OS temporary
cleanup or explicit cleanup, including viewer downloads. Remote archives require
an explicit Copy to local first. Remote↔remote copies, move/rename/delete/New Folder,
metadata editing, search, file clipboard export, SSH agents/MFA/proxies/custom SSH
config and other protocols remain unsupported.

## Evidence and honest boundaries

- [Recovery](recovery.json): ten pinned connection/provider/panel/credential/job
  classes inspected with CFR/javap; reference bytes verified unchanged.
- [Contract](contract.md), [fixture freeze](fixture-freeze.json) and
  [errata](fixture-errata.json): seeded32+16 recipes and protocol scenarios preceded
  production Rust. Server root validation and encrypted-key authorization needed
  corrections after Rust began; raw-name and lost-ack instrumentation came later.
  GUI traces were recorded after implementation. No fully blind executable-server
  or GUI-fixture claim is made.
- [Library selection](library-selection.json), probe source/result beside it:
  ssh2 0.9.6 verified against unprivileged loopback OpenSSH. Host key was available
  before authentication; key auth passed, generic rename could not overwrite,
  advertised POSIX rename replaced without a client unlink, and socket shutdown
  interrupted a blocked read in100ms. The application therefore implements a
  bounded SFTPv3 subset over authenticated subsystem channels, rather than
  trusting advisory generic rename flags. Final Linux credentials use pinned
  dbus-secret-service4.1.0 directly; keyring3.6.3 remains the Mac adapter.
- [Predefined](comparison.json)/[reserved](comparison-reserved.json):32+16 scoped
  original/native provider-listing/stream cases, zero differences, independent
  source/destination hashes, exact regular-file names/types/length and observed
  whole-second dates. The Java adapter calls pinned SFTPFile ls/input/output
  methods; it does not rewrite the provider. Skip integrity is a native check;
  this adapter is not full original CopyJob collision/staging/trust/UI parity.
  Original UserInfo accepts trust prompts, and original rename can delete first;
  native behavior intentionally follows the safer declared contract.
- [Services](services.json): two real SFTP tests plus one credential test exercised
  in unavailable/unlocked modes (four executions, three distinct tests). Password,
  plain/encrypted keys,48 frozen transfers, hidden trees, link/name/denied guards,
  stale destination, no-POSIX refusal, upload/download partial cancellation,
  same-session listing beside paused copy, timeout/socket shutdown and lost commit
  acknowledgment pass. Added assertions cover precommit network loss and Open
  source changes/failed-copy cleanup. Credentials use a dedicated bus without
  service activation directories and private0700 runtime/keyring/data paths;
  only disposable profile IDs/keys/credentials are used.
- [Native](native.json): final Linux first-use trust, session-only auth, both copy
  directions, Skip/Overwrite, hidden data, exact remote-view clipboard, external
  retained copies, structured bookmark, tab duplicate/close, disconnect/reconnect,
  surviving-tab refresh and normal exit. Source bytes and non-secret config/logs
  are asserted. The44-action trace assumes1120×720 and records its actual binary.
- [Regressions](regressions.json) and
  [properties/viewer regression](regression-properties-viewer.json): fresh native
  W07–W14 checks on the preceding W15 candidate. These precede final bounded
  credential/trust adjustments and retain that binary provenance. Final W15
  workspace/services/native receipts identify the final implementation separately.
- [Workspace](validation.json):106 ordinary Rust tests,51 Python tests, format,
  locked/offline build, Clippy and strict OpenSpec. Four service-dependent tests
  are ignored in the ordinary workspace invocation (one historical W10 test plus
  the three W15 service tests); separate receipts count only actual executions.
- [Release notice audit](license-audit.json): all429 locked external package notices
  pass the real release collector; no hosted release or Mac packaging is claimed.
- [Notices](../../../third_party/sftp/README.md): exact published files/checksums,
  including libssh2 and bundled OpenSSL. Linux needs OpenSSL/DBus development
  packages to build and their system libraries at runtime. Mac builds enable
  pinned vendored OpenSSL3.6.3 to avoid a Homebrew runtime dependency; this host
  does not establish the Mac build or Keychain result.

Protocol references: [SFTPv3 draft](https://datatracker.ietf.org/doc/html/draft-ietf-secsh-filexfer-02)
and [OpenSSH extensions](https://github.com/openssh/openssh-portable/blob/master/PROTOCOL).
Packet512KiB, chunk32KiB, listing20000, tree100000/depth256 and Open512MiB are native
bounds. Close/length acknowledgment is not a cryptographic transfer or power-loss
durability guarantee; test hashes are checked independently. Stable-tree checks
are not race-free filesystem access, and remote metadata identity is weaker.

## Reproduce

Acquire the pinned reference/JDK for original probes. Cache locked Cargo sources;
install the Linux build prerequisites, SSH key generator, Xvfb/XTest/Pillow/xclip,
DBus and gnome-keyring for isolated native service checks. Paramiko4.0.0 is test-only:

```sh
python3 -m venv /tmp/mucrab-w15-venv
/tmp/mucrab-w15-venv/bin/pip install 'paramiko==4.0.0'
python3 -B tools/w15_inspect.py
python3 -B tools/w15_fixtures.py
cargo build --workspace --all-features --locked --offline
python3 -B tools/w15.py --server-python /tmp/mucrab-w15-venv/bin/python
python3 -B tools/w15.py --reserved --server-python /tmp/mucrab-w15-venv/bin/python
python3 -B tools/w15_prepare.py artifacts/gui/W15-replay
/tmp/mucrab-w15-venv/bin/python -B tools/w15_server.py artifacts/gui/W15-replay
```

Keep that disposable server running; in another terminal:

```sh
python3 -B tools/w15_services.py --server-run artifacts/gui/W15-replay
python3 -B tools/w15_gui.py configure --run artifacts/gui/W15-replay
xvfb-run -a python3 -B tools/linux_gui.py artifacts/gui/W15-replay/final-launch.json --replay artifacts/gui/W15-replay/actions.json
python3 -B tools/w15_gui.py verify --run artifacts/gui/W15-replay
python3 -B tools/w15_validate.py
```

Service tests intentionally change their disposable files, so use a separate
fresh run/server for an exact native GUI replay (as the recorded receipts do).
Preparation/configuration refuse to overwrite. Stop the fixture server afterward.
`tools/w15_regressions.py prepare` prints the earlier replay commands; replay the
W11/W12 restart afterward, then `verify` writes only W15 receipts.

## Pending acceptance

Native macOS build/Keychain/UI; full original connection/job/window parity; native
input during active remote transfer tab/app closure and expanded disconnect races;
other servers, key formats, IPv6 and unusual network behavior remain unverified.
The initial provider has no resume/automatic retry, remote archive/search/mutation
extension, strong remote inode identity or complete secret-memory erasure claim.
Tests/scope/limits are explicit in the OpenSpec task list and maintained status.
