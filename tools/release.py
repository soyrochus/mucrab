#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
"""Prepare corresponding source and native bundles for tag-triggered releases.

Requires Python 3.12+, Cargo, and Git. Binary packaging on macOS also uses
codesign for an ad-hoc signature (no signing certificate or notarization).
"""

import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import tarfile
import tempfile
import tomllib


TAG_PATTERN = re.compile(r"v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)")
TARGETS = {
    "x86_64-unknown-linux-gnu": "linux-x86_64",
    "aarch64-apple-darwin": "macos-arm64",
}


def release_version(tag):
    if not TAG_PATTERN.fullmatch(tag):
        raise ValueError(f"Expected vMAJOR.MINOR.PATCH, got {tag!r}")
    return tag[1:]


def set_version(root, version):
    """Change only workspace versions in the exported source, preserving pins."""
    release_version("v" + version)
    manifest_path = root / "Cargo.toml"
    text = manifest_path.read_text()
    manifest = tomllib.loads(text)
    workspace = manifest["workspace"]
    old_version = workspace["package"]["version"]
    names = set()
    for member in workspace["members"]:
        package = tomllib.loads((root / member / "Cargo.toml").read_text())["package"]
        if package["version"] != {"workspace": True}:
            raise ValueError(f"{member} must inherit the workspace version")
        names.add(package["name"])
    pattern = r'(\[workspace\.package\]\s*\n(?:(?!\[)[^\n]*\n)*?version\s*=\s*)"[^"\n]+"'
    text, count = re.subn(pattern, lambda match: match[1] + json.dumps(version), text)
    if count != 1:
        raise ValueError("Could not locate the workspace version")

    lock_path = root / "Cargo.lock"
    lock_text = lock_path.read_text()
    blocks = lock_text.split("[[package]]")
    found = set()
    for index, block in enumerate(blocks[1:], 1):
        package = tomllib.loads("[[package]]" + block)["package"][0]
        if "source" not in package and package["name"] in names:
            if package["version"] != old_version:
                raise ValueError("Workspace and lockfile versions disagree")
            blocks[index], count = re.subn(
                r'^version = "[^"\n]+"$',
                lambda _: f'version = "{version}"',
                block,
                count=1,
                flags=re.MULTILINE,
            )
            if count != 1:
                raise ValueError("Could not update a workspace lockfile version")
            found.add(package["name"])
    if found != names:
        raise ValueError("Missing workspace packages in Cargo.lock")
    manifest_path.write_text(text)
    lock_path.write_text("[[package]]".join(blocks))


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def collect_licenses(root):
    """Keep declared licenses and notice files, plus an index of all locked crates.

    The accompanying source archive also keeps every vendored source file and
    license header, including material that has no separate license file.
    """
    destination = root / "licenses"
    destination.mkdir()
    inventory = []
    supplement_root = root / "third_party" / "release-dependencies"
    supplement_path = supplement_root / "manifest.json"
    supplements = json.loads(supplement_path.read_text()) if supplement_path.exists() else {}
    lock = tomllib.loads((root / "Cargo.lock").read_text())
    for package in lock["package"]:
        if "source" not in package:
            continue
        if package["source"] != "registry+https://github.com/rust-lang/crates.io-index":
            raise ValueError("Update release packaging for this non-crates.io dependency")
        directory = f'{package["name"]}-{package["version"]}'
        crate = root / "vendor" / directory
        metadata = tomllib.loads((crate / "Cargo.toml").read_text())["package"]
        files = {
            path for path in crate.rglob("*")
            if path.is_file() and re.match(
                r"^(licen[cs]e|copying|notice|copyright|authors)([._-]|$)",
                path.name, re.IGNORECASE,
            )
        }
        if declared := metadata.get("license-file"):
            path = (crate / declared).resolve()
            if not path.is_relative_to(crate.resolve()) or not path.is_file():
                raise ValueError(f"Invalid license-file in {directory}")
            files.add(path)
        copied = []
        for path in sorted(files, key=lambda path: path.resolve().relative_to(crate.resolve()).as_posix()):
            relative = path.resolve().relative_to(crate.resolve())
            target = destination / directory / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(path, target)
            copied.append(relative.as_posix())
        supplemental = []
        if not copied:
            key = f'{package["name"]}@{package["version"]}'
            record = supplements.get("packages", {}).get(key)
            if not record:
                raise ValueError(f"No license notices for {key}; add pinned supplemental notices")
            vcs = json.loads((crate / ".cargo_vcs_info.json").read_text())
            if vcs["git"]["sha1"] != record["crate_revision"]:
                raise ValueError(f"Supplemental notice revision mismatch for {key}")
            for name in record["notices"]:
                path = (supplement_root / name).resolve()
                if not path.is_relative_to(supplement_root.resolve()):
                    raise ValueError("Supplemental notice escapes its directory")
                digest = hashlib.sha256(path.read_bytes()).hexdigest()
                if digest != supplements["files"][name]["sha256"]:
                    raise ValueError(f"Supplemental notice checksum mismatch: {name}")
                supplemental.append(path.relative_to(root.resolve()).as_posix())
            if not supplemental:
                raise ValueError(f"Empty supplemental notices for {key}")
        inventory.append({
            "name": package["name"], "version": package["version"],
            "source": package["source"], "checksum": package["checksum"],
            "license": metadata.get("license"), "notice_files": copied,
            "authors": metadata.get("authors", []), "supplemental_notices": supplemental,
            "corresponding_source": f"vendor/{directory}",
        })
    write_json(destination / "dependencies.json", inventory)


