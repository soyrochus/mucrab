#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
"""Reproduce the quarantined original-sorter stall; exit 2 means nontermination observed."""

from datetime import datetime, timezone
import json
import os
import sys
import uuid

import acquire_mucommander as acquisition
from w01 import ROOT, REFERENCE, capture, digest
from w02c import materialize, java_capture


def main():
    acquisition.run(verify_only=True)
    source = ROOT / "fixtures/W02c-metadata-sorting/reference-stall.json"
    item = json.loads(source.read_text()); item["id"] = "reference-stall"
    spec = dict(sets=[item], generated_sets=0, generated_seed=0, real_entries=[])
    run = ROOT / "artifacts/runs" / ("W02c-reference-stall-" + datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + uuid.uuid4().hex[:8]); run.mkdir(parents=True)
    request = next(materialize(run, spec, False)); text = json.dumps(request) + "\n"
    (run / "requests.jsonl").write_text(text)
    jars = sorted((REFERENCE / "bundle").glob("*.jar")) + sorted((REFERENCE / "app").glob("*.jar"))
    cp = os.pathsep.join(map(str, jars)); classes = run / "classes"; classes.mkdir()
    _, compilation = capture(run, "javac", ["javac", "-proc:none", "-encoding", "UTF-8", "-cp", cp, "-d", str(classes), str(ROOT / "tools/java/W02Oracle.java"), str(ROOT / "tools/java/W02cOracle.java")])
    command = ["java", "--add-opens=java.base/java.io=ALL-UNNAMED", "-Djava.awt.headless=true", "-Duser.home=" + str(run / "home"), "-cp", str(classes) + os.pathsep + cp, "W02cOracle"]
    try:
        java_capture(run, command, text, timeout=20)
    except ValueError:
        if not (run / "incomplete.json").exists(): raise
        stderr = (run / "java.stderr").read_text()
        observed = '"criterion":"DATE","ascending":false,"directories_first":true' in stderr and "FileTableModel.sort(" in stderr
        if not observed: raise ValueError("Timeout occurred outside the expected original sort")
        acquisition.run(verify_only=True)
        record = dict(status="reference_nontermination_observed", equivalence_cases_run=0, exit_code=2, timeout_seconds=20,
            reason=item["reason"], fixture=source.relative_to(ROOT).as_posix(), fixture_sha256=digest(source),
            source_sha256=digest(ROOT / "tools/java/W02cOracle.java"), compilation=compilation,
            command=[s.replace(str(ROOT), "{repository}") for s in command], raw_run=run.relative_to(ROOT).as_posix(),
            stdout_sha256=digest(run / "java.stdout"), stderr_sha256=digest(run / "java.stderr"), reference_unchanged=True,
            earlier_incomplete_runs=["artifacts/runs/W02c-20261016T140701Z-904e6753", "artifacts/runs/W02c-20261016T141045Z-4dd64cbc"])
        acquisition.write_json(run / "report.json", record); acquisition.write_json(ROOT / "evidence/workflows/W02c-metadata-sorting/reference-stall.json", record)
        print("Original descending DATE sort nontermination observed; no equivalence claimed; " + str(run.relative_to(ROOT)))
        return 2
    raise ValueError("Original stall did not reproduce; investigate rather than claiming success")


if __name__ == "__main__":
    sys.exit(main())
