#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
"""Prepare/recheck isolated native and original W06 GUI runs; never launch the reference tree."""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import shutil
import subprocess
import uuid
import xml.etree.ElementTree as ET

import acquire_mucommander as acquisition
from w01 import ROOT, REFERENCE, digest
from w03 import materialize_tree, snapshot
from w06 import comparable, make_zip, payload

FIXTURE = ROOT / "fixtures/W06-zip-browsing-extraction/gui.json"
SPEC = ROOT / "fixtures/W06-zip-browsing-extraction/cases.json"


def observe(root):
    result = snapshot(root)
    for item in result:
        if item["kind"] == "directory":
            item.pop("modified_ms", None)
    return comparable(result)


def verify(run, side):
    provenance = json.loads((run / "provenance.json").read_text())
    if side == "native" and digest(ROOT / "target/debug/mucrab") != provenance["native_binary_sha256"]:
        raise ValueError("Native executable changed since GUI preparation")
    expected = json.loads((run / "expected-trees.json").read_text())
    for name in ("left", "right"):
        actual = observe(run / (side + "-fixture") / name)
        if actual != expected[name]:
            acquisition.write_json(run / (side + "-tree-difference.json"), dict(pane=name, expected=expected[name], actual=actual))
            raise ValueError("ZIP GUI resulting tree differs: " + side + "/" + name)
    acquisition.write_json(run / (side + "-result.json"), dict(status="passed", tree_checks=2,
        bytes_modes_dates=True, archive_integrity=True, normalization="whole-second dates; directory dates omitted"))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--original-probe", type=Path)
    parser.add_argument("--verify-native", type=Path)
    args = parser.parse_args()
    if args.verify_native:
        run = args.verify_native.resolve()
        verify(run, "native")
        actions = json.loads((run / "native/actions.json").read_text())
        stages = {action.get("capture") for action in actions}
        if not {"native-selected-review", "native-cancelled", "native-whole-complete", "native-reverse-complete", "native-partial-details", "native-invalid-zip"}.issubset(stages):
            raise ValueError("Incomplete native ZIP GUI action sequence")
        if observe(run / "partial-destination") != json.loads((run / "expected-partial.json").read_text()):
            raise ValueError("ZIP GUI partial tree differs from original method result")
        print("Native ZIP GUI full and partial extraction trees verified")
        return
    if args.original_probe:
        if not os.environ.get("DISPLAY", "").startswith(":") or os.environ.get("DISPLAY") == ":0":
            raise ValueError("Use an isolated xvfb-run display")
        run = args.original_probe.resolve()
        launch = json.loads((run / "original-probe-launch.json").read_text())
        env = dict(os.environ)
        env.pop("WAYLAND_DISPLAY", None)
        env["XDG_SESSION_TYPE"] = "x11"
        with (run / "original-probe.log").open("w") as log:
            result = subprocess.run(launch["argv"], cwd=launch["cwd"], env=env, stdout=log, stderr=log, timeout=90, umask=0o022)
        if result.returncode:
            raise ValueError("Original ZIP GUI probe failed; see " + str(run / "original-probe.log"))
        verify(run, "original")
        states = json.loads((run / "original-probe/states.json").read_text())
        if len(states) != 12:
            raise ValueError("Incomplete original ZIP GUI states")
        acquisition.run(verify_only=True)
        print("Original ZIP GUI full jobs and both-pane browsing verified")
        return
    acquisition.run(verify_only=True)
    run = ROOT / "artifacts/gui" / ("W06-" + datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + uuid.uuid4().hex[:8])
    run.mkdir(parents=True)
    fixture = json.loads(FIXTURE.read_text())
    spec = json.loads(SPEC.read_text())
    for side in ("native", "original"):
        folder = run / (side + "-fixture")
        for pane in ("left", "right"):
            materialize_tree(folder / pane, fixture[pane], spec)
        make_zip(folder / "left/sample.zip", dict(entries=fixture["archive_entries"]), spec)
        make_zip(folder / "right/return.zip", dict(entries=fixture["return_entries"]), spec)
    expected = {side: observe(run / "native-fixture" / side) for side in ("left", "right")}
    def additions(entries):
        rows = {}
        stamp = int(datetime(*spec["zip_date"], tzinfo=timezone.utc).timestamp() * 1000)
        for entry in entries:
            path = entry["path"].rstrip("/")
            for parent in Path(path).parents:
                if str(parent) != ".":
                    rows[parent.as_posix()] = dict(path=parent.as_posix(), kind="directory", mode=0o755)
            if entry["path"].endswith("/"):
                rows[path] = dict(path=path, kind="directory", mode=0o755)
            else:
                import hashlib
                data = payload(entry)
                rows[path] = dict(path=path, kind="file", mode=0o644, modified_ms=stamp, size=len(data), sha256=hashlib.sha256(data).hexdigest())
        return rows
    for side, entries in (("left", fixture["return_entries"]), ("right", fixture["archive_entries"])):
        rows = {row["path"]: row for row in expected[side]}
        rows.update(additions(entries))
        expected[side] = sorted(rows.values(), key=lambda row: row["path"])
    acquisition.write_json(run / "expected-trees.json", expected)
    partial = next(item for item in spec["cases"] if item["id"] == "partial-stops-after-error")
    make_zip(run / "partial.zip", partial, spec)
    (run / "bad.zip").write_bytes(b"not a ZIP\n")
    materialize_tree(run / "partial-destination", partial["destination"], spec)
    report = json.loads((ROOT / "evidence/workflows/W06-zip-browsing-extraction/comparison-predefined.json").read_text())
    after = json.loads((ROOT / report["raw_run"] / "original-after.json").read_text())
    acquisition.write_json(run / "expected-partial.json", comparable(next(item["destination"] for item in after if item["id"] == "partial-stops-after-error-skip")))
    runtime = run / "reference"
    shutil.copytree(REFERENCE, runtime)
    for source in REFERENCE.rglob("*"):
        if source.is_file() and digest(source) != digest(runtime / source.relative_to(REFERENCE)):
            raise ValueError("Original runtime copy differs")
    profile = run / "profile"
    profile.mkdir()
    preferences = ET.parse(ROOT / "fixtures/W01-local-browse-sort/reference-preferences.xml")
    for node in preferences.iter():
        if node.tag == "show_hidden_files":
            node.text = "false"
    preferences.write(profile / "preferences.xml", encoding="utf-8", xml_declaration=True)
    folders = [str(run / "original-fixture" / side) for side in ("left", "right")]
    original = ["java", "-Duser.timezone=UTC", "-Duser.home=" + str(run / "home"), "--add-opens=java.base/java.net=ALL-UNNAMED", "--add-opens=java.base/java.io=ALL-UNNAMED", "--add-opens=java.desktop/javax.swing.plaf.basic=ALL-UNNAMED", "-cp", str(runtime / "mucommander-1.6.2.jar"), "com.mucommander.main.muCommander", "--preferences", str(profile)] + folders
    acquisition.write_json(run / "native-launch.json", dict(argv=[str(ROOT / "target/debug/mucrab")] + [str(run / "native-fixture" / side) for side in ("left", "right")], cwd=str(ROOT), title="muCrab", output=str(run / "native"), umask=0o022, env={"PATH": "/nonexistent"}))
    classes = run / "probe-classes"
    classes.mkdir()
    probes = [ROOT / "tools/java" / name for name in ("W06GuiProbe.java", "W03GuiProbe.java", "W02bGuiProbe.java")]
    subprocess.run(["javac", "-proc:none", "-d", str(classes)] + list(map(str, probes)), check=True)
    manifest = run / "probe-manifest.mf"
    manifest.write_text("Manifest-Version: 1.0\nPremain-Class: W06GuiProbe\n\n")
    agent = run / "original-gui-probe.jar"
    subprocess.run(["jar", "--create", "--file", str(agent), "--manifest", str(manifest), "-C", str(classes), "."], check=True)
    acquisition.write_json(run / "original-probe-launch.json", dict(argv=[original[0], "-javaagent:" + str(agent) + "=" + str(run / "original-probe")] + original[1:], cwd=str(runtime)))
    acquisition.write_json(run / "provenance.json", dict(fixture_sha256=digest(FIXTURE), cases_sha256=digest(SPEC), reference_copy_verified=True,
        native_binary_sha256=digest(ROOT / "target/debug/mucrab"), probe_sources=[dict(path=str(p.relative_to(ROOT)), sha256=digest(p)) for p in probes],
        preference_sha256=digest(profile / "preferences.xml"), helper_sha256=digest(Path(__file__)), probe_jar_sha256=digest(agent), preparation_only=True))
    acquisition.run(verify_only=True)
    print(run.relative_to(ROOT))


if __name__ == "__main__":
    previous = os.umask(0o022)
    try:
        main()
    finally:
        os.umask(previous)
