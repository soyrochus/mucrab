#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
"""Acquire and verify the pinned binary distribution; never execute its contents."""

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import stat
import subprocess
import sys
import tempfile
import zipfile


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "tools/manifests/mucommander.json"


def sha256_stream(stream):
    digest = hashlib.sha256()
    for block in iter(lambda: stream.read(1024 * 1024), b""):
        digest.update(block)
    return digest.hexdigest()


def sha256_file(path):
    with path.open("rb") as stream:
        return sha256_stream(stream)


def verify_archive(path, expected):
    if not path.is_file():
        raise ValueError("Missing archive: {}".format(path))
    if path.stat().st_size != expected["size_bytes"]:
        raise ValueError("Archive size mismatch: {}".format(path))
    if sha256_file(path) != expected["sha256"]:
        raise ValueError("Archive SHA-256 mismatch: {}".format(path))


def acquire_archive(path, expected):
    if path.exists():
        verify_archive(path, expected)
        print("Using checksum-verified cached archive.")
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    handle, name = tempfile.mkstemp(prefix=path.name + ".", suffix=".partial", dir=str(path.parent))
    os.close(handle)
    temporary = Path(name)
    try:
        print("Downloading {}".format(expected["url"]), flush=True)
        subprocess.run(
            [
                "curl", "--fail", "--location", "--silent", "--show-error",
                "--proto", "=https", "--proto-redir", "=https",
                "--connect-timeout", "15", "--max-time", "300", "--retry", "2",
                "--output", str(temporary), expected["url"],
            ],
            check=True,
        )
        verify_archive(temporary, expected)
        temporary.replace(path)
    finally:
        temporary.unlink(missing_ok=True)


def validated_members(archive):
    members = archive.infolist()
    seen = set()
    for entry in members:
        name = entry.filename.rstrip("/")
        parts = name.split("/")
        mode = entry.external_attr >> 16
        if (
            not name or "\\" in name or "\x00" in name
            or PurePosixPath(name).is_absolute()
            or re.match(r"^[A-Za-z]:", name)
            or any(part in ("", ".", "..") for part in parts)
        ):
            raise ValueError("Unsafe archive path: {!r}".format(entry.filename))
        if name in seen:
            raise ValueError("Duplicate archive path: {}".format(name))
        seen.add(name)
        if stat.S_IFMT(mode) not in (0, stat.S_IFREG, stat.S_IFDIR):
            raise ValueError("Unsupported archive entry type: {}".format(name))
        if entry.flag_bits & 1:
            raise ValueError("Encrypted archive entry: {}".format(name))
    return members


def inventory_archive(path):
    inventory = []
    with zipfile.ZipFile(path) as archive:
        for entry in validated_members(archive):
            if not entry.is_dir():
                with archive.open(entry) as stream:
                    digest = sha256_stream(stream)
                inventory.append({
                    "path": entry.filename,
                    "size_bytes": entry.file_size,
                    "sha256": digest,
                })
    return sorted(inventory, key=lambda item: item["path"])


def verify_distribution(destination, inventory):
    if not destination.is_dir() or destination.is_symlink():
        raise ValueError("Missing or invalid distribution directory: {}".format(destination))
    actual = set()
    for path in destination.rglob("*"):
        if path.is_symlink():
            raise ValueError("Unexpected symlink in distribution: {}".format(path))
        if path.is_file():
            actual.add(path.relative_to(destination).as_posix())
    expected = {item["path"] for item in inventory}
    if actual != expected:
        raise ValueError("Distribution file set differs: missing={}, unexpected={}".format(
            sorted(expected - actual), sorted(actual - expected),
        ))
    for item in inventory:
        path = destination / item["path"]
        if path.stat().st_size != item["size_bytes"] or sha256_file(path) != item["sha256"]:
            raise ValueError("Distribution content mismatch: {}".format(item["path"]))


