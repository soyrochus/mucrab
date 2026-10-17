#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
"""Run original Java W01 behavior and, optionally, compare the Rust adapter."""

import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import platform
import random
import subprocess
import sys
import uuid
import zipfile

import acquire_mucommander as acquisition

ROOT = Path(__file__).resolve().parents[1]
REFERENCE = ROOT / "artifacts/reference/mucommander-1.6.2-1"
SPEC = ROOT / "fixtures/W01-local-browse-sort/cases.json"
SOURCE = ROOT / "tools/java/W01Oracle.java"
def evidence_directory(workflow):
    """Keep fresh Mac results beside, rather than over, Linux receipts."""
    base = ROOT / "evidence"
    if platform.system() == "Darwin":
        base = base / "platforms" / ("macos-" + platform.machine())
    destination = base / "workflows" / workflow
    destination.mkdir(parents=True, exist_ok=True)
    return destination


EVIDENCE = evidence_directory("W01-local-browse-sort")


def digest(path):
    return acquisition.sha256_file(path)


def capture(run, name, command, input_text=None):
    result = subprocess.run(command, cwd=ROOT, input=input_text, text=True,
                            capture_output=True, timeout=180)
    record = {"argv": [str(s).replace(str(ROOT), "{repository}") for s in command],
              "exit_code": result.returncode}
    for stream in ("stdout", "stderr"):
        path = run / (name + "." + stream)
        path.write_text(getattr(result, stream), encoding="utf-8")
        record[stream] = {"path": path.relative_to(ROOT).as_posix(), "sha256": digest(path)}
    acquisition.write_json(run / (name + ".command.json"), record)
    if result.returncode:
        raise ValueError("{} failed ({}); see {}".format(name, result.returncode, run))
    return result.stdout, record


def variants(case):
    for ascending in (True, False):
        for directories_first in (True, False):
            for mode in ("NATURAL", "LEXICOGRAPHIC"):
                yield dict(case, id="{}-{}-{}-{}".format(case["id"], ascending, directories_first, mode),
                           ascending=ascending, directories_first=directories_first, mode=mode, locale="en-US")


def generated_sets(seed, count):
    rng = random.Random(seed)
    prefixes = ["a", "A", "é", "é", "🦀", "中", "x", "", "pre-", ".", "α"]
    return [{"id": "generated-{}-{}".format(seed, i),
             "names": [rng.choice(prefixes) + rng.choice(["", "0", "00"]) + str(rng.randrange(120))
                       + rng.choice(["", "x2", "x10", ".txt", "é"]) for _ in range(14)],
             "directories": [0, 3, 9]} for i in range(count)]


def requests(run, spec, reserved):
    result = []
    sets = generated_sets(spec["reserved_seed"], 16) if reserved else (
        spec["synthetic_sets"] + generated_sets(spec["generated_seed"], 8))
    for item in sets:
        entries = []
        folder = run / "synthetic" / item["id"]
        folder.mkdir(parents=True)
        for i, name in enumerate(item["names"]):
            path = folder / str(i)
            directory = i in item.get("directories", [])
            if directory:
                path.mkdir()
            else:
                path.write_bytes(b"fixture\n")
            entries.append({"name": name, "is_directory": directory, "is_symlink": False, "path": str(path)})
        result.extend(variants({"id": item["id"], "kind": "synthetic", "entries": entries}))
    real = run / "browse"
    real.mkdir()
    for entry in spec["real_entries"]:
        path = real / entry["name"]
        if entry["kind"] == "directory":
            path.mkdir()
        elif entry["kind"] == "symlink":
            path.symlink_to(entry["target"])
        else:
            path.write_bytes(b"muCrab fixture\n")
    (real / "zFolder/child10").write_bytes(b"ten\n")
    (real / "zFolder/child2").write_bytes(b"two\n")
    for name, path in (("real", real), ("child", real / "zFolder"), ("empty-dir", real / "empty")):
        result.extend(variants({"id": name, "kind": "directory", "path": str(path)}))
    denied = run / "denied"
    denied.mkdir()
    denied.chmod(0)
    try:
        list(denied.iterdir())
    except PermissionError:
        pass
    else:
        denied.chmod(0o700)
        raise ValueError("Access-denied fixture is readable by this account; no passing result can be claimed")
    for name, path in (("missing", real / "absent"), ("not-directory", real / "file2.txt"), ("denied", denied)):
        result.append(dict(id=name, kind="directory", path=str(path), locale="en-US",
                           mode="NATURAL", ascending=True, directories_first=True))
    return result


