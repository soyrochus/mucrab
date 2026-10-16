#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
"""Compare original hidden filtering/reload workers with both native pane states."""

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
import platform
import random
import subprocess
import sys
import uuid
import zipfile

import acquire_mucommander as acquisition
from w01 import ROOT, REFERENCE, capture, digest

SPEC = ROOT / "fixtures/W02b-hidden-filtering/cases.json"
SOURCE = ROOT / "tools/java/W02bOracle.java"
EVIDENCE = ROOT / "evidence/workflows/W02b-hidden-filtering"


def generated(spec, reserved):
    rng = random.Random(spec["reserved_seed"] if reserved else spec["generated_seed"])
    count = spec["reserved_sequences"] if reserved else spec["generated_sequences"]
    for index in range(count):
        mode = rng.choice(["NATURAL", "LEXICOGRAPHIC"])
        panes = []
        for _ in range(2):
            names = ["file2.txt"] + rng.sample(spec["names"], rng.randrange(len(spec["names"]) + 1))
            rng.shuffle(names)
            panes.append(dict(names=names, parent=rng.choice([True, False])))
        show = initial = rng.choice([True, False])
        events = []
        for _ in range(spec["events_per_sequence"]):
            op = rng.choice(["toggle_hidden", "toggle_hidden", "refresh", "click_name", "space", "release",
                "next", "previous", "first", "last", "mark_next", "mark_previous", "all", "none", "invert", "sort"])
            if op == "toggle_hidden":
                show = not show
                event = dict(op=op)
            else:
                pane = rng.randrange(2)
                event = dict(op=op, pane=pane)
                if op == "click_name":
                    names = [name for name in panes[pane]["names"] if show or not name.startswith(".")]
                    if panes[pane]["parent"]:
                        names.append("..")
                    event.update(name=rng.choice(names), ctrl=rng.choice([True, False]), shift=rng.choice([True, False]))
                elif op == "sort":
                    event.update(ascending=rng.choice([True, False]), directories_first=rng.choice([True, False]), mode=mode)
            events.append(event)
        yield dict(id=("reserved" if reserved else "generated") + "-" + str(index),
            mode=mode, show_hidden=initial, panes=panes, events=events)


def materialize(run, spec, reserved):
    cases = list(generated(spec, True)) if reserved else spec["cases"] + list(generated(spec, False))
    for original in cases:
        case = json.loads(json.dumps(original))
        for index, pane in enumerate(case["panes"]):
            path = run / "fixtures" / case["id"] / str(index)
            path.mkdir(parents=True)
            for name in pane["names"]:
                target = path / name
                kind = spec["kinds"].get(name, "file")
                if kind == "directory":
                    target.mkdir()
                    (target / "child2").write_bytes(b"child fixture\n")
                elif kind == "symlink":
                    target.symlink_to(spec["targets"][name])
                else:
                    target.write_bytes(b"hidden filtering fixture\n")
            pane["path"] = str(path)
        yield case


def decode(text, inputs):
    if not inputs or not any(any(e["op"] == "toggle_hidden" for e in r["events"]) for r in inputs):
        raise ValueError("Filtering sequences must execute a toggle")
    results = [json.loads(line) for line in text.splitlines()]
    if [r.get("id") for r in results] != [r["id"] for r in inputs]:
        raise ValueError("Missing, extra or reordered filtering sequences")
    for request, result in zip(inputs, results):
        states = result.get("states", [])
        if result.get("status") != "ok" or len(states) != len(request["events"]) + 1:
            raise ValueError("Every filtering transition must return a complete state")
        show = request["show_hidden"]
        for index, state in enumerate(states):
            if index and request["events"][index - 1]["op"] == "toggle_hidden":
                show = not show
            if type(state.get("show_hidden")) is not bool or state["show_hidden"] != show or len(state.get("panes", [])) != 2:
                raise ValueError("Invalid shared filtering state")
            for pane, actual in zip(request["panes"], state["panes"]):
                expected = [n for n in pane["names"] if show or not n.startswith(".")] + ([".."] if pane["parent"] else [])
                visible, marks = actual["visible"], actual["marks"]
                if sorted(visible) != sorted(expected) or len(marks) != len(visible) or any(type(m) is not bool for m in marks):
                    raise ValueError("Invalid filtered rows/marks")
                if pane["parent"] and visible[0] != "..":
                    raise ValueError("Parent must remain first")
                focus = actual.get("focus_row")
                if visible and (type(focus) is not int or not 0 <= focus < len(visible) or actual["focused"] != visible[focus]):
                    raise ValueError("Invalid filtered focus")
                if not visible and (focus is not None or actual["focused"] is not None):
                    raise ValueError("An empty filtered listing cannot have focus")
                marked = [n for n, m in zip(visible, marks) if m]
                selected = marked or ([visible[focus]] if visible and visible[focus] != ".." else [])
                if ".." in marked or actual["marked"] != marked or actual["selected"] != selected or actual["marked_count"] != len(marked):
                    raise ValueError("Invalid filtered effective selection")
    return results


