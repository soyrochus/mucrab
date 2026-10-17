# File opening

- Enter and File → Open activate the focused item, independently of marks. Double-click activates the clicked item. Enter auto-repeat must not dispatch additional requests.
- Parent/folder/ZIP navigation remains available. Local links resolve to their target; missing or cyclic targets report errors.
- Linux dispatches an absolute filename as one argument to `xdg-open`; macOS uses `/usr/bin/open`. No shell command is constructed from filenames.
- Preparing and launching must not block the UI or replace W07 mutation progress/results. Preparation can be cancelled; already dispatched applications remain independent.
- Archive member extraction reuses guarded W06 extraction, including CRC, traversal, link and resource-limit checks. No launch occurs after an extraction error or cancellation.
- Each extracted member gets its own private directory and original leaf name, with no executable permissions. Successful copies survive muCrab exit; the OS temporary-directory policy or explicit user cleanup determines later removal. No ZIP writeback occurs.
- Launcher creation and nonzero exit errors must be visible. A successful dispatch means only that the request was sent, not proof that a document appeared.
- Window close waits for cancelled preparation to acknowledge completion, without waiting for external applications. At most one preparation and 64 outstanding launcher processes are accepted.
