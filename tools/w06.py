#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
"""Compare actual muCommander ZIP listings/extraction with isolated native trees."""
import argparse
import copy
from datetime import datetime, timezone
import io
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
from w01 import ROOT, REFERENCE, capture, digest
from w02c import java_capture
from w03 import materialize_tree, snapshot

SPEC = ROOT / "fixtures/W06-zip-browsing-extraction/cases.json"
EVIDENCE = ROOT / "evidence/workflows/W06-zip-browsing-extraction"
SOURCE = ROOT / "tools/java/W06Oracle.java"


def payload(entry):
    if "text" in entry:
        return entry["text"].encode()
    size = entry.get("size", 0)
    pattern = bytes((i + entry.get("pattern", 3)) % 251 for i in range(251))
    return (pattern * ((size + 250) // 251))[:size]


def make_zip(path, item, spec):
    output = io.BytesIO()
    with zipfile.ZipFile(output, "w") as archive:
        archive.comment = item.get("comment", "").encode()
        for entry in item["entries"]:
            info = zipfile.ZipInfo(entry["path"], tuple(entry.get("date", spec["zip_date"])))
            info.create_system = 3
            directory = entry["path"].endswith("/")
            info.external_attr = ((0o040000 if directory else 0o100000) | entry.get("mode", 0o755 if directory else 0o644)) << 16
            if directory:
                info.external_attr |= 0x10
            info.compress_type = zipfile.ZIP_STORED if entry.get("method", "deflate") == "stored" else zipfile.ZIP_DEFLATED
            archive.writestr(info, payload(entry))
    data = output.getvalue()
    if item.get("corrupt") == "header":
        data = b"not a ZIP archive\n"
    elif item.get("corrupt") == "truncated":
        data = data[:-17]
    path.write_bytes(data)
    os.utime(path, ns=(spec["default_modified_ms"] * 1_000_000,) * 2)


def generated(spec, reserved):
    rng = random.Random(spec["reserved_seed"] if reserved else spec["generated_seed"])
    for i in range(spec["reserved_sets"] if reserved else spec["generated_sets"]):
        entries = []
        for j in range(rng.randrange(3, 10)):
            path = rng.choice(["", "folder/", "folder/nested/", ".hidden/"]) + rng.choice(["File", "é", "two words", "中", ".dot"]) + str(j) + ".bin"
            entries.append(dict(path=path, size=rng.choice([0, 1, 17, 4097, 65539]), pattern=rng.randrange(251),
                                mode=rng.choice([0o444, 0o600, 0o644, 0o751]), method=rng.choice(["stored", "deflate"])))
        yield dict(id=("reserved" if reserved else "generated") + f"-{i}", entries=entries,
                   prefix="folder/" if i % 3 == 0 and any(e["path"].startswith("folder/") for e in entries) else "",
                   destination=[dict(path=entries[0]["path"], kind="file", size=17, pattern=250)])


def cases(spec, reserved):
    items = list(generated(spec, True)) if reserved else spec["cases"] + list(generated(spec, False))
    for item in items:
        for policy in item.get("policies", spec["policies"]):
            yield dict(item, id=item["id"] + "-" + policy.lower(), policy=policy)


def prepare(run, side, items, spec):
    requests, before = [], []
    for item in items:
        folder = run / side / item["id"]
        folder.mkdir(parents=True)
        archive = folder / "fixture.zip"
        make_zip(archive, item, spec)
        destination = folder / "destination"
        materialize_tree(destination, item.get("destination", []), spec)
        os.chmod(destination, item.get("destination_mode", 0o755))
        if item.get("destination_mode") == 0o555 and os.access(destination, os.W_OK):
            raise ValueError("Destination permission fixture is writable")
        paths = [""]
        for entry in item["entries"]:
            parts = entry["path"].rstrip("/").split("/")
            for count in range(1, len(parts) + int(entry["path"].endswith("/"))):
                prefix = "/".join(parts[:count]) + "/"
                if prefix not in paths:
                    paths.append(prefix)
        paths.extend(item.get("extra_list_paths", []))
        request = dict(id=item["id"], archive=str(archive), destination=str(destination), prefix=item.get("prefix", ""),
                       selected=item.get("selected"), list_paths=paths, policy=item["policy"], extract=item.get("extract", True))
        requests.append(request)
        before.append(dict(id=item["id"], archive_sha256=digest(archive), destination=snapshot(destination), root_mode=destination.stat().st_mode & 0o777))
    return requests, before


def decode(text, requests):
    results = [json.loads(line) for line in text.splitlines()]
    if not requests or [r.get("id") for r in results] != [r["id"] for r in requests]:
        raise ValueError("Missing, reordered or zero ZIP cases")
    for request, result in zip(requests, results):
        if result.get("status") == "error":
            if result != dict(id=request["id"], status="error", error="archive_unavailable"):
                raise ValueError("Malformed ZIP error")
            continue
        if result.get("status") != "ok" or type(result.get("success")) is not bool or type(result.get("extracted")) is not bool or result["extracted"] != request["extract"]:
            raise ValueError("Malformed ZIP execution result")
        listings = result.get("listings", [])
        if [entry.get("path") for entry in listings] != request["list_paths"]:
            raise ValueError("Missing or reordered ZIP folder listings")
        for listing in listings:
            if listing.get("status") not in ("ok", "error") or not isinstance(listing.get("entries"), list):
                raise ValueError("Malformed ZIP listing")
            if listing["status"] == "error" and listing["entries"]:
                raise ValueError("Failed ZIP listing contains entries")
            for entry in listing["entries"]:
                metadata = entry.get("metadata", {})
                if not isinstance(entry.get("name"), str) or any(type(entry.get(key)) is not bool for key in ("is_directory", "is_symlink")) or any(type(metadata.get(key)) is not int for key in ("size", "modified_ms")) or (metadata.get("extension") is not None and type(metadata["extension"]) is not str):
                    raise ValueError("Incomplete ZIP entry metadata")
    return results


def observe(requests, before):
    observations = []
    for request, old in zip(requests, before):
        destination = Path(request["destination"])
        if digest(Path(request["archive"])) != old["archive_sha256"]:
            raise ValueError("Extraction changed the source archive")
        tree = snapshot(destination)
        # UnpackJob leaves directory dates at creation/last child-write time.
        for entry in tree:
            if entry["kind"] == "directory":
                entry.pop("modified_ms", None)
        observations.append(dict(id=request["id"], archive_sha256=old["archive_sha256"], destination=tree, root_mode=destination.stat().st_mode & 0o777))
    return observations


def comparable(value):
    # Original ZipEntry.dosToJavaTime leaves Calendar's current milliseconds
    # intact. DOS timestamps have two-second precision; raw values are retained.
    value = copy.deepcopy(value)
    if isinstance(value, dict):
        for key, item in value.items():
            value[key] = item // 1000 * 1000 if key == "modified_ms" else comparable(item)
    elif isinstance(value, list):
        value = [comparable(item) for item in value]
    return value


def run_suite(compare, reserved, rust_adapter):
    if platform.system() != "Linux" or os.geteuid() == 0:
        raise ValueError("W06 comparisons require unprivileged Linux")
    acquisition.run(verify_only=True)
    run_id = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + uuid.uuid4().hex[:8]
    run = ROOT / "artifacts/runs" / ("W06-" + run_id)
    run.mkdir(parents=True)
    spec = json.loads(SPEC.read_text())
    items = list(cases(spec, reserved))
    requests, before = prepare(run, "original", items, spec)
    acquisition.write_json(run / "original-before.json", before)
    (run / "requests.jsonl").write_text("".join(json.dumps(r) + "\n" for r in requests))
    jars = sorted((REFERENCE / "bundle").glob("*.jar")) + sorted((REFERENCE / "app").glob("*.jar"))
    cp = os.pathsep.join(map(str, jars))
    classes = run / "classes"
    classes.mkdir()
    _, compiler = capture(run, "javac", ["javac", "-proc:none", "-encoding", "UTF-8", "-cp", cp, "-d", str(classes), str(SOURCE)])
    output, java = java_capture(run, ["java", "--add-opens=java.base/java.io=ALL-UNNAMED", "-Djava.awt.headless=true", "-Duser.home=" + str(run / "home"), "-cp", str(classes) + os.pathsep + cp, "W06Oracle"], "", timeout=120)
    original = decode(output, requests)
    after = observe(requests, before)
    acquisition.write_json(run / "original-after.json", after)
    if not any(r.get("success") for r in original):
        raise ValueError("No successful original ZIP extraction")
    commands = dict(javac=compiler, java=java)
    differences = []
    if compare:
        native_requests, native_before = prepare(run, "rust", items, spec)
        if before != native_before:
            raise ValueError("Initial ZIP fixtures differ")
        acquisition.write_json(run / "rust-before.json", native_before)
        text = "".join(json.dumps(r) + "\n" for r in native_requests)
        output, commands["rust"] = capture(run, "rust", [str(rust_adapter), "--adapter-w06"], text)
        native = decode(output, native_requests)
        native_after = observe(native_requests, native_before)
        acquisition.write_json(run / "rust-after.json", native_after)
        for left, right in zip(original, native):
            if comparable(left) != comparable(right):
                differences.append(dict(id=left["id"], component="listing_extraction", expected=left, actual=right))
        for left, right in zip(after, native_after):
            if comparable(left) != comparable(right):
                differences.append(dict(id=left["id"], component="filesystem", expected=left, actual=right))
    acquisition.run(verify_only=True)
    files = [SPEC, SOURCE, Path(__file__), ROOT / "tools/w03.py", ROOT / "tools/w02c.py", ROOT / "tools/w01.py", ROOT / "Cargo.toml", ROOT / "Cargo.lock"]
    if (EVIDENCE / "contract.md").exists():
        files.append(EVIDENCE / "contract.md")
    files += sorted((ROOT / "crates").rglob("*.rs")) + sorted((ROOT / "crates").rglob("Cargo.toml"))
    report = dict(schema_version=1, workflow="W06-zip-browsing-extraction", run_id=run_id,
        suite="reserved" if reserved else "predefined", status=("differences" if differences else "passed") if compare else "oracle_ready",
        scope="Original ZIP provider/archive tree and UnpackJob processFile with fixed collision policy; full GUI lifecycle separate",
        normalization="DOS-derived modified times compared at whole-second precision: original Calendar retains nondeterministic current milliseconds. Directory dates omitted; all raw observations retained.",
        source_revision=subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        source_status=subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True).splitlines(),
        source_inputs=[dict(path=str(p.relative_to(ROOT)), sha256=digest(p)) for p in files],
        classpath=[dict(path=str(p.relative_to(ROOT)), sha256=digest(p)) for p in jars], commands=commands,
        host=dict(system=platform.system(), architecture=platform.machine(), uid=os.geteuid(), locale="en-US", timezone="UTC", umask="022"),
        fixture_seed=spec["reserved_seed"] if reserved else spec["generated_seed"], oracle_cases=len(items), cases_run=len(items) if compare else 0,
        listings_checked=sum(len(r.get("listings", [])) for r in original) if compare else 0,
        entries_checked=sum(len(listing["entries"]) for r in original for listing in r.get("listings", [])) if compare else 0,
        extraction_jobs=sum(r.get("extracted", False) for r in original) if compare else 0,
        tree_checks=len(items) if compare else 0, archive_integrity_checks=len(items) * (2 if compare else 1),
        differences=differences, skipped=[], reference_unchanged=True, raw_run=str(run.relative_to(ROOT)),
        request_sha256=digest(run / "requests.jsonl"), original_after_sha256=digest(run / "original-after.json"),
        rust_adapter=dict(path=str(rust_adapter).replace(str(ROOT), "{repository}"), sha256=digest(rust_adapter)) if compare else None,
        reproduce="target/debug/mucrab-compare --suite w06" + (" --reserved" if reserved else ""))
    acquisition.write_json(run / "report.json", report)
    acquisition.write_json(EVIDENCE / (("comparison-" if compare else "oracle-") + report["suite"] + ".json"), report)
    print(f"{report['status']}: {len(items)} ZIP cases, {len(differences)} differences; {run.relative_to(ROOT)}")
    return int(bool(differences))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["oracle", "compare"])
    parser.add_argument("--reserved", action="store_true")
    parser.add_argument("--rust-adapter", type=Path, default=ROOT / "target/debug/mucrab-compare")
    args = parser.parse_args()
    mask = os.umask(0o022)
    try:
        return run_suite(args.action == "compare", args.reserved, args.rust_adapter.resolve())
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
        print("W06 unavailable: " + str(error), file=sys.stderr)
        return 2
    finally:
        os.umask(mask)


if __name__ == "__main__":
    sys.exit(main())
