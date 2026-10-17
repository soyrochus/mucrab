#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Bounded W10 pinned-job/history/matching comparisons; separate native gates."""
import argparse
from datetime import datetime,timezone
import hashlib,json,os,random,subprocess,uuid
from pathlib import Path
import acquire_mucommander as acquisition
from w01 import ROOT,REFERENCE,capture,digest

EVIDENCE=ROOT/'evidence/workflows/W10-everyday-file-management'
def snapshot(root):
 out=[]
 for p in sorted(root.rglob('*')):
  out.append(dict(path=str(p.relative_to(root)),kind='link' if p.is_symlink() else 'directory' if p.is_dir() else 'file',content=os.readlink(p) if p.is_symlink() else hashlib.sha256(p.read_bytes()).hexdigest() if p.is_file() else None))
 return out

def main(reserved):
 acquisition.run(verify_only=True)
 spec=json.loads((ROOT/'fixtures/W10-everyday-file-management/contract.json').read_text());rng=random.Random(spec['reserved_seed'] if reserved else spec['predefined_seed'])
 run=ROOT/'artifacts/runs'/('W10-'+datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')+'-'+uuid.uuid4().hex[:8]);run.mkdir(parents=True)
 requests={};trees={}
 for side in ['original','native']:
  rng=random.Random(spec['reserved_seed'] if reserved else spec['predefined_seed'])
  root=run/side;root.mkdir();items=[]
  for i in range(24):
   # The seed is frozen; generated cases are not claimed as blind holdouts.
   tag=f'case{i}'
   for op in ['mkdir','delete','copy']:
    base=root/(tag+'-'+op);base.mkdir();src=base/'source';src.mkdir();dst=base/'destination';dst.mkdir()
    name=rng.choice(['café 中','space & %.txt','- leading','plain'])+str(i)+'-'+str(rng.randrange(10000))
    if op=='mkdir':paths=[src/name]
    else:
     (src/'folder').mkdir();(src/'folder/.hidden').write_text('hidden');(src/name).write_bytes(rng.randbytes(17))
     (src/'outside').write_text('outside');(src/'folder/link').symlink_to('../outside');(src/'broken').symlink_to('missing')
     paths=[src/'folder',src/name,src/'broken']
     if op=='copy':
      other=base/'other';other.mkdir();(other/'extra').write_text('other parent');paths.append(other/'extra')
    items.append(dict(id=tag+'-'+op,operation=op,directory=str(src),destination=str(dst),paths=list(map(str,paths))))
   # Advance seeded generator identically on each side, without affecting paths.
  for i,(name,query,expected) in enumerate(spec['quick_find']['examples']):items.append(dict(id=f'quick{i}',operation='quick',name=name,query=query))
  for i,count in enumerate([3,105,4]):
   base=root/('history'+str(i));base.mkdir();paths=[]
   for j in range(count):p=base/str(j);p.mkdir();paths.append(str(p))
   r=dict(id='history'+str(i),operation='history',paths=paths)
   if i==2:
    branch=base/'branch';branch.mkdir();r.update(rewind=1,destination=str(branch))
   items.append(r)
  # Randomized but reproducible request order in the reserved execution.
  if reserved:random.Random(spec['reserved_seed']).shuffle(items)
  requests[side]=items;trees[side]=root
  (run/(side+'-requests.jsonl')).write_text(''.join(json.dumps(r,ensure_ascii=False)+'\n' for r in items))
 jars=sorted((REFERENCE/'bundle').glob('*.jar'))+sorted((REFERENCE/'app').glob('*.jar'));cp=os.pathsep.join(map(str,jars));classes=run/'classes';classes.mkdir()
 capture(run,'javac',['javac','-proc:none','-encoding','UTF-8','-cp',cp,'-d',str(classes),str(ROOT/'tools/java/W10Oracle.java')])
 responses={}
 for side,command in [('original',['java','--add-opens=java.base/java.io=ALL-UNNAMED','-Djava.awt.headless=true','-Duser.language=en','-Duser.country=US','-Duser.home='+str(run/'home'),'-cp',str(classes)+os.pathsep+cp,'W10Oracle']),('native',[str(ROOT/'target/debug/mucrab-compare'),'--adapter-w10'])]:
  payload=(run/(side+'-requests.jsonl')).read_text();p=subprocess.run(command,input=payload,text=True,capture_output=True,timeout=90)
  (run/(side+'-stdout.jsonl')).write_text(p.stdout);(run/(side+'-stderr.log')).write_text(p.stderr)
  if p.returncode:raise RuntimeError(side+' adapter failed: '+p.stderr[-2000:])
  rows=[json.loads(l) for l in p.stdout.splitlines()];assert len(rows)==len(requests[side]);responses[side]={r['id']:r for r in rows}
 differences=[];intentional=[]
 for key,a in responses['original'].items():
  b=responses['native'][key]
  if a!=b:
   if key=='quick1' and a['matches'] and not b['matches']:intentional.append(dict(id=key,reason='Declared prefix-only vs original substring matching',original=a,native=b))
   else:differences.append(dict(id=key,original=a,native=b))
 # History fixture names are the same; only compare paths/contents/kinds.
 assert snapshot(trees['original'])==snapshot(trees['native']),'Filesystem trees differ'
 report=dict(status='passed' if not differences else 'failed',scope='MkdirJob/DeleteJob/CopyJob.processFile, LocalLocationHistory additions/branching/capacity and QuickSearch.matches; no UI or Trash parity claim',reserved=reserved,cases=len(responses['native']),unexpected_differences=differences,intentional_differences=intentional,filesystem_equal=True,seed=spec['reserved_seed'] if reserved else spec['predefined_seed'],run=str(run.relative_to(ROOT)),source_sha256={n:digest(ROOT/n) for n in ['tools/java/W10Oracle.java','crates/compare/src/w10.rs','crates/core/src/everyday.rs','crates/core/src/history.rs']},reference_unchanged=True)
 acquisition.run(verify_only=True);acquisition.write_json(run/'comparison.json',report);acquisition.write_json(EVIDENCE/('comparison-reserved.json' if reserved else 'comparison.json'),report)
 print(json.dumps(report,indent=2));assert not differences
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--reserved',action='store_true');main(p.parse_args().reserved)
