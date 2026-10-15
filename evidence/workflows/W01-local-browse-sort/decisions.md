# W01 decisions and discrepancies

- **No Java/Rust differences remain in the committed suites.** The first
  predefined run passed, followed by the reserved generated run. Later runs
  refresh provenance after helper/report changes; raw runs retain unique IDs.
- **A planning assumption changed before implementation:** the original GUI
  table uses a custom quicksort, so preserving input order for equal names
  would not reproduce it. The adapter invokes that original private method,
  and Rust reproduces its index permutation using an explicit stack.
- **Java adapter initialization:** direct local-file/comparator invocation
  works through `FileFactory`. Constructing the original table model initially
  failed because translation resources had not been initialized. Initializing
  the original `Translator` with its bundled dictionary resolves that dependency.
  Adapter compilation uses `-proc:none`; original classes are loaded from JARs.
- **Collation choice:** pin ICU4X `icu_collator` 2.2.1 and compiled collation data
  2.2.0, rather than rely on host ICU or ordinary string/natural sorting. The
  evidence is empirical for en-US and these fixtures. Other locales/data
  versions require new comparisons.
- **GUI execution:** a restricted launch of the original aborted during native
  startup. Desktop access and a local JDK `jpackage` wrapper allowed inspection
  from an isolated runtime/profile. These wrappers are ignored development
  artifacts, not distributed Java or muCrab packages.
- **GUI automation:** the Mac was initially locked and the user unlocked it.
  Instantaneous or indexed actions could leave state unchanged. Sustained clicks
  and screenshot coordinates established representative original navigation;
  native egui controls worked with sustained indexed clicks. Failed input
  attempts were not counted as successful observations. Original right-pane
  navigation and exhaustive original GUI control automation are not claimed.
- **Host/filesystem boundary:** APFS case-insensitive and canonically equivalent
  lookup was observed on this Mac. Duplicate/case-equivalent/empty names therefore
  use an explicit synthetic label layer, not misleading physical-file fixtures.
- **Tool contribution:** REA supplied the earlier container inventory and class
  fingerprint. This increment uses CFR and `javap` for JVM recovery, original
  Java method execution for the oracle, and agent-authored Rust for the port.
  It does not demonstrate an automatic REA JAR-to-Rust compiler.
- **Scope choices:** all entries are shown; link classification follows the
  original, while opening links is deferred. Parent navigation is a separate
  control. Selection restoration, filtering, file mutations, archives, alternate
  locales/platforms, and large-directory responsiveness are not W01 claims.
- **Navigation review:** canonicalize successful folder inputs before storing
  them. Otherwise Parent after `child/..` could return to the lexical child
  instead of the actual parent. The navigation regression covers this case;
  entry-label comparison and original sorting remain unchanged.
