# W16 design

## Context

The desktop uses egui/eframe 0.36. `appearance.rs` starts with egui's default visuals and adjusts a few metrics and colours. `browser_ui.rs` already custom-paints virtualized tables and icons; the application also has native macOS menus. The opportunity is coordinated presentation across these existing surfaces, not a framework replacement.

Deckcraft demonstrates the benefit of semantic surfaces, consistent icon treatment, deliberate spacing and hierarchy. Its presentation canvas and ribbon are different from a file manager; copy neither its layout nor its assets. Existing W08 screenshots predate tabs, search, properties, viewer and SFTP, so capture the current application before implementation.

## Goals / Non-Goals

**Goals:** A coherent light/dark design; readable hierarchy; clear focus and active-pane states; compact file browsing; consistent existing dialogs; preserved keyboard and native-menu behaviour; verifiable improvements at supported window sizes and scales.

**Non-Goals:** A ribbon, a new UI toolkit, new file-management commands, reordered data, persistent tab sessions, changed shortcuts, density preferences, decorative animation, changes to original-binary parity scoring, or fixing W17's worker lifecycle in the styling layer.

## Decisions

### 1. Centralise semantic tokens and metrics

Define a desktop-owned theme value with roles for window chrome, content, raised surfaces, fields, primary/secondary/muted text, subtle borders, separators, accent, selection, focus, hover, pressed, disabled, warning, error and shadow. Widgets use these roles rather than unrelated inline colours. Supply complete light and dark palettes and apply both to egui's theme-specific styles so System changes retain the design.

Starting palette for implementation, to be checked in rendered output:

| Role | Light | Dark |
|---|---|---|
| Window chrome | `#F4F6F8` | `#1C2026` |
| Main content | `#FFFFFF` | `#222831` |
| Raised surface | `#FFFFFF` | `#2B333E` |
| Field fill | `#E8ECF1` | `#181D24` |
| Primary text | `#202936` | `#EDF1F7` |
| Secondary text | `#526174` | `#B6C2D1` |
| Accent | `#245FA8` | `#80B8FF` |
| Selection fill | `#245FA8` | `#285787` |
| Selection text | `#FFFFFF` | `#FFFFFF` |

Complete the remaining state roles before rollout. Check actual text/background pairs, including hover and selection; token names alone do not establish contrast. Use 4.5:1 for normal text and 3:1 for large text and essential non-text indicators. Disabled controls remain recognisable but need not meet the normal-text target. Colour is accompanied by focus geometry, state marks, text or icons as appropriate.

Initial logical-point metrics: a 4-point spacing unit; 8-point workspace inset; 6-point pane padding; 32-point navigation controls; 18-point toolbar icons; 23-point compact file rows; 6-point ordinary control corners and 8-point panel/dialog corners. These are starting values, not a reason to clip content at the existing 820×500 minimum. File-table density remains compact; give navigation and action groups more space rather than inflating every row. Popups use restrained shadows; table rows do not.

**Alternative considered:** changing `Visuals` in place without shared roles is smaller initially but leaves custom-painted tables and feature dialogs inconsistent.

### 2. Make typography and icons deliberate

Define small/caption, body/button and section-title styles around 11, 13 and 16 logical points, with explicit emphasis for active tabs and section labels. File names remain primary; extension, size, dates and status use secondary treatment with adequate contrast. Keep numeric sizes consistently aligned.

Use existing fonts initially. Evaluate a redistributable UI face such as Inter only if the rendered comparison warrants bundling it; verify original provenance, licence, notice, size and fallback behaviour before adding any font. Do not silently depend on a locally installed font or lose glyph coverage. A new font is not an acceptance prerequisite.

Unify original code-drawn icons around a shared design grid, stroke thickness, optical bounds and scale-aware placement. Preserve recognisable folder/file/archive/link distinctions. Navigation actions use quiet, unboxed idle states; hover, press, focus and disabled states are still explicit. Custom widgets retain accessibility labels, keyboard activation and the same enabled checks as menu commands.

**Alternative considered:** importing a large icon/font package adds assets and dependency work before consistent geometry and state styling have been tried.

### 3. Establish file-manager hierarchy

Compose the window as menu/toolbar, two pane workspaces and a compact status/command region. Group navigation separately from file mutations using spacing and restrained separators. Keep paths editable, tabs visible and both pane headers aligned. Distinguish the active pane and keyboard focus without surrounding every control with a high-contrast box.

Keep all six existing bottom commands available and preserve their order and function-key labels. Group related commands visually; do not invent a permanently privileged mutation, hide commands in a new overflow menu, or make destructive actions the default accent. Primary/default dialog actions can receive stronger emphasis; cancellation and secondary actions stay easy to locate. Error, progress and cancellation information remains more prominent than idle status.

At narrow widths, use existing table scrolling and deliberate text elision with full-value access. Preserve editable full paths, resizable columns and accessible controls. Retain per-tab widths, splitter interaction, focus/mark distinctions, independent sorting and all W09–W15 behaviour. Native macOS menus remain native.

### 4. Apply the system across existing features

Migrate appearance and shared widgets first, then panes/navigation, then commands and feature dialogs. Cover copy/move/rename, ZIP extraction, deletion/paste, bookmarks, search, properties, viewer and SFTP, including empty, loading, disconnected, failed, paused and cancelling states. W17 owns when browsing state transitions occur; W16 styles those states through the shared presentation contract without changing cancellation or commit semantics.

**Alternative considered:** polishing only the main window leaves the most consequential confirmations and failure states visually inconsistent.

### 5. Verify visuals and behaviour separately

Use disposable fixtures with Unicode and long names, links, archives, populated and empty panes, marked/focused entries and representative dialogs. Capture current and revised native output with the same fixtures, dimensions, theme and scale. Include 1120×720 and 820×500 logical layouts, light/dark, 100%/200% scaling, and a live System theme change. Record actual host/backend/font/scale; do not compare raw pixels across different hosts as if they were identical renderers.

Inspect hierarchy, alignment, clipping, contrast and icon consistency. Retain targeted interaction checks for focus, keyboard, shortcuts and virtualized tables; avoid tests that merely repeat palette constants. Record Linux/macOS evidence separately and leave unavailable host checks pending. A screenshot is evidence of presentation, not proof of operation correctness or full original-application parity.

## Risks / Trade-offs

- More spacing can reduce file visibility → keep 23-point file rows initially and validate the minimum window before expanding surrounding controls.
- Quiet buttons can become undiscoverable → retain tooltips, labels where already present, keyboard focus and visible hover/pressed feedback.
- Muted metadata can become unreadable → measure rendered contrast pairs and preserve explicit selection text colours.
- Font and scaling differences can alter geometry → test fallback text and both target scales; keep asset addition optional.
- W16 and W17 touch the same desktop files → use explicit component/loading contracts, prefer W17 decomposition first and replay interaction regressions after reconciliation.

## Migration Plan

Land theme primitives, shared controls, workspace layout and feature surfaces in reviewable passes. Keep appearance persistence and external data formats unchanged. Capture visual evidence and update the current UI documentation after the final pass. An implementation rollback restores desktop presentation code; no stored-data migration is needed. Existing W09–W15 unfinished tasks retain their status.

## Open Questions

No product decision blocks implementation. Resolve the optional font choice and final palette adjustments from same-host rendered evidence, recording the selected assets and metrics. A host unavailable during implementation remains an explicit verification gap, not a completed task.
