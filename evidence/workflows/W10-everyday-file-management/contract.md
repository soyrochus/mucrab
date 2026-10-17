# W10 frozen implementation boundary

The OpenSpec requirements and `fixtures/W10-everyday-file-management/contract.json`
freeze the five-stage scope and test seeds before native implementation.
Pinned recovery is recorded in recovery.json. Mkdir/Delete use original dialogs;
clipboard actions use ClipboardSupport and CopyJob; navigation uses the pane's
LocalLocationHistory (capacity100). QuickFindAction is a separate search-URL
command; type-to-find is QuickSearch. W10 implements the latter user workflow,
with explicitly narrower prefix matching rather than original substring/case
ranking. Original history cursor updates precede attempted navigation; W10 commits
only on success. These differences must not be normalized away as parity passes.

Adapter exploration initially considered xclip/wl-copy for publication. Native
probing showed that egui needs a plain-text clipboard offer as well as file URLs.
The implemented Linux X11 publisher uses the already locked x11rb 0.13.2 for both
formats (and GNOME copied-files); xclip imports URI lists and GIO supplies Trash.
Pure Wayland clipboard publication remains outside verified support. macOS uses
Foundation/AppKit through JXA and awaits native verification. url 2.5.8 is a new
pinned parser dependency; its IDNA/form-encoding graph and notices are preserved.

All filenames remain arguments/data, never script fragments. Missing services
fail visibly. Clipboard and Trash probes use isolated sessions and disposable
data directories. Mac native verification requires a Mac host. The fixture
category/seed contract preceded Rust implementation; the runnable generator and
exact GUI replay were added afterwards. No blind holdout or exhaustive original
UI parity claim is made.

Sources: GIO trash error/support behavior
https://docs.gtk.org/gio/method.File.trash.html and Foundation API
https://developer.apple.com/documentation/foundation/filemanager/trashitem(at:resultingitemurl:).
These describe platform APIs, not proof that adapters pass a native host check.
