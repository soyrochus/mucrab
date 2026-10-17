# W15 frozen implementation contract

Profiles contain ID/label/host/port/user/auth kind/key path/absolute remote start
path only. Typed remote locations contain profile ID + normalized absolute POSIX
path; never a local filesystem path or secret-bearing URL. URL entry is rejected
with a generic error; profile-backed sftp://profile-id/path is the only remote
location display/input. Failed navigation preserves the previous location/history.

SSH2 0.9.6 supplies verified handshake/password/private-key/channel operations;
SFTP v3 packets are bounded to512KiB and names strictly decoded. Require the
advertised posix-rename@openssh.com version1 extension for overwrite, and v3 rename
without overwrite for Skip. No delete-first fallback. State the stable-tree,
second-resolution metadata and ambiguous publication limitations. SHA256 host
fingerprint and algorithm are inspected before auth; atomically saved application
trust is endpoint-bound. First use requires explicit approval. Changed keys fail
until a separate Forget Trust action. No secret enters profiles/URLs/logs/argv/env.

Finite5second SSH calls, bounded connect-address attempts and channel reads on
workers; explicit Disconnect shuts down the leased socket and cancels associated
work. Per-tab generation replacements discard old results even when already queued.
Tab close cancels listing reads, not captured transfer leases. One mutation job;
32KiB transfer chunks, 20k listing entries,100k tree entries and depth256. Hidden
children included. Never follow local/remote symlinks or special files in mixed
copies/open. Validate local inode/mode/size/time pins and remote kind/size/mtime.

Stage files exclusively with unique owned names, close and verify length before
publish. Sources/existing destinations survive cancellation/precommit failure;
report owned partial paths. Failed/ambiguous commit is never retried automatically.
No cryptographic transfer/durability claim; test harness checks independent hashes.
Downloads/open copies are owner-only nonexecutable, validated leaf names, max512MiB;
external copies survive exit and never write back. Optional viewer receives an
already prepared local copy within W14 limits. Unsupported actions are disabled.

Credential strings use zeroizing wrappers with residual library/GUI memory limits.
Remember explicitly uses native Keychain/Secret Service on worker threads only;
unavailable storage visibly fails and session auth remains possible without a
plaintext fallback. Deletion removes the profile-owned secret or reports failure,
never user keys. Native host gates, original UI parity and untested server features
remain pending. Generator/server/protocol fixture hashes are frozen before Rust.