def decode_responses(text, inputs):
    lines = [json.loads(line) for line in text.splitlines()]
    if len(lines) != len(inputs) or [line.get("id") for line in lines] != [i["id"] for i in inputs]:
        raise ValueError("Adapter returned incomplete, reordered, or extra responses")
    for line in lines:
        if line.get("status") == "error":
            if line.get("error") != "directory_unavailable":
                raise ValueError("Unexpected adapter error")
            continue
        if line.get("status") != "ok":
            raise ValueError("Adapter returned an unknown status")
        n = len(line["entries"])
        if sorted(line["order"]) != list(range(n)) or len(line["pairs"]) != n:
            raise ValueError("Invalid permutation or comparison matrix")
        if any(len(row) != n or any(sign not in (-1, 0, 1) for sign in row) for row in line["pairs"]):
            raise ValueError("Invalid comparison signs")
    return lines


def run_suite(compare, reserved, rust_adapter):
    acquisition.run(verify_only=True)
    run_id = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + uuid.uuid4().hex[:8]
    run = ROOT / "artifacts/runs" / ("W01-" + run_id)
    run.mkdir(parents=True)
    try:
        return execute_suite(compare, reserved, rust_adapter, run_id, run)
    finally:
        if (run / "denied").exists():
            (run / "denied").chmod(0o700)


