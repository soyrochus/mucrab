# W06 recovery decisions and discrepancy history

- Original-only runs `W06-20261017T091304Z-5c35ab2b` and
  `W06-20261017T091448Z-5e11ab58` precede native ZIP code. Twenty-five named,
  eighteen generated and twenty-four withheld reserved sets and their seeds
  were fixed before implementation. The first native predefined and reserved
  comparisons passed without changing accepted expected outcomes.
- Original ZipEntry.dosToJavaTime mutates a shared Calendar's year/month/day/
  hour/minute/second fields without clearing milliseconds. The raw observations
  show a common clock-dependent fraction. The frozen contract compares dates at
  whole-second precision, preserves raw observations and excludes same-second
  conditional collision boundaries. Native uses deterministic UTC DOS dates.
- ArchiveEntryTree uses first-encounter order, synthesizes parent folders from
  first-descendant metadata, and replaces later explicit-folder metadata. No
  sorted tree library or host enumeration order substitutes for these rules.
- UnpackJob aborts an archive after an actual transfer error, while ordinary
  collision skips continue. W03's recursive-copy continuation is not reused.
  Source ZIP hashes and complete independent destination trees are measured.
- The original ZIP provider uses bounded stored/Inflater streams without a
  corresponding CRC check on this method path. Native CRC/size checks and strict
  path/link/duplicate/resource guards are extra integrity measures; corrupt or
  unsafe cases are not counted as equivalent original behavior.
- Rust zip 9.0.4 supplies container and stream decoding only. Its filesystem
  extraction convenience API is not used. Existing external package pins remain
  unchanged. Source/notice packaging includes all 387 locked external crates.
- An exploratory native replay at `W06-20261017T092714Z-29b11272` established
  review coordinates and verified final/partial trees. The frozen replay removes
  an initial off-target checkbox click and focuses a listing row after All
  before F5. Keyboard actions require listing focus. Final replay uses fresh
  trees and the final executable hash; exploratory captures are not substituted.
- Original GUI harness attempts are retained separately. Run
  `W06-20261017T093627Z-8d98cb65` stalled in raw screen capture with inherited
  Wayland environment; the helper now removes WAYLAND_DISPLAY and sets an X11
  session. Run `W06-20261017T093904Z-fbff93be` used String navigation, which can
  open the original archive-browsing confirmation path. The probe now uses the
  actual resolved archive object and original folder-change worker, as folder
  activation does. Run `W06-20261017T094147Z-a97d75a8` reached the real ZIP root
  before an early hidden-menu lookup failed; startup now waits for menu readiness.
  Run `W06-20261017T094322Z-87ffc725` completed selected extraction and opened the
  real whole-archive dialog, but its inspector tried to decode an existing binary
  collision fixture as UTF-8. It now compares raw bytes while waiting. None of
  these harness failures changed the oracle, fixtures or native ZIP rules.
  The corrected original run `W06-20261017T094547Z-9527943c` passes all twelve
  stages and three actual jobs. Native run
  `W06-20261017T094505Z-9401b9f8` passed after the command-line help update;
  the final corrected binary is identified in GUI observations and validation.
- The first local corresponding-source build was launched with --manifest-path
  from the outer repository, so Cargo did not discover the source archive's
  relative .cargo/config.toml. The retained log is `artifacts/w06-release-build.log`.
  Building from the extracted source directory with an empty Cargo cache and
  --frozen succeeds, as the documented release workflow does. Later help text
  updates require a fresh source export and final package smoke test.
- Optional original logging/bundle-resolution and shutdown warnings remain
  outside the workflow claims. Source binaries/JARs are preserved; reference GUI
  programs run only from verified copies with disposable homes and preferences.
- Final review found that Apply opened the reviewed archive before checking its
  file type again. A replacement FIFO could block that open. Apply now checks
  regular-file metadata and the review snapshot before opening, then checks the
  opened descriptor again. The existing replacement guard test includes a FIFO
  created with the Unix `mkfifo` utility. This prevents a static replacement from
  blocking; it does not establish race-free concurrent replacement handling.
  All component suites, native GUI replays and packaging checks were rerun on
  this correction. The earlier successful native run remains an intermediate
  record; GUI observations identify the final executable and run.