def archive(directory, output):
    output.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(output, "w:gz") as bundle:
        bundle.add(directory, arcname=directory.name)


def source_bundle(repo, tag, toolchain, output):
    version = release_version(tag)
    repo, output = repo.resolve(), output.resolve()
    commit = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=repo, text=True,
    ).strip()
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary) / f"mucrab-{tag}-source"
        root.mkdir()
        exported = subprocess.check_output(["git", "archive", "HEAD"], cwd=repo)
        with tarfile.open(fileobj=io.BytesIO(exported)) as tree:
            tree.extractall(root, filter="data")
        set_version(root, version)
        subprocess.run(
            ["cargo", "vendor", "--locked", "--versioned-dirs", "vendor"],
            cwd=root, check=True, stdout=subprocess.DEVNULL,
        )
        config = root / ".cargo" / "config.toml"
        if config.exists():
            raise ValueError("Merge existing Cargo config before preparing a release")
        config.parent.mkdir(exist_ok=True)
        config.write_text(
            '[source.crates-io]\nreplace-with = "vendored-sources"\n\n'
            '[source.vendored-sources]\ndirectory = "vendor"\n'
        )
        collect_licenses(root)
        write_json(root / "release.json", {
            "tag": tag, "version": version, "commit": commit,
            "rust_toolchain": toolchain,
            "source_archive": f"mucrab-{tag}-source.tar.gz",
            "version_note": "Workspace versions set from the tag in this exported source only.",
        })
        (root / "BUILDING.md").write_text(
            f"# Building muCrab {tag}\n\n"
            f"Tagged commit: `{commit}`. CI toolchain: Rust `{toolchain}`.\n"
            "Workspace package versions in Cargo.toml and Cargo.lock were set to "
            f"`{version}` for this release; external dependency pins are unchanged.\n\n"
            "Install the Rust toolchain and native build prerequisites described in "
            "docs/releases.md. From this directory, build without a registry connection:\n\n"
            "```sh\ncargo build --release --frozen -p mucrab-desktop\n"
            "./target/release/mucrab --version\n```\n\n"
            "Cargo uses the included vendor directory. No Java, Node, REA, or reference "
            "JARs are needed to build or run the desktop application. Rust and native "
            "system libraries are external build prerequisites.\n\n"
            "LICENSE, NOTICE.md, third_party/, licenses/dependencies.json, and all "
            "vendored sources preserve licensing and provenance. The binary bundles "
            "on this release were built from this source archive.\n"
        )
        path = output / f"{root.name}.tar.gz"
        archive(root, path)
    return path


