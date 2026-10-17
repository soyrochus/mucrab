# ZIP reader provenance

muCrab uses the Rust `zip` crate **9.0.4**, pinned in Cargo.lock, for ZIP
container parsing, stored/deflate streams and CRC validation. The MIT license
is preserved unmodified in `LICENSE`, copied from the published crate. Cargo's
package checksum is `a954114b47a54df49f83d2722814e86b23d4bb1b54a1b90fb7d92b41619ddc22`.

The version was selected from the current official release documentation.
Only `deflate-flate2` is enabled; flate2 **1.1.10** was already locked and now
also has a direct core dependency with its Rust backend enabled. Existing
external package pins are unchanged. The added graph consists of zip 9.0.4,
nix 0.31.3 and typed-path 0.12.3; their source packages retain their own notices.
The release workflow vendors and inventories the complete graph.

The library's filesystem extraction convenience methods are not used. muCrab
implements the recovered archive-tree, selection and UnpackJob sequencing rules
and its own path/link/size/review guards. ZIP64, encryption and methods other
than stored/deflate remain outside the declared W06 increment. The native reader
and runtime require no Java code or Java runtime.
