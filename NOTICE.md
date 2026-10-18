# Attribution and licensing

Original muCrab material is copyright (C) 2026 muCrab contributors and is licensed
under the GNU General Public License, version 3 (`GPL-3.0-only`). The full license
text is in [LICENSE](LICENSE). Third-party material retains its own copyright
notices and applicable terms.

## muCommander attribution

muCrab is an independent port of selected workflows from
[muCommander](https://github.com/mucommander/mucommander). Credit belongs to the
[muCommander contributors](https://github.com/mucommander/mucommander/graphs/contributors)
for the reference application. Existing upstream copyrights and license grants
are retained for any incorporated material.

The reference is muCommander **1.6.2**, tag **`1.6.2-1`**, implementation revision
**`3bc2118`**. [third_party/mucommander/license.txt](third_party/mucommander/license.txt)
is an unmodified copy of the official distribution's license file, including its
GPLv3, LGPLv3 and third-party notices. It describes the reference distribution;
the Rust dependency notices are indexed below. The
[acquisition receipt](evidence/acquisition/mucommander-1.6.2-1.json) records the
archive origin and checksum. Workflow-specific recovery provenance is retained in
[the implementation ledger](mucrab-implementation-plan.md) and `evidence/workflows/`.

The generated simple-case table records observed OpenJDK character mappings;
its [provenance receipt](evidence/workflows/W02c-metadata-sorting/case-data.json)
identifies the probe and data hashes. No JDK implementation code or Java runtime
is bundled in the native application.

## Runtime dependencies and fonts

The following indexes preserve original license texts, copyright notices and
provenance. [Cargo.lock](Cargo.lock) pins exact dependency versions and checksums;
each dependency source package retains its applicable terms.

| Component | Licensing and preserved notices |
| --- | --- |
| eframe/egui 0.36.2 and default fonts | Framework: MIT OR Apache-2.0; fonts retain their separate terms. [Framework/font notices](third_party/egui/README.md). |
| ICU4X collation and normalization | Unicode License V3 and ICU adaptation attribution. [ICU4X notices](third_party/icu4x/README.md). |
| chrono 0.4.45 | MIT OR Apache-2.0. [License and provenance](third_party/chrono/README.md). |
| zip 9.0.4 and compression dependencies | zip: MIT; transitive dependencies retain their own terms. [ZIP notices](third_party/zip/README.md). |
| url 2.5.8 and IDNA/form-encoding dependencies | MIT OR Apache-2.0. [URL notices](third_party/url/README.md). |
| x11rb 0.13.2 | MIT; its notices are collected from the locked source package during release packaging. |
| image 0.25.10, zune-jpeg 0.5.15, zune-core 0.5.3 and crc32fast 1.5.2 | MIT OR Apache-2.0; zune also offers Zlib. [Codec notices](third_party/viewer-codecs/README.md). |
| SSH, credentials and vendored OpenSSL | ssh2 and credential crates retain MIT/Apache-2.0 terms; libssh2 retains its BSD notice, and OpenSSL its published notices. [SFTP dependency notices](third_party/sftp/README.md). |
| muda 0.21.1, crossbeam-channel 0.5.17 and keyboard-types 0.8.3 | MIT OR Apache-2.0. [Native-menu notices](third_party/native-menu/README.md). |

System services such as `gio`, `xclip` and macOS `osascript` are invoked at runtime
and are not bundled with muCrab.

## Investigation tools

The following tools are used for development and investigation, separately from
the native application's runtime:

- [REA](https://github.com/morluto/rea) **6.5.0**: MIT.
  [Preserved license](third_party/rea/LICENSE); package integrity is recorded in
  [the tool manifest](tools/manifests/analysis-tools.json) and
  [npm lockfile](tools/rea/package-lock.json).
- [CFR](https://github.com/leibnitz27/cfr) **0.152**: MIT, copyright (c) 2011–2019
  Lee Benfield. [Preserved license](third_party/cfr/LICENSE); origin and checksums
  are recorded in [the tool manifest](tools/manifests/analysis-tools.json).

The installed OpenJDK supplies `javap` and original-runtime probes; it is not
redistributed. Investigation commands and results are retained in
[toolchain evidence](evidence/toolchain/readiness.json). Test-only Java/JSch
binaries and Python fixture libraries are not application dependencies.

## Notices in release bundles

Binary bundles include `LICENSE`, this `NOTICE.md`, `third_party/` and collected
dependency notices in `licenses/`. The generated `licenses/dependencies.json`
records each locked external crate's declared license, authors, checksum and
source location. The corresponding-source archive retains the complete vendored
Rust sources, including embedded license headers. Additional pinned notices are
indexed under [release dependencies](third_party/release-dependencies/README.md).
See [release packaging](docs/releases.md) for the collection process.