def run_suite(compare, reserved, rust_adapter):
    if platform.system() not in ("Linux", "Darwin"):
        raise ValueError("This contract covers Unix dotfiles only")
    acquisition.run(verify_only=True)
    run_id = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + uuid.uuid4().hex[:8]
    run = ROOT / "artifacts/runs" / ("W02b-" + run_id)
    run.mkdir(parents=True)
    spec = json.loads(SPEC.read_text())
    requests = list(materialize(run, spec, reserved))
    text = "".join(json.dumps(r) + "\n" for r in requests)
    (run / "requests.jsonl").write_text(text)
    jars = sorted((REFERENCE / "bundle").glob("*.jar")) + sorted((REFERENCE / "app").glob("*.jar"))
    cp = os.pathsep.join(str(p) for p in jars)
    classes = run / "classes"; classes.mkdir()
    _, compilation = capture(run, "javac", ["javac", "-proc:none", "-encoding", "UTF-8", "-cp", cp,
        "-d", str(classes), str(ROOT / "tools/java/W02Oracle.java"), str(SOURCE)])
    output, java = capture(run, "java", ["java", "--add-opens=java.base/java.io=ALL-UNNAMED", "-Djava.awt.headless=true",
        "-Duser.home=" + str(run / "home"), "-cp", str(classes) + os.pathsep + cp, "W02bOracle"], text)
    if "Caught exception while changing folder" in (run / "java.stderr").read_text():
        raise ValueError("Original reload worker caught an exception; this is an incomplete run")
    original = decode(output, requests)
    commands = dict(javac=compilation, java=java)
    differences = []
    if compare:
        actual, commands["rust"] = capture(run, "rust", [str(rust_adapter), "--adapter-w02b"], text)
        actual = decode(actual, requests)
        for expected, observed in zip(original, actual):
            for index, (left, right) in enumerate(zip(expected["states"], observed["states"])):
                if left != right:
                    differences.append(dict(id=expected["id"], state=index, expected=left, actual=right))
    for name, argv in (("java-version", ["java", "-version"]), ("javac-version", ["javac", "-version"]),
                       ("rustc-version", ["rustc", "-vV"])):
        _, commands[name] = capture(run, name, argv)
    acquisition.run(verify_only=True)
    files = [SPEC, SOURCE, ROOT / "tools/java/W02Oracle.java", ROOT / "tools/w02b.py", ROOT / "tools/w01.py",
        ROOT / "Cargo.lock", ROOT / "Cargo.toml", EVIDENCE / "contract.md"]
    files += sorted((ROOT / "crates").rglob("*.rs")) + sorted((ROOT / "crates").rglob("Cargo.toml"))
    reference_classes = []
    for jar_name, entries in (("app/mucommander-core-1.6.2.jar", ["com/mucommander/ui/main/ConfigurableFolderFilter",
        "com/mucommander/ui/main/table/FileTable", "com/mucommander/ui/main/table/FileTable$FolderChangeThread",
        "com/mucommander/ui/main/table/FileTableModel"]), ("bundle/mucommander-commons-file-1.6.2.jar", [
        "com/mucommander/commons/file/filter/AttributeFileFilter", "com/mucommander/commons/file/protocol/local/LocalFile"])):
        with zipfile.ZipFile(REFERENCE / jar_name) as archive:
            for entry in entries:
                reference_classes.append(dict(jar=jar_name, entry=entry + ".class", sha256=hashlib.sha256(archive.read(entry + ".class")).hexdigest()))
    snapshots = sum(len(result["states"]) for result in original)
    report = dict(schema_version=1, run_id=run_id, workflow="W02b-hidden-filtering", suite="reserved" if reserved else "predefined",
        status=("differences" if differences else "passed") if compare else "oracle_ready",
        scope="Original ConfigurableFolderFilter and FolderChangeThread.run; same-folder reload inputs supplied; full application dispatch excluded",
        source_revision=subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        source_status=subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True).splitlines(),
        host=dict(system=platform.system(), architecture=platform.machine(), platform=platform.platform()),
        source_inputs=[dict(path=p.relative_to(ROOT).as_posix(), sha256=digest(p)) for p in files],
        classpath=[dict(path=p.relative_to(ROOT).as_posix(), sha256=digest(p)) for p in jars],
        reference_classes=reference_classes, commands=commands,
        java_adapter_classes=[dict(path=p.relative_to(ROOT).as_posix(), sha256=digest(p)) for p in sorted(classes.glob("*.class"))],
        fixture_seed=spec["reserved_seed"] if reserved else spec["generated_seed"], oracle_sequences=len(requests),
        sequences_run=len(requests) if compare else 0, snapshots_checked=snapshots if compare else 0,
        pane_snapshots_checked=snapshots * 2 if compare else 0,
        toggles_executed=sum(e["op"] == "toggle_hidden" for r in requests for e in r["events"]),
        differences=differences, skipped=[], reference_unchanged=True, raw_run=run.relative_to(ROOT).as_posix(),
        request_sha256=digest(run / "requests.jsonl"),
        rust_adapter=dict(path="{repository}/" + rust_adapter.relative_to(ROOT).as_posix(), sha256=digest(rust_adapter)) if compare else None,
        reproduce="target/debug/mucrab-compare --suite w02b" + (" --reserved" if reserved else ""))
    acquisition.write_json(run / "report.json", report)
    acquisition.write_json(EVIDENCE / (("comparison-" if compare else "oracle-") + report["suite"] + ".json"), report)
    print("{}: {} sequences, {} global snapshots, {} differences; {}".format(report["status"], len(requests), snapshots, len(differences), run.relative_to(ROOT)))
    return int(bool(differences))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["oracle", "compare"])
    parser.add_argument("--reserved", action="store_true")
    parser.add_argument("--rust-adapter", type=lambda s: ROOT / s, default=ROOT / "target/debug/mucrab-compare")
    args = parser.parse_args()
    try:
        return run_suite(args.action == "compare", args.reserved, args.rust_adapter)
    except (ValueError, OSError, KeyError, TypeError, IndexError, subprocess.SubprocessError) as error:
        print("W02b unavailable: " + str(error), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
