#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
"""Prepare isolated, reproducible W01 GUI launch commands without launching a GUI."""

import argparse
from datetime import datetime, timezone
import json
from pathlib import Path
import platform
import shutil
import subprocess
import uuid

import acquire_mucommander as acquisition
from w01 import ROOT, REFERENCE, EVIDENCE, capture, digest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mac-apps", action="store_true", help="Also create local .app wrappers for UI automation using jpackage")
    args = parser.parse_args()
    acquisition.run(verify_only=True)
    comparison = json.loads((EVIDENCE / "comparison-predefined.json").read_text())
    fixture = ROOT / comparison["raw_run"] / "browse"
    if not fixture.is_dir():
        raise ValueError("Regenerate fixtures with mucrab-compare --suite w01 first")
    run_id = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + uuid.uuid4().hex[:8]
    run = ROOT / "artifacts/gui" / ("W01-" + run_id)
    runtime = run / "reference"
    shutil.copytree(REFERENCE, runtime)
    profile = run / "profile"
    profile.mkdir()
    settings = ROOT / "fixtures/W01-local-browse-sort/reference-preferences.xml"
    shutil.copyfile(settings, profile / "preferences.xml")
    java_options = ["-Duser.home=" + str(run / "home"),
        "--add-opens=java.base/java.net=ALL-UNNAMED", "--add-opens=java.base/java.io=ALL-UNNAMED",
        "--add-opens=java.desktop/javax.swing.plaf.basic=ALL-UNNAMED"]
    if platform.system() == "Darwin":
        java_options.extend(["--add-exports=java.desktop/com.apple.eawt=ALL-UNNAMED",
                             "--add-exports=java.desktop/com.apple.laf=ALL-UNNAMED"])
    arguments = ["--preferences", str(profile), str(fixture), str(fixture)]
    reference_command = ["java"] + java_options + ["-cp", str(runtime / "mucommander-1.6.2.jar"),
        "com.mucommander.main.muCommander"] + arguments
    native_command = [str(ROOT / "target/debug/mucrab"), str(fixture), str(fixture)]
    report = {"scope": "GUI preparation only; no UI behavior verified", "run_id": run_id,
        "reference_cwd": runtime.relative_to(ROOT).as_posix(), "reference_argv": reference_command,
        "native_argv": native_command, "settings_sha256": digest(settings),
        "fixture_run": comparison["raw_run"], "reference_copy_unchanged": True}
    # Verify the prepared copy before execution. Runtime caches/config may then change.
    for original in REFERENCE.rglob("*"):
        if original.is_file() and digest(original) != digest(runtime / original.relative_to(REFERENCE)):
            raise ValueError("Reference copy differs: " + str(original))
    if args.mac_apps:
        if platform.system() != "Darwin":
            raise ValueError(".app preparation is only verified on macOS")
        destination = run / "apps"
        command = ["jpackage", "--type", "app-image", "--name", "MuCommanderW01", "--app-version", "1.6.2",
            "--mac-package-identifier", "org.mucrab.reference.w01", "--dest", str(destination),
            "--input", str(runtime), "--main-jar", "mucommander-1.6.2.jar", "--main-class", "com.mucommander.main.muCommander",
            "--arguments", " ".join(json.dumps(arg) for arg in arguments)]
        for option in java_options:
            command.extend(["--java-options", option])
        _, report["jpackage"] = capture(run, "jpackage", command)
        import plistlib
        native_app = destination / "muCrab.app/Contents"
        (native_app / "MacOS").mkdir(parents=True)
        shutil.copy2(ROOT / "target/debug/mucrab", native_app / "MacOS/mucrab")
        with (native_app / "Info.plist").open("wb") as stream:
            plistlib.dump({"CFBundleIdentifier": "org.mucrab.w01", "CFBundleExecutable": "mucrab",
                "CFBundleName": "muCrab", "CFBundlePackageType": "APPL", "CFBundleVersion": "0.1.0",
                "NSHighResolutionCapable": True}, stream)
        report["reference_app"] = str(destination / "MuCommanderW01.app")
        report["native_app"] = str(native_app.parent)
        report["note"] = "Reference .app defaults to the fixture. For the native .app, pass fixture paths to its executable or use Open in each pane."
    acquisition.run(verify_only=True)
    output = run / "launch.json"
    acquisition.write_json(output, report)
    print("Launch commands saved to " + output.relative_to(ROOT).as_posix())
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
