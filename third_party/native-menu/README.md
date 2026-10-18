# Native macOS menu notices

Unmodified license files from the published muda 0.21.1, crossbeam-channel 0.5.17
and keyboard-types 0.8.3 crates. All three offer MIT OR Apache-2.0. The manifest
records exact crates.io archive checksums from Cargo.lock and SHA-256 hashes of
every preserved license file, including additional third-party notices.

muda is enabled only for macOS, without its default Linux/GTK feature. It wraps
AppKit menus; the application keeps its existing egui window and command handlers.
Linux continues to use the in-window egui menu. Release packaging also retains the
complete locked dependency sources and collects their notices automatically.
