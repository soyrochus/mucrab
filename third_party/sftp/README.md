# SFTP and credential dependency notices

Pinned published crate license files are preserved unmodified here. ssh2/libssh2
supply SSH transport/authenticated subsystem channels; bounded SFTP v3 framing is
muCrab code. keyring uses native Secret Service/Keychain with no plaintext fallback.
The release notice collector includes all locked transitive dependencies and
corresponding sources. manifest.json records versions and exact notice hashes.

Linux uses dbus-secret-service directly with a five-second maximum prompt wait
and refuses locked credentials/collections without attempting unlock; macOS uses
keyring with the native Keychain adapter. macOS additionally enables ssh2
vendored OpenSSL with the locked openssl-src3.6.3 package, whose notices are
included. Linux uses system OpenSSL3 and libdbus at runtime.
