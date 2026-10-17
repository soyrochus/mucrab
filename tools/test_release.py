# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
"""Release version, license completeness, archive layout and asset checks."""

import hashlib
import json
from pathlib import Path
import plistlib
import shutil
import tarfile
import tempfile
import tomllib
import unittest
from unittest.mock import patch

import release


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    def test_only_complete_stable_tags_are_accepted(self):
        for tag in ("v0.9.0", "v1.20.300", "v0.0.0"):
            self.assertEqual(release.release_version(tag), tag[1:])
        for tag in ("0.9.0", "v1.2", "v1.2.3.4", "v1.2.3-rc1", "v1.2.3+build",
                    "v01.2.3", "v1.02.3", "v1.2.03", "v1.2.3\n", "v１.2.3", "v1.2.x"):
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                release.release_version(tag)

    def test_exported_versions_change_without_changing_external_pins_or_checkout(self):
        repo = Path(__file__).resolve().parents[1]
        for name in ("Cargo.toml", "Cargo.lock", "crates"):
            source = repo / name
            if source.is_dir():
                shutil.copytree(source, self.root / name)
            else:
                shutil.copy2(source, self.root / name)
        original_manifest = (repo / "Cargo.toml").read_bytes()
        original_lock = (repo / "Cargo.lock").read_bytes()
        release.set_version(self.root, "0.9.0")
        manifest = tomllib.loads((self.root / "Cargo.toml").read_text())
        self.assertEqual(manifest["workspace"]["package"]["version"], "0.9.0")
        before = tomllib.loads(original_lock.decode())["package"]
        after = tomllib.loads((self.root / "Cargo.lock").read_text())["package"]
        self.assertEqual([p for p in before if "source" in p], [p for p in after if "source" in p])
        local = [p for p in after if "source" not in p]
        self.assertEqual({p["name"] for p in local}, {"mucrab-core", "mucrab-desktop", "mucrab-compare"})
        self.assertTrue(all(p["version"] == "0.9.0" for p in local))
        self.assertEqual((repo / "Cargo.toml").read_bytes(), original_manifest)
        self.assertEqual((repo / "Cargo.lock").read_bytes(), original_lock)

    def license_fixture(self):
        (self.root / "Cargo.lock").write_text(
            'version = 4\n[[package]]\nname = "example"\nversion = "1.2.3"\n'
            'source = "registry+https://github.com/rust-lang/crates.io-index"\n'
            'checksum = "fake-test-checksum"\n'
        )
        crate = self.root / "vendor" / "example-1.2.3"
        crate.mkdir(parents=True)
        (crate / "Cargo.toml").write_text(
            '[package]\nname = "example"\nversion = "1.2.3"\n'
            'license = "MIT"\nlicense-file = "legal/terms.txt"\n'
        )
        (crate / "legal").mkdir()
        (crate / "legal" / "terms.txt").write_text("declared license")
        (crate / "LICENSE-MIT").write_text("MIT notice")
        (crate / "legal" / "NOTICE").write_text("nested notice")
        return crate

    def test_dependency_inventory_retains_declared_and_nested_notices(self):
        self.license_fixture()
        release.collect_licenses(self.root)
        inventory = json.loads((self.root / "licenses" / "dependencies.json").read_text())
        self.assertEqual(inventory[0]["license"], "MIT")
        self.assertEqual(inventory[0]["notice_files"], ["LICENSE-MIT", "legal/NOTICE", "legal/terms.txt"])
        self.assertEqual(
            (self.root / "licenses" / "example-1.2.3" / "legal" / "terms.txt").read_text(),
            "declared license",
        )

    def test_missing_dependency_source_prevents_packaging(self):
        crate = self.license_fixture()
        shutil.rmtree(crate)
        with self.assertRaises(FileNotFoundError):
            release.collect_licenses(self.root)

    def test_a_crate_without_notice_files_requires_pinned_supplemental_notices(self):
        crate = self.license_fixture()
        (crate / "Cargo.toml").write_text('[package]\nname = "example"\nversion = "1.2.3"\nlicense = "MIT"\n')
        (crate / "LICENSE-MIT").unlink()
        shutil.rmtree(crate / "legal")
        with self.assertRaisesRegex(ValueError, "No license notices"):
            release.collect_licenses(self.root)
        shutil.rmtree(self.root / "licenses")
        release.write_json(crate / ".cargo_vcs_info.json", {"git": {"sha1": "test-revision"}})
        notices = self.root / "third_party" / "release-dependencies"
        notices.mkdir(parents=True)
        text = notices / "MIT.txt"
        text.write_text("preserved notice")
        release.write_json(notices / "manifest.json", {
            "packages": {"example@1.2.3": {"notices": ["MIT.txt"], "crate_revision": "test-revision"}},
            "files": {"MIT.txt": {"sha256": hashlib.sha256(text.read_bytes()).hexdigest()}},
        })
        release.collect_licenses(self.root)
        inventory = json.loads((self.root / "licenses" / "dependencies.json").read_text())
        self.assertEqual(inventory[0]["supplemental_notices"], ["third_party/release-dependencies/MIT.txt"])
        shutil.rmtree(self.root / "licenses")
        text.write_text("modified")
        with self.assertRaisesRegex(ValueError, "checksum mismatch"):
            release.collect_licenses(self.root)

    def test_declared_license_cannot_escape_vendor_directory(self):
        crate = self.license_fixture()
        path = crate / "Cargo.toml"
        path.write_text(path.read_text().replace("legal/terms.txt", "../../outside.txt"))
        (self.root / "outside.txt").write_text("outside")
        with self.assertRaisesRegex(ValueError, "Invalid license-file"):
            release.collect_licenses(self.root)

    def bundle_fixture(self):
        source = self.root / "source"
        source.mkdir()
        for name in ("LICENSE", "NOTICE.md"):
            (source / name).write_text(f"test {name}")
        for name in ("third_party", "licenses"):
            (source / name).mkdir()
            (source / name / "notice.txt").write_text("preserved notice")
        release.write_json(source / "release.json", {
            "tag": "v0.9.0", "version": "0.9.0", "commit": "example",
            "rust_toolchain": "1.99.1", "source_archive": "mucrab-v0.9.0-source.tar.gz",
        })
        binary = self.root / "mucrab"
        binary.write_bytes(b"test executable contents")
        binary.chmod(0o644)
        return source, binary

    def test_linux_archive_contains_executable_source_reference_and_notices(self):
        source, binary = self.bundle_fixture()
        path = release.binary_bundle(source, binary, "x86_64-unknown-linux-gnu", self.root)
        with tarfile.open(path) as bundle:
            prefix = "mucrab-v0.9.0-linux-x86_64/"
            self.assertEqual(bundle.getmember(prefix + "mucrab").mode, 0o755)
            self.assertEqual(bundle.extractfile(prefix + "mucrab").read(), binary.read_bytes())
            for name in ("LICENSE", "NOTICE.md", "third_party/notice.txt", "licenses/notice.txt"):
                self.assertIn(prefix + name, bundle.getnames())
            self.assertIn(b"mucrab-v0.9.0-source.tar.gz", bundle.extractfile(prefix + "README.txt").read())

    def test_macos_app_metadata_and_executable_are_packaged_after_signing(self):
        source, binary = self.bundle_fixture()
        with patch.object(release.subprocess, "run") as signing:
            path = release.binary_bundle(source, binary, "aarch64-apple-darwin", self.root)
        self.assertEqual(signing.call_count, 2)
        with tarfile.open(path) as bundle:
            prefix = "mucrab-v0.9.0-macos-arm64/muCrab.app/Contents/"
            info = plistlib.loads(bundle.extractfile(prefix + "Info.plist").read())
            self.assertEqual(info["CFBundleExecutable"], "mucrab")
            self.assertEqual(info["CFBundleShortVersionString"], "0.9.0")
            self.assertEqual(bundle.getmember(prefix + "MacOS/mucrab").mode, 0o755)

    def test_missing_notices_prevent_a_binary_bundle(self):
        source, binary = self.bundle_fixture()
        (source / "LICENSE").unlink()
        with self.assertRaises(FileNotFoundError):
            release.binary_bundle(source, binary, "x86_64-unknown-linux-gnu", self.root)
        self.assertFalse((self.root / "mucrab-v0.9.0-linux-x86_64.tar.gz").exists())

    def test_checksums_require_exact_asset_set_and_match_contents(self):
        assets = self.root / "dist"
        assets.mkdir()
        names = [f"mucrab-v0.9.0-{name}.tar.gz" for name in (*release.TARGETS.values(), "source")]
        for name in names:
            (assets / name).write_bytes(name.encode())
        release.checksums(assets, "v0.9.0")
        for line in (assets / "SHA256SUMS").read_text().splitlines():
            digest, name = line.split("  ")
            self.assertEqual(digest, hashlib.sha256((assets / name).read_bytes()).hexdigest())
        (assets / names[0]).unlink()
        with self.assertRaisesRegex(ValueError, "missing="):
            release.checksums(assets, "v0.9.0")
        (assets / names[0]).touch()
        (assets / "unexpected.tar.gz").touch()
        with self.assertRaisesRegex(ValueError, "extra="):
            release.checksums(assets, "v0.9.0")


if __name__ == "__main__":
    unittest.main()
