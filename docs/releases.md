# Tagged releases

Push a stable version tag to run [the release workflow](../.github/workflows/release.yml):

```sh
git tag -a v0.9.0 -m 'muCrab v0.9.0'
git push origin v0.9.0
```

The commit being tagged must contain the workflow and packaging tools. Only
canonical `vMAJOR.MINOR.PATCH` tags qualify; prereleases, suffixes and leading
zeroes are skipped. The workflow sets the exported workspace package versions
from the tag, so `mucrab --version` matches the release. It changes neither the
committed manifests nor any external dependency pins.

After every build and check succeeds, the workflow publishes a GitHub Release
with generated notes and these downloads (using `v0.9.0` as an example):

| Download | Contents |
| --- | --- |
| `mucrab-v0.9.0-linux-x86_64.tar.gz` | Linux x86_64 executable and notices |
| `mucrab-v0.9.0-macos-arm64.tar.gz` | Apple Silicon `muCrab.app` and notices |
| `mucrab-v0.9.0-source.tar.gz` | Exact build source, Cargo.lock, all vendored Rust dependencies, build instructions and notices |
| `SHA256SUMS` | SHA-256 checksums for all three archives |

The binary archives each contain a top-level folder. Extract the Linux archive
and run `./mucrab` inside it. On macOS, extract the archive and open `muCrab.app`
or move it to Applications. Its command-line executable is
`muCrab.app/Contents/MacOS/mucrab`. Java is not required.

Linux builds run on Ubuntu 24.04 with the GNU target and use system libraries;
these are not static binaries for every distribution. An OpenGL-capable desktop
and X11 or Wayland runtime libraries are required. Wayland GUI behavior remains
unverified. macOS builds run on macOS 15/Apple Silicon with a macOS 13 deployment
target; older macOS versions have not been tested. The app is ad-hoc signed,
without Developer ID signing or notarization, so macOS may require approval in
System Settings → Privacy & Security.

The workflow builds from the source archive using `--frozen`, which prohibits
registry access or lockfile changes. Each bundle includes `LICENSE`, `NOTICE.md`,
`third_party/`, collected dependency notices in `licenses/`, and provenance in
`release.json`. `licenses/dependencies.json` records every locked external crate,
its declared license, authors, checksum and source directory. Where Cargo
packages omit standalone license files, [pinned supplementary notices](../third_party/release-dependencies/README.md)
are included and their checksums and crate revisions are verified. Complete crate sources,
including embedded license headers, are retained in the source download. GitHub's
automatic tag-source downloads do not contain vendored dependencies or the
release version adjustments; use the explicit `mucrab-…-source.tar.gz` asset to
rebuild the distributed binaries.

To build that source on Ubuntu 24.04, install Rust (CI pins **1.99.1**) and:

```sh
sudo apt-get install pkg-config libx11-dev libxi-dev libxcursor-dev libxrandr-dev libxkbcommon-dev libwayland-dev libgl1-mesa-dev
cargo build --release --frozen -p mucrab-desktop
```

On macOS, install the Xcode command-line tools and Rust, then run the same Cargo
command. Rust and system build tools must already be installed for an offline
build. `BUILDING.md` in the source archive records its version and tagged commit.

Linux release jobs run the Rust workspace tests and Python tool tests. The real
cross-device move tests use `/dev/shm` as a separate filesystem. macOS runs the
library tests. Both targets check formatting, build release mode and smoke-test
`--version` and `--help`; release CI does not run the original Java behavioral
comparisons or GUI replays. Their recorded scope remains in the implementation
ledger. The first hosted execution of this workflow is still pending.

Only the publication job receives `contents: write`; it uses the automatic
`GITHUB_TOKEN`, with no extra repository secret required. An existing draft can
be completed on rerun. Already published releases are never overwritten: use a
new version tag for a replacement. Do not move a published version tag.
