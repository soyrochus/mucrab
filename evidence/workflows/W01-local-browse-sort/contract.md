# W01 contract — local browsing and name ordering

Status: frozen on 2026-10-15 and verified within the scope below. See `README.md`,
comparison reports, and GUI observations in this directory. Changes require an
explained contract revision and a comparison rerun.

## Inputs and scope

Reference: muCommander 1.6.2 / release 1.6.2-1; JAR and class pins are in the
acquisition/toolchain evidence and `mucrab-implementation-plan.md`.

Initial platform: macOS arm64, local filesystem. Locale is explicitly **en-US**,
independent of the host/UI language. Criterion is NAME. Test every combination
of ascending/descending, directories-first on/off, and NATURAL/LEXICOGRAPHIC.
Other locales and sort criteria are unsupported in this increment.

Real listing inputs are existing directories, with all entries shown, including
dotfiles and symbolic links. Directory classification follows links, matching
`LocalFile.isDirectory()` / `java.io.File.isDirectory()`. A broken link remains
an entry and is not a directory. Names must be valid Unicode; invalid native
filename encodings produce a visible unsupported-name error instead of silent
replacement. Opening symbolic links, filtering, special macOS hidden flags, archives, automatic refresh,
file opening, selection semantics, and all file mutations are deferred.

The real directory enumerator is `FileFactory.getFile(path).ls()`. Pairwise and
sorting comparisons consume its returned names and directory flags in the
same input order on both sides. Rust enumeration is checked separately by entry
identity/name and classification. Filesystem enumeration order itself is not
promised across hosts. No normalization of case or Unicode names is permitted.

Synthetic inputs use uniquely identified real backing files/directories, with
explicit label strings supplied through the comparator's `nameFunc`. This tests
duplicate, empty, and canonically equivalent labels without pretending they are
distinct physical entries on a case-insensitive/normalizing filesystem.

## Recovered ordering

Static sources are CFR 0.152 output and JDK `javap` against the original class:
`com.mucommander.commons.file.util.FileComparator`. Findings will be validated
with the original runtime, not a recompiled decompilation.

1. With directories-first enabled, directory/file comparison returns before
   applying descending inversion.
2. NATURAL considers only the first ASCII digit group. It requires identical
   prefixes and matching group positions, removes leading ASCII zeroes, compares
   remaining lengths then digits, and falls back on equal numeric values.
3. Fallback is ICU4J 78.3 locale collation at tertiary strength, then primary
   strength when tertiary returns zero. LEXICOGRAPHIC uses this fallback directly.
4. Descending reverses the name comparison, while directories-first is retained.
5. **The actual table uses a custom quicksort, not a stable sort.** The oracle
   invokes the original `FileTableModel.sort(FileComparator,int,int)` via
   reflection with its original index array. Rust must reproduce that index
   permutation, including ties, for the same input order. Do not invent an
   extra filename tie-breaker or assume stable sorting.

Call-site evidence from original `FileTableModel.getNameFunc()` shows local
folders use `AbstractFile::getName`; search folders use a different function and
are excluded. `getFileComparator()` reads the lexicographic preference, default
false. `FileTable` reads `filename_locale`; W01 sets it explicitly to en-US.
`SortInfo` defaults to name/ascending/folders-first. The model keeps the parent
entry outside the sorted child array. muCrab exposes parent navigation separately.

## Results and errors

Protocol: one JSON object per line, one output per input ID. Successful output
contains entry metadata, a full matrix of comparison signs (-1/0/1), and sorted
indices into the original entry array. Full-list equality requires equal index
permutations, not just that both arrays look sorted.

Missing paths, non-directory paths, and directories whose enumeration fails
produce `status: error`, `error: directory_unavailable` at the comparison
boundary. Native UI may show the more useful OS error. A failed navigation does
not change the pane's last successfully opened location or its contents.

The native workflow must open a path, enter a directory, return to its parent,
refresh, and change each sort option independently in both panes. Parent is a
separate control; regular files have no open action in W01. Empty folders and
errors must be visible. Show the fixed locale and deferred capabilities in UI.
The native pane stores canonical folder locations so relative/dot-component
input does not make Parent return to a lexical child. This is path resolution;
entry labels are not normalized. Link-navigation parity remains outside scope.

## Verification and limitations

Freeze fixture definitions and generator seeds before Rust implementation.
Run the original Java adapters first, then implement Rust. Reserve generated
cases for a separate post-implementation run. Compare every pair sign and
complete index list. Assert real enumeration sets and link classifications
separately; feed identical enumeration order to the comparator layer.

An access-denied fixture counts only if the current process is actually denied
access. Unexpected access, missing adapters, zero cases, partial output, invalid
JSON, and crashes cannot be reported as success. Preserve failing outputs.

ICU4X is a candidate for standalone Rust collation; equivalence is limited to
the tested locale/data/fixtures and must be established empirically. Runtime
comparison evidence and original/native GUI evidence are separate acceptance
requirements. This contract alone closes neither.
