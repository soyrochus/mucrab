# W16 tasks

All implementation tasks are pending. Specs: `desktop-visual-system` and `desktop-workspace-presentation`. W17 owns browse lifecycle and structural refactoring; agree its component/loading interfaces before editing overlapping desktop modules. Token work and baseline capture can proceed independently.

## 1. Baseline and visual contract

- [ ] 1.1 Capture the current implemented workspace and representative dialogs using disposable fixtures; record Linux/macOS host, backend, fonts, dimensions and scale without substituting historical W08 screenshots.
- [ ] 1.2 Inventory existing controls, shortcuts, accessible names, enabled checks and feature states; record the W09–W15 behaviour that the visual pass must preserve.
- [ ] 1.3 Finalise the role palette and metrics from the design; measure normal/selected/hovered text contrast and essential focus/state indicators in both themes.
- [ ] 1.4 Evaluate existing typography against the reference captures; retain the existing fonts or record verified source/licence/notices and fallback coverage before bundling a new face.

## 2. Shared appearance and controls

- [ ] 2.1 Implement semantic colour tokens and geometry/text metrics in desktop-owned modules; migrate theme application for both explicit appearances and live System changes.
- [ ] 2.2 Implement reusable quiet action buttons and standard control states, retaining labels, tooltips, keyboard focus/activation, screen-reader names and command availability checks.
- [ ] 2.3 Unify code-drawn toolbar/navigation/file icons around shared optical bounds, strokes and scale-aware placement; inspect enabled and disabled rendering.
- [ ] 2.4 Apply typography roles to file names, metadata, section labels, active tabs and status text; verify numeric alignment and long Unicode fallback text.

## 3. Workspace composition

- [ ] 3.1 Restyle the top toolbar and per-pane path/navigation headers with aligned groups, editable full paths and restrained separators, preserving native macOS menus.
- [ ] 3.2 Restyle tab strips, pane surfaces, splitter and active/focused/marked states; retain independent per-tab widths, sorting, marks and focus semantics.
- [ ] 3.3 Restyle table headers, rows and metadata while preserving compact row density, resizable columns, virtualized rendering and keyboard/horizontal/vertical navigation.
- [ ] 3.4 Group the six bottom commands without changing their order, shortcut labels, enabled conditions or confirmation rules; distinguish idle, error, progress, paused and cancelling status.
- [ ] 3.5 Reconcile workspace state presentation with W17's loading/failure contract; keep previous-versus-pending locations clear without implementing worker policy in styling code.

## 4. Feature surfaces

- [ ] 4.1 Apply shared styles and deliberate default/secondary/destructive action emphasis to copy, move, rename, batch rename, ZIP extraction, deletion and paste confirmations.
- [ ] 4.2 Apply shared styles to bookmarks and search, including empty, loading, result, cancelled and failed states.
- [ ] 4.3 Apply shared styles to properties and text/image viewer surfaces while preserving field/viewer keyboard and clipboard precedence.
- [ ] 4.4 Apply shared styles to SFTP profiles, host verification, disconnected state, transfer progress and errors without exposing secrets or changing trust decisions.

## 5. Verification and evidence

- [ ] 5.1 Inspect 1120×720 and 820×500 logical layouts in light/dark at 100% and 200% scale with long paths/names, empty/populated panes and representative feature dialogs; correct clipping, overlap and unreachable controls.
- [ ] 5.2 Verify contrast, focus, mark distinction, disabled controls, error visibility and original versus fallback text rendering; record final palette/metrics and asset attribution if applicable.
- [ ] 5.3 Run targeted interaction regressions for path editing, shortcuts, tab/pane switching, splitter/column resize, file marks, virtualized tables, viewer text copy and native menus on available Linux/macOS hosts.
- [ ] 5.4 Capture and inspect final before/after evidence with matched fixtures and host/scale metadata; record unavailable host checks as pending.
- [ ] 5.5 Run workspace formatting, strict all-target/all-feature Clippy, all-feature Rust tests, documentation and Python tests with the required filesystem/service isolation; validate this OpenSpec change strictly.
- [ ] 5.6 Update current UI documentation and W16 evidence with completed scope, intentional visual differences and remaining checks; preserve unfinished W09–W15 gates and reconcile W17 overlap.
