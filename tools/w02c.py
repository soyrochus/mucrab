#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
"""Compare original metadata comparators/table transitions and native metadata."""

import argparse
from datetime import datetime, timezone
import json
import os
import platform
from pathlib import Path
import random
import subprocess
import sys
import uuid
import unicodedata

import acquire_mucommander as acquisition
from w01 import ROOT, REFERENCE, capture, digest, evidence_directory

SPEC = ROOT / "fixtures/W02c-metadata-sorting/cases.json"
SOURCE = ROOT / "tools/java/W02cOracle.java"
EVIDENCE = evidence_directory('W02c-metadata-sorting')
CRITERIA = ("NAME", "SIZE", "DATE", "EXTENSION")


def java_capture(run, argv, text, timeout=90):
    """Keep partial original output even if a VM stalls or crashes."""
    request = run / "requests.jsonl"
    stdout = run / "java.stdout"; stderr = run / "java.stderr"
    with request.open() as inp, stdout.open("w") as out, stderr.open("w") as err:
        process = subprocess.Popen(argv, cwd=ROOT, stdin=inp, stdout=out, stderr=err, text=True)
        try: code = process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            process.kill(); process.wait()
            acquisition.write_json(run / "incomplete.json", dict(status="unavailable", reason="original adapter timeout", command=argv))
            raise ValueError("Original adapter timed out; partial output retained at " + str(run))
    record = dict(argv=[str(s).replace(str(ROOT), "{repository}") for s in argv], exit_code=code,
        stdout=dict(path=stdout.relative_to(ROOT).as_posix(), sha256=digest(stdout)),
        stderr=dict(path=stderr.relative_to(ROOT).as_posix(), sha256=digest(stderr)))
    acquisition.write_json(run / "java.command.json", record)
    if code: raise ValueError("Original adapter failed; see " + str(stderr))
    return stdout.read_text(), record


def generated(spec, reserved):
    rng = random.Random(spec["reserved_seed"] if reserved else spec["generated_seed"])
    count = spec["reserved_sets"] if reserved else spec["generated_sets"]
    for i in range(count):
        entries = []
        for j in range(rng.randrange(5, 16)):
            entries.append(dict(name=rng.choice(["file", "File", "é", "中", ".hidden", "x"]) + str(j) + rng.choice(["", ".TXT", ".txt", ".İ", ".𐐀"]),
                is_directory=rng.choice([False, False, True]), is_symlink=False,
                metadata=dict(size=rng.choice(spec["size_values"]), modified_ms=rng.choice(spec["date_values"]), extension=rng.choice(spec["extensions"]))))
        yield dict(id=("reserved" if reserved else "generated") + "-" + str(i), entries=entries, parent=rng.choice([True, False]))


