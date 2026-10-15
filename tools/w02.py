#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
"""Compare original binary selection transitions with the Rust implementation."""

import argparse
from datetime import datetime, timezone
import json
import os
import platform
import random
import subprocess
import sys
import uuid
import zipfile
import hashlib

import acquire_mucommander as acquisition
from w01 import ROOT, REFERENCE, capture, digest

SPEC = ROOT / "fixtures/W02-selection-marking/cases.json"
SOURCE = ROOT / "tools/java/W02Oracle.java"
EVIDENCE = ROOT / "evidence/workflows/W02-selection-marking"


def generated(spec, reserved):
    rng = random.Random(spec["reserved_seed"] if reserved else spec["generated_seed"])
    for i in range(spec["reserved_sequences"] if reserved else spec["generated_sequences"]):
        names = ["file10.txt", "folder2", "file2.txt", ".hidden", "folder10", "Alpha.txt", "link-file"][:rng.randrange(1, 8)]
        parent = rng.choice([True, False])
        events = []
        for _ in range(spec["events_per_sequence"]):
            op = rng.choice(["click", "click", "space", "release", "next", "previous", "first", "last",
                             "mark_next", "mark_previous", "all", "none", "invert", "sort"])
            event = {"op": op}
            if op == "click":
                event.update(row=rng.randrange(len(names) + int(parent)), ctrl=rng.choice([True, False]), shift=rng.choice([True, False]))
            elif op == "sort":
                event.update(ascending=rng.choice([True, False]), directories_first=rng.choice([True, False]), mode=rng.choice(["NATURAL", "LEXICOGRAPHIC"]))
            events.append(event)
        yield dict(id=("reserved" if reserved else "generated") + "-" + str(i), names=names, parent=parent, events=events)


def materialize(run, spec, reserved):
    cases = list(generated(spec, reserved)) if reserved else spec["cases"] + list(generated(spec, False))
    for case in cases:
        folder = run / "fixtures" / case["id"]
        folder.mkdir(parents=True)
        for name in case["names"]:
            path = folder / name
            kind = spec["kinds"].get(name, "file")
            if kind == "directory":
                path.mkdir()
            elif kind == "symlink":
                path.symlink_to("file2.txt")
            else:
                path.write_bytes(b"selection fixture\n")
        case = dict(case, path=str(folder))
        yield case


def decode(text, inputs):
    if not inputs or not any(request["events"] for request in inputs):
        raise ValueError("No nonempty selection sequences")
    results = [json.loads(line) for line in text.splitlines()]
    if [r.get("id") for r in results] != [r["id"] for r in inputs]:
        raise ValueError("Missing, extra or reordered adapter cases")
    for request, result in zip(inputs, results):
        if result.get("status") != "ok" or len(result.get("states", [])) != len(request["events"]) + 1:
            raise ValueError("Adapter did not execute every transition")
        expected_names = sorted(request["names"] + ([".."] if request["parent"] else []))
        for state in result["states"]:
            visible, marks = state["visible"], state["marks"]
            if sorted(visible) != expected_names or len(marks) != len(visible) or any(type(m) is not bool for m in marks):
                raise ValueError("Invalid visible rows/marks")
            focus = state.get("focus_row")
            if visible and (type(focus) is not int or not 0 <= focus < len(visible) or state.get("focused") != visible[focus]):
                raise ValueError("Invalid focus")
            if not visible and (focus is not None or state.get("focused") is not None):
                raise ValueError("Empty listing has focus")
            marked = [name for name, mark in zip(visible, marks) if mark]
            selected = marked or ([visible[focus]] if visible and visible[focus] != ".." else [])
            if state["marked"] != marked or state["marked_count"] != len(marked) or state["selected"] != selected or ".." in marked:
                raise ValueError("Invalid effective selection/count")
    return results


