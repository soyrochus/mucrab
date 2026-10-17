#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
"""Recover job lifecycle, progress and cancellation from pinned JARs."""

from datetime import datetime, timezone
import hashlib
import os
import uuid
import zipfile

import acquire_mucommander as acquisition
import analysis_tools
from w01 import ROOT, REFERENCE, capture, digest

EVIDENCE = ROOT / "evidence/workflows/W07-background-operations"
CLASSES = {
    "bundle/mucommander-commons-file-1.6.2.jar": ["com.mucommander.commons.file.AbstractFile"],
    "app/mucommander-core-1.6.2.jar": [
        "com.mucommander.job.impl.FileJob",
        "com.mucommander.job.FileJobState",
        "com.mucommander.job.impl.TransferFileJob",
        "com.mucommander.job.impl.AbstractCopyJob",
        "com.mucommander.job.impl.CopyJob",
        "com.mucommander.job.impl.MoveJob",
        "com.mucommander.job.impl.UnpackJob",
        "com.mucommander.job.impl.BatchRenameJob",
    ],
}


def main():
    acquisition.run(verify_only=True)
    cfr = analysis_tools.verify_tools(analysis_tools.read_manifest())
    run = ROOT / "artifacts/runs" / ("W07-recovery-" + datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + uuid.uuid4().hex[:8])
    run.mkdir(parents=True)
    jars = sorted((REFERENCE / "bundle").glob("*.jar")) + sorted((REFERENCE / "app").glob("*.jar"))
    classpath = os.pathsep.join(str(path) for path in jars)
    records = []
    for relative, names in CLASSES.items():
        jar = REFERENCE / relative
        with zipfile.ZipFile(jar) as archive:
            for name in names:
                entry = name.replace(".", "/")
                bytecode = archive.read(entry + ".class")
                short = name.rsplit(".", 1)[-1]
                target = run / (short + ".class")
                target.write_bytes(bytecode)
                _, listing = capture(run, short + "-javap", ["javap", "-c", "-p", "-s", "-classpath", classpath, name])
                _, decompiler = capture(run, short + "-cfr", ["java", "-jar", str(cfr), str(target), "--extraclasspath", classpath,
                    "--outputdir", str(run / "decompiled"), "--silent", "true"])
                source = run / "decompiled" / (entry + ".java")
                # CFR emits inner classes as part of their enclosing source.
                if not source.exists() and "$" in entry:
                    source = run / "decompiled" / (entry.split("$")[0] + ".java")
                records.append(dict(class_name=name, jar=relative, jar_sha256=digest(jar),
                    class_sha256=hashlib.sha256(bytecode).hexdigest(),
                    decompiled_source=source.relative_to(ROOT).as_posix(), source_sha256=digest(source),
                    javap=listing, cfr=decompiler))
    acquisition.run(verify_only=True)
    report = dict(scope="W07 background-operations static recovery; runtime and GUI gates separate",
        run=run.relative_to(ROOT).as_posix(), cfr_version="0.152", cfr_sha256=digest(cfr),
        classes=records, reference_unchanged=True, reproduce="python3 -B tools/w06_inspect.py")
    acquisition.write_json(run / "report.json", report)
    acquisition.write_json(EVIDENCE / "recovery.json", report)
    print("Recovered {} classes: {}".format(len(records), run.relative_to(ROOT)))


if __name__ == "__main__":
    main()