def extract_distribution(path, destination, inventory):
    if destination.exists():
        verify_distribution(destination, inventory)
        return
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".mucommander-", dir=str(destination.parent)) as temporary:
        staging = Path(temporary) / "distribution"
        staging.mkdir()
        with zipfile.ZipFile(path) as archive:
            for entry in validated_members(archive):
                target = staging / entry.filename
                if entry.is_dir():
                    target.mkdir(parents=True, exist_ok=True)
                else:
                    target.parent.mkdir(parents=True, exist_ok=True)
                    with archive.open(entry) as source, target.open("xb") as output:
                        shutil.copyfileobj(source, output)
                    permissions = (entry.external_attr >> 16) & 0o777
                    if permissions:
                        target.chmod(permissions)
        verify_distribution(staging, inventory)
        staging.rename(destination)


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def run(verify_only=False):
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    if manifest["schema_version"] != 1:
        raise ValueError("Unsupported acquisition manifest version")
    reference_id = "mucommander-" + manifest["release_tag"]
    archive = ROOT / "artifacts/downloads" / manifest["archive"]["filename"]
    destination = ROOT / "artifacts/reference" / reference_id
    receipt_path = ROOT / "evidence/acquisition" / (reference_id + ".json")

    if verify_only:
        verify_archive(archive, manifest["archive"])
    else:
        acquire_archive(archive, manifest["archive"])
    inventory = inventory_archive(archive)
    by_path = {item["path"]: item for item in inventory}
    for required in manifest["required_files"]:
        if required not in by_path:
            raise ValueError("Required reference file is missing: {}".format(required))
    preserved_license = ROOT / "third_party/mucommander/license.txt"
    if by_path["license.txt"]["sha256"] != sha256_file(preserved_license):
        raise ValueError("Reference license differs from the preserved upstream notice")

    if verify_only:
        verify_distribution(destination, inventory)
    else:
        extract_distribution(archive, destination, inventory)

    launcher = destination / ("mucommander-" + manifest["version"] + ".jar")
    with zipfile.ZipFile(launcher) as jar:
        java_manifest = jar.read("META-INF/MANIFEST.MF").decode("utf-8")
    manifest_fields = dict(line.split(": ", 1) for line in java_manifest.splitlines() if ": " in line)
    if manifest_fields.get("Specification-Version") != manifest["version"]:
        raise ValueError("Launcher version does not match the pinned release")
    if manifest_fields.get("Implementation-Version") != manifest["implementation_revision"]:
        raise ValueError("Launcher revision does not match the pinned release")

    receipt = {
        "schema_version": 1,
        "project": manifest["project"],
        "version": manifest["version"],
        "release_tag": manifest["release_tag"],
        "implementation_revision": manifest["implementation_revision"],
        "input_manifest": MANIFEST.relative_to(ROOT).as_posix(),
        "archive": dict(manifest["archive"], local_path=archive.relative_to(ROOT).as_posix()),
        "distribution": {
            "local_path": destination.relative_to(ROOT).as_posix(),
            "file_count": len(inventory),
            "jar_count": sum(item["path"].endswith(".jar") for item in inventory),
            "uncompressed_size_bytes": sum(item["size_bytes"] for item in inventory),
            "all_files_match_archive": True,
        },
        "launcher": {
            "main_class": manifest_fields.get("Main-Class"),
            "specification_version": manifest_fields.get("Specification-Version"),
            "implementation_version": manifest_fields.get("Implementation-Version"),
        },
        "key_files": [by_path[name] for name in manifest["required_files"]],
        "upstream_notice_matches_preserved_copy": True,
        "scope": "binary_acquisition_and_integrity",
        "application_execution": "not_performed_by_this_tool",
        "behavioral_equivalence": "not_assessed",
    }
    if verify_only:
        if json.loads(receipt_path.read_text(encoding="utf-8")) != receipt:
            raise ValueError("Acquisition evidence differs from the verified reference")
    else:
        write_json(ROOT / "artifacts/inventories" / (reference_id + ".json"), inventory)
        write_json(receipt_path, receipt)
    print("Verified archive SHA-256: {}".format(manifest["archive"]["sha256"]))
    print("Verified distribution: {} files, {} JARs".format(
        receipt["distribution"]["file_count"], receipt["distribution"]["jar_count"],
    ))
    print("Reference: {}".format(destination.relative_to(ROOT)))
    print("Evidence: {}".format(receipt_path.relative_to(ROOT)))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--verify-only", action="store_true", help="Check the cached reference and evidence without network access or writes")
    args = parser.parse_args()
    try:
        run(verify_only=args.verify_only)
    except (OSError, ValueError, KeyError, zipfile.BadZipFile, subprocess.CalledProcessError) as error:
        print("Acquisition failed: {}".format(error), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
