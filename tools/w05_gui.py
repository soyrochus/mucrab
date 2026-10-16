#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
"""Prepare an isolated original/native W05 GUI fixture; never launch the reference tree."""

from datetime import datetime, timezone
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import uuid
import xml.etree.ElementTree as ET

import acquire_mucommander as acquisition
from w01 import ROOT, REFERENCE, digest
from w03 import materialize_tree, snapshot


def verify_trees(run, name):
    expected = json.loads((run/"expected-trees.json").read_text())
    for side in ("left", "right"):
        actual = snapshot(run/name/side)
        if actual != expected[side]: raise ValueError("GUI batch tree differs: " + name + "/" + side)
    acquisition.write_json(run/(name+"-result.json"), dict(status="passed", tree_checks=2, entry_checks=sum(map(len,expected.values())),
        byte_hashes=True, metadata=True, hidden_children=True, links=True))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--original-probe", type=Path, help="Run the prepared original GUI probe on an isolated Xvfb display")
    parser.add_argument("--verify-native", type=Path)
    args = parser.parse_args()
    if args.verify_native:
        run=args.verify_native.resolve()
        verify_trees(run, "native-fixture")
        if snapshot(run/"partial-fixture",["old"]) != json.loads((run/"expected-partial.json").read_text()):raise ValueError("Native batch partial tree differs from original component result")
        acquisition.write_json(run/"native-partial-result.json",dict(status="passed",tree_checks=1,selected_content_preserved=True,reference_case="apply-folder-merge-partial"))
        print("Native GUI batch rename and partial trees verified"); return

    if args.original_probe:
        if not os.environ.get("DISPLAY", "").startswith(":") or os.environ.get("DISPLAY") == ":0":
            raise ValueError("Run the original probe via isolated xvfb-run")
        run = args.original_probe.resolve()
        spec = json.loads((run / "original-probe-launch.json").read_text())
        with (run / "original-probe.log").open("w") as log:
            env = dict(os.environ); env.pop("WAYLAND_DISPLAY", None)
            result = subprocess.run(spec["argv"], cwd=spec["cwd"], env=env, stdout=log, stderr=log, timeout=60, umask=0o022)
        if result.returncode:
            raise ValueError("Original GUI probe failed; see " + str(run / "original-probe.log"))
        states = json.loads((run / "original-probe/states.json").read_text())
        labels=["original-initial","original-marked","original-default-preview","original-duplicate-preview","original-cancelled","original-forward-preview","original-forward-confirmation","original-forward-complete","original-reverse-preview","original-reverse-complete"]
        if [s["stage"] for s in states]!=labels:raise ValueError("Missing original batch stages")
        if states[7]["panes"][0]["marked"] != ["package","keep.dat","old-one.txt"] or states[7]["panes"][1]["marked"] != ["reply.txt"]:raise ValueError("Original batch did not preserve unchanged/skipped/inactive marks")
        if states[-1]["panes"][1]["marked"]:raise ValueError("Original batch did not unmark successful changed rows")
        previews=json.loads((run/"original-probe/previews.json").read_text())
        if previews[1]["rename_enabled"] or not previews[2]["rename_enabled"]:raise ValueError("Original batch preview conflict/apply state wrong")
        verify_trees(run, "original-fixture")
        acquisition.run(verify_only=True)
        acquisition.write_json(run / "original-probe/result.json", dict(status="passed", stages=10, pane_states=20,
            capture="Original Swing window printAll; raw X screen captures also retained", source_sha256=digest(ROOT / "tools/java/W05GuiProbe.java"),
            reference_unchanged=True))
        print("Original GUI action probe passed: " + str(run / "original-probe/states.json"))
        return
    acquisition.run(verify_only=True)
    run = ROOT / "artifacts/gui" / ("W05-" + datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + uuid.uuid4().hex[:8])
    run.mkdir(parents=True)
    fixture = json.loads((ROOT / "fixtures/W05-batch-rename/gui.json").read_text())
    spec = json.loads((ROOT / "fixtures/W05-batch-rename/cases.json").read_text())
    for name in ("native-fixture", "original-fixture"):
        for side in ("left", "right"): materialize_tree(run/name/side, fixture[side], spec)
    left=snapshot(run/"native-fixture/left");right=snapshot(run/"native-fixture/right")
    expected_left=[dict(entry,path="new-two.bin") if entry["path"]=="old-two.bin" else entry for entry in left]
    expected_right=[dict(entry,path="item-001.txt" if entry["path"]=="reply.txt" else "item-002.txt") for entry in right]
    acquisition.write_json(run/"expected-trees.json",dict(left=sorted(expected_left,key=lambda e:e["path"]),right=sorted(expected_right,key=lambda e:e["path"])))
    partial=next(case for case in spec["cases"] if case["id"]=="apply-folder-merge-partial")
    materialize_tree(run/"partial-fixture",partial["source"],spec)
    report=json.loads((ROOT/"evidence/workflows/W05-batch-rename/comparison-predefined.json").read_text())
    after=json.loads((ROOT/report["raw_run"]/"original-after.json").read_text())
    acquisition.write_json(run/"expected-partial.json",next(case["tree"] for case in after if case["id"]==partial["id"]))
    runtime = run / "reference"
    shutil.copytree(REFERENCE, runtime)
    for source in REFERENCE.rglob("*"):
        if source.is_file() and digest(source) != digest(runtime / source.relative_to(REFERENCE)):
            raise ValueError("Reference runtime copy differs: " + str(source))
    profile = run / "profile"; profile.mkdir()
    preferences = ET.parse(ROOT / "fixtures/W01-local-browse-sort/reference-preferences.xml")
    for node in preferences.iter():
        if node.tag == "show_hidden_files":
            node.text = "false"
    preferences.write(profile / "preferences.xml", encoding="utf-8", xml_declaration=True)
    folders = [str(run / "original-fixture" / side) for side in ("left", "right")]
    reference = ["java", "-Duser.home=" + str(run / "home"),
        "--add-opens=java.base/java.net=ALL-UNNAMED", "--add-opens=java.base/java.io=ALL-UNNAMED",
        "--add-opens=java.desktop/javax.swing.plaf.basic=ALL-UNNAMED",
        "-cp", str(runtime / "mucommander-1.6.2.jar"), "com.mucommander.main.muCommander",
        "--preferences", str(profile)] + folders
    for name, argv, cwd, title in (("native", [str(ROOT / "target/debug/mucrab")] + [str(run / "native-fixture" / side) for side in ("left", "right")], ROOT, "muCrab"),
                                  ("original", reference, runtime, "muCommander")):
        acquisition.write_json(run / (name + "-launch.json"), dict(argv=argv, cwd=str(cwd), title=title,
            output=str(run / name), umask=0o022, env={"PATH": "/nonexistent"} if name == "native" else {}))
    classes = run / "probe-classes"; classes.mkdir()
    probe_source = ROOT / "tools/java/W05GuiProbe.java"
    subprocess.run(["javac", "-proc:none", "-d", str(classes), str(probe_source), str(ROOT / "tools/java/W02bGuiProbe.java"), str(ROOT / "tools/java/W04GuiProbe.java")], check=True)
    manifest = run / "probe-manifest.mf"
    manifest.write_text("Manifest-Version: 1.0\nPremain-Class: W05GuiProbe\n\n")
    agent = run / "original-gui-probe.jar"
    subprocess.run(["jar", "--create", "--file", str(agent), "--manifest", str(manifest), "-C", str(classes), "."], check=True)
    acquisition.write_json(run / "original-probe-launch.json", dict(argv=[reference[0], "-javaagent:" + str(agent) + "=" + str(run / "original-probe")] + reference[1:], cwd=str(runtime)))
    acquisition.write_json(run / "provenance.json", dict(fixture_sha256=digest(ROOT / "fixtures/W05-batch-rename/gui.json"),
        partial_reference=dict(report=report["raw_run"]+"/report.json",report_sha256=digest(ROOT/report["raw_run"]/"report.json"),case=partial["id"],expected_sha256=digest(run/"expected-partial.json")),
        reference_copy_verified=True, preference_sha256=digest(profile / "preferences.xml"),
        native_binary_sha256=digest(ROOT / "target/debug/mucrab"), probe_source_sha256=digest(probe_source), helper_sha256=digest(ROOT / "tools/java/W02bGuiProbe.java"),
        probe_jar_sha256=digest(agent), preparation_only=True))
    acquisition.run(verify_only=True)
    print(run.relative_to(ROOT))


if __name__ == "__main__":
    main()