def binary_bundle(source, binary, target, output):
    metadata = json.loads((source / "release.json").read_text())
    tag = metadata["tag"]
    version = release_version(tag)
    if metadata["version"] != version:
        raise ValueError("Source release metadata disagrees with tag")
    platform = TARGETS[target]
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary) / f"mucrab-{tag}-{platform}"
        root.mkdir()
        for name in ("LICENSE", "NOTICE.md", "third_party", "licenses"):
            original = source / name
            if original.is_dir():
                shutil.copytree(original, root / name)
            else:
                shutil.copy2(original, root / name)
        metadata["target"] = target
        write_json(root / "release.json", metadata)
        if "apple-darwin" in target:
            app = root / "muCrab.app"
            executable = app / "Contents" / "MacOS" / "mucrab"
            executable.parent.mkdir(parents=True)
            with (app / "Contents" / "Info.plist").open("wb") as stream:
                plistlib.dump({
                    "CFBundleName": "muCrab", "CFBundleDisplayName": "muCrab",
                    "CFBundleIdentifier": "io.github.soyrochus.mucrab",
                    "CFBundleExecutable": "mucrab", "CFBundlePackageType": "APPL",
                    "CFBundleShortVersionString": version, "CFBundleVersion": version,
                    "NSHighResolutionCapable": True,
                    "LSMinimumSystemVersion": "13.0",
                }, stream)
            launch = "Open muCrab.app, or run ./muCrab.app/Contents/MacOS/mucrab.\n"
            launch += (
                "The app is ad-hoc signed, without Developer ID signing or notarization.\n"
                "macOS may require approval in System Settings > Privacy & Security.\n"
            )
        else:
            executable = root / "mucrab"
            launch = "Run ./mucrab from an X11 or Wayland desktop session.\n"
            launch += "Built on Ubuntu 24.04; see docs/releases.md in the source archive for dependencies.\n"
        shutil.copy2(binary, executable)
        executable.chmod(0o755)
        if "apple-darwin" in target:
            subprocess.run(["codesign", "--force", "--sign", "-", str(app)], check=True)
            subprocess.run(["codesign", "--verify", "--strict", str(app)], check=True)
        (root / "README.txt").write_text(
            f"muCrab {tag} ({platform})\n\n" + launch
            + "No Java runtime is required.\n\n"
            + f"Corresponding source: {metadata['source_archive']} on the same GitHub Release.\n"
            + "License: GPL-3.0-only; see LICENSE, NOTICE.md, third_party/ and licenses/.\n"
            + "Experimental build; see README.md and the implementation ledger in the source archive for scope.\n"
        )
        path = output / f"{root.name}.tar.gz"
        archive(root, path)
    return path


def checksums(directory, tag):
    """Fail closed if any expected source or binary archive is missing or extra."""
    release_version(tag)
    expected = {f"mucrab-{tag}-{platform}.tar.gz" for platform in TARGETS.values()}
    expected.add(f"mucrab-{tag}-source.tar.gz")
    actual = {path.name for path in directory.iterdir() if path.name != "SHA256SUMS"}
    if actual != expected:
        raise ValueError(f"Release assets differ: missing={expected - actual}, extra={actual - expected}")
    lines = []
    for name in sorted(expected):
        with (directory / name).open("rb") as stream:
            digest = hashlib.file_digest(stream, "sha256").hexdigest()
        lines.append(f"{digest}  {name}\n")
    (directory / "SHA256SUMS").write_text("".join(lines))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    validate = sub.add_parser("validate-tag")
    validate.add_argument("tag")
    source = sub.add_parser("source")
    source.add_argument("--repo", type=Path, default=Path.cwd())
    source.add_argument("--tag", required=True)
    source.add_argument("--toolchain", required=True)
    source.add_argument("--output", type=Path, required=True)
    binary = sub.add_parser("binary")
    binary.add_argument("--source", type=Path, required=True)
    binary.add_argument("--binary", type=Path, required=True)
    binary.add_argument("--target", choices=TARGETS, required=True)
    binary.add_argument("--output", type=Path, required=True)
    sums = sub.add_parser("checksums")
    sums.add_argument("--directory", type=Path, required=True)
    sums.add_argument("--tag", required=True)
    args = parser.parse_args()
    if args.command == "validate-tag":
        # Non-release v* tags succeed, but skip all downstream jobs.
        valid = bool(TAG_PATTERN.fullmatch(args.tag))
        lines = f"valid={str(valid).lower()}\n"
        if valid:
            lines += f"version={release_version(args.tag)}\n"
        if output := os.environ.get("GITHUB_OUTPUT"):
            with open(output, "a") as stream:
                stream.write(lines)
        print(lines, end="")
    elif args.command == "source":
        print(source_bundle(args.repo, args.tag, args.toolchain, args.output))
    elif args.command == "binary":
        print(binary_bundle(args.source, args.binary, args.target, args.output))
    elif args.command == "checksums":
        checksums(args.directory, args.tag)


if __name__ == "__main__":
    main()