def run_suite(compare, reserved, rust_adapter):
    acquisition.run(verify_only=True)
    run_id = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + uuid.uuid4().hex[:8]
    run = ROOT / "artifacts/runs" / ("W02-" + run_id)
    run.mkdir(parents=True)
    spec = json.loads(SPEC.read_text())
    inputs = list(materialize(run, spec, reserved))
    text = "".join(json.dumps(r) + "\n" for r in inputs)
    (run / "requests.jsonl").write_text(text)
    jars = sorted((REFERENCE / "bundle").glob("*.jar")) + sorted((REFERENCE / "app").glob("*.jar"))
    classpath = os.pathsep.join(str(p) for p in jars)
    classes = run / "classes"; classes.mkdir()
    _, compile_record = capture(run, "javac", ["javac", "-proc:none", "-encoding", "UTF-8", "-cp", classpath, "-d", str(classes), str(SOURCE)])
    java_text, java_record = capture(run, "java", ["java", "--add-opens=java.base/java.io=ALL-UNNAMED", "-Djava.awt.headless=true",
        "-Duser.home=" + str(run / "home"), "-cp", str(classes) + os.pathsep + classpath, "W02Oracle"], text)
    original = decode(java_text, inputs)
    differences = []
    commands = {"javac": compile_record, "java": java_record}
    for name, cmd in (("java-version", ["java", "-version"]), ("rustc-version", ["rustc", "-vV"])):
        _, commands[name] = capture(run, name, cmd)
    if compare:
        rust_text, commands["rust"] = capture(run, "rust", [str(rust_adapter), "--adapter-w02"], text)
        actual = decode(rust_text, inputs)
        for expected, observed in zip(original, actual):
            for i, (left, right) in enumerate(zip(expected["states"], observed["states"])):
                if left != right:
                    differences.append(dict(id=expected["id"], state=i, expected=left, actual=right))
    acquisition.run(verify_only=True)
    files = [SPEC, SOURCE, ROOT / "tools/w02.py", ROOT / "Cargo.lock", ROOT / "Cargo.toml", EVIDENCE / "contract.md"]
    files += sorted((ROOT / "crates").rglob("*.rs")) + sorted((ROOT / "crates").rglob("Cargo.toml"))
    class_hashes = []
    with zipfile.ZipFile(REFERENCE / "app/mucommander-core-1.6.2.jar") as archive:
        for entry in archive.namelist():
            if entry.endswith(".class") and (entry.startswith("com/mucommander/ui/main/table/FileTable") or
                entry.startswith("com/mucommander/ui/action/impl/Mark") or entry.startswith("com/mucommander/ui/action/impl/Select") or
                entry.startswith("com/mucommander/ui/action/impl/UnmarkAll") or entry.startswith("com/mucommander/ui/action/impl/InvertSelection")):
                class_hashes.append(dict(entry=entry, sha256=hashlib.sha256(archive.read(entry)).hexdigest()))
    report = dict(schema_version=1, run_id=run_id, workflow="W02-selection-marking", suite="reserved" if reserved else "predefined",
        status=("differences" if differences else "passed") if compare else "oracle_ready",
        scope="isolated original binary methods and actions; normalized post-Swing mouse events; full GUI dispatch excluded",
        source_revision=subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        source_status=subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True).splitlines(),
        host=dict(system=platform.system(), architecture=platform.machine(), platform=platform.platform()),
        source_inputs=[dict(path=p.relative_to(ROOT).as_posix(), sha256=digest(p)) for p in files],
        classpath=[dict(path=p.relative_to(ROOT).as_posix(), sha256=digest(p)) for p in jars],
        reference_classes=class_hashes, java_adapter_classes=[dict(path=p.relative_to(ROOT).as_posix(), sha256=digest(p)) for p in sorted(classes.glob("*.class"))],
        commands=commands, fixture_seed=spec["reserved_seed"] if reserved else spec["generated_seed"],
        rust_adapter=dict(path=str(rust_adapter).replace(str(ROOT), "{repository}"), sha256=digest(rust_adapter)) if compare else None,
        sequences_run=len(inputs) if compare else 0, oracle_sequences=len(inputs),
        snapshots_checked=sum(len(r["states"]) for r in original) if compare else 0,
        events_executed=sum(len(r["events"]) for r in inputs), differences=differences, skipped=[], reference_unchanged=True,
        raw_run=run.relative_to(ROOT).as_posix(), request_sha256=digest(run / "requests.jsonl"),
        reproduce="target/debug/mucrab-compare --suite w02" + (" --reserved" if reserved else ""))
    name = ("comparison-" if compare else "oracle-") + report["suite"] + ".json"
    acquisition.write_json(run / "report.json", report)
    acquisition.write_json(EVIDENCE / name, report)
    print("{}: {} oracle sequences, {} compared snapshots, {} differences; {}".format(report["status"], report["oracle_sequences"], report["snapshots_checked"], len(differences), run.relative_to(ROOT)))
    return 1 if differences else 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["oracle", "compare"])
    parser.add_argument("--reserved", action="store_true")
    parser.add_argument("--rust-adapter", type=lambda s: ROOT / s, default=ROOT / "target/debug/mucrab-compare")
    args = parser.parse_args()
    try:
        return run_suite(args.action == "compare", args.reserved, args.rust_adapter)
    except (ValueError, OSError, KeyError, TypeError, IndexError, subprocess.SubprocessError) as error:
        print("W02 unavailable: " + str(error), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
