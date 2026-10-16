#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
"""Compare original MoveJob against native moves on same and different filesystems."""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import tempfile
import uuid

import acquire_mucommander as acquisition
from w01 import ROOT, REFERENCE, capture, digest
from w02c import java_capture
from w03 import generated, materialize_tree, snapshot, decode

SPEC=ROOT/'fixtures/W04-move-rename/cases.json'
EVIDENCE=ROOT/'evidence/workflows/W04-move-rename'
SOURCE=ROOT/'tools/java/W04Oracle.java'


def cases(spec,reserved):
    items=list(generated(spec,reserved)) if reserved else spec['cases']+list(generated(spec,False))
    for item in items:
        for arrangement in item.get('arrangements',['same','cross']):
            for policy in item.get('policies',spec['policies']):
                yield dict(item,id=item['id']+'-'+arrangement+'-'+policy.lower(),arrangement=arrangement,policy=policy)


def identity(path):
    try:
        info=path.lstat();return [info.st_dev,info.st_ino]
    except FileNotFoundError:return None


def source_date_mask(item):
    # Frozen by input paths, not inferred from execution outcomes: removing
    # children assigns surviving source directories a runtime-clock timestamp.
    names=item['names'];paths=set()
    for entry in item['source']:
        path=Path(entry['path'])
        for parent in [path] + list(path.parents):
            if parent==Path('.'):continue
            if any(parent==Path(name) or parent.is_relative_to(name) for name in names):
                if parent!=path or entry['kind']=='directory':paths.add(parent.as_posix())
    return sorted(paths)


def prepare(run,external,side,items,spec):
    requests=[];before=[]
    for item in items:
        folder=run/side/item['id'];source=folder/'source'
        destination=(external/side/item['id']/'destination') if item['arrangement']=='cross' else folder/'destination'
        materialize_tree(source,item['source'],spec);materialize_tree(destination,item['destination'],spec)
        if 'destination_use' in item:destination=folder/item['destination_use']
        os.chmod(source,item.get('source_root_mode',0o755));os.chmod(destination,item.get('destination_root_mode',0o755))
        same=source.stat().st_dev==destination.stat().st_dev
        if same!=(item['arrangement']=='same'):raise ValueError('Requested filesystem arrangement was not established: '+item['id'])
        request=dict(id=item['id'],source_directory=str(source),destination_directory=str(destination),names=item['names'],
            policy=item['policy'],new_name=item.get('new_name'),rename_mode=item.get('rename_mode',False))
        requests.append(request)
        before.append(dict(id=item['id'],source=snapshot(source),destination=snapshot(destination),source_mode=source.stat().st_mode & 0o777,destination_mode=destination.stat().st_mode & 0o777,
            source_identities={name:identity(source/name) for name in item['names']}))
        for entry in item['source']:
            if entry.get('mode')==0 and os.access(source/entry['path'],os.R_OK):raise ValueError('Denied source fixture is readable')
    return requests,before


def comparable_before(states):
    return [{k:v for k,v in state.items() if k!='source_identities'} for state in states]


def observe(requests,items,before):
    states=[]
    for request,item,old in zip(requests,items,before):
        source=Path(request['source_directory']);dest=Path(request['destination_directory']);mask=source_date_mask(item)
        ignored=list(item.get('ignore_destination_mtime',[]))
        if source==dest:ignored+=mask
        identities=[]
        for name in request['names']:
            target=request['new_name'] or name;now=identity(dest/target)
            identities.append(dict(name=name,source_present=identity(source/name) is not None,destination_present=now is not None,
                identity_retained=now is not None and now==old['source_identities'][name],destination_identity=now))
        states.append(dict(id=request['id'],source=snapshot(source,mask),destination=snapshot(dest,ignored),source_mode=source.stat().st_mode & 0o777,destination_mode=dest.stat().st_mode & 0o777,identities=identities,
            ignored_source_dates=mask,ignored_destination_dates=ignored))
    return states


