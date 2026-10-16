#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
"""Observe original cross-filesystem symlink fallback; deliberately exits 2, zero equivalence cases."""
from datetime import datetime,timezone
import json
import os
from pathlib import Path
import sys
import tempfile
import uuid
import acquire_mucommander as acquisition
from w01 import ROOT,REFERENCE,capture,digest
from w02c import java_capture
from w03 import materialize_tree,snapshot,decode


def main():
    acquisition.run(verify_only=True)
    run=ROOT/'artifacts/runs'/('W04-symlink-fallback-'+datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')+'-'+uuid.uuid4().hex[:8]);run.mkdir(parents=True)
    external=Path(tempfile.mkdtemp(prefix='mucrab-W04-symlink-'))
    if external.stat().st_dev==ROOT.stat().st_dev:raise ValueError('Real cross-filesystem input required')
    fixture=ROOT/'fixtures/W04-move-rename/symlink-fallback.json';spec=json.loads((ROOT/'fixtures/W04-move-rename/cases.json').read_text());requests=[];before=[]
    for case in json.loads(fixture.read_text())['cases']:
        source=run/case['id']/'source';dest=external/case['id'];materialize_tree(source,case['source'],spec);materialize_tree(dest,[],spec)
        requests.append(dict(id=case['id'],source_directory=str(source),destination_directory=str(dest),names=case['names'],policy='SKIP',new_name=None,rename_mode=False))
        before.append(dict(id=case['id'],source=snapshot(source),destination=snapshot(dest)))
    (run/'requests.jsonl').write_text(''.join(json.dumps(r)+'\n' for r in requests));acquisition.write_json(run/'before.json',before)
    jars=sorted((REFERENCE/'bundle').glob('*.jar'))+sorted((REFERENCE/'app').glob('*.jar'));cp=os.pathsep.join(map(str,jars));classes=run/'classes';classes.mkdir()
    _,compiler=capture(run,'javac',['javac','-proc:none','-cp',cp,'-d',str(classes),str(ROOT/'tools/java/W04Oracle.java')])
    text,java=java_capture(run,['java','--add-opens=java.base/java.io=ALL-UNNAMED','-Djava.awt.headless=true','-Duser.home='+str(run/'home'),'-cp',str(classes)+os.pathsep+cp,'W04Oracle'],'')
    outcomes=decode(text,requests);after=[dict(id=r['id'],source=snapshot(Path(r['source_directory'])),destination=snapshot(Path(r['destination_directory']))) for r in requests]
    acquisition.write_json(run/'after.json',after)
    file_state,dir_state=after
    if not all(o['outcomes'][0]['success'] for o in outcomes):raise ValueError('Original probe did not finish expected fallback')
    if next(e for e in file_state['destination'] if e['path']=='link')['kind']!='file':raise ValueError('Original file-link conversion not observed')
    if any(e['path']=='target/child' for e in dir_state['source']) or next(e for e in dir_state['destination'] if e['path']=='link')['kind']!='directory':raise ValueError('Original directory-link target removal not observed')
    acquisition.run(verify_only=True)
    report=dict(status='reference_symlink_fallback_hazard_observed',equivalence_cases_run=0,exit_code=2,
        findings=['File symlink becomes a regular destination file','Directory symlink fallback moves children out of its source target, then unlinks the symlink'],
        native_decision='Permit supported rename fast paths only; decline fallback through symlinks',
        fixture=dict(path=fixture.relative_to(ROOT).as_posix(),sha256=digest(fixture)),source=dict(path='tools/java/W04Oracle.java',sha256=digest(ROOT/'tools/java/W04Oracle.java')),
        commands=dict(javac=compiler,java=java),raw_run=run.relative_to(ROOT).as_posix(),external_root=str(external),reference_unchanged=True,
        before=dict(path=(run/'before.json').relative_to(ROOT).as_posix(),sha256=digest(run/'before.json')),after=dict(path=(run/'after.json').relative_to(ROOT).as_posix(),sha256=digest(run/'after.json')))
    acquisition.write_json(run/'report.json',report);acquisition.write_json(ROOT/'evidence/workflows/W04-move-rename/symlink-fallback.json',report)
    print('Original symlink fallback hazard observed; zero equivalence cases; '+str(run.relative_to(ROOT)))
    return 2


if __name__=='__main__':sys.exit(main())
