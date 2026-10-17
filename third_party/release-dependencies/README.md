# Supplementary dependency notices

Some locked Cargo packages omit the license files from their upstream workspace.
These unmodified notices fill that gap for release packaging. `manifest.json`
maps exact crate versions to notice files, records their SHA-256 hashes, and
lists immutable upstream URLs. Revisions come from each published crate's
`.cargo_vcs_info.json`; duplicate license texts are stored once. Only licensing
material was retrieved from the upstream repositories.

Newer objc2 workspace notices link to standard license texts. The referenced
MIT, Apache-2.0 and Zlib texts are preserved from SPDX license-list-data
**v3.27.0**. The `dispatch` 0.2.0 upstream revision contains no standalone license
file; its published Cargo metadata declares MIT and names Steven Sheldon as
author. Its inventory retains that metadata and the standard MIT text. These
standard texts retain SPDX's placeholders; they do not invent copyright grants.
Original crate sources and headers remain in the corresponding-source download.

The egui font notices are separately preserved in `../egui/`. Original notices
from the remaining crates are collected directly from their vendored packages.
Every binary and source bundle includes this directory and the generated
`licenses/dependencies.json` inventory. If a dependency without a standalone
notice changes, update these pins before releasing; the packaging helper fails
when a required supplemental notice is missing, modified, or for another revision.
