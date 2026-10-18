# W14 — Built-in viewer

**Mac checkpoint — 2026-10-18:** [workspace/comparison/native evidence](../../platforms/macos-arm64/W11-W15.md)
passes 68+52 text/image cases, exact Unicode/CRLF clipboard, find, image controls,
Light/Dark/narrow layouts, ZIP lease cleanup and retained TextEdit handoff.

Implemented and Linux-verified on 2026-10-17. F3 / File → View captures the focused
file or resolved file link, independently of marks. Enter/double-click retain
W09 default-application opening. Preview failures are visible and never launch
an external app automatically; Open externally is explicit.

Text supports strict UTF-8 with optional BOM and BOM-tagged UTF-16LE/BE. Exact
decoded characters and CR/LF/CRLF are retained for selection/copy; virtual rows,
Select All and literal case-sensitive next/previous wrapping find are available.
Limits:16MiB input,100000 lines and64KiB decoded UTF-8 bytes per line including
its terminator. Binary controls and malformed encodings are rejected.

PNG/JPEG support fit/100%, manual zoom0.05–16, drag pan, transparency checkerboard
and all eight EXIF orientations. Fit may scale below the manual zoom floor.
Limits:32MiB encoded,64MiB RGBA/sample buffers,16384 pixels per dimension and the
reported GPU limit. Decoder limits apply before pixel allocation, PNG chunks
are CRC-checked and JPEG entropy decoding is strict. ICC/color management and
other formats are outside scope. There is one current GPU texture.

A background worker reads in64KiB chunks, catches decoder panics, revalidates
captured sources and discards stale/abandoned results, including already queued
results. Replacement and close retain worker ownership until acknowledgment.
Bounded codec/OS calls must return before cancellation is acknowledged.

ZIP viewing uses W06 extraction guards with a private owned lease; close,
replacement and failure remove only that lease. Explicit external opening makes
an independent W09 retained copy. It survives viewer/app exit and never writes
back to the archive. Source/archive bytes stay unchanged.

## Evidence and scope

- [Recovery](recovery.json): ten pinned viewer/action/editor/preparation classes
  investigated with CFR/javap. Original InternalView can calculate folder size;
  native file View reports an error for folders and exposes totals in Properties.
- [Contract](contract.md), [freeze](fixture-freeze.json) and
  [decoder selection](decoder-selection.json): generator, recipes,40 encoded
  images, policy and allocation/license review preceded Rust changes. Existing
  image/zune/crc versions were retained; features/direct dependency edges changed.
  Exact notices are under [third_party](../../../third_party/viewer-codecs/README.md).
- [Predefined](comparison.json) and [reserved](comparison-reserved.json):68+52
  valid text/image outcomes with zero unexpected differences. Text calls pinned
  TextViewer.loadDocument and actual TextEditorImpl/RSyntaxTextArea. Theme-free
  test construction fills original editor fields without overriding methods.
  Images compare the JDK ImageIO boundary called by the recovered loader, plus
  independent Pillow oriented pixels; this is not the full original viewer.
  There are23 predefined/8 reserved declared differences: Swing normalizes line
  endings and ImageIO exposes uncorrected orientation. PNG pixels match exactly;
  maximum JPEG channel delta1 meets frozen tolerance2 (reserved PNG delta0).
- [Workspace validation](validation.json):101 Rust/51 Python tests; one unrelated
  W10 native-service test is ignored here. Tests include malformed encodings,
  long/file/line-count bounds, corrupt/truncated images, forged huge dimensions,
  GPU limits, EXIF1–8, changed sources, canceled generations, panic handling,
  exact virtualized selection, ZIP CRC/traversal and owned-lease cleanup.
- [Native](native.json): exact Unicode/CRLF clipboard, find-selected text,
  text/image Escape close, fit/100%/zoom/pan, light/dark/narrow layouts, ZIP viewer
  cleanup and an independently retained external copy. The screenshot names
  actual-text-dark/actual-image-dark/image-dark-narrow are the corrected dark
  checks; earlier attempted dark captures remain in the trace honestly.
- [Concurrency](concurrent-native.json): viewer loads while a real copy is paused;
  folder totals also complete without advancing its current path.
- [Regressions](regressions.json): fresh W07–W12 native checks on the initial
  candidate. Final queued-result abandonment is separately tested and followed
  by repeated W13/W14 native checks; historical receipts are not relabeled.

## Reproduce and remaining gates

```sh
python3 -B tools/w14_inspect.py
cargo build --workspace --all-features --locked --offline
python3 -B tools/w14.py
python3 -B tools/w14.py --reserved
python3 -B tools/w13_w14_validate.py
```

Native setup/replay/verification commands are shared with
[W13](../W13-properties-permissions/README.md#reproduce). Traces were recorded
after implementation; raw captures/logs are in the ignored run paths referenced
by receipts. Python original probes need the acquired reference JARs and JDK;
the native application requires neither.

Full original-window parity, alternate GPU limits/backends and
broader font/IME/accessibility coverage remain pending. The bundled Linux font
lacks some CJK glyphs; decoded strings and clipboard are exact despite missing
glyph boxes. No fallback font was downloaded. Filesystem race freedom, editing,
hex/regex/search replacement, streaming large text and extra image formats remain
outside this contract.
