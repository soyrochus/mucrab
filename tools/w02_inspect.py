#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
"""Recover the bounded selection/marking workflow from the pinned binaries."""

from datetime import datetime, timezone
import hashlib
import os
import uuid
import zipfile

import acquire_mucommander as acquisition
import analysis_tools
from w01 import ROOT, REFERENCE, capture, digest

EVIDENCE = ROOT / "evidence/workflows/W02-selection-marking"
NAMES = ["FileTable", "FileTableModel", "SortInfo"]
ACTIONS = ["SelectForwardAction", "SelectBackwardAction", "SelectNextRowAction",
           "SelectPreviousRowAction", "SelectFirstRowAction", "SelectLastRowAction",
           "MarkForwardAction", "MarkBackwardAction", "MarkNextRowAction",
           "MarkPreviousRowAction", "MarkToFirstRowAction", "MarkToLastRowAction",
           "MarkSelectedFileAction", "MarkAllAction", "UnmarkAllAction", "InvertSelectionAction"]


def main():
    acquisition.run(verify_only=True)
    cfr = analysis_tools.verify_tools(analysis_tools.read_manifest())
    run = ROOT / "artifacts/runs" / ("W02-recovery-" + datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + uuid.uuid4().hex[:8])
    run.mkdir(parents=True)
    jar = REFERENCE / "app/mucommander-core-1.6.2.jar"
    classpath = os.pathsep.join(str(p) for p in sorted((REFERENCE / "bundle").glob("*.jar")) + sorted((REFERENCE / "app").glob("*.jar")))
    records = []
    with zipfile.ZipFile(jar) as archive:
        for name in (["com.mucommander.ui.main.table." + n for n in NAMES] +
                     ["com.mucommander.ui.action.impl." + n for n in ACTIONS]):
            entry = name.replace(".", "/")
            if entry + ".class" not in archive.namelist():
                continue
            bytecode = archive.read(entry + ".class")
            short = name.rsplit(".", 1)[-1]
            target = run / (short + ".class")
            target.write_bytes(bytecode)
            _, listing = capture(run, short + "-javap", ["javap", "-c", "-p", "-classpath", classpath, name])
            _, decompiler = capture(run, short + "-cfr", ["java", "-jar", str(cfr), str(target), "--extraclasspath", classpath,
                "--outputdir", str(run / "decompiled"), "--silent", "true"])
            source = run / "decompiled" / (entry + ".java")
            records.append({"class": name, "jar": jar.relative_to(REFERENCE).as_posix(), "jar_sha256": digest(jar),
                "class_sha256": hashlib.sha256(bytecode).hexdigest(), "decompiled_source": source.relative_to(ROOT).as_posix(),
                "source_sha256": digest(source), "javap": listing, "cfr": decompiler})
    acquisition.run(verify_only=True)
    report = {"scope": "W02 selection/marking static recovery; runtime/GUI gates separate",
              "run": run.relative_to(ROOT).as_posix(), "cfr_version": "0.152", "cfr_sha256": digest(cfr),
              "classes": records, "reference_unchanged": True}
    acquisition.write_json(EVIDENCE / "recovery.json", report)
    print("Recovered {} classes: {}".format(len(records), run.relative_to(ROOT)))


if __name__ == "__main__":
    main()