def comparable_after(state):
    return dict(state,identities=[{k:v for k,v in entry.items() if k!='destination_identity'} for entry in state['identities']])


def assert_selected_content_preserved(requests, before, after):
    """Every selected byte stream/link survives at its old or mapped new path."""
    for request, old, new in zip(requests, before, after):
        remaining={entry['path']:entry for entry in new['source']}
        destination={entry['path']:entry for entry in new['destination']}
        for entry in old['source']:
            path=Path(entry['path'])
            selected=next((name for name in request['names'] if path==Path(name) or path.is_relative_to(name)),None)
            if selected is None:continue
            mapped=(Path(request['new_name'] or selected)/path.relative_to(selected)).as_posix()
            def same_data(actual):
                if actual is None or actual['kind']!=entry['kind']:return False
                if entry['kind']=='file':return (actual['size'],actual['sha256'])==(entry['size'],entry['sha256'])
                if entry['kind']=='symlink':return actual['target']==entry['target']
                return True
            if not same_data(remaining.get(entry['path'])) and not same_data(destination.get(mapped)):
                raise ValueError('Selected input content lost: '+request['id']+'/'+entry['path'])


def run_suite(compare,reserved,rust_adapter):
    if platform.system()!='Linux' or os.geteuid()==0:raise ValueError('W04 requires unprivileged Linux')
    acquisition.run(verify_only=True)
    run_id=datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')+'-'+uuid.uuid4().hex[:8]
    run=ROOT/'artifacts/runs'/('W04-'+run_id);run.mkdir(parents=True)
    external=Path(tempfile.mkdtemp(prefix='mucrab-W04-'+run_id+'-'))
    if external.stat().st_dev==ROOT.stat().st_dev:raise ValueError('/tmp must provide a real second filesystem for the frozen matrix')
    spec=json.loads(SPEC.read_text());items=list(cases(spec,reserved))
    requests,before=prepare(run,external,'original',items,spec)
    acquisition.write_json(run/'original-before.json',before)
    (run/'requests.jsonl').write_text(''.join(json.dumps(r)+'\n' for r in requests))
    jars=sorted((REFERENCE/'bundle').glob('*.jar'))+sorted((REFERENCE/'app').glob('*.jar'));cp=os.pathsep.join(map(str,jars));classes=run/'classes';classes.mkdir()
    _,compiler=capture(run,'javac',['javac','-proc:none','-encoding','UTF-8','-cp',cp,'-d',str(classes),str(SOURCE)])
    output,java=java_capture(run,['java','--add-opens=java.base/java.io=ALL-UNNAMED','-Djava.awt.headless=true','-Duser.home='+str(run/'home'),'-cp',str(classes)+os.pathsep+cp,'W04Oracle'],'',timeout=90)
    original=decode(output,requests);after=observe(requests,items,before);acquisition.write_json(run/'original-after.json',after)
    assert_selected_content_preserved(requests,before,after)
    if not any(outcome['success'] for result in original for outcome in result['outcomes']):raise ValueError('No successful original move execution')
    differences=[];commands=dict(javac=compiler,java=java)
    if compare:
        native_requests,native_before=prepare(run,external,'rust',items,spec)
        if comparable_before(before)!=comparable_before(native_before):raise ValueError('Initial original/Rust move trees differ')
        (run/'rust-requests.jsonl').write_text(''.join(json.dumps(r)+'\n' for r in native_requests))
        acquisition.write_json(run/'rust-before.json',native_before)
        output,commands['rust']=capture(run,'rust',[str(rust_adapter),'--adapter-w04'],(run/'rust-requests.jsonl').read_text())
        native=decode(output,native_requests);native_after=observe(native_requests,items,native_before);acquisition.write_json(run/'rust-after.json',native_after)
        try:assert_selected_content_preserved(native_requests,native_before,native_after)
        except ValueError as error:differences.append(dict(component='content_conservation',message=str(error)))
        for left,right,expected,actual in zip(original,native,after,native_after):
            if left!=right:differences.append(dict(id=left['id'],component='outcomes',expected=left,actual=right))
            if comparable_after(expected)!=comparable_after(actual):differences.append(dict(id=left['id'],component='filesystem_results',expected=comparable_after(expected),actual=comparable_after(actual)))
    acquisition.run(verify_only=True)
    files=[SPEC,SOURCE,ROOT/'tools/w04.py',ROOT/'tools/w03.py',ROOT/'tools/w02c.py',ROOT/'tools/w01.py',ROOT/'Cargo.toml',ROOT/'Cargo.lock']
    if (EVIDENCE/'contract.md').exists():files.append(EVIDENCE/'contract.md')
    files+=sorted((ROOT/'crates').rglob('*.rs'))+sorted((ROOT/'crates').rglob('Cargo.toml'))
    report=dict(schema_version=1,workflow='W04-move-rename',run_id=run_id,suite='reserved' if reserved else 'predefined',
        status=('differences' if differences else 'passed') if compare else 'oracle_ready',
        scope='Original MoveJob.processFile, rename/copy-delete/merge/policies, real same/cross filesystem trees; lifecycle/UI excluded',
        source_revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),source_status=subprocess.check_output(['git','status','--porcelain'],cwd=ROOT,text=True).splitlines(),
        source_inputs=[dict(path=p.relative_to(ROOT).as_posix(),sha256=digest(p)) for p in files],
        host=dict(system=platform.system(),architecture=platform.machine(),uid=os.geteuid(),umask='022',source_device=ROOT.stat().st_dev,destination_device=external.stat().st_dev),
        external_root=str(external),classpath=[dict(path=p.relative_to(ROOT).as_posix(),sha256=digest(p)) for p in jars],
        java_adapter_classes=[dict(path=p.relative_to(ROOT).as_posix(),sha256=digest(p)) for p in sorted(classes.glob('*.class'))],commands=commands,
        fixture_seed=spec['reserved_seed'] if reserved else spec['generated_seed'],oracle_cases=len(items),cases_run=len(items) if compare else 0,
        same_filesystem_cases=sum(i['arrangement']=='same' for i in items) if compare else 0,cross_filesystem_cases=sum(i['arrangement']=='cross' for i in items) if compare else 0,
        rename_cases=sum(i.get('rename_mode',False) for i in items) if compare else 0,top_level_outcomes=sum(len(i['names']) for i in items) if compare else 0,
        source_tree_checks=len(items) if compare else 0,destination_tree_checks=len(items) if compare else 0,
        identity_relation_checks=sum(len(i['names']) for i in items) if compare else 0,content_conservation_checks=len(items)*(2 if compare else 1),
        differences=differences,skipped=[],reference_unchanged=True,raw_run=run.relative_to(ROOT).as_posix(),request_sha256=digest(run/'requests.jsonl'),original_after_sha256=digest(run/'original-after.json'),
        rust_adapter=dict(path=str(rust_adapter).replace(str(ROOT),'{repository}'),sha256=digest(rust_adapter)) if compare else None,
        reproduce='target/debug/mucrab-compare --suite w04'+(' --reserved' if reserved else ''))
    acquisition.write_json(run/'report.json',report);acquisition.write_json(EVIDENCE/(('comparison-' if compare else 'oracle-')+report['suite']+'.json'),report)
    print('{}: {} move/rename cases, {} differences; {}'.format(report['status'],len(items),len(differences),run.relative_to(ROOT)))
    return int(bool(differences))


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('action',choices=['oracle','compare']);parser.add_argument('--reserved',action='store_true');parser.add_argument('--rust-adapter',type=Path,default=ROOT/'target/debug/mucrab-compare')
    args=parser.parse_args();mask=os.umask(0o022)
    try:return run_suite(args.action=='compare',args.reserved,args.rust_adapter.resolve())
    except (OSError,ValueError,KeyError,TypeError,subprocess.SubprocessError) as error:print('W04 unavailable: '+str(error),file=sys.stderr);return 2
    finally:os.umask(mask)


if __name__=='__main__':sys.exit(main())
