# W08 — classic two-pane desktop UI

W08 is prioritized ahead of W07 by the user's instruction on 2026-10-17.
It changes presentation and action routing, retaining the W01–W06 core contracts.
The visual reference is the pinned muCommander 1.6.2 binary's previously captured
interface and recorded accessibility menu hierarchy. No upstream Java source or
new behavioral oracle is used. This is a visual/interaction increment, not a new
claim of complete muCommander feature or pixel parity.

## Acceptance

- Compact menu and icon toolbar above two file tables; selection status and a
  fixed file-operation command bar below. Branding/help move into Help dialogs.
- File, Mark, View, Go, Window and Help menus group the implemented actions in
  the reference's order. Unimplemented bookmarks, networking, delete, mkdir,
  viewing/editing, and history actions are not presented as working features.
- Each pane has an editable location, folder icon, parent and refresh controls.
  Enter opens a typed path; Enter in the table enters a regular folder or ZIP;
  Backspace returns to the parent. Existing navigation guards still apply.
- Drawn folder, file, ZIP, parent, link, and toolbar icons replace text prefixes
  and avoid dependence on platform emoji or optional fonts.
- Name, extension, size and modified UTC columns are simultaneously visible.
  Size is right aligned; header clicks sort and reverse; column borders resize
  adjacent columns. A draggable center divider changes the pane proportions;
  double-click or Window → Split equally resets it. Narrow panes scroll
  horizontally. Unsupported permissions/owner/group metadata stays deferred.
- Entire rows are clickable. Focus, marks, active/inactive panes and keyboard
  focus remain distinct; marking, focus identity and independent sorting follow
  the existing core. Listing rendering is virtualized for large folders.
- Exactly Light, Dark and System appearance modes, with System the default.
  The OS light/dark signal is handled live through eframe/egui. If unavailable,
  System uses Light. Both palettes share compact geometry, high-contrast text,
  modest borders and lightly rounded controls; no Java theme catalog.
- Save appearance to `$XDG_CONFIG_HOME/mucrab/appearance`, otherwise
  `$HOME/.config/mucrab/appearance` on Linux or
  `$HOME/Library/Application Support/mucrab/appearance` on macOS. Missing or
  unrecognized settings use System. Save failures appear in the status area.
- Menus, toolbar, command bar and F5/F6/F2 route to the existing reviews/jobs.
  F5 and its bottom command become extraction inside a ZIP. Invalid archive
  move/rename actions stay disabled. Review dialogs use consistent action rows;
  dismissal returns keyboard focus to the file list.
- Show selection counts and known selected file bytes, read-only archive state,
  job status and result details. Existing job/collision/error guards remain.

## Verification boundary

Native interaction and captures: Linux X11/Xvfb. Headless egui input tests cover
live reported OS-theme changes, explicit overrides, missing-signal fallback,
10,000-row virtualization, full-width row clicks, End/Home/Space, header sorting,
selection identity and focused-row scrolling. Native replay covers real folder
navigation, F5/F2/F6, batch rename, ZIP extraction, marks, header sorting, divider
and column dragging, 820×500 minimum size, Help, hidden files, palettes and saved
preferences. Its verifier checks exact actions, final trees/bytes and ZIP digest.

Native macOS system menu integration, physical OS theme notifications, Wayland,
other display scales and Windows remain unverified. Menus currently render
inside the window on all platforms. Pane/column widths and hidden-file state
are session settings, not persisted. Free-space reporting and extra file types
are deferred. W07's progress/cancellation lifecycle is unchanged.
