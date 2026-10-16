#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
"""Compare original batch-name previews/jobs with native code on isolated trees."""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import platform
import random
import subprocess
import sys
import uuid

import acquire_mucommander as acquisition
from w01 import ROOT, REFERENCE, capture, digest
from w02c import java_capture
from w03 import materialize_tree, snapshot
from w04 import identity, source_date_mask, assert_selected_content_preserved

SPEC=ROOT/'fixtures/W05-batch-rename/cases.json'
SOURCE=ROOT/'tools/java/W05Oracle.java'
EVIDENCE=ROOT/'evidence/workflows/W05-batch-rename'


def generated(spec,reserved):
    rng=random.Random(spec['reserved_seed'] if reserved else spec['generated_seed'])
    for i in range(spec['reserved_sets'] if reserved else spec['generated_sets']):
        names=[rng.choice(['Alpha','bETA','plain','two Words','.hidden'])+str(j)+rng.choice(['.TXT','.bin','']) for j in range(rng.randrange(2,9))]
        source=[dict(path=name,kind='file',size=rng.choice([0,1,17,4097,65539]),pattern=rng.randrange(251),mode=rng.choice([0o444,0o600,0o644,0o751])) for name in names]
        options=[dict(mask='[C]-[N].[E]',counter_start=str(rng.randrange(-5,15)),counter_step=str(rng.randrange(-3,5)),digits=rng.randrange(1,6)),
                 dict(mask='[N].[E]',case_mode=rng.randrange(5)),dict(mask='batch-[C10,2,3]-[N].[E]')]
        yield dict(id=('reserved' if reserved else 'generated')+'-'+str(i),names=names,source=source,options=options,apply=True,root_mode=0o555 if i%11==0 else 0o755)


def prepare(run,side,items,spec):
    requests=[];before=[]
    for item in items:
        folder=run/side/item['id']/'source';materialize_tree(folder,item['source'],spec);os.chmod(folder,item.get('root_mode',0o755))
        request=dict(id=item['id'],directory=str(folder),names=item['names'],options=item['options'],apply=item['apply']);requests.append(request)
        before.append(dict(id=item['id'],tree=snapshot(folder),root_mode=folder.stat().st_mode&0o777,identities={name:identity(folder/name) for name in item['names']}))
        if item.get('root_mode')==0o555 and os.access(folder,os.W_OK):raise ValueError('Permission fixture writable by executing uid')
    return requests,before


def preview_flags(rows):
    original={row['name']:i for i,row in enumerate(rows)};seen=set()
    for i,row in enumerate(rows):
        name=row['new_name']
        if name in seen:return True,False
        if name in original and original[name]!=i:return False,True
        seen.add(name)
    return False,False


def decode(text,requests):
    results=[json.loads(line) for line in text.splitlines()]
    if not requests or not any(r['names'] and r['options'] for r in requests) or [r.get('id') for r in results]!=[r['id'] for r in requests]:raise ValueError('Missing/reordered/zero-work batch cases')
    for request,result in zip(requests,results):
        if result.get('status')!='ok' or len(result.get('previews',[]))!=len(request['options'])+1:raise ValueError('Incomplete batch previews')
        for preview in result['previews']:
            if any(type(preview.get(key)) is not bool for key in ['duplicate_names','old_names_conflict','rename_enabled']) or type(preview.get('changed_count')) is not int:raise ValueError('Malformed batch flag/count types')
            rows=preview.get('rows',[])
            if [r.get('name') for r in rows]!=request['names'] or any(type(r.get('new_name')) is not str for r in rows):raise ValueError('Invalid batch rows')
            duplicate,conflict=preview_flags(rows)
            expected=dict(rows=rows,duplicate_names=duplicate,old_names_conflict=conflict,rename_enabled=not(duplicate or conflict),changed_count=sum(r['name']!=r['new_name'] for r in rows))
            if preview!=expected:raise ValueError('Inconsistent batch conflict/count/enable state')
        last=result['previews'][-1];unchanged=[r['name'] for r in last['rows'] if r['name']==r['new_name']]
        applied=request['apply'] and last['rename_enabled'] and last['changed_count']>0
        if type(result.get('applied')) is not bool or result['applied']!=applied or result.get('unchanged_names')!=unchanged:raise ValueError('Invalid batch apply/unchanged result')
        expected=[{k:r[k] for k in ['name','new_name']} for r in last['rows'] if r['name']!=r['new_name']] if applied else []
        outcomes=result.get('outcomes',[])
        if [{k:r.get(k) for k in ['name','new_name']} for r in outcomes]!=expected or any(type(r.get('success')) is not bool for r in outcomes):raise ValueError('Missing/malformed/reordered batch outcomes')
    return results


