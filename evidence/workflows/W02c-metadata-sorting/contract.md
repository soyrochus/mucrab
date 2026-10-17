# W02c contract — size, modified date and extension sorting

Status: frozen 2026-10-16 after original-only run
`20261016T141444Z-7c72ef61`: 38 sequences/698 snapshots, including four explicitly
pair-only extreme sequences. No Rust metadata implementation existed yet.
Reference: pinned muCommander 1.6.2-1, original comparator/model/table binaries.
First host: Linux x86_64/ext4, OpenJDK 25.0.4.1, en-US collation. Existing name
sorting and selection contracts remain regression gates.

## Recovered rules

- Criteria: NAME, SIZE, DATE (modified time), EXTENSION. Permissions, owner,
  group, remote filesystems and archive entries are separate future increments.
- Directories-first returns before any criterion or descending inversion.
  With this option on, folders precede ordinary files in either direction.
- SIZE treats directories as zero bytes. File size -1 (unknown) becomes
  Long.MAX_VALUE. Subtract the effective signed 64-bit values with Java wrapping
  arithmetic; do not replace it with unsigned or mathematical integer ordering.
- DATE subtracts signed 64-bit modified timestamps, also with Java wrapping.
  Equal primary values fall back to the recovered name comparator. Descending
  reverses that name tie-break as well as the primary criterion. Extreme
  synthetic arithmetic may violate comparator transitivity/antisymmetry; the
  oracle's pair signs remain the target. Original quicksort can fail to terminate
  on such data in descending mode; extreme arithmetic fixtures are explicitly
  pair-only and return null order/selection, never ordered-list coverage.
- EXTENSION uses original AbstractFile.getExtension: the substring after the
  final dot, unless it is the first character or final character, in which case
  extension is null. This also applies to directory names. `.config` and `x.`
  have none; `.config.toml` and `archive.tar.gz` have toml and gz respectively.
  StringUtils.compareIgnoreCase orders null before non-null, uses Java simple
  Unicode case comparison, and falls back to name comparison on equality.
- Pin the JDK 25.0.4.1 simple-case data in the native binary. Rust full Unicode
  case expansion/lowercasing and locale collation are not substitutes for this
  criterion. Valid Unicode scalar strings only; unpaired Java surrogates are
  excluded. Other JDK/Unicode data versions require new verification.
- Initial ordering is name/ascending/folders-first. Subsequent sorts start from
  the current permutation, preserve focused/marked entry identities, and reorder
  effective selection into current visible order. The parent stays outside the
  sorted child array and cannot be marked. Native panes retain separate criteria,
  direction and folders-first. The original natural/lexicographic preference is
  frozen per sequence. Mixed per-pane mode/refresh equivalence remains excluded.

## Metadata and oracle boundaries

Real fixtures execute original FileFactory enumeration and original metadata
getters. LocalFile.getSize/getDate delegate to java.io.File.length/lastModified:
follow symlink targets; broken links report size/date zero. Directory size is
recorded as returned but ignored by SIZE comparison. Timestamp units are signed
Unix milliseconds. Real fixtures include pre-epoch millisecond timestamps,
zero/tied sizes, dotfiles, multi-dot/trailing-dot names, Unicode and links.
The Rust enumerator is checked independently for every real entry's name,
classification, size, modified time and extension; enumeration order is aligned
only at the comparator boundary. No path, Unicode, case or time normalization.

Synthetic fixtures use separate real backing files/directories. A ProxyFile
subclass supplies literal name/size/date/extension fields from frozen inputs;
it does not implement comparison, table sorting or selection. This explicit
data-only layer tests negative/unknown/extreme metadata, null vs empty extension,
Unicode case variants and collation ties that cannot all be real filenames.
Synthetic results are not filesystem evidence.

The oracle executes original FileTableModel initialization/getFileComparator,
original FileComparator, original model quicksort and FileTable.sortTable,
original focus/mark methods and original selection retrieval. ReflectionFactory
bypasses full desktop construction as in W02a. No comparator/sorter/selection
method is overridden or recompiled. Full GUI/shortcut dispatch, file watching,
metadata races and failure during enumeration are outside this component suite.

The native UI exposes Name, Size, Modified and Extension in both panes, keeps
focus/marks through sort changes, and shows useful metadata for the selected
criterion. Native navigation, refresh and hidden filtering keep the criterion.
Refresh/navigation integration receives Rust/native checks; earlier component
suites retain their own boundaries. The Rust desktop remains Java-independent.

Sortable metadata has size -1 or a nonnegative signed 64-bit value, and modified
time within ±(i64::MAX / 2) milliseconds. This makes primary subtraction safe
for full sort transitions. Native enumeration rejects dates outside this bound;
negative non-sentinel sizes and extreme dates occur only in synthetic pair
tests. The pre-Rust candidate run stalled on generated-0-natural at descending
DATE/folders-first; preserved requests, thread stacks and reproduction input
document that original limitation. Fixture-generation bounds were revised
before contract freeze/Rust implementation, with the original candidate retained.

## Acceptance

Freeze fixture definitions and predefined/reserved seeds before Rust. Run the
original adapter first. Compare all pair signs, full original index permutations,
sort options and complete selection state after every transition. Require a
nonzero scoped run, exact fixture identities/metadata, all expected snapshots,
and independent real enumeration. Reject missing, reordered, malformed or extra
output. Do not assert generic sortedness to discard Java arithmetic edge cases.

Run predefined and reserved suites and all W01/W02/W02b regressions. Retain raw
commands/outputs and compact provenance, meaningful integration/protocol tests,
representative native/original GUI observations, and format/Clippy/build checks.
Other hosts, physical desktop/Wayland behavior and exhaustive GUI parity remain
unverified. Update the ledger and notices at the completed checkpoint.
