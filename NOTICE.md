# Attribution and provenance

## muCrab

Application name: **muCrab**. Command name: **`mucrab`**. Tagline: **Two claws, two panes.**

Original muCrab material is copyright (C) 2026 muCrab contributors and is licensed under the GNU General Public License, version 3 (`GPL-3.0-only`). The license text is in [LICENSE](LICENSE).

## muCommander

muCrab is an independent experimental port of selected workflows from [muCommander](https://github.com/mucommander/mucommander). Credit belongs to the [muCommander contributors](https://github.com/mucommander/mucommander/graphs/contributors) for the reference application. Existing upstream copyrights and license grants are retained for any incorporated material.

The reference release is **1.6.2**, tag **`1.6.2-1`**. Its distribution manifest identifies implementation revision **`3bc2118`**.

The file [third_party/mucommander/license.txt](third_party/mucommander/license.txt) is an unmodified copy of `license.txt` from the official binary archive:

- Archive: `mucommander-1.6.2-1-portable-x86_64.zip`.
- Origin: [official GitHub release asset](https://github.com/mucommander/mucommander/releases/download/1.6.2-1/mucommander-1.6.2-1-portable-x86_64.zip).
- Archive SHA-256: `e6769537bbb56cc52c1289ebef2e3d051912bb8b10691d320f28b945f15819a4`.
- Notice imported on: **2026-10-15**.

The license file was imported from the checksum-verified archive downloaded during the earlier feasibility investigation. The complete binary archive has subsequently been acquired inside this repository using [the acquisition helper](tools/acquire_mucommander.py), with provenance recorded in [the acquisition evidence](evidence/acquisition/mucommander-1.6.2-1.json). Downloaded and extracted binaries are kept in ignored local storage.

The preserved file contains muCommander's licensing statement, GPLv3 and LGPLv3 texts, and notices for third-party works. It describes the upstream distribution and is not a dependency inventory for the Rust application. Additional file-specific and dependency notices will be preserved as material is introduced.

## Rust desktop framework and fonts

The Rust workspace packages are licensed under `GPL-3.0-only`. The desktop scaffold depends on eframe/egui **0.36.2** and their dependency graph pinned by [Cargo.lock](Cargo.lock). The egui project offers its framework code under `MIT OR Apache-2.0`; the [MIT notice](third_party/egui/LICENSE-MIT) is preserved here. Other dependencies retain their own terms and notices in their Cargo source packages.

The embedded default fonts retain their original licenses, independently of the application license. [The framework/font notice index](third_party/egui/README.md) records the preserved notices and their provenance. The [release workflow](.github/workflows/release.yml) packages complete locked Rust dependency sources in a corresponding-source archive and includes collected dependency notices and a license inventory with every binary bundle. See [release packaging](docs/releases.md).

## Recovered sorting and ICU4X

The Rust name comparator and table index sorting were reconstructed from muCommander 1.6.2's `FileComparator` and `FileTableModel` binaries. The upstream muCommander attribution and GPL notices above apply. [W01 recovery and comparison evidence](evidence/workflows/W01-local-browse-sort/contract.md) identifies the original inputs and tested behavior; decompiled Java implementation source is not distributed in Git.

The focus/marking state and related native interactions were reconstructed from
the original `FileTable`, `FileTableModel`, and selection action binaries.
The same upstream attribution and GPL notices apply to this recovered behavior.
[The W02 contract](evidence/workflows/W02-selection-marking/contract.md) records
the runtime adapter boundary and deferred interactions.

Hidden-file filtering and reload restoration were reconstructed from
`ConfigurableFolderFilter`, `LocalFile`, `FileTable$FolderChangeThread` and their
callers in the same pinned binaries. The same upstream attribution and GPL
notices apply. [The W02b contract](evidence/workflows/W02b-hidden-filtering/contract.md)
records the Unix dotfile scope and original runtime/GUI boundaries.

The native collator uses ICU4X `icu_collator` **2.2.1**, with compiled collation data **2.2.0**. [The preserved ICU4X license](third_party/icu4x/LICENSE) includes Unicode License V3 and ICU adaptation attribution. [The provenance note](third_party/icu4x/README.md) explains the dependency and scope. muCrab remains `GPL-3.0-only`; dependency versions and checksums are recorded in `Cargo.lock`.

macOS directory listings also use the already pinned ICU4X `icu_normalizer`
**2.2.0** to expose NFC filenames as observed from the original Java application.

## Metadata sorting and date display

Size, modified-time and extension comparison were recovered from the same pinned
muCommander comparator, file and table binaries. The upstream attribution and
GPL notices above apply. [The W02c contract](evidence/workflows/W02c-metadata-sorting/contract.md)
records the original-method boundary and signed arithmetic limitations.

The native simple-case table contains observed input/output character facts
from installed OpenJDK 25.0.4.1, generated by our own probe. It includes no JDK
implementation code or runtime. The probe, manifest and data hashes are recorded
in [the case-data receipt](evidence/workflows/W02c-metadata-sorting/case-data.json).

The desktop and core use chrono **0.4.45**, with default features disabled, to
display modified timestamps and expand batch-rename date tokens in UTC. Chrono offers `MIT OR Apache-2.0`; its unmodified
[license notice](third_party/chrono/LICENSE.txt) and [provenance](third_party/chrono/README.md)
are preserved. It does not supply file comparison behavior. Release packaging
retains the complete vendored sources and applicable notices as described above.

## ZIP browsing and extraction

ZIP tree construction, selection/base-depth handling, collision choices and
partial extraction results were recovered from the pinned muCommander ZIP
provider, archive tree, CopyDialog and UnpackJob binaries. The upstream GPL
attribution above applies. [The W06 contract](evidence/workflows/W06-zip-browsing-extraction/contract.md)
records the DOS timestamp normalization, measured boundary and native guards.

Container parsing and stored/deflate/CRC reading use Rust zip **9.0.4** (MIT),
with [its preserved license and provenance](third_party/zip/README.md). flate2
**1.1.10** remains pinned. The release workflow preserves their complete sources
and notices along with nix and typed-path, and the native application requires
no Java runtime. Library filesystem-extraction convenience methods are not used.

## Recursive local copy

Recursive copy, collision policies, transfer metadata and partial outcomes were
reconstructed from the pinned muCommander `CopyJob`, `AbstractCopyJob`,
`TransferFileJob`, `FileJob`, `FileCollisionChecker` and local-file binaries.
The upstream attribution and GPL notices above apply. [The W03 contract](evidence/workflows/W03-recursive-copy/contract.md)
records the original runtime boundary and native path guards. Original-JAR
adapters and full-application dialog observations remain test-only; the native
copy implementation uses Rust's standard library and requires no Java runtime.

## Local move and single-item rename

Move, rename-first/copy-delete fallback, source retention after partial failures,
and focused single-item rename were reconstructed from the pinned `MoveJob`,
`LocalFile`, transfer/collision callers and `FileTable$FilenameEditor` binaries.
The upstream attribution and GPL notices above apply. [The W04 contract](evidence/workflows/W04-move-rename/contract.md)
records real same/cross-filesystem observations and the native guard that declines
fallback through symlinks. Original dialogs/editor and JAR execution are test-only;
the native implementation uses Rust's standard library and requires no Java.

## Local batch rename

Batch masks, ranges, counters, literal replacement, preview conflict checks and
ordered rename application were reconstructed from the pinned `BatchRenameDialog`
(and token/model classes), `BatchRenameJob`, confirmation dialog and action
binaries. The upstream attribution and GPL notices above apply.
[The W05 contract](evidence/workflows/W05-batch-rename/contract.md) records the
ASCII case/UTC date scope, unsupported manual overrides and native stale-plan
guard. Original-JAR execution and Swing controls remain test-only. Native batch
renaming reuses the Rust move implementation and the already pinned chrono;
no Java runtime is required.

## Investigation tools

[REA](https://github.com/morluto/rea) **6.5.0** and [CFR](https://github.com/leibnitz27/cfr) **0.152** are installed locally as investigation tools. Their downloaded implementations and dependencies are in ignored storage, separate from the future Rust application's source and dependencies. [Readiness evidence](evidence/toolchain/readiness.json) records their actual commands and results.

REA-authored code uses the MIT license. [third_party/rea/LICENSE](third_party/rea/LICENSE) is copied verbatim from the pinned official npm package, whose registry integrity value and dependency lock are recorded in [the tool manifest](tools/manifests/analysis-tools.json) and [npm lockfile](tools/rea/package-lock.json). REA's third-party dependencies retain their own notices in the local installation.

CFR uses the MIT license. [third_party/cfr/LICENSE](third_party/cfr/LICENSE) is copied verbatim from [the upstream `0.152` tag](https://raw.githubusercontent.com/leibnitz27/cfr/0.152/LICENSE), copyright (c) 2011-2019 Lee Benfield. The decompiler JAR comes from [the author's official download](https://www.benf.org/other/cfr/cfr-0.152.jar). Its recorded SHA-256 is a locally computed reproducibility pin; its MD5 matches the value published by the author. The license-file digest is also recorded in the tool manifest.

OpenJDK's installed `javap` supplies the bytecode listing. The installed JDK is not redistributed by this repository. Decompiled muCommander output remains derived upstream material under its original applicable terms and is kept in ignored local storage at this stage.

## Everyday file management (W10)

Folder/deletion, clipboard, history and type-to-find behavior was investigated
through the pinned Mkdir/Delete/Copy jobs, clipboard support, LocalLocationHistory
and QuickSearch binaries. The original attribution and GPL notices above apply;
[W10 evidence](evidence/workflows/W10-everyday-file-management/README.md) records
runtime boundaries and intentional differences.

Native clipboard URL parsing adds `url` **2.5.8** and its locked IDNA/form-encoding
dependencies, offered under MIT or Apache-2.0. Their
[preserved notices and provenance](third_party/url/README.md) are included here.
Linux selection ownership directly uses the existing locked **x11rb 0.13.2**
(MIT); release packaging collects its dependency notices. `gio`, `xclip` and
macOS `osascript` are system services invoked at runtime and are not bundled.


## Tabs, bookmarks and file search (W11/W12)

Tab factories/actions, bookmark values/persistence and search actions, predicates,
providers and job behavior were investigated from pinned muCommander binaries
using CFR, javap and original runtime probes. The upstream attribution and GPL
notices above apply. [W11 evidence](evidence/workflows/W11-tabs-bookmarks/README.md)
and [W12 evidence](evidence/workflows/W12-file-search/README.md) distinguish
recovered boundaries, chosen native differences and outstanding host checks.
The runtime remains Rust-only. Bookmark JSON reuses the already locked
serde_json; search dates reuse chrono. No new dependency versions are introduced.


## Properties and built-in viewer (W13/W14)

Properties/permissions/date jobs and text/image viewing were investigated through
pinned muCommander actions, dialogs, workers and loaders. The upstream attribution
and GPL notices above apply. [W13 evidence](evidence/workflows/W13-properties-permissions/README.md)
and [W14 evidence](evidence/workflows/W14-built-in-viewer/README.md) record exact
runtime boundaries and declared differences. No Java implementation/runtime is
bundled in the native application.

W14 enables PNG/JPEG on already locked image **0.25.10** (MIT OR Apache-2.0),
uses zune-jpeg **0.5.15** and zune-core **0.5.3** (MIT OR Apache-2.0 OR Zlib)
for strict JPEG decoding, and crc32fast **1.5.2** (MIT OR Apache-2.0) for complete
PNG CRC checks. [Preserved codec notices](third_party/viewer-codecs/README.md)
include exact published license files and provenance hashes. No dependency version
changed; release packaging retains all transitive notices and vendored sources.


## SFTP connections and transfers (W15)

The connection, SFTP provider, credential and temporary-copy boundaries were
recovered from the pinned muCommander binaries. Upstream attribution/GPL notices
above apply; [W15 evidence](evidence/workflows/W15-sftp-connections/README.md)
distinguishes provider/stream comparisons from native trust/staging differences.

SSH transport uses ssh2 **0.9.6** (MIT OR Apache-2.0) and libssh2-sys **0.3.3**,
including libssh2's BSD notice. Bounded SFTP v3 framing is muCrab-authored.
Native credentials use dbus-secret-service **4.1.0** on Linux and keyring **3.6.3**
with Apple Keychain on macOS (MIT OR Apache-2.0). zeroize **1.9.0**, base64 **0.22.1**
and percent-encoding **2.3.2** preserve their published licenses. macOS additionally
builds OpenSSL **3.6.3** through locked openssl-src **300.6.1+3.6.3**; Linux uses
system OpenSSL3/libdbus. [SFTP notices](third_party/sftp/README.md) preserve exact
license files and checksums. Release source/notices collection includes all
locked transitive dependencies and bundled OpenSSL notices.

The disposable test server uses Paramiko **4.0.0** only in an external test virtual
environment; it is not an application dependency or bundled runtime. Original
Java/JSch binaries likewise remain test-only. No personal hosts/keys/credentials
are required by the fixture suite.
