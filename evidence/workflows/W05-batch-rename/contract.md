# W05 contract — bounded local batch rename

Frozen 2026-10-16 after original-only run
`W05-20261016T203943Z-ae8c407c`, before native batch implementation. The pinned
muCommander 1.6.2-1 JARs are the oracle. Recovery covers 19 classes including
BatchRenameDialog's actual token/parser/model methods, BatchRenameJob and its
MoveJob callers. Thirty named fixture sets plus 24 generated sets (seed 755051)
precede Rust; 32 reserved sets (seed 991075) stay withheld until implementation.
The initial run `W05-20261016T203924Z-c7a7eac7` is retained; the second uses the
original PathUtils helper for exact constructor-equivalent old-name keys.

## Inputs and preview behavior

Use a nonempty ordered effective selection of distinct direct children in one
existing local folder. Include regular files, folders and supported link entries
under W04's bounds. Default mask `[N].[E]`, empty literal search/replacement,
unchanged case, counter start/step strings `1`, and one counter digit.
Each option change regenerates the entire unblocked preview in selection order.
The initial preview and every subsequent configuration are measured.

- `[N]` takes the portion before the last dot and `[E]` after it, including for
  dotfiles and folder names. No dot means whole name/empty extension. A final
  period in the fully generated/replaced name is removed once before case change.
- N/E/P ranges use original one-based UTF-16 indices, negative positions from
  the end, `start-end`, open end (default 999), and `start,count`. Start zero
  means whole value. End/range bounds and signed integer overflow follow the
  recovered Java methods. A range that splits a surrogate pair is declined by
  native code rather than manufacturing an invalid filename.
- `[P]` and its ranges use the current parent folder's name. `[C]` uses the
  controls; `[Cstart,step,digits]` overrides them per token. Each counter token
  has its own state, resets on regeneration, formats without grouping, and
  advances once per selected row, including unchanged rows. Integers parse as
  signed 32-bit values with original defaults on invalid/overflow text; counter
  arithmetic wraps. Negative digit counts clamp to zero; native accepts explicit
  padding up to 12 digits and control widths 1–5.
- `[YMDhms]` emits each recognized character's modified-time component, including
  repeated characters, ignoring other characters within a date token. Date
  tokens are verified in UTC, years 1970–9999; other timezones/calendar eras are
  outside scope. Original default host timezone is explicitly set to UTC.
- Unknown bracket tokens are literal, empty `[]` disappears, `[[]` emits `[`,
  and an unmatched `[` copies the remaining text. No regex replacement: search
  is literal and replaces all occurrences; an empty search does nothing.
- Case modes are unchanged, lower, upper, first-character upper/rest lower, and
  first-character upper after each literal space. The last mode leaves other
  characters unchanged. Native case conversion is bounded to ASCII text;
  Unicode names remain supported with unchanged case. Unsupported conversion,
  empty first-character upper and split-surrogate results produce a native
  preview error without writes. They are outside accepted equivalence cases.
- Duplicate detection scans in row order. The first repeated target or target
  equal to another selected original name blocks Apply; scanning stops at that
  first conflict. No temporary names or cycle resolution. These flags match the
  original, even if an illegal leaf name would otherwise leave its button enabled.
  Native additionally disables application of empty/dot/path/unsafe leaf names
  and an entirely unchanged plan. These guards are not oracle parity claims.

## Apply behavior, integrity and UI

The original removes unchanged rows before constructing BatchRenameJob. The
ordered changed rows feed its newNames list; original processFile sets the name
for the current top-level index then invokes real MoveJob behavior. Fixtures
condition the inherited original collision field to a fixed apply-to-all SKIP
choice; ASK/interactive choices and lifecycle are separate GUI observations.
Native batch apply likewise skips existing files, merges folders under W04 rules,
continues after errors, and has no rollback. Earlier successful renames remain.
A merge may partially move children and retain the source parent. Source removal
and all W04 guards remain intact. Successful ordinary renames retain bytes,
metadata, links and inode identity. Unchanged rows have no job outcome.

Java and Rust run on separate equivalent owned fixture trees. Compare every
preview row, conflict/enable/count state, unchanged list, ordered job outcomes,
complete resulting tree (paths/kinds/bytes/SHA-256/modes/declared timestamps/link
text), root modes and logical source-before/target-after identity relations.
Retain raw device/inode IDs; unrelated raw IDs are not equated. Every selected
byte stream/link must survive at its original or mapped path. Blocked/preview-only
cases must leave the full tree unchanged. Source directory dates potentially
changed by removing children use W04's static input-derived mask; root/atime/
ctime/link-inode dates remain unmeasured. Permission lifts for independent
inspection occur outside execution and restore exact modes. Root cannot pass.

The native UI provides a Batch rename button, editable mask/search/replacement,
ASCII case modes and counter controls, live old→new preview, conflict/error
messages, explicit Apply and Cancel. It acts on the active pane's effective
selection. Apply executes the reviewed plan, with an additional pre-mutation
check of selected-entry identity/metadata and regenerated names; a stale plan
is declined. This check is an integrity guard, not race safety. Successful
changed rows unmark; unchanged/skipped/failed marks survive if their identities
still exist. Refresh both panes and preserve independent sort settings. Focus
must remain a valid visible row; exact full-original focus/OS dispatch parity is
not claimed. A basic worker does not implement W07 scheduling/cancellation.

Manual per-row editing, block/unblock state, removing rows from the preview,
range-selection helper dialogs, other locales/Unicode case conversion,
non-UTC date tokens, invalid UTF-16 filenames, excessive padding/resource limits,
remote/archive providers, races, ACLs/xattrs/other owners/special bits,
mid-stream faults and arbitrary collision prompts/overwrite choices are outside
this increment. Those original controls are recovered but are not presented as
implemented. Native guards are tested separately and never inflate equivalence.

## Original adapter and acceptance boundary

ReflectionFactory bypasses only BatchRenameDialog's top-level window constructor;
field-only wiring supplies real Swing widgets/collections, FileSet and old-name
keys, then executes unchanged original getTblNames/generateNewNames/parser/
checkForDuplicates/removeUnchangedFiles and BatchRenameJob methods. There are no
behavior overrides, transformed classes, rewritten Java parser or recompiled
upstream implementation. Full original dialog construction, live controls,
confirmation and jobs are observed separately in isolated Xvfb GUI runs.

Require nonzero complete ordered requests/previews/outcomes; reject missing,
reordered, malformed or inconsistent outputs as unavailable. Require original-only
before Rust, withheld reserved cases after implementation, zero unexplained
required differences, source/target integrity, native preview/apply/cancel and
partial-error demonstrations, original actual GUI observations, all earlier
predefined/reserved suites, meaningful tests, fingerprints, notices, documentation
and a committed handoff. Passing is scoped to stable owned unprivileged Linux
trees, en-US and UTC, not full muCommander batch-dialog parity.
