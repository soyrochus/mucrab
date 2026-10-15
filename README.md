# muCrab

**Two claws, two panes.**

muCrab is an experimental Rust port of selected [muCommander](https://github.com/mucommander/mucommander) file-management workflows, reconstructed from its published Java binaries. The intended application name is **muCrab** and its command name is **`mucrab`**.

The experiment will use [REA](https://github.com/morluto/rea), a JVM decompiler, and observations of the original application to recover behavior, implement it in Rust, and compare the two implementations.

## Current status

This repository contains the experiment baseline: its description, license, upstream attribution, and ignore rules. Artifact acquisition, tool setup, the Cargo workspace, and application implementation are subsequent steps. There is no runnable Rust application or behavioral-equivalence result yet.

## Reference application

The planned reference is muCommander **1.6.2**, release tag **`1.6.2-1`**:

- [Official release](https://github.com/mucommander/mucommander/releases/tag/1.6.2-1).
- [Binary distribution](https://github.com/mucommander/mucommander/releases/download/1.6.2-1/mucommander-1.6.2-1-portable-x86_64.zip).
- Published archive SHA-256: `e6769537bbb56cc52c1289ebef2e3d051912bb8b10691d320f28b945f15819a4`.

The input is the complete distribution, including the launcher, application JARs, dependencies, resources, and notices. The bundled runtime's platform must be distinguished from the host JDK selected to run the Java application. Acquisition will retain the distribution layout and verify the pinned archive before use.

## Initial demo scope

| Workflow | Intended comparison |
| --- | --- |
| Two-pane navigation and selection | Locations, listings, and selection behavior |
| Sorting and filtering | Ordering, numeric names, locale behavior, and ties |
| Recursive copy, move, and rename | Resulting paths, contents, and selected metadata |
| Batch renaming | Generated names, collision handling, and errors |
| ZIP browsing and extraction | Entry names and extracted contents |
| Background file operations | Completion, cancellation, and failure outcomes |

This scope is an initial demonstration target. Complete muCommander feature parity, remote and cloud filesystems, additional archive formats, and support across operating systems require further increments.

## Execution sequence

1. Establish and commit this repository baseline.
2. Add reproducible acquisition scripts and a checksum manifest; download and extract the reference into ignored local storage.
3. Pin REA and a JVM decompiler, then verify their actual capabilities against the reference. REA's inspected JADX adapter requires an APK, so desktop JAR decompilation needs a separate JVM tool or a verified adapter. Record the producing tool for each finding.
4. Create a Cargo workspace separating the Rust core, desktop interface, and comparison harness. Commit `Cargo.lock` for reproducible application builds.
5. Recover and implement one workflow at a time, beginning with file sorting. Compare each increment against the original binary before expanding the scope.
6. Run the agreed comparison suite and demonstrate fresh scenarios with the compiled Rust application running independently of Java.

Upstream Java implementation source will be withheld during reconstruction; license and attribution material remains available. Decompiled representations, binary metadata, and observations of the original are permitted inputs. Findings from the earlier feasibility investigation must be identified as prior evidence if reused.

## Verification approach

Each workflow will have explicit inputs and acceptance criteria established before its Rust implementation. A harness will exercise the original JARs and the Rust implementation against equivalent fixtures, comparing observable results and recording discrepancies. Any normalization of paths, timestamps, or platform details must be stated explicitly.

The first implementation milestone is one real behavior recovered from the pinned JARs, implemented in Rust, and passing comparisons against the original. The broader demo aims for approximately 100 predefined cases plus withheld cases, with all critical file-integrity cases required to pass. These are acceptance targets, not completed checks.

Fixtures will include numeric and Unicode names, existing destinations, symbolic links, unreadable files, and cancellation. File operations will run inside disposable fixture directories. GUI demonstrations supplement the behavioral comparisons. Build success alone does not establish equivalence.

The experiment will distinguish evidence that binary-to-Rust reconstruction works from evidence that REA improves the process. Tool provenance, human interventions, and remaining gaps will be recorded with the results.

## Planned repository layout

- `artifacts/`: ignored downloads, extracted distributions, and generated decompiler output.
- `tools/`: reproducible acquisition, inspection, and comparison helpers.
- `crates/`: Rust application and comparison components.
- `fixtures/`: reproducible test inputs.
- `evidence/`: selected provenance records and comparison results suitable for Git.
- `third_party/`: preserved upstream notices and their provenance.

Only the baseline files and preserved notices exist at this stage.

## License and attribution

muCrab-authored material is licensed under the **GNU General Public License, version 3** (`GPL-3.0-only`); see [LICENSE](LICENSE). Upstream material retains its original copyright notices and applicable terms. See [NOTICE.md](NOTICE.md) and the [preserved muCommander license file](third_party/mucommander/license.txt).

Distribution of the Rust executable will include the corresponding buildable source, build instructions, the GPL text, and applicable notices. Additional dependencies and copied material must retain their own notices and be compatible with the application's distribution terms.
