# egui framework and default font notices

The desktop scaffold pins eframe/egui **0.36.2**. Their source packages declare `MIT OR Apache-2.0`; this repository preserves the MIT notice from [the upstream `0.36.2` tag](https://raw.githubusercontent.com/emilk/egui/0.36.2/LICENSE-MIT) in [LICENSE-MIT](LICENSE-MIT). The application remains GPLv3; upstream components retain their own terms.

The `epaint_default_fonts` **0.36.2** dependency embeds the following unmodified fonts. [font-provenance.json](fonts/font-provenance.json) records their SHA-256 values and copyright/license URLs extracted from their TrueType name records.

| Font | Preserved notice | Provenance |
| --- | --- | --- |
| Hack Regular | [Hack-LICENSE.txt](fonts/Hack-LICENSE.txt) | Full license string extracted from the bundled font's name record 13; includes Source Foundry MIT, DejaVu public-domain statement, and Bitstream Vera terms |
| Noto Emoji Regular | [OFL.txt](fonts/OFL.txt) | Copied verbatim from the dependency's `fonts/OFL.txt`; copyright record is preserved in `font-provenance.json` |
| Ubuntu Light | [Ubuntu-UFL.txt](fonts/Ubuntu-UFL.txt) | Ubuntu Font Licence 1.0 copied from [Google Fonts' Ubuntu package](https://raw.githubusercontent.com/google/fonts/main/ufl/ubuntu/UFL.txt); the embedded font identifies this license and Canonical copyright |
| Emoji Icon Font | [emoji-icon-font-mit-license.txt](fonts/emoji-icon-font-mit-license.txt) | Copied verbatim from the dependency's font directory; copyright John Slegers |

These files cover the egui framework notice and its bundled default fonts. They do not replace the notices of the other dependencies resolved in `Cargo.lock`. Release packaging will include the applicable dependency sources and notices.
