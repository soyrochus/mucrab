# W15 design

## Provider and location boundary

Define structured locations for Local, existing ZIP and SFTP(profile ID, absolute
remote path). Profiles contain label, host, port, user, auth method, optional
private-key path and initial remote directory; passwords/passphrases are never
embedded. Validate URI escaping, IPv6/port/user/path boundaries and display values
as data. Reject password-bearing URLs instead of persisting or logging them.
Bookmarks reference profiles, not live sessions or secrets. A missing profile
leaves a bookmark editable and reports a failed open without changing the pane.

Wrap remote list/stat/read/write/stage/commit operations behind explicit provider
capabilities. Do not broaden local filesystem functions to accept remote-looking
strings. Keep W11 stable tab identity and snapshot paths/connection identity for
jobs. Remote listings use generation-tagged bounded workers; a slow response must
not overwrite a newer tab/location. Use lstat-style link observations, never
follow remote links for recursive copy or file opening in this first increment.
Unsupported/non-Unicode names fail visibly without lossy collision-prone rewriting.

## Host trust, authentication and secrets

Perform host-key verification before sending authentication credentials. Maintain
an application-owned, atomically saved host-key store keyed by host and port.
Known matching keys continue; first-use trust shows host/port, key algorithm and
SHA-256 fingerprint for an explicit user decision. Cancellation makes no trusted
entry. A changed key is a connection failure until a separate deliberate trust
management action replaces/removes it; there is no automatic accept-all path.

Initial authentication supports password or a user-selected private-key file,
with an in-memory passphrase when needed. Do not modify key files or execute
shell commands to interpolate paths/credentials. Defer SSH agents, interactive
MFA and custom SSH-config/proxy parsing. Explicitly requesting Remember Secret
may use macOS Keychain or an available Linux Secret Service adapter; unavailable
storage disables remembering and permits session-only authentication. Never
fallback to plaintext config, environment variables, command arguments or logs.
Deleting a profile removes its owned stored credential reference with visible
errors; it never removes user key files. Redact errors/telemetry and profiles
consistently. Document residual memory limitations instead of promising complete
secret erasure that the chosen library/runtime cannot establish.

## Session and job lifetime

Each connected tab retains a session lease; jobs pin the intended profile/session
and paths independently of the active tab. Closing a tab does not interrupt an
already reviewed transfer. Explicit Disconnect cancels associated work, waits for
acknowledgment and closes sessions; visible remote tabs become disconnected and
cannot mutate a stale listing. Reconnect re-verifies trust/authentication and only
commits a new location after a successful list. Failed connects retain the last
successful pane location and history cursor.

Use finite connection/handshake and operation timeouts and provider-supported
cancellation. A stalled call must unblock by timeout or channel/session shutdown;
no UI thread waits on network I/O. Separate read-only listing/preparation from
W07's single mutation slot. Application close requests cancellation and waits
for worker acknowledgment under that same lifecycle, reporting partial results.

## Copies and publication

Support local↔SFTP regular files and directory trees only, recursively including
hidden descendants of explicitly selected folders. Preserve the existing effective
selection rules and origin-tab-only unmarking. Remote symlinks and special files
are unsupported and reported, never silently followed. Local-only operations
retain their existing four policies; mixed local/SFTP review initially offers
Skip and Overwrite only. Remote→remote and unsupported mutations stay disabled.

Review captures endpoint identities, destination, names and policy. Revalidate
available local pins and remote kind/size/mtime assumptions before applying;
report the weaker remote identity/precision guarantees explicitly. Do not claim
inode-level or race-free remote identity from SFTP metadata. Reject same-endpoint
overlaps and remote destination link components under the negotiated stable-tree
contract. Require advertised/tested server rename behavior for safe publication.

Write to uniquely owned staging paths and commit only after successful stream
close and declared length checks. Skip must not replace an existing destination.
Overwrite must use a verified replace/rename capability; if unavailable, fail
while preserving the existing destination, never delete it first as a fallback.
Do not call close/size acknowledgment cryptographic content verification or a
power-loss durability guarantee. Verify test transfers independently by hashes.

Pause/cancel at bounded transfer chunks, retain and identify owned partial staging
paths, preserve sources and report per-root outcomes. Do not automatically retry
an ambiguous commit or resume a stale transfer. Refresh affected surviving tabs
and retain marks for unfinished roots. Never infer a remote move from clipboard
Cut state. New remote transfers require a new explicit review after a failure.

## Remote file opening

Enter/double-click on a remote folder navigates it; explicit activation of a
regular file downloads one private retained copy and uses W09 dispatch. Preserve
a validated original leaf name and non-executable owner-only permissions. Bound
the download to 512 MiB, validate available before/after metadata and never launch
a failed/cancelled/known-changed download. Successful external copies survive
muCrab exit under the existing temporary-cleanup policy; edits do not update the
server. If W14 is present, viewing uses the same prepared copy under viewer limits
without making it a required dependency. Remote links, archive browsing, remote
search and system clipboard URL export remain unsupported in this increment.

## Evidence and unresolved choices

Choose library and native credential adapters after probing host-key callbacks,
private-key support, remote rename semantics, timeouts, cancellation and licensing.
Do not lock in an unverified implementation from this proposal alone. Test against
a disposable server, including changed host keys, invalid auth, denied paths,
network loss before/after commit, collisions and slow/cancelled streams. Native
credential-store fixtures must be isolated. Record unavailable servers/hosts as
pending; mock adapters supplement rather than replace real protocol evidence.
