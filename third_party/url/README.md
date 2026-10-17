# URL parsing dependency notices

W10 uses `url` 2.5.8 for native clipboard file URLs. These unmodified notices
come from the local Cargo source packages verified against Cargo.lock. The
provenance receipt records their hashes and declared licenses. The added graph
includes idna 1.1.0, idna_adapter 1.2.2 and form_urlencoded 1.2.2; all offer MIT
or Apache-2.0 licensing. ICU4X notices are already preserved separately.

Linux file clipboard ownership also uses the existing locked x11rb 0.13.2,
under MIT. Its sources/notices are collected with the full dependency graph by
the release packaging workflow. GIO, xclip and osascript are external runtime
services; they are not bundled into the Rust executable.