def events(count, parent):
    result = []
    if count:
        for row in sorted({0, count // 2, count - 1}):
            result.append(dict(op="mark", row=row + int(parent)))
        result.append(dict(op="focus", row=count // 2 + int(parent)))
        for criterion in CRITERIA:
            for ascending in (True, False):
                for folders in (True, False):
                    result.append(dict(op="sort", criterion=criterion, ascending=ascending, directories_first=folders))
    return result


def materialize(run, spec, reserved):
    sets = list(generated(spec, True)) if reserved else spec["sets"] + list(generated(spec, False))
    for item in sets:
        folder = run / "synthetic" / item["id"]; folder.mkdir(parents=True)
        entries = json.loads(json.dumps(item["entries"]))
        for index, entry in enumerate(entries):
            path = folder / str(index)
            if entry["is_directory"]: path.mkdir()
            else: path.write_bytes(b"metadata backing\n")
            entry["path"] = str(path)
        for mode in ("NATURAL", "LEXICOGRAPHIC"):
            transitions = events(len(entries), item["parent"])
            if item.get("pairs_only"): transitions = [e for e in transitions if e["op"] == "sort"]
            yield dict(id=item["id"] + "-" + mode.lower(), kind="synthetic", path=str(folder), entries=entries,
                mode=mode, parent=item["parent"], pairs_only=item.get("pairs_only", False), events=transitions)
    folder = run / "real"; folder.mkdir()
    for entry in spec["real_entries"]:
        path = folder / entry["name"]
        if entry["kind"] == "directory": path.mkdir()
        elif entry["kind"] == "symlink": path.symlink_to(entry["target"])
        else: path.write_bytes(bytes([entry["size"] % 251]) * entry["size"])
    for entry in spec["real_entries"]:
        if entry["kind"] != "symlink":
            ns = entry["modified_ms"] * 1_000_000
            os.utime(folder / entry["name"], ns=(ns, ns))
    for mode in ("NATURAL", "LEXICOGRAPHIC"):
        for parent in (True, False):
            yield dict(id="real-" + mode.lower() + "-" + str(parent), kind="directory", path=str(folder), mode=mode,
                parent=parent, names=[unicodedata.normalize("NFC", entry["name"]) if platform.system() == "Darwin" else entry["name"] for entry in spec["real_entries"]], events=events(len(spec["real_entries"]), parent))


def decode(text, requests):
    results = [json.loads(line) for line in text.splitlines()]
    if not requests or not any(e["op"] == "sort" for r in requests for e in r["events"]) or [r.get("id") for r in results] != [r["id"] for r in requests]:
        raise ValueError("Missing, extra or reordered metadata cases")
    for request, result in zip(requests, results):
        if result.get("status") != "ok" or len(result.get("states", [])) != len(request["events"]) + 1:
            raise ValueError("Incomplete metadata transitions")
        entries = result["entries"]; n = len(entries)
        if request["kind"] == "synthetic":
            expected = [{key: entry[key] for key in ("name", "is_directory", "is_symlink", "metadata")} for entry in request["entries"]]
            if entries != expected: raise ValueError("Synthetic metadata/identity fixture was not executed")
        elif sorted(e["name"] for e in entries) != sorted(request["names"]):
            raise ValueError("Real entry enumeration is incomplete")
        for entry in entries:
            meta = entry["metadata"]
            if (type(meta["size"]) is not int or type(meta["modified_ms"]) is not int
                    or not all(-(2**63) <= meta[k] < 2**63 for k in ("size", "modified_ms"))
                    or not (meta["extension"] is None or isinstance(meta["extension"], str))):
                raise ValueError("Invalid metadata fields")
        criterion, ascending, folders = "NAME", True, True
        for index, state in enumerate(result["states"]):
            if index:
                event = request["events"][index - 1]
                if event["op"] == "sort":
                    criterion, ascending, folders = event["criterion"], event["ascending"], event["directories_first"]
            if type(state["ascending"]) is not bool or type(state["directories_first"]) is not bool or (state["criterion"], state["ascending"], state["directories_first"], state["mode"]) != (criterion, ascending, folders, request["mode"]):
                raise ValueError("Invalid sort options")
            order, pairs = state["order"], state["pairs"]
            if len(pairs) != n or any(len(row) != n or any(type(sign) is not int or sign not in (-1, 0, 1) for sign in row) for row in pairs):
                raise ValueError("Invalid full permutation or pair matrix")
            if request.get("pairs_only"):
                if order is not None or state["selection"] is not None: raise ValueError("Extreme pair-only cases cannot claim ordered-list coverage")
                continue
            if sorted(order) != list(range(n)): raise ValueError("Invalid complete permutation")
            # Java wrapping subtraction can be non-transitive; do not substitute
            # generic sortedness/antisymmetry assertions for exact oracle output.
            selected = state["selection"]
            visible = ([".."] if request["parent"] else []) + [entries[i]["name"] for i in order]
            marks = selected["marks"]; focus = selected["focus_row"]
            if selected["visible"] != visible or len(marks) != len(visible) or any(type(mark) is not bool for mark in marks):
                raise ValueError("Invalid visible identities/marks")
            if visible and (type(focus) is not int or not 0 <= focus < len(visible) or selected["focused"] != visible[focus]):
                raise ValueError("Invalid focus")
            if not visible and (focus is not None or selected["focused"] is not None):
                raise ValueError("Empty listing has focus")
            marked = [name for name, mark in zip(visible, marks) if mark]
            effective = marked or ([visible[focus]] if visible and not (request["parent"] and focus == 0) else [])
            if ".." in marked or (selected["marked"], selected["marked_count"], selected["selected"]) != (marked, len(marked), effective):
                raise ValueError("Invalid effective selection")
    return results


def run_suite(compare, reserved, rust_adapter):
    if platform.system() not in ("Linux", "Darwin"):
        raise ValueError("W02c requires Linux or macOS")
    acquisition.run(verify_only=True)
    run_id = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + uuid.uuid4().hex[:8]
    run = ROOT / "artifacts/runs" / ("W02c-" + run_id); run.mkdir(parents=True)
    spec = json.loads(SPEC.read_text()); requests = list(materialize(run, spec, reserved))
    text = "".join(json.dumps(r) + "\n" for r in requests); (run / "requests.jsonl").write_text(text)
    jars = sorted((REFERENCE / "bundle").glob("*.jar")) + sorted((REFERENCE / "app").glob("*.jar"))
    cp = os.pathsep.join(map(str, jars)); classes = run / "classes"; classes.mkdir()
    _, compilation = capture(run, "javac", ["javac", "-proc:none", "-encoding", "UTF-8", "-cp", cp, "-d", str(classes), str(ROOT / "tools/java/W02Oracle.java"), str(SOURCE)])
    output, java = java_capture(run, ["java", "--add-opens=java.base/java.io=ALL-UNNAMED", "-Djava.awt.headless=true", "-Duser.home=" + str(run / "home"), "-cp", str(classes) + os.pathsep + cp, "W02cOracle"], text)
    original = decode(output, requests); commands = dict(javac=compilation, java=java); differences = []; enumeration_checks = 0
    if compare:
        inputs = [dict(r, entries=out["entries"], operation="run") for r, out in zip(requests, original)]
        actual_text, commands["rust"] = capture(run, "rust", [str(rust_adapter), "--adapter-w02c"], "".join(json.dumps(r) + "\n" for r in inputs))
        actual = decode(actual_text, inputs)
        for expected, observed in zip(original, actual):
            if expected["entries"] != observed["entries"]: differences.append(dict(id=expected["id"], component="metadata"))
            for index, (left, right) in enumerate(zip(expected["states"], observed["states"])):
                if left != right: differences.append(dict(id=expected["id"], state=index, expected=left, actual=right))
        real = [dict(r, operation="enumerate") for r in requests if r["kind"] == "directory"]
        enum_text, commands["rust-enumeration"] = capture(run, "rust-enumeration", [str(rust_adapter), "--adapter-w02c"], "".join(json.dumps(r) + "\n" for r in real))
        enumerated = [json.loads(line) for line in enum_text.splitlines()]
        if [r["id"] for r in enumerated] != [r["id"] for r in real]: raise ValueError("Incomplete native metadata enumeration")
        by_id = {r["id"]: r for r in original}
        for result in enumerated:
            enumeration_checks += 1
            if result.get("status") != "ok" or sorted(result["entries"], key=lambda e: e["name"]) != sorted(by_id[result["id"]]["entries"], key=lambda e: e["name"]):
                differences.append(dict(id=result["id"], component="native_metadata_enumeration"))
    for name, argv in (("java-version", ["java", "-version"]), ("javac-version", ["javac", "-version"]), ("rustc-version", ["rustc", "-vV"])):
        _, commands[name] = capture(run, name, argv)
    acquisition.run(verify_only=True)
    files = [SPEC, SOURCE, ROOT / "tools/java/W02Oracle.java", ROOT / "tools/w02c.py", ROOT / "tools/w01.py", ROOT / "Cargo.lock", ROOT / "Cargo.toml", ROOT / "evidence/workflows/W02c-metadata-sorting/contract.md"]
    if (ROOT / "tools/manifests/java-case-data.json").exists(): files.append(ROOT / "tools/manifests/java-case-data.json")
    files += sorted((ROOT / "crates").rglob("*.rs")) + sorted((ROOT / "crates").rglob("Cargo.toml"))
    states = sum(len(out["states"]) for out in original)
    report = dict(schema_version=1, run_id=run_id, workflow="W02c-metadata-sorting", suite="reserved" if reserved else "predefined", status=("differences" if differences else "passed") if compare else "oracle_ready",
        scope="Original FileComparator, FileTableModel sorting, FileTable focus/marks; explicit synthetic metadata fields and independent real enumeration; GUI excluded",
        source_revision=subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(), source_status=subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True).splitlines(),
        source_inputs=[dict(path=p.relative_to(ROOT).as_posix(), sha256=digest(p)) for p in files], host=dict(system=platform.system(), architecture=platform.machine(), platform=platform.platform()),
        classpath=[dict(path=p.relative_to(ROOT).as_posix(), sha256=digest(p)) for p in jars], java_adapter_classes=[dict(path=p.relative_to(ROOT).as_posix(), sha256=digest(p)) for p in sorted(classes.glob("*.class"))],
        commands=commands, fixture_seed=spec["reserved_seed"] if reserved else spec["generated_seed"], oracle_sequences=len(requests), sequences_run=len(requests) if compare else 0,
        snapshots_checked=states if compare else 0, pair_signs_checked=sum(len(out["entries"])**2 * len(out["states"]) for out in original) if compare else 0,
        pair_only_sequences=sum(r.get("pairs_only", False) for r in requests),
        ordered_lists_checked=sum(len(out["states"]) for r, out in zip(requests, original) if not r.get("pairs_only")) if compare else 0, enumeration_checks=enumeration_checks, differences=differences, skipped=[], reference_unchanged=True, raw_run=run.relative_to(ROOT).as_posix(), request_sha256=digest(run / "requests.jsonl"),
        rust_adapter=dict(path=str(rust_adapter).replace(str(ROOT), "{repository}"), sha256=digest(rust_adapter)) if compare else None,
        reproduce="target/debug/mucrab-compare --suite w02c" + (" --reserved" if reserved else ""))
    acquisition.write_json(run / "report.json", report); acquisition.write_json(EVIDENCE / (("comparison-" if compare else "oracle-") + report["suite"] + ".json"), report)
    print("{}: {} sequences, {} snapshots, {} differences; {}".format(report["status"], len(requests), states, len(differences), run.relative_to(ROOT)))
    return int(bool(differences))


def main():
    parser = argparse.ArgumentParser(description=__doc__); parser.add_argument("action", choices=["oracle", "compare"])
    parser.add_argument("--reserved", action="store_true"); parser.add_argument("--rust-adapter", type=Path, default=ROOT / "target/debug/mucrab-compare")
    args = parser.parse_args()
    try: return run_suite(args.action == "compare", args.reserved, args.rust_adapter.resolve())
    except (ValueError, OSError, KeyError, TypeError, IndexError, subprocess.SubprocessError) as error:
        print("W02c unavailable: " + str(error), file=sys.stderr); return 2


if __name__ == "__main__":
    sys.exit(main())
