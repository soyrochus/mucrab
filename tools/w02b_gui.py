#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
"""Prepare an isolated original/native W02b GUI fixture; never launch the reference tree."""

from datetime import datetime, timezone
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import uuid
import xml.etree.ElementTree as ET

import acquire_mucommander as acquisition
from w01 import ROOT, REFERENCE, digest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--original-probe", type=Path, help="Run the prepared original GUI probe on an isolated Xvfb display")
    args = parser.parse_args()
    if args.original_probe:
        if not os.environ.get("DISPLAY", "").startswith(":") or os.environ.get("DISPLAY") == ":0":
            raise ValueError("Run the original probe via isolated xvfb-run")
        run = args.original_probe.resolve()
        spec = json.loads((run / "original-probe-launch.json").read_text())
        with (run / "original-probe.log").open("w") as log:
            result = subprocess.run(spec["argv"], cwd=spec["cwd"], stdout=log, stderr=log, timeout=60)
        if result.returncode:
            raise ValueError("Original GUI probe failed; see " + str(run / "original-probe.log"))
        states = json.loads((run / "original-probe/states.json").read_text())
        labels = ["original-hidden-off", "original-hidden-on", "original-hidden-and-visible-marked",
                  "original-filtered-focus-fallback", "original-reshown-hidden-marks-cleared"]
        if [state["stage"] for state in states] != labels:
            raise ValueError("Original GUI probe did not capture every stage")
        for index, state in enumerate(states):
            if len(state["panes"]) != 2:
                raise ValueError("Original GUI probe did not inspect both panes")
            for pane in state["panes"]:
                if (".hidden" in pane["visible"]) != (index in (1, 2, 4)):
                    raise ValueError("Original GUI visible entries did not follow the menu action")
        for index in (3, 4):
            left, right = states[index]["panes"]
            if (left["marked"], right["marked"], left["focused"], right["focused"]) != (["file2.txt"], ["Alpha.txt"], "file10.txt", "folder2"):
                raise ValueError("Original GUI did not preserve surviving marks/focus or clear hidden marks")
        acquisition.run(verify_only=True)
        acquisition.write_json(run / "original-probe/result.json", dict(status="passed", stages=5, pane_states=10,
            capture="Original Swing window printAll; raw X screen captures also retained", source_sha256=digest(ROOT / "tools/java/W02bGuiProbe.java"),
            reference_unchanged=True))
        print("Original GUI action probe passed: " + str(run / "original-probe/states.json"))
        return
    acquisition.run(verify_only=True)
    report = json.loads((ROOT / "evidence/workflows/W02b-hidden-filtering/comparison-predefined.json").read_text())
    fixture = ROOT / report["raw_run"] / "fixtures/visible-focus-and-surviving-marks-natural"
    if not fixture.is_dir():
        raise ValueError("Regenerate the W02b predefined comparisons first")
    run = ROOT / "artifacts/gui" / ("W02b-" + datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + uuid.uuid4().hex[:8])
    runtime = run / "reference"
    shutil.copytree(REFERENCE, runtime)
    for source in REFERENCE.rglob("*"):
        if source.is_file() and digest(source) != digest(runtime / source.relative_to(REFERENCE)):
            raise ValueError("Reference runtime copy differs: " + str(source))
    profile = run / "profile"; profile.mkdir()
    only_hidden = run / "only-hidden"; only_hidden.mkdir()
    (only_hidden / ".hidden").write_bytes(b"GUI hidden-only fixture\n")
    preferences = ET.parse(ROOT / "fixtures/W01-local-browse-sort/reference-preferences.xml")
    for node in preferences.iter():
        if node.tag == "show_hidden_files":
            node.text = "false"
    preferences.write(profile / "preferences.xml", encoding="utf-8", xml_declaration=True)
    folders = [str(fixture / str(i)) for i in range(2)]
    reference = ["java", "-Duser.home=" + str(run / "home"),
        "--add-opens=java.base/java.net=ALL-UNNAMED", "--add-opens=java.base/java.io=ALL-UNNAMED",
        "--add-opens=java.desktop/javax.swing.plaf.basic=ALL-UNNAMED",
        "-cp", str(runtime / "mucommander-1.6.2.jar"), "com.mucommander.main.muCommander",
        "--preferences", str(profile)] + folders
    for name, argv, cwd, title in (("native", [str(ROOT / "target/debug/mucrab")] + folders, ROOT, "muCrab"),
                                  ("original", reference, runtime, "muCommander")):
        acquisition.write_json(run / (name + "-launch.json"), dict(argv=argv, cwd=str(cwd), title=title,
            output=str(run / name), env={"PATH": "/nonexistent"} if name == "native" else {}))
    classes = run / "probe-classes"; classes.mkdir()
    probe_source = ROOT / "tools/java/W02bGuiProbe.java"
    subprocess.run(["javac", "-proc:none", "-d", str(classes), str(probe_source)], check=True)
    manifest = run / "probe-manifest.mf"
    manifest.write_text("Manifest-Version: 1.0\nPremain-Class: W02bGuiProbe\n\n")
    agent = run / "original-gui-probe.jar"
    subprocess.run(["jar", "--create", "--file", str(agent), "--manifest", str(manifest), "-C", str(classes), "."], check=True)
    acquisition.write_json(run / "original-probe-launch.json", dict(argv=[reference[0], "-javaagent:" + str(agent) + "=" + str(run / "original-probe")] + reference[1:], cwd=str(runtime)))
    acquisition.write_json(run / "provenance.json", dict(fixture_run=report["raw_run"], source_revision=report["source_revision"],
        reference_copy_verified=True, preference_sha256=digest(profile / "preferences.xml"),
        native_binary_sha256=digest(ROOT / "target/debug/mucrab"), probe_source_sha256=digest(probe_source),
        probe_jar_sha256=digest(agent), preparation_only=True))
    acquisition.run(verify_only=True)
    print(run.relative_to(ROOT))


if __name__ == "__main__":
    main()