def observe(requests,items,before,results):
    after=[]
    for request,item,old,result in zip(requests,items,before,results):
        folder=Path(request['directory']);mask=source_date_mask(dict(item,source=item['source'])) if result['applied'] else []
        rows=result['previews'][-1]['rows'];identities=[]
        for row in rows:
            new=identity(folder/row['new_name']) if row['new_name'] and '/' not in row['new_name'] else None
            identities.append(dict(name=row['name'],identity_retained=new is not None and new==old['identities'][row['name']],destination_identity=new))
        state=dict(id=request['id'],tree=snapshot(folder,mask),root_mode=folder.stat().st_mode&0o777,identities=identities,ignored_dates=mask);after.append(state)
        if not result['applied'] and old['tree']!=state['tree']:raise ValueError('Preview-only/blocked batch changed the tree')
        for row in rows:
            r=dict(id=request['id'],names=[row['name']],new_name=row['new_name'] if result['applied'] else row['name'])
            assert_selected_content_preserved([r],[dict(source=old['tree'])],[dict(source=state['tree'],destination=state['tree'])])
    return after


def comparable(states):
    return [dict(s,identities=[{k:v for k,v in row.items() if k!='destination_identity'} for row in s['identities']]) for s in states]


def run_suite(compare,reserved,rust_adapter):
    if platform.system()!='Linux' or os.geteuid()==0:raise ValueError('W05 requires unprivileged Linux')
    acquisition.run(verify_only=True)
    run_id=datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')+'-'+uuid.uuid4().hex[:8];run=ROOT/'artifacts/runs'/('W05-'+run_id);run.mkdir(parents=True)
    spec=json.loads(SPEC.read_text());items=list(generated(spec,reserved)) if reserved else spec['cases']+list(generated(spec,False))
    requests,before=prepare(run,'original',items,spec);acquisition.write_json(run/'original-before.json',before)
    (run/'requests.jsonl').write_text(''.join(json.dumps(r)+'\n' for r in requests))
    jars=sorted((REFERENCE/'bundle').glob('*.jar'))+sorted((REFERENCE/'app').glob('*.jar'));cp=os.pathsep.join(map(str,jars));classes=run/'classes';classes.mkdir()
    _,compiler=capture(run,'javac',['javac','-proc:none','-encoding','UTF-8','-cp',cp,'-d',str(classes),str(SOURCE)])
    output,java=java_capture(run,['java','--add-opens=java.base/java.io=ALL-UNNAMED','-Djava.awt.headless=true','-Duser.home='+str(run/'home'),'-cp',str(classes)+os.pathsep+cp,'W05Oracle'],'',timeout=90)
    original=decode(output,requests);after=observe(requests,items,before,original);acquisition.write_json(run/'original-after.json',after)
    if not any(outcome['success'] for result in original for outcome in result['outcomes']):raise ValueError('No original batch rename succeeded')
    differences=[];commands=dict(javac=compiler,java=java)
    if compare:
        native_requests,native_before=prepare(run,'rust',items,spec)
        if [{k:v for k,v in b.items() if k!='identities'} for b in before]!=[{k:v for k,v in b.items() if k!='identities'} for b in native_before]:raise ValueError('Initial batch trees differ')
        acquisition.write_json(run/'rust-before.json',native_before);(run/'rust-requests.jsonl').write_text(''.join(json.dumps(r)+'\n' for r in native_requests))
        output,commands['rust']=capture(run,'rust',[str(rust_adapter),'--adapter-w05'],(run/'rust-requests.jsonl').read_text())
        native=decode(output,native_requests);native_after=observe(native_requests,items,native_before,native);acquisition.write_json(run/'rust-after.json',native_after)
        for left,right in zip(original,native):
            if left!=right:differences.append(dict(id=left['id'],component='preview_apply',expected=left,actual=right))
        for left,right in zip(comparable(after),comparable(native_after)):
            if left!=right:differences.append(dict(id=left['id'],component='filesystem',expected=left,actual=right))
    acquisition.run(verify_only=True)
    files=[SPEC,SOURCE,ROOT/'tools/w05.py',ROOT/'tools/w04.py',ROOT/'tools/w03.py',ROOT/'tools/w02c.py',ROOT/'tools/w01.py',ROOT/'Cargo.toml',ROOT/'Cargo.lock']
    if (EVIDENCE/'contract.md').exists():files.append(EVIDENCE/'contract.md')
    files+=sorted((ROOT/'crates').rglob('*.rs'))+sorted((ROOT/'crates').rglob('Cargo.toml'))
    report=dict(schema_version=1,workflow='W05-batch-rename',run_id=run_id,suite='reserved' if reserved else 'predefined',status=('differences' if differences else 'passed') if compare else 'oracle_ready',
        scope='Original batch pattern/preview methods and BatchRenameJob with fixed original SKIP choice; headless field-only UI wiring; full GUI/lifecycle separate',
        source_revision=subprocess.check_output(['git','rev-parse','HEAD'],text=True,cwd=ROOT).strip(),source_status=subprocess.check_output(['git','status','--porcelain'],text=True,cwd=ROOT).splitlines(),
        source_inputs=[dict(path=str(p.relative_to(ROOT)),sha256=digest(p)) for p in files],classpath=[dict(path=str(p.relative_to(ROOT)),sha256=digest(p)) for p in jars],
        java_adapter_classes=[dict(path=str(p.relative_to(ROOT)),sha256=digest(p)) for p in sorted(classes.glob('*.class'))],commands=commands,
        host=dict(system=platform.system(),architecture=platform.machine(),uid=os.geteuid(),locale='en-US',timezone='UTC',umask='022'),fixture_seed=spec['reserved_seed'] if reserved else spec['generated_seed'],
        oracle_cases=len(items),cases_run=len(items) if compare else 0,previews_checked=sum(len(r['previews']) for r in original) if compare else 0,
        row_previews_checked=sum(len(p['rows']) for r in original for p in r['previews']) if compare else 0,apply_jobs=sum(r['applied'] for r in original) if compare else 0,
        ordered_outcomes=sum(len(r['outcomes']) for r in original) if compare else 0,tree_checks=len(items) if compare else 0,content_conservation_checks=len(items)*(2 if compare else 1),
        identity_relation_checks=sum(len(r['names']) for r in requests) if compare else 0,differences=differences,skipped=[],reference_unchanged=True,
        raw_run=str(run.relative_to(ROOT)),request_sha256=digest(run/'requests.jsonl'),original_after_sha256=digest(run/'original-after.json'),rust_adapter=dict(path=str(rust_adapter).replace(str(ROOT),'{repository}'),sha256=digest(rust_adapter)) if compare else None,
        reproduce='target/debug/mucrab-compare --suite w05'+(' --reserved' if reserved else ''))
    acquisition.write_json(run/'report.json',report);acquisition.write_json(EVIDENCE/(('comparison-' if compare else 'oracle-')+report['suite']+'.json'),report)
    print('{}: {} batch cases, {} differences; {}'.format(report['status'],len(items),len(differences),run.relative_to(ROOT)))
    return int(bool(differences))


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('action',choices=['oracle','compare']);parser.add_argument('--reserved',action='store_true');parser.add_argument('--rust-adapter',type=Path,default=ROOT/'target/debug/mucrab-compare')
    args=parser.parse_args();mask=os.umask(0o022)
    try:return run_suite(args.action=='compare',args.reserved,args.rust_adapter.resolve())
    except (OSError,ValueError,KeyError,TypeError,subprocess.SubprocessError) as error:print('W05 unavailable: '+str(error),file=sys.stderr);return 2
    finally:os.umask(mask)


if __name__=='__main__':sys.exit(main())
