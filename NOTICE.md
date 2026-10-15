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

The embedded default fonts retain their original licenses, independently of the application license. [The framework/font notice index](third_party/egui/README.md) records the preserved notices and their provenance. Complete dependency source and notice packaging will accompany a distributable release; this is a local development experiment.

## Recovered sorting and ICU4X

The Rust name comparator and table index sorting were reconstructed from muCommander 1.6.2's `FileComparator` and `FileTableModel` binaries. The upstream muCommander attribution and GPL notices above apply. [W01 recovery and comparison evidence](evidence/workflows/W01-local-browse-sort/contract.md) identifies the original inputs and tested behavior; decompiled Java implementation source is not distributed in Git.

The focus/marking state and related native interactions were reconstructed from
the original `FileTable`, `FileTableModel`, and selection action binaries.
The same upstream attribution and GPL notices apply to this recovered behavior.
[The W02 contract](evidence/workflows/W02-selection-marking/contract.md) records
the runtime adapter boundary and deferred interactions.

The native collator uses ICU4X `icu_collator` **2.2.1**, with compiled collation data **2.2.0**. [The preserved ICU4X license](third_party/icu4x/LICENSE) includes Unicode License V3 and ICU adaptation attribution. [The provenance note](third_party/icu4x/README.md) explains the dependency and scope. muCrab remains `GPL-3.0-only`; dependency versions and checksums are recorded in `Cargo.lock`.

## Investigation tools

[REA](https://github.com/morluto/rea) **6.5.0** and [CFR](https://github.com/leibnitz27/cfr) **0.152** are installed locally as investigation tools. Their downloaded implementations and dependencies are in ignored storage, separate from the future Rust application's source and dependencies. [Readiness evidence](evidence/toolchain/readiness.json) records their actual commands and results.

REA-authored code uses the MIT license. [third_party/rea/LICENSE](third_party/rea/LICENSE) is copied verbatim from the pinned official npm package, whose registry integrity value and dependency lock are recorded in [the tool manifest](tools/manifests/analysis-tools.json) and [npm lockfile](tools/rea/package-lock.json). REA's third-party dependencies retain their own notices in the local installation.

CFR uses the MIT license. [third_party/cfr/LICENSE](third_party/cfr/LICENSE) is copied verbatim from [the upstream `0.152` tag](https://raw.githubusercontent.com/leibnitz27/cfr/0.152/LICENSE), copyright (c) 2011-2019 Lee Benfield. The decompiler JAR comes from [the author's official download](https://www.benf.org/other/cfr/cfr-0.152.jar). Its recorded SHA-256 is a locally computed reproducibility pin; its MD5 matches the value published by the author. The license-file digest is also recorded in the tool manifest.

OpenJDK's installed `javap` supplies the bytecode listing. The installed JDK is not redistributed by this repository. Decompiled muCommander output remains derived upstream material under its original applicable terms and is kept in ignored local storage at this stage.