def execute_suite(compare, reserved, rust_adapter, run_id, run):
    spec = json.loads(SPEC.read_text())
    inputs = requests(run, spec, reserved)
    input_text = "".join(json.dumps(item, ensure_ascii=False) + "\n" for item in inputs)
    (run / "requests.jsonl").write_text(input_text)
    jars = sorted((REFERENCE / "bundle").glob("*.jar")) + sorted((REFERENCE / "app").glob("*.jar"))
    classpath = os.pathsep.join(str(jar) for jar in jars)
    classes = run / "classes"
    classes.mkdir()
    _, compile_record = capture(run, "javac", ["javac", "-proc:none", "-encoding", "UTF-8", "-cp", classpath,
                                               "-d", str(classes), str(SOURCE)])
    commands = {"javac": compile_record}
    for name, cmd in (("java-version", ["java", "-version"]), ("javac-version", ["javac", "-version"]),
                      ("rustc-version", ["rustc", "-vV"])):
        _, commands[name] = capture(run, name, cmd)
    java_text, commands["java"] = capture(run, "java", ["java", "-Djava.awt.headless=true",
        "-Duser.home=" + str(run / "home"), "-cp", str(classes) + os.pathsep + classpath, "W01Oracle"], input_text)
    original = decode_responses(java_text, inputs)
    differences = []
    enumeration_checks = 0
    if compare:
        rust_inputs = []
        for item, observed in zip(inputs, original):
            # Compare actual enumerators independently; align order only at the comparator boundary.
            if item["kind"] == "directory":
                rust_inputs.append(dict(item, id=item["id"] + "-enumeration", operation="enumerate"))
            if observed["status"] == "ok":
                rust_inputs.append(dict(item, entries=observed["entries"], operation="compare"))
        rust_text, commands["rust"] = capture(run, "rust", [str(rust_adapter), "--adapter"],
            "".join(json.dumps(item, ensure_ascii=False) + "\n" for item in rust_inputs))
        rust_results = decode_responses(rust_text, rust_inputs)
        by_id = {item["id"]: item for item in rust_results}
        for item, expected in zip(inputs, original):
            if item["kind"] == "directory":
                enumeration_checks += 1
                actual = by_id[item["id"] + "-enumeration"]
                key = lambda entry: (entry["name"], entry["is_directory"], entry["is_symlink"])
                if expected["status"] != actual["status"] or (expected["status"] == "ok" and
                        sorted(map(key, expected["entries"])) != sorted(map(key, actual["entries"]))):
                    differences.append({"id": item["id"], "component": "enumeration"})
            if expected["status"] == "ok":
                actual = by_id[item["id"]]
                for component in ("status", "entries", "pairs", "order"):
                    if expected.get(component) != actual.get(component):
                        differences.append({"id": item["id"], "component": component})
    successful = [item for item in original if item["status"] == "ok"]
    if len(original) != len(inputs) or not successful:
        raise ValueError("No complete nonempty comparison run")
    if {item["id"] for item in original if item["status"] == "error"} != {"missing", "not-directory", "denied"}:
        raise ValueError("Unexpected oracle error outcomes")
    acquisition.run(verify_only=True)
    class_entry = "com/mucommander/commons/file/util/FileComparator.class"
    with zipfile.ZipFile(REFERENCE / "bundle/mucommander-commons-file-1.6.2.jar") as archive:
        import hashlib
        class_hash = hashlib.sha256(archive.read(class_entry)).hexdigest()
    tracked_inputs = [SPEC, SOURCE, ROOT / "tools/w01.py", ROOT / "Cargo.lock", ROOT / "Cargo.toml",
                      ROOT / "evidence/workflows/W01-local-browse-sort/contract.md"]
    for folder in (ROOT / "crates/core", ROOT / "crates/compare"):
        tracked_inputs.extend(sorted(folder.rglob("*.rs")))
        tracked_inputs.append(folder / "Cargo.toml")
    report = {
        "schema_version": 1, "run_id": run_id, "workflow": "W01-local-browse-sort",
        "scope": "original comparator, original table quicksort, and local enumeration; GUI excluded",
        "status": ("differences" if differences else "passed") if compare else "oracle_ready",
        "suite": "reserved" if reserved else "predefined", "locale": "en-US",
        "host": {"system": platform.system(), "architecture": platform.machine(), "platform": platform.platform()},
        "source_revision": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "source_status": subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True).splitlines(),
        "source_inputs": [{"path": p.relative_to(ROOT).as_posix(), "sha256": digest(p)} for p in tracked_inputs],
        "reference_class_sha256": class_hash,
        "classpath": [{"path": jar.relative_to(ROOT).as_posix(), "sha256": digest(jar)} for jar in jars],
        "commands": commands, "fixture_seed": spec["reserved_seed"] if reserved else spec["generated_seed"],
        "java_adapter_classes": [{"path": p.relative_to(ROOT).as_posix(), "sha256": digest(p)}
                                 for p in sorted(classes.glob("*.class"))],
        "rust_adapter": {"path": str(rust_adapter).replace(str(ROOT), "{repository}"),
                         "sha256": digest(rust_adapter)} if compare else None,
        "cases_run": len(inputs) if compare else 0, "oracle_cases_run": len(inputs),
        "ordered_lists_checked": len(successful) if compare else 0,
        "pair_signs_checked": sum(len(item["entries"]) ** 2 for item in successful) if compare else 0,
        "enumeration_checks": enumeration_checks,
        "error_cases": [item["id"] for item in original if item["status"] == "error"],
        "differences": differences, "skipped_cases": [],
        "raw_run": run.relative_to(ROOT).as_posix(),
        "request_sha256": digest(run / "requests.jsonl"),
        "reference_unchanged": True, "gui_verified": False,
        "limitations": ["One host and en-US only; synthetic labels are distinguished from real entries",
                        "Identical input order at comparator boundary; enumeration sets checked independently",
                        "No GUI interaction or file mutation is established by this report"]}
    acquisition.write_json(run / "report.json", report)
    output = EVIDENCE / (("comparison-" if compare else "oracle-") + report["suite"] + ".json")
    acquisition.write_json(output, report)
    print(json.dumps({key: report[key] for key in ("status", "run_id", "suite", "cases_run", "oracle_cases_run",
        "ordered_lists_checked", "pair_signs_checked", "enumeration_checks", "differences")}))
    print("Evidence: " + output.relative_to(ROOT).as_posix())
    return 1 if differences else 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["oracle", "compare"])
    parser.add_argument("--reserved", action="store_true")
    parser.add_argument("--rust-adapter", type=Path, default=ROOT / "target/debug/mucrab-compare")
    args = parser.parse_args()
    try:
        return run_suite(args.action == "compare", args.reserved, args.rust_adapter.resolve())
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        print("W01 unavailable: {}".format(error), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
