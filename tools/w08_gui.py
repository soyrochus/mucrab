#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
"""Prepare or verify W08's isolated desktop replay; never uses the live desktop.

Use the stable default run path for the pixel-coordinate replay at 1120x720.
Preparation refuses to overwrite an existing run. No original Java jobs are run:
this is a UI regression, using W01-W06's already recovered core operations.
"""
import argparse
import hashlib
import json
from pathlib import Path
import zipfile

ROOT = Path(__file__).resolve().parents[1]
RUN = ROOT / "artifacts/gui/W08-desktop-ui"
CONTENT = b"UI verification\n"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def prepare():
    RUN.mkdir(parents=True, exist_ok=False)
    left, right = RUN / "left", RUN / "right"
    (left / "Documents").mkdir(parents=True)
    right.mkdir()
    (left / "Documents/inside.txt").write_bytes(b"Nested file\n")
    (left / "notes.txt").write_bytes(CONTENT)
    (left / ".hidden").write_bytes(b"hidden\n")
    (left / "a-very-long-file-name-to-check-column-clipping-and-hover.txt").write_bytes(b"long\n")
    (left / "link").symlink_to("notes.txt")
    with zipfile.ZipFile(left / "sample.zip", "w") as archive:
        archive.writestr(zipfile.ZipInfo("folder/readme.txt", (2020, 1, 1, 0, 0, 0)), b"Archive contents\n")
    config = RUN / "config/mucrab"
    config.mkdir(parents=True)
    # Restart with a saved explicit preference; the replay changes it to Light
    # and finally System. The headless test separately injects live OS changes.
    (config / "appearance").write_text("dark\n")
    launch = dict(argv=[str(ROOT / "target/debug/mucrab"), str(left), str(right)],
                  cwd=str(ROOT), title="muCrab", output=str(RUN / "native"),
                  env={"XDG_CONFIG_HOME": str(RUN / "config")})
    (RUN / "launch.json").write_text(json.dumps(launch, indent=2) + "\n")
    (RUN / "before.json").write_text(json.dumps({"archive_sha256": digest(left / "sample.zip")}, indent=2) + "\n")
    print(RUN / "launch.json")


def verify():
    left, right = RUN / "left", RUN / "right"
    expected_left = {"Documents", "a-very-long-file-name-to-check-column-clipping-and-hover.txt",
                     ".hidden", "link", "batch_notes.txt", "renamed.txt", "sample.zip"}
    if {p.name for p in left.iterdir()} != expected_left:
        raise ValueError("Unexpected left-pane tree after copy/rename/move/batch rename")
    for path in [left / "batch_notes.txt", left / "renamed.txt"]:
        if path.read_bytes() != CONTENT:
            raise ValueError("File contents changed: " + str(path))
    if (left / "link").readlink() != Path("notes.txt"):
        raise ValueError("Link identity changed")
    if (left / "Documents/inside.txt").read_bytes() != b"Nested file\n" or (left / ".hidden").read_bytes() != b"hidden\n":
        raise ValueError("Unselected source changed")
    if (left / "a-very-long-file-name-to-check-column-clipping-and-hover.txt").read_bytes() != b"long\n":
        raise ValueError("Long-name source changed")
    if {str(p.relative_to(right)) for p in right.rglob("*")} != {"folder", "folder/readme.txt"}:
        raise ValueError("Unexpected move/extraction destination tree")
    if (right / "folder/readme.txt").read_bytes() != b"Archive contents\n":
        raise ValueError("ZIP extraction contents differ")
    before = json.loads((RUN / "before.json").read_text())
    if digest(left / "sample.zip") != before["archive_sha256"]:
        raise ValueError("Source archive changed")
    if (RUN / "config/mucrab/appearance").read_text().strip() != "system":
        raise ValueError("Appearance changes were not persisted")
    expected_actions = json.loads((ROOT / "fixtures/W08-desktop-ui/native-gui-actions.json").read_text())
    expected_actions.insert(0, dict(op="capture", name="initial", wait=2))
    actual_actions = json.loads((RUN / "native/actions.json").read_text())
    if actual_actions != expected_actions:
        raise ValueError("UI replay incomplete or differs from the recorded sequence")
    from PIL import Image
    for name, dark in [("initial", True), ("light-final", False)]:
        with Image.open(RUN / "native" / (name + ".png")) as image:
            pixel = image.getpixel((20, 350))[:3]
        if (max(pixel) < 70) != dark:
            raise ValueError("Unexpected palette in " + name)
    print("W08 replay verified: exact actions, copy/rename/move/batch/ZIP trees and bytes, archive integrity, saved theme, light/dark captures")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["prepare", "verify"])
    args = parser.parse_args()
    prepare() if args.action == "prepare" else verify()
