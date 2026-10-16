#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
"""Reproduce pinned JDK simple-case facts used by native extension comparison."""

import argparse
from datetime import datetime, timezone
import json
import subprocess
import uuid

import acquire_mucommander as acquisition
from w01 import ROOT, capture, digest

MANIFEST = ROOT / "tools/manifests/java-case-data.json"
SOURCE = ROOT / "tools/java/W02cCaseData.java"
TARGET = ROOT / "crates/core/src/java_case_data.rs"


def render(data):
    lines = ["// SPDX-License-Identifier: GPL-3.0-only", "// Copyright (C) 2026 muCrab contributors", "",
        "// Generated JDK 25.0.4.1 simple Unicode case facts. Regenerate with", "// python3 -B tools/w02c_case_data.py; see tools/manifests/java-case-data.json.",
        "#[rustfmt::skip]", "pub(crate) const JAVA_SIMPLE_FOLD: &[(u32, u32)] = &["]
    lines.extend("    (0x{:x}, 0x{:x}),".format(cp, folded) for cp, folded in data["fold"])
    return "\n".join(lines + ["];", ""])


def main():
    parser = argparse.ArgumentParser(description=__doc__); parser.add_argument("--verify-only", action="store_true")
    args = parser.parse_args(); pin = json.loads(MANIFEST.read_text())
    run = ROOT / "artifacts/runs" / ("W02c-case-data-" + datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + uuid.uuid4().hex[:8])
    run.mkdir(parents=True)
    _, compiler = capture(run, "javac", ["javac", "-proc:none", "-d", str(run), str(SOURCE)])
    text, java = capture(run, "java", ["java", "-cp", str(run), "W02cCaseData"])
    data = json.loads(text)
    if data["java_version"] != pin["java_version"] or digest(run / "java.stdout") != pin["facts_sha256"] or len(data["fold"]) != pin["mappings"]:
        raise ValueError("JDK case data differs from the pinned facts; do not silently update it")
    expected = render(data)
    if args.verify_only:
        if TARGET.read_text() != expected: raise ValueError("Committed native case data differs")
    else: TARGET.write_text(expected)
    receipt = dict(scope="JDK simple-case data, not Java/Rust behavioral equivalence", producer_source=SOURCE.relative_to(ROOT).as_posix(),
        producer_sha256=digest(SOURCE), java_version=data["java_version"], mappings=len(data["fold"]), facts_sha256=pin["facts_sha256"],
        native_data=TARGET.relative_to(ROOT).as_posix(), native_sha256=digest(TARGET), commands=dict(javac=compiler, java=java), raw_run=run.relative_to(ROOT).as_posix())
    acquisition.write_json(run / "report.json", receipt)
    if not args.verify_only: acquisition.write_json(ROOT / "evidence/workflows/W02c-metadata-sorting/case-data.json", receipt)
    print("Verified {} simple-case mappings from JDK {}; {}".format(len(data["fold"]), data["java_version"], TARGET.relative_to(ROOT)))


if __name__ == "__main__":
    main()
