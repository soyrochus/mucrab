# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
"""Checks for integrity failures and archive extraction boundaries."""

import hashlib
from pathlib import Path
import stat
import tempfile
import unittest
from unittest.mock import patch
import zipfile

import acquire_mucommander as acquisition


class AcquisitionTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    def archive(self, entries):
        path = self.root / "fixture.zip"
        with zipfile.ZipFile(path, "w") as archive:
            for name, data in entries:
                archive.writestr(name, data)
        return path

    def test_corrupt_archive_is_rejected_without_downloading_over_it(self):
        path = self.root / "fixture.zip"
        path.write_bytes(b"wrong")
        expected = {"size_bytes": 5, "sha256": hashlib.sha256(b"right").hexdigest()}
        with patch.object(acquisition.subprocess, "run") as download:
            with self.assertRaisesRegex(ValueError, "SHA-256 mismatch"):
                acquisition.acquire_archive(path, expected)
            download.assert_not_called()
        self.assertEqual(path.read_bytes(), b"wrong")

    def test_partial_download_is_removed_and_never_admitted(self):
        path = self.root / "downloads/fixture.zip"
        expected = {"url": "https://example.invalid/fixture.zip", "size_bytes": 5, "sha256": "0" * 64}

        def bad_download(command, **kwargs):
            Path(command[command.index("--output") + 1]).write_bytes(b"wrong")

        with patch.object(acquisition.subprocess, "run", side_effect=bad_download):
            with self.assertRaisesRegex(ValueError, "SHA-256 mismatch"):
                acquisition.acquire_archive(path, expected)
        self.assertFalse(path.exists())
        self.assertEqual(list(path.parent.iterdir()), [])

    def test_extracts_nested_files_and_reruns_without_replacing_them(self):
        path = self.archive([("launcher.jar", b"launcher"), ("bundle/library.jar", b"library")])
        inventory = acquisition.inventory_archive(path)
        destination = self.root / "reference"
        acquisition.extract_distribution(path, destination, inventory)
        before = (destination / "bundle/library.jar").stat().st_mtime_ns
        acquisition.extract_distribution(path, destination, inventory)
        self.assertEqual((destination / "bundle/library.jar").read_bytes(), b"library")
        self.assertEqual((destination / "bundle/library.jar").stat().st_mtime_ns, before)

    def test_detects_changed_missing_and_unexpected_extracted_files(self):
        path = self.archive([("app/core.jar", b"original")])
        inventory = acquisition.inventory_archive(path)
        destination = self.root / "reference"
        acquisition.extract_distribution(path, destination, inventory)
        core = destination / "app/core.jar"
        core.write_bytes(b"modified")
        with self.assertRaisesRegex(ValueError, "content mismatch"):
            acquisition.verify_distribution(destination, inventory)
        core.unlink()
        with self.assertRaisesRegex(ValueError, "missing="):
            acquisition.verify_distribution(destination, inventory)
        core.write_bytes(b"original")
        (destination / "extra.txt").write_text("unexpected")
        with self.assertRaisesRegex(ValueError, "unexpected="):
            acquisition.verify_distribution(destination, inventory)

    def test_rejects_unsafe_paths_before_writing_distribution(self):
        for name in ("../escape", "/absolute", "C:/drive", "bundle\\escape", "app/../escape"):
            with self.subTest(name=name):
                path = self.archive([("valid.txt", b"valid"), (name, b"bad")])
                with self.assertRaisesRegex(ValueError, "Unsafe archive path"):
                    acquisition.extract_distribution(path, self.root / "reference", [])
                self.assertFalse((self.root / "reference").exists())
                self.assertFalse((self.root / "escape").exists())

    def test_rejects_symlinks_in_archive_and_extracted_tree(self):
        link = zipfile.ZipInfo("link")
        link.create_system = 3
        link.external_attr = (stat.S_IFLNK | 0o777) << 16
        path = self.archive([(link, b"../outside")])
        with self.assertRaisesRegex(ValueError, "Unsupported archive entry type"):
            acquisition.inventory_archive(path)
        path = self.archive([("core.jar", b"original")])
        inventory = acquisition.inventory_archive(path)
        destination = self.root / "reference"
        acquisition.extract_distribution(path, destination, inventory)
        (destination / "core.jar").unlink()
        (destination / "core.jar").symlink_to(path)
        with self.assertRaisesRegex(ValueError, "Unexpected symlink"):
            acquisition.verify_distribution(destination, inventory)


if __name__ == "__main__":
    unittest.main()
