#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
"""Reproduce scoped W01 decompilation and bytecode provenance from verified JARs."""

from datetime import datetime, timezone
import hashlib
import os
from pathlib import Path
import subprocess
import uuid
import zipfile

import acquire_mucommander as acquisition
import analysis_tools
from w01 import ROOT, REFERENCE, EVIDENCE, capture, digest


def main():
    acquisition.run(verify_only=True)
    cfr = analysis_tools.verify_tools(analysis_tools.read_manifest())
    run = ROOT / "artifacts/runs" / ("W01-recovery-" + datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + uuid.uuid4().hex[:8])
    run.mkdir(parents=True)
    classpath = os.pathsep.join(str(p) for p in sorted((REFERENCE / "bundle").glob("*.jar")) + sorted((REFERENCE / "app").glob("*.jar")))
    mapping = {
        "bundle/mucommander-commons-file-1.6.2.jar": ["com.mucommander.commons.file.util.FileComparator", "com.mucommander.commons.file.protocol.local.LocalFile"],
        "app/mucommander-core-1.6.2.jar": ["com.mucommander.ui.main.table.FileTableModel", "com.mucommander.ui.main.table.FileTable", "com.mucommander.ui.main.table.SortInfo", "com.mucommander.ui.main.table.Column"],
        "bundle/mucommander-preferences-1.6.2.jar": ["com.mucommander.conf.MuPreference"],
        "mucommander-1.6.2.jar": ["com.mucommander.main.Configuration"],
    }
    records = []
    for jar, names in mapping.items():
        with zipfile.ZipFile(REFERENCE / jar) as archive:
            for name in names:
                entry = name.replace(".", "/")
                bytecode = archive.read(entry + ".class")
                short = name.rsplit(".", 1)[-1]
                target = run / (short + ".class")
                target.write_bytes(bytecode)
                _, listing = capture(run, short + "-javap", ["javap", "-c", "-p", "-s", "-classpath", str(REFERENCE / jar), name])
                _, decompiler = capture(run, short + "-cfr", ["java", "-jar", str(cfr), str(target), "--extraclasspath", classpath,
                    "--outputdir", str(run / "decompiled"), "--silent", "true"])
                source = run / "decompiled" / (entry + ".java")
                records.append({"class": name, "jar": jar, "jar_sha256": digest(REFERENCE / jar),
                    "class_sha256": hashlib.sha256(bytecode).hexdigest(),
                    "decompiled_source": source.relative_to(ROOT).as_posix(), "source_sha256": digest(source),
                    "javap": listing, "cfr": decompiler})
    acquisition.run(verify_only=True)
    report = {"scope": "W01 scoped static recovery; runtime and GUI results are separate", "cfr_version": "0.152",
              "cfr_sha256": digest(cfr), "classes": records, "reference_unchanged": True,
              "reproduce": "python3 -B tools/w01_inspect.py"}
    acquisition.write_json(EVIDENCE / "recovery.json", report)
    acquisition.write_json(run / "report.json", report)
    print("Recovered {} pinned classes; evidence/workflows/W01-local-browse-sort/recovery.json".format(len(records)))


if __name__ == "__main__":
    main()
