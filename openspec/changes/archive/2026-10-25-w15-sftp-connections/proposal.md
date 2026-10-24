# W15 — SFTP connections and transfers

## Why

muCrab currently supports local files and a bounded ZIP provider only. Users
cannot connect to another machine or use the two panes for local/remote copies.
SFTP is a useful first remote provider with one explicit authentication and
transfer boundary, without attempting all muCommander protocols at once.

## What Changes

- Add saved non-secret connection profiles and Connect/Disconnect for SFTP.
- Verify server host keys before authentication, with explicit first-use trust
  and visible refusal of unexpected key changes.
- Support password and private-key-file authentication, keeping passwords/key
  passphrases in memory unless the user explicitly opts into an available OS
  credential store. Never save secrets in profiles, URLs, bookmarks or logs.
- Browse remote directories with local-style metadata, sorting, marks and
  structured per-tab history/bookmarks.
- Review local→SFTP and SFTP→local copies of regular files and directory trees,
  with Skip/Overwrite policies, staging, progress, cancellation and partial results.
- Download a focused regular remote file to a retained private copy for explicit
  W09 default-application opening, without automatic remote writeback.

The initial provider does not implement remote→remote copies, move/rename/delete,
manual remote New Folder, permission/date editing, remote ZIP/search operations,
external remote-file clipboard export, terminal commands, SSH-agent/MFA support,
other protocols or cloud providers. Disable unsupported existing actions explicitly.

## Dependencies and Impact

Build on W11 structured locations/stable tab identity and W07 job control, while
preserving W09/W10 local behavior. W12–W14 are not hard dependencies; an installed
W14 viewer may consume an already downloaded local copy within its own limits.
Introduce a provider/session boundary instead of passing SFTP display strings to
local PathBuf/filesystem APIs. Select a pinned SSH/SFTP library and credential
adapters only after interoperability, cancellation and license probes.

## Verification

Recover the original connection/SFTP/provider/credential callers from the pinned
binary distribution. Freeze the executable protocol/server fixture generator,
predefined/reserved cases, host-key and failure scenarios before Rust code.
Use a disposable local SSH/SFTP server and test credentials, never personal hosts
or keys. Native Linux/macOS credential and UI checks remain separate gates.
This is a planned change, not a claim that SFTP is already supported.
