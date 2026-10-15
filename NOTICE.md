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

The preserved file contains muCommander's licensing statement, GPLv3 and LGPLv3 texts, and notices for third-party works. It describes the upstream distribution and is not a dependency inventory for the future Rust application. Additional file-specific and dependency notices will be preserved as material is introduced.

## Investigation tools

[REA](https://github.com/morluto/rea), [CFR](https://github.com/leibnitz27/cfr), and [Vineflower](https://github.com/Vineflower/vineflower) are investigation tools considered for the experiment. Their code is not included in this baseline. Future evidence will record which tool and version produced each recovered finding.
