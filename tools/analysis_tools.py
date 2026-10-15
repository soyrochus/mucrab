#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
"""Set up local analysis tools and record their static JAR capabilities."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import zipfile

import acquire_mucommander as acquisition


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "tools/manifests/analysis-tools.json"
REA_PROJECT = ROOT / "tools/rea"
REA_PACKAGE = REA_PROJECT / "node_modules/rea-agents"
OUTPUT = ROOT / "artifacts/analysis/toolchain"


def read_manifest():
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    if manifest["schema_version"] != 1:
        raise ValueError("Unsupported analysis-tool manifest version")
    return manifest


def require_runtime(command):
    path = shutil.which(command)
    if path is None:
        raise ValueError("Required runtime is missing from PATH: {}".format(command))
    return path


def supported_node(version):
    match = re.fullmatch(r"v?(\d+)\.(\d+)\.(\d+)", version.strip())
    if match is None:
        return False
    major, minor, _ = map(int, match.groups())
    return (major == 22 and minor >= 19) or (major == 24 and minor >= 11) or major >= 26


def check_lock(manifest):
    package = json.loads((REA_PROJECT / "package.json").read_text())
    lock = json.loads((REA_PROJECT / "package-lock.json").read_text())
    expected = manifest["rea"]
    if package["dependencies"].get("rea-agents") != expected["version"]:
        raise ValueError("REA dependency must use the exact pinned version")
    locked = lock["packages"]["node_modules/rea-agents"]
    if (locked["version"], locked["resolved"], locked["integrity"]) != (
        expected["version"], expected["tarball_url"], expected["integrity"],
    ):
        raise ValueError("REA lockfile does not match the pinned registry artifact")


def verify_tools(manifest):
    check_lock(manifest)
    for name in ("node", "java", "javap"):
        require_runtime(name)
    for relative, expected in manifest["rea"]["runtime_files_sha256"].items():
        if acquisition.sha256_file(REA_PACKAGE / relative) != expected:
            raise ValueError("Installed REA runtime file differs: {}".format(relative))
    cfr = ROOT / "tools/.cache/cfr" / manifest["cfr"]["filename"]
    acquisition.verify_archive(cfr, manifest["cfr"])
    if hashlib.md5(cfr.read_bytes()).hexdigest() != manifest["cfr"]["publisher_md5"]:
        raise ValueError("CFR does not match the author's published MD5")
    for name in ("rea", "cfr"):
        notice = ROOT / "third_party" / name / "LICENSE"
        if acquisition.sha256_file(notice) != manifest[name]["license_sha256"]:
            raise ValueError("Preserved {} tool license differs".format(name))
    if (REA_PACKAGE / "LICENSE").read_bytes() != (ROOT / "third_party/rea/LICENSE").read_bytes():
        raise ValueError("Installed REA license differs from the preserved copy")
    return cfr


def setup(manifest, offline=False):
    node = require_runtime("node")
    version = subprocess.check_output([node, "--version"], text=True).strip()
    if not supported_node(version):
        raise ValueError("Node {} does not satisfy {}".format(version, manifest["runtimes"]["node_engine"]))
    check_lock(manifest)
    command = [
        require_runtime("npm"), "ci", "--ignore-scripts", "--no-audit", "--no-fund",
        "--cache", str(ROOT / "tools/.cache/npm"), "--registry", "https://registry.npmjs.org/",
    ]
    if offline:
        command.append("--offline")
    subprocess.run(command, cwd=str(REA_PROJECT), check=True, timeout=300)
    cfr = ROOT / "tools/.cache/cfr" / manifest["cfr"]["filename"]
    if offline:
        acquisition.verify_archive(cfr, manifest["cfr"])
    else:
        acquisition.acquire_archive(cfr, manifest["cfr"])
    verify_tools(manifest)
    print("Pinned REA and CFR are installed locally. Run the probe to check their capabilities.")


def capture(name, command, expected_exit=0):
    OUTPUT.mkdir(parents=True, exist_ok=True)
    result = subprocess.run(command, cwd=str(ROOT), capture_output=True, text=True, timeout=180)
    record = {"argv": command, "exit_code": result.returncode}
    for stream in ("stdout", "stderr"):
        path = OUTPUT / (name + "." + stream)
        path.write_text(getattr(result, stream), encoding="utf-8")
        record[stream] = {
            "path": path.relative_to(ROOT).as_posix(),
            "size_bytes": path.stat().st_size,
            "sha256": acquisition.sha256_file(path),
        }
    acquisition.write_json(OUTPUT / (name + ".command.json"), record)
    if result.returncode != expected_exit:
        raise ValueError("{} exited {}; expected {}. See {}".format(name, result.returncode, expected_exit, OUTPUT))
    record["argv"] = [part.replace(str(ROOT), "{repository}") for part in command]
    return result, record


def inventory_facts(evidence, jar_digest, class_entry, class_digest):
    if evidence["subject"]["digest"]["sha256"] != jar_digest:
        raise ValueError("REA inspected a different JAR")
    substep = next(step for step in evidence["normalized_result"]["substeps"] if step["operation"] == "inventory_artifact")
    inventory = substep["evidence"]["normalized_result"]
    occurrence = next(item for item in inventory["occurrences"] if item["logical_path"] == class_entry)
    node = next(item for item in inventory["nodes"] if item["artifact_id"] == occurrence["artifact_id"])
    if node["sha256"] != class_digest or occurrence["hash_status"] != "verified":
        raise ValueError("REA class fingerprint differs from the original JAR entry")
    return {
        "evidence_id": evidence["evidence_id"],
        "provider": evidence["provider"],
        "jar_sha256": jar_digest,
        "coverage": inventory["coverage"],
        "coverage_status": inventory["coverage_status"],
        "limitations": inventory["limitations"],
        "occurrence_count": len(inventory["occurrences"]),
        "class_entry": class_entry,
        "class_sha256": node["sha256"],
        "class_artifact_format": occurrence["artifact_format"],
        "class_hash_status": occurrence["hash_status"],
        "jvm_semantic_analysis": "not_provided_by_this_inventory",
    }


def android_boundary(error):
    details = error.get("details", {})
    if (
        error.get("code") != "capability_unavailable"
        or details.get("provider_id") != "jadx"
        or details.get("operation") != "inspect_android_package"
        or "provide one standalone APK" not in details.get("reason", "")
    ):
        raise ValueError("REA failed for a reason other than the expected APK-only target boundary")
    return {
        "code": error["code"],
        "category": error["category"],
        "provider_id": details["provider_id"],
        "operation": details["operation"],
        "reason": details["reason"].replace(str(ROOT), "{repository}"),
        "expected_rejection": True,
    }


def probe(manifest):
    cfr = verify_tools(manifest)
    acquisition.run(verify_only=True)
    commands = {}
    versions = {}
    for name, command in (
        ("node", ["node", "--version"]), ("npm", ["npm", "--version"]),
        ("java", ["java", "-version"]), ("javap", ["javap", "-version"]),
        ("rea", ["node", str(REA_PACKAGE / "scripts/rea.mjs"), "--version"]),
        ("cfr", ["java", "-jar", str(cfr), "--version"]),
    ):
        result, commands[name] = capture(name + "-version", command)
        versions[name] = (result.stdout + result.stderr).strip()
    if not supported_node(versions["node"]) or versions["rea"] != manifest["rea"]["version"]:
        raise ValueError("Node or REA runtime version does not match the selected policy")
    if versions["cfr"] != "CFR " + manifest["cfr"]["version"]:
        raise ValueError("Unexpected CFR runtime version")
    java_match = re.search(r'version "(\d+)', versions["java"])
    if java_match is None or int(java_match.group(1)) < manifest["runtimes"]["minimum_java_major"]:
        raise ValueError("A full JDK 17 or newer is required")
    if int(versions["javap"].split(".")[0]) != int(java_match.group(1)):
        raise ValueError("java and javap must use the same JDK major version")

    reference_manifest = json.loads(acquisition.MANIFEST.read_text())
    reference = ROOT / "artifacts/reference" / ("mucommander-" + reference_manifest["release_tag"])
    selected = manifest["probe"]
    jar = reference / selected["jar"]
    with zipfile.ZipFile(jar) as archive:
        bytecode = archive.read(selected["class_entry"])
    class_digest = hashlib.sha256(bytecode).hexdigest()
    class_path = OUTPUT / "FileComparator.class"
    class_path.write_bytes(bytecode)
    rea = ["node", str(REA_PACKAGE / "scripts/rea.mjs")]
    result, commands["rea_inventory"] = capture("rea-jar-inventory", rea + ["inspect-artifact", str(jar), "--json"])
    inventory = inventory_facts(json.loads(result.stdout), acquisition.sha256_file(jar), selected["class_entry"], class_digest)
    result, commands["rea_android_boundary"] = capture("rea-jar-android-boundary", rea + ["inspect-android-package", str(jar), "--json"], expected_exit=1)
    boundary = android_boundary(json.loads(result.stdout))
    bytecode_listing, commands["javap_listing"] = capture("javap-sorter", ["javap", "-classpath", str(jar), "-c", "-p", "-s", selected["class_name"]])
    classpath = os.pathsep.join(str(reference / item) for item in selected["classpath"])
    source = OUTPUT / "decompiled" / (selected["class_entry"][:-6] + ".java")
    source.unlink(missing_ok=True)
    _, commands["cfr_decompile"] = capture("cfr-sorter", ["java", "-jar", str(cfr), str(class_path), "--extraclasspath", classpath, "--outputdir", str(OUTPUT / "decompiled"), "--silent", "true"])
    text = source.read_text(encoding="utf-8")
    if "public class FileComparator" not in text or "compareByFilename" not in text:
        raise ValueError("CFR did not recover the selected class and methods")
    if "could not load" in text.lower() or "decompilation failed" in text.lower():
        raise ValueError("CFR reported incomplete class resolution or decompilation")
    if "Code:" not in bytecode_listing.stdout or "compareByFilename" not in bytecode_listing.stdout:
        raise ValueError("javap did not return a bytecode listing for the selected methods")
    acquisition.run(verify_only=True)

    receipt = {
        "schema_version": 1,
        "scope": "static_analysis_tool_readiness",
        "tool_manifest_sha256": acquisition.sha256_file(MANIFEST),
        "npm_lock_sha256": acquisition.sha256_file(REA_PROJECT / "package-lock.json"),
        "host": {"system": platform.system(), "architecture": platform.machine()},
        "versions": versions,
        "commands": commands,
        "rea_jar_inventory": inventory,
        "rea_android_boundary": boundary,
        "jvm_decompilation": {
            "producer": "CFR " + manifest["cfr"]["version"],
            "class_entry": selected["class_entry"],
            "class_sha256": class_digest,
            "class_file_major_version": int.from_bytes(bytecode[6:8], "big"),
            "source_path": source.relative_to(ROOT).as_posix(),
            "source_sha256": acquisition.sha256_file(source),
            "source_lines": len(text.splitlines()),
            "unresolved_class_warning": False,
            "bytecode_listing_producer": "JDK javap",
            "previously_examined_in_feasibility_investigation": selected["previously_examined_in_feasibility_investigation"],
        },
        "reference_unchanged_after_probe": True,
        "application_execution": "not_performed",
        "gui_compatibility": "not_assessed",
        "behavioral_equivalence": "not_assessed",
        "global_agent_configuration_changes": "none",
        "limitations": [
            "REA coverage describes ZIP entries; it does not establish JVM method semantics.",
            "CFR output and javap disassembly are derived representations; no equivalence test has run.",
            "The APK-only refusal was checked without installing JADX or a native analysis provider.",
            "npm lifecycle scripts are disabled; native, browser and instrumentation providers were not validated.",
            "The selected class was already inspected during feasibility work; this probe is not a blind reconstruction.",
        ],
    }
    receipt_path = ROOT / "evidence/toolchain/readiness.json"
    acquisition.write_json(receipt_path, receipt)
    print("Static tool readiness verified: REA inventory, explicit APK-only rejection, CFR and javap.")
    print("Evidence: {}".format(receipt_path.relative_to(ROOT)))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("setup", "probe"))
    parser.add_argument("--offline", action="store_true", help="Use cached packages for setup; probe never downloads")
    args = parser.parse_args()
    try:
        manifest = read_manifest()
        if args.action == "setup":
            setup(manifest, offline=args.offline)
        else:
            probe(manifest)
    except (OSError, ValueError, KeyError, StopIteration, zipfile.BadZipFile, subprocess.SubprocessError) as error:
        print("Analysis-tool {} failed: {}".format(args.action, error), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
