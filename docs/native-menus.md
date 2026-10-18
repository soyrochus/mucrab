# Native macOS menus

On macOS, muCrab uses AppKit's system menu bar through the target-specific muda
0.21.1 dependency. Linux retains the existing in-window egui menu. The toolbar,
file operations, review dialogs and core remain shared.

`crates/desktop/src/native_menu.rs` owns the native menu objects on the UI thread.
The menu event callback only queues a menu ID and wakes eframe. The UI revalidates
availability before dispatching one command per frame through the existing app
commands. Enabled states and checkmarks follow the active pane and modal state;
bookmark items refresh by stable bookmark ID after additions, edits or reloads.

The application menu provides About, Services, Hide and Quit; File, Edit, Mark,
View, Go, Window and Help expose the existing workflows. Edit routes clipboard
commands to egui text/viewer controls or the existing file-clipboard adapter based
on focus. Listing shortcuts are registered only while a listing has focus, so
text controls keep their editing shortcuts. Quit and Close window use the same
worker cancellation and acknowledgment path as the window close button.

Verification uses isolated preferences and disposable folders, with the host OS
appearance unchanged. Mac desktop tests cover focus routing, modal/tab limits,
checked settings and deferred Quit. The Linux desktop tests compile and exercise
the retained menu path. Native Quit was also verified from a paused 100,000-file copy: the app exited
cleanly, partial destination files stopped changing and every source retained its
exact contents. Native acceptance results and capture hashes are recorded
in [the Mac menu evidence](../evidence/platforms/macos-arm64/native-menu.json).

During native acceptance, the existing macOS file-clipboard service returned an
OS service failure when invoked from Edit → Copy. Text Cut/Copy/Paste worked, as
did File → Copy to the other pane. No file-clipboard adapter changes are included
in this integration.
