# W16 — Desktop visual polish

## Why

muCrab's compact file manager works well, but default widget surfaces, weak visual hierarchy and tightly packed navigation make the interface feel less considered than its functionality. A coherent egui design system can improve clarity and perceived quality while retaining the two-pane workflow and efficient file tables.

## What Changes

- Introduce semantic light/dark colour tokens and shared spacing, typography, border and corner metrics, including live System appearance changes.
- Give navigation, tabs, file tables, status information and command groups distinct visual roles; reduce persistent button outlines and redundant separators.
- Apply consistent icon geometry and interaction states, and improve text emphasis and metadata hierarchy.
- Style existing dialogs, search, properties, viewer and SFTP surfaces consistently, including progress, error and loading states.
- Preserve shortcuts, selection semantics, action availability, native macOS menus and the existing minimum window size; verify keyboard, narrow-window and display-scale behaviour.
- Record repeatable before/after visual evidence and targeted interaction regressions.

## Capabilities

### New Capabilities

- `desktop-visual-system`: Shared visual tokens, type and icon treatment, accessible interaction states and theme switching across existing desktop surfaces.
- `desktop-workspace-presentation`: Two-pane layout hierarchy, compact file-table presentation, command grouping, responsive layout and visual verification.

### Modified Capabilities

None. There are no archived baseline specs in `openspec/specs/`; W09–W15 capabilities remain in their existing changes. This change adds cross-cutting presentation requirements without redefining those workflow contracts.

## Impact

Primarily `crates/desktop/src/appearance.rs`, `app.rs`, `browser_ui.rs` and feature UI modules. Keep Rust, egui/eframe and the core/desktop boundary. No new renderer or UI framework is required. A bundled font, if selected, must have verified redistribution terms, notices and a fallback; no Deckcraft assets or source need be copied.

W17 owns background browsing, state invariants and structural refactoring; W16 owns their visual treatment. The changes can land separately, but overlapping desktop edits must be reconciled. Desktop decomposition from W17 is preferable before the broad W16 component pass. Neither change closes earlier pending parity/platform gates. This proposal adds planning artifacts only; it does not implement the UI.
