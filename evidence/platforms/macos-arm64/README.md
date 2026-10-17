# macOS arm64 verification — 2026-10-17

This extends the bounded Linux verification to **macOS 26.6.2 / arm64 / APFS**,
as an unprivileged user, with Rust/Cargo **1.99.0**, OpenJDK **21.0.10**,
Python **3.14.5**, Node **24.12.0** and npm **11.7.0**. Linux receipts remain
unchanged under `evidence/workflows/`; this directory owns the fresh Mac results.

## Automated acceptance

[Validation](validation.json) records commands, exit codes, log hashes, source
hashes and distinct filesystem device IDs. Build, formatting, Clippy with denied
warnings, **47 Rust tests**, **51 Python tests**, CLI version/help, the intentional
zero-case readiness exit 2, reference integrity and the static analysis probe
pass. [Toolchain evidence](toolchain/readiness.json) records the Mac producers.

All **18** predefined/reserved original-JAR/Rust executions pass with zero
behavioral differences and no omitted cases:

| Workflow | Cases / sequences across both runs |
| --- | ---: |
| W01 browsing and name sorting | 342 |
| W02a selection and marking | 60 |
| W02b hidden-file filtering | 56 |
| W02c metadata sorting | 74 |
| W03 recursive copy | 242 |
| W04 move/focused rename | 479, including 216 real cross-device cases |
| W05 batch rename | 86 |
| W06 ZIP browsing/extraction | 259 |
| W07 real full-job lifecycle/cancellation | 30 |

The individual receipts are under [workflows](workflows/). Counts include
synthetic/component combinations; they are not distinct end-to-end user journeys.
W07 executes the unchanged original application from a verified disposable copy
with an isolated preference profile. Its macOS launcher includes the required
JDK exports for `com.apple.eawt`, `com.apple.laf` and `com.apple.eio`.

W04/W07 and the Rust move tests use a mounted disposable APFS disk image as the
second filesystem. The normal Mac `/tmp` and checkout share a device. Neither
EXDEV nor permission errors are mocked; access-denied fixtures run without root.

## Native desktop acceptance

[Native receipt](native.json) records observed `cua_repl` mouse/keyboard input,
accessibility states, screenshot hashes and the tested executable/source hashes.
The Mac action sequence is recorded separately from the Linux X11 pixel replay.

W08 covers Enter/Backspace navigation, reviewed left-to-right copy, focused F2
rename in the right pane, F6 move back, live batch preview/apply, ZIP navigation
and selected extraction, hidden-file visibility, marking/sorting, pane resizing,
the minimum 820×500 content area, Help, and Light/Dark/System preference changes.
The shared filesystem validator checks both final trees, all expected bytes,
the unchanged ZIP, preserved symlink and saved `system` preference.

W07 covers real Pause/Resume/Cancel, rejection of a second mutation, browsing and
marking another location while paused, retention of unrelated marks after
acknowledgement, and closing the window during a paused job. The first pause was
observed twice with an unchanged 2,741 children / 57,561 bytes. After three
cancelled jobs, 9,387 children remain, including one valid partial child; all
20,000 source files and later/unrelated sources are unchanged. Earlier completed
work remains and later roots were not started. The fixture session closed
without driver termination and the LaunchServices wait completed successfully.

Captures include [Dark](gui/w08-dark.png), [Light](gui/w08-light.png),
[minimum window](gui/w08-minimum-window.png), [batch preview](gui/w08-batch-preview.png),
[paused copy](gui/w07-paused.png), [browsing while paused](gui/w07-browsing-while-paused.png)
and [close while paused](gui/w07-close-paused.png).

## Portability fixes established by this run

- Mac directory enumeration now exposes NFC names before extension/sorting rules,
  matching the original listing. ICU4X `icu_normalizer` 2.2.0 was already locked;
  the only dependency change adds its direct macOS core dependency edge.
- Case-only rename works on case-insensitive APFS. Distinct hard-link aliases
  remain rejected. The regression checks the actual directory spelling because
  the old spelling still resolves on APFS.
- Parallel test fixtures use a process-local atomic counter as well as time;
  macOS timestamps alone produced duplicate temporary directories.
- Release notice ordering uses relative POSIX path strings, giving stable
  inventories on the Mac Python runtime. Existing notice tests cover the result.
- Component tools accept unprivileged Mac runs and retain platform-specific
  receipts. The release workflow now runs the complete Mac Rust/Python suite
  with a real mounted APFS test filesystem; hosted execution remains pending.

APFS resolves canonically equivalent filename spellings. W03/W04/W05/W06 compare
filesystem path identities in NFC on Mac, retain raw observations and reject
canonical collisions. File bytes, links and declared metadata remain checked.
ZIP entry names stay exact; W06 retains its existing whole-second DOS-date rule.
W05 supplies selected names in the NFD form exposed by the original direct-child
adapter; synthetic comparator inputs remain unchanged. These normalizations do
not establish parity for arbitrary filesystems or other Unicode/locale inputs.

## Repeat

Use Python **3.12+**; `/usr/bin/python3` on this host is 3.9 and cannot run the
release tests. The verified interpreter is `/opt/homebrew/bin/python3`.

```sh
mkdir -p artifacts/macos-verification/cross-device
hdiutil create -size 768m -fs APFS -volname muCrabVerify -type SPARSE \
  -o artifacts/macos-verification/cross-device.sparseimage
hdiutil attach artifacts/macos-verification/cross-device.sparseimage -nobrowse \
  -mountpoint "$PWD/artifacts/macos-verification/cross-device"
/opt/homebrew/bin/python3 -B tools/verify_macos.py \
  --cross-device-root artifacts/macos-verification/cross-device \
  --allow-reference-gui
hdiutil detach "$PWD/artifacts/macos-verification/cross-device"
```

If the image already exists, attach it rather than recreating it. New runs use
unique raw directories. The command fails if any check fails or a genuine second
filesystem is missing. GUI fixture preparation refuses to overwrite old runs;
repeat native input against fresh `w07_gui`/`w08_gui` fixture directories and
retain the observations rather than substituting the Linux action log.

## Remaining boundaries

The live macOS Appearance change test awaits user permission to temporarily
switch the OS preference and restore it. Current System rendering and injected
live theme changes pass; actual OS notification delivery is not yet claimed.
Menus render in-window; native macOS menu integration is not implemented.
Other macOS versions, Wayland, Windows, other locales, arbitrary owners/ACLs,
xattrs/resource forks and exhaustive shortcut parity remain outside this scope.
The minimum deployment target is not a test of older macOS releases. Ad-hoc
signing is separate from Developer ID signing/notarization, and no release is
published by these local checks.
