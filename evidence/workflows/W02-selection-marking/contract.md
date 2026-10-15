# W02a contract — focus and marking in a loaded directory

Frozen 2026-10-15 before Rust implementation. This bounded increment covers
selection and marking; filtering and additional sort criteria remain separate.
Reference: the pinned muCommander 1.6.2-1 JARs. Locale: en-US. Unicode local
names, name sorting, directories, files, and links retain W01's scope.

## Recovered rules

- There is one focused row, distinct from the marked set. A parent row `..`,
  when present, is row zero, remains outside sorting, and cannot be marked.
- `getSelectedFiles()` returns marked files in current visible order. With no
  marks, it returns the focused child alone; focused parent or no rows returns
  an empty set. Focusing another row never clears marks.
- Plain click changes focus. Ctrl-click changes focus and toggles that child.
  Shift-click toggles an inclusive range from previous focus to clicked row,
  using the **clicked row's** previous marked state. Shift takes precedence
  over Ctrl. This is a normalized post-Swing selection event: first call the
  original `changeSelection(row,0,false,false)`, then its `mousePressed` handler.
  This keeps Swing's platform menu-shortcut toggle separate from file marks. Full platform
  mouse dispatch is a separate GUI observation, not proved by this adapter.
- Up/Down clamp at the ends; Home/End select first/last. Shift+Up/Down toggles
  the departing row then moves focus one row, using original marking actions.
- Space toggles the current child and advances (parent just advances). At the
  final row it toggles once; further held-key actions are ignored until release.
  Release clears both original repetition flags. A new press can toggle again.
- Mark all, unmark all, and invert affect every child, including directories,
  links and hidden entries. Parent is always excluded; repeated set is idempotent.
- Sorting mutates the current index permutation, preserves focused identity,
  and preserves marks by underlying entry identity. Marked/selected output
  follows the resulting visible order. W01's original quicksort also applies
  on subsequent sorts, including collation ties.
- State belongs to each pane independently. Native Tab switches the active
  pane; shortcuts apply only while a pane's listing has keyboard focus.

## Oracle boundary

`tools/java/W02Oracle.java` calls original `FileTableModel` initialization,
marking, retrieval and sorting, original `FileTable` focus/mouse/Space/selection
methods, and original action `performAction` methods. A serialization constructor
initializes the JTable superclass while bypassing FileTable's full desktop
wiring. MainFrame is a field-only fixture with `activeTable` set, and action
constructors are bypassed while retaining AbstractAction initialization. No
behavioral method is overridden or replaced. ReflectionFactory is JDK-specific;
unsupported JDKs fail the suite rather than skip. This isolates binary behavior;
it does not verify launch wiring, OS shortcut registration, repainting or a LAF.

Adapter key release directly resets the two fields found in original
`keyReleased`; shortcut matching/quick-search guards are outside scope. Initial
focus is explicit fixture input. Invalid rows are rejected at the boundary.
Every valid transition returns visible names, focus row/name, per-row marks,
marked count, ordered marked identities, and effective selected identities.
Missing/extra/reordered transitions and adapter errors cannot count as passes.

## Native integration and limits

Expose focus and marks distinctly, with marked count and All/None/Invert controls.
Implement the actions above in both panes, preserving W01 folder browsing and
sorting. Show an unmarkable parent row in addition to the existing Parent button.
File opening/mutations, middle-button marking/dragging, Page/Block navigation,
quick search, delayed rename, and configurable keymaps remain deferred.

Refresh/navigation restoration is recovered statically from `setCurrentFolder`
and `FolderChangeThread`: refresh preserves surviving focused/marked names;
return to parent focuses the departed folder; another folder starts at row zero
with no marks. Failed navigation keeps state. These integration rules receive
Rust tests and representative GUI checks, not the headless transition suite's
equivalence claim. Filesystem identity remains within W01's canonical local
path scope; external rename/automatic refresh/races are not covered.

Freeze predefined cases and reserved generator seeds before implementation.
Run Java first, compare full snapshots after **every** event, then run reserved
sequences. Keep W01's 342 comparisons passing. Report static, runtime and native
GUI evidence separately; do not claim whole-application equivalence.
