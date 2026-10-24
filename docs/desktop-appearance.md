# Desktop appearance

W16 keeps the compact two-pane file manager and adds a common visual language to
its egui surfaces. No new font, icon asset or UI framework is bundled.

`appearance.rs` defines semantic light/dark colours for chrome, content, raised
surfaces, fields, text, borders, accent, selection, hover, press, warnings and
errors. Both palettes are installed so live System appearance changes preserve
the application styling. Saved Light, Dark and System settings keep their meaning.

Typography uses the existing bundled font coverage with 11-point captions,
13-point body/buttons and 16-point headings. Active tabs use explicit emphasis and
white selection text. Metadata has a quieter colour while meeting the normal-text
contrast target. Automated checks measure text, selection, warning, error and
essential focus contrast against their actual background roles.

Navigation uses quiet 32-point controls and 18-point code-drawn icons on the shared
design grid. File icons remain 16 points and file rows remain 23 points. Pane
surfaces have restrained borders, rounded corners and more internal spacing;
navigation groups, editable paths, tabs and status retain distinct roles. Existing
feature windows and confirmations inherit the same palette and metrics. Primary
confirmation actions have explicit emphasis without new default keyboard actions.

The six bottom commands retain their order, shortcuts and availability. Native
macOS menus stay native. Focus, marks, sorting, tab ownership and clipboard routing
retain their existing semantics. Loading state is separate from the retained
listing under W17's background-browsing contract.

Visual verification uses disposable Unicode/long-name, folder/link/ZIP and empty
folder fixtures. The matrix covers 1120×720 and 820×500 logical layouts, Light/Dark
and 100%/200% scale. Native screenshots and keyboard interaction evidence are
recorded separately from operation tests and original/native comparisons.

See [W16 evidence](../evidence/workflows/W16-desktop-visual-polish/README.md).
Linux/X11 was exercised locally; macOS native visual/menu regression and physical
OS theme-notification delivery remain separate verification gates. Earlier
W09–W15 gaps are not closed by this styling pass.
