# muCrab

**Two claws, two panes.**

muCrab is an experimental Rust port of selected [muCommander](https://github.com/mucommander/mucommander) file-management workflows, reconstructed from its published Java binaries. The intended application name is **muCrab** and its command name is **`mucrab`**.

The experiment will use [REA](https://github.com/morluto/rea), a JVM decompiler, and observations of the original application to recover behavior, implement it in Rust, and compare the two implementations.

## Current status

The repository baseline, reproducible binary acquisition, static analysis tool setup, and Cargo workspace are complete. The native **muCrab** window builds and opens with an empty two-pane preview. File browsing, file operations, sorting, and Java/Rust comparison adapters are not implemented yet; no behavioral equivalence has been established. The downloaded Java reference and the pinned REA/CFR tools remain available for the first migration increment.

## Build and run the Rust scaffold

Requirements: Rust **1.95** or newer, Cargo, and a native desktop session for the GUI. The scaffold was built and launched on **macOS arm64** with Rust/Cargo **1.99.0**. Other platforms have not been verified.

The [Cargo workspace](Cargo.toml) contains:

| Package | Role | Current contents |
| --- | --- | --- |
| `mucrab-core` | Framework-independent file-management behavior | Pane identity; recovered operations will follow |
| `mucrab-desktop` | Native `mucrab` executable | Two-pane window using eframe/egui 0.36.2 with the OpenGL backend |
| `mucrab-compare` | Java/Rust comparison harness | Entry point that reports `not_ready` and zero cases run |

All three packages use `GPL-3.0-only` and are marked non-publishable. [Cargo.lock](Cargo.lock) pins the resolved dependencies; it is committed. Build and launch from the repository root:

```sh
cargo build --workspace --locked
cargo run --locked -p mucrab-desktop
```

The preview does not read or modify folders, and starting it does not require Java, Node, REA, or the reference JARs. Check the native command-line entry points without opening a window:

```sh
cargo run --locked -p mucrab-desktop -- --version
cargo run --locked -p mucrab-compare -- --status
```

The comparison status command deliberately exits with **code 2** while its adapters are unavailable. Its JSON reports `status: "not_ready"` and `cases_run: 0`; this is not a passing equivalence result.

Workspace checks:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
```

The current Rust suite has one integration guard proving that an unavailable comparison cannot be reported as a successful run. There are no ported-behavior tests yet. Once dependencies are cached, the Cargo build, test, and Clippy commands also work with `--offline`.

Capture the native preview and exit automatically (verified on macOS):

```sh
mkdir -p artifacts/workspace
EFRAME_SCREENSHOT_TO="$PWD/artifacts/workspace/mucrab-preview.png" \
  cargo run --locked -p mucrab-desktop --features screenshot
```

The optional `screenshot` feature is for development captures. [Workspace readiness evidence](evidence/workspace/readiness.json) records the compiler, source and lockfile digests, verified commands, expected harness refusal, and captured window. Native rendering establishes scaffold readiness only.

## Reference application

The reference is muCommander **1.6.2**, release tag **`1.6.2-1`**:

- [Official release](https://github.com/mucommander/mucommander/releases/tag/1.6.2-1).
- [Binary distribution](https://github.com/mucommander/mucommander/releases/download/1.6.2-1/mucommander-1.6.2-1-portable-x86_64.zip).
- Published archive SHA-256: `e6769537bbb56cc52c1289ebef2e3d051912bb8b10691d320f28b945f15819a4`.

The input is the complete distribution, including the launcher, application JARs, dependencies, resources, and notices. This portable archive includes a Windows executable launcher and a shell launcher, with no bundled JRE. A host JDK is installed and works for the static analysis tools; application startup and compatibility of the bundled native components remain unverified. Acquisition preserves the distribution layout and does not execute its contents.

## Acquire or verify the reference

Requirements: Python 3.9 or newer, `curl`, and network access for the first download. From the repository root:

```sh
python3 -B tools/acquire_mucommander.py
```

The [pinned manifest](tools/manifests/mucommander.json) supplies the official release URL, archive size, and SHA-256. The script verifies the archive before extraction, checks all extracted file contents, and confirms the launcher version, implementation revision, and preserved license notice. Repeating the command verifies the cached archive and distribution before reusing them. A mismatch stops acquisition and reports the affected input.

- Download: `artifacts/downloads/mucommander-1.6.2-1-portable-x86_64.zip`.
- Extracted reference: `artifacts/reference/mucommander-1.6.2-1/`.
- Launcher JAR: `artifacts/reference/mucommander-1.6.2-1/mucommander-1.6.2.jar`.
- Core JAR: `artifacts/reference/mucommander-1.6.2-1/app/mucommander-core-1.6.2.jar`.
- Full file inventory: `artifacts/inventories/mucommander-1.6.2-1.json` (ignored).
- [Acquisition evidence](evidence/acquisition/mucommander-1.6.2-1.json) (tracked).

The acquired distribution contains **202 files and 196 JARs**. The compact evidence records the input digest, distribution counts, and selected file digests; the local inventory contains the digest of every extracted file. These records establish acquisition and byte integrity only.

Verify the local reference and its evidence without network access or writes:

```sh
python3 -B tools/acquire_mucommander.py --verify-only
```

Check acquisition failure handling and extraction boundaries:

```sh
python3 -B -m unittest discover -s tools -p 'test_*.py' -v
```

Keep the extracted tree unchanged as the binary reference. Later runtime tests will use a separate working copy and disposable configuration.

## Analysis tools

The [analysis-tool manifest](tools/manifests/analysis-tools.json) pins **REA 6.5.0** and **CFR 0.152**. REA's dependencies are pinned in [the npm lockfile](tools/rea/package-lock.json). The current probe ran on macOS arm64 with Node **24.12.0**, npm **11.7.0**, and OpenJDK / `javap` **21.0.10**. Setup requires a Node version supported by REA and a full JDK 17 or newer; it uses the installed runtimes rather than installing them.

Install the tools locally, with npm lifecycle scripts disabled:

```sh
python3 -B tools/analysis_tools.py setup
```

Repeat installation from the populated local cache without network access:

```sh
python3 -B tools/analysis_tools.py setup --offline
```

Run the static capability probe against the verified muCommander reference:

```sh
python3 -B tools/analysis_tools.py probe
```

The probe does not download tools, execute muCommander classes, or modify global agent configuration. It writes raw command arguments, stdout, stderr, bytecode, and decompiler output under ignored `artifacts/analysis/toolchain/`. Its compact [readiness evidence](evidence/toolchain/readiness.json) is tracked in Git and records each producer, version, command, output digest, and limitation.

| Producer | Observed capability | Boundary |
| --- | --- | --- |
| REA `inspect-artifact` | Inventoried the file-library JAR and verified the selected class fingerprint | Coverage concerns ZIP entries; the class format is reported as `unknown`, with no JVM method semantics |
| REA `inspect-android-package` | Explicitly rejected the JAR because it requires a standalone APK | This is an expected format refusal, not a successful Java decompilation |
| CFR | Decompiled `FileComparator` into 163 lines of Java with its dependencies resolved | One static smoke test; output correctness is not established |
| JDK `javap` | Returned method signatures and bytecode instructions for the same class | Disassembly supports investigation; it does not establish equivalence |

The selected class was previously examined during the feasibility investigation. This repeatable probe establishes tool readiness and must not be presented as a blind reconstruction or a new behavioral discovery. The input class has JVM class-file major version **55**.

REA is installed under ignored `tools/rea/node_modules/`; CFR is stored under ignored `tools/.cache/cfr/`. The npm lockfile carries REA's registry integrity value. CFR's SHA-256 was computed from the official author download and pinned locally; the author's published MD5 also matches. That SHA-256 is a reproducibility pin, not a publisher-supplied signature. Preserved tool licenses and provenance are listed in [NOTICE.md](NOTICE.md).

No JADX, Hopper, Ghidra, or other native analysis engine is needed for these probes. Native, browser, and instrumentation providers are outside this step's verified capabilities. GUI startup, Java behavior, and Rust equivalence remain to be tested.

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

The [implementation plan and progress ledger](mucrab-implementation-plan.md) records the current checkpoint, machine handoff instructions, workflow gates, and evidence requirements. Update it during implementation so work can resume in another session or checkout.

1. **Complete:** establish and commit this repository baseline.
2. **Complete:** add reproducible acquisition scripts and a checksum manifest; download and extract the reference into ignored local storage.
3. **Complete for static inspection:** pin REA and a JVM decompiler, verify JAR inventory, the APK-only refusal, class decompilation, and bytecode inspection against the reference, and record tool provenance and limits.
4. **Complete:** create a GPLv3 Cargo workspace separating the Rust core, native desktop interface, and comparison-harness entry point; commit `Cargo.lock`, verify the build and unavailable-harness guard, and capture the native preview.
5. Recover and implement one workflow at a time, beginning with file sorting. Compare each increment against the original binary before expanding the scope.
6. Run the agreed comparison suite and demonstrate fresh scenarios with the compiled Rust application running independently of Java.

Upstream Java implementation source will be withheld during reconstruction; license and attribution material remains available. Decompiled representations, binary metadata, and observations of the original are permitted inputs. Findings from the earlier feasibility investigation must be identified as prior evidence if reused.

## Verification approach

Each workflow will have explicit inputs and acceptance criteria established before its Rust implementation. A harness will exercise the original JARs and the Rust implementation against equivalent fixtures, comparing observable results and recording discrepancies. Any normalization of paths, timestamps, or platform details must be stated explicitly.

The first implementation milestone is one real behavior recovered from the pinned JARs, implemented in Rust, and passing comparisons against the original. The broader demo aims for approximately 100 predefined cases plus withheld cases, with all critical file-integrity cases required to pass. These are acceptance targets, not completed checks.

Fixtures will include numeric and Unicode names, existing destinations, symbolic links, unreadable files, and cancellation. File operations will run inside disposable fixture directories. GUI demonstrations supplement the behavioral comparisons. Build success alone does not establish equivalence.

The experiment will distinguish evidence that binary-to-Rust reconstruction works from evidence that REA improves the process. Tool provenance, human interventions, and remaining gaps will be recorded with the results.

## Repository layout

- `artifacts/`: ignored downloads, extracted distributions, and generated decompiler output.
- `tools/`: reproducible acquisition, inspection, and comparison helpers.
- `crates/`: Rust application and comparison components.
- `fixtures/`: reproducible test inputs.
- `evidence/`: selected provenance records and comparison results suitable for Git.
- `third_party/`: preserved upstream notices and their provenance.

The baseline, acquisition and analysis helpers, pinned tool manifests, Rust workspace, dependency lockfiles, helper/readiness checks, and acquisition/toolchain/workspace evidence exist at this stage. Behavior adapters, comparison fixtures, and migrated workflows will be added in later steps.

## License and attribution

muCrab-authored material is licensed under the **GNU General Public License, version 3** (`GPL-3.0-only`); see [LICENSE](LICENSE). Upstream material retains its original copyright notices and applicable terms. See [NOTICE.md](NOTICE.md) and the [preserved muCommander license file](third_party/mucommander/license.txt).

Distribution of the Rust executable will include the corresponding buildable source, build instructions, the GPL text, and applicable notices. Additional dependencies and copied material must retain their own notices and be compatible with the application's distribution terms.
