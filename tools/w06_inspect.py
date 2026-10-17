#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
"""Recover ZIP browsing and extraction from pinned JARs."""

from datetime import datetime, timezone
import hashlib
import os
import uuid
import zipfile

import acquire_mucommander as acquisition
import analysis_tools
from w01 import ROOT, REFERENCE, capture, digest

EVIDENCE = ROOT / "evidence/workflows/W06-zip-browsing-extraction"
CLASSES = {
    "bundle/mucommander-format-zip-1.6.2.jar": [
        "com.mucommander.commons.file.archive.zip.ZipArchiveFile",
        "com.mucommander.commons.file.archive.zip.ZipFormatProvider",
        "com.mucommander.commons.file.archive.zip.provider.ZipFile",
        "com.mucommander.commons.file.archive.zip.provider.ZipEntry",
    ],
    "bundle/mucommander-commons-file-1.6.2.jar": [
        "com.mucommander.commons.file.archive.AbstractArchiveFile",
        "com.mucommander.commons.file.archive.AbstractROArchiveFile",
        "com.mucommander.commons.file.archive.ArchiveEntryTree",
        "com.mucommander.commons.file.archive.ArchiveEntry",
        "com.mucommander.commons.file.archive.AbstractArchiveEntryFile",
        "com.mucommander.commons.file.archive.ROArchiveEntryFile",
        "com.mucommander.commons.file.FileFactory",
        "com.mucommander.commons.file.util.PathUtils",
        "com.mucommander.commons.file.protocol.local.LocalFile",
    ],
    "app/mucommander-core-1.6.2.jar": [
        "com.mucommander.job.impl.UnpackJob",
        "com.mucommander.job.impl.UnpackJob$ProxiedEntryFile",
        "com.mucommander.job.impl.CopyJob",
        "com.mucommander.job.impl.AbstractCopyJob",
        "com.mucommander.job.FileCollisionChecker",
        "com.mucommander.job.impl.TransferFileJob",
        "com.mucommander.ui.action.impl.UnpackAction",
        "com.mucommander.ui.dialog.file.UnpackDialog",
        "com.mucommander.ui.dialog.file.CopyDialog",
        "com.mucommander.ui.main.FolderPanel",
        "com.mucommander.core.LocationChanger",
        "com.mucommander.core.ChangeFolderThread",
        "com.mucommander.core.BrowseLocationThread",
        "com.mucommander.ui.main.table.FileTableModel",
    ],
}


def main():
    acquisition.run(verify_only=True)
    cfr = analysis_tools.verify_tools(analysis_tools.read_manifest())
    run = ROOT / "artifacts/runs" / ("W06-recovery-" + datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + uuid.uuid4().hex[:8])
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
    report = dict(scope="W06 zip-browsing-extraction static recovery; runtime and GUI gates separate",
        run=run.relative_to(ROOT).as_posix(), cfr_version="0.152", cfr_sha256=digest(cfr),
        classes=records, reference_unchanged=True, reproduce="python3 -B tools/w06_inspect.py")
    acquisition.write_json(run / "report.json", report)
    acquisition.write_json(EVIDENCE / "recovery.json", report)
    print("Recovered {} classes: {}".format(len(records), run.relative_to(ROOT)))


if __name__ == "__main__":
    main()
