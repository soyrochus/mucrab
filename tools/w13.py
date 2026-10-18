#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Execute original metadata jobs and native plans on separate frozen fixtures."""
import argparse,json,os,platform,stat,subprocess,uuid
from pathlib import Path
from datetime import datetime,timezone
import acquire_mucommander as acquisition
from w01 import ROOT,REFERENCE,capture,digest,evidence_directory

def state(path):
 s=path.lstat();return dict(mode=stat.S_IMODE(s.st_mode),mtime_ns=s.st_mtime_ns,atime_ns=s.st_atime_ns,size=s.st_size,uid=s.st_uid,gid=s.st_gid,links=s.st_nlink,kind='Link' if path.is_symlink() else 'Folder' if path.is_dir() else 'File')
def main(reserved,fixture_root=None):
 acquisition.run(verify_only=True);cases=json.loads((ROOT/'fixtures/W13-properties-permissions'/('reserved.json' if reserved else 'cases.json')).read_text());run=ROOT/'artifacts/runs'/('W13-'+datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')+'-'+uuid.uuid4().hex[:8]);run.mkdir(parents=True);requests={};paths={};before={}
 data=run if fixture_root is None else fixture_root.resolve()/run.name
 for side in ['original','native']:
  requests[side]=[];paths[side]={};before[side]={}
  for case in cases:
   folder=data/side/case['id'];folder.mkdir(parents=True);p=folder/case['name'];target=folder/'target';kind=case['kind']
   if kind=='folder':p.mkdir()
   elif kind=='link':target.write_bytes(b'x'*case['size']);p.symlink_to('target')
   else:
    p.write_bytes(b'x'*case['size'])
    if kind=='hardlink':os.link(p,target)
   if kind!='link':os.utime(p,ns=(1_500_000_000_000_000_000,case['mtime_ms']*1_000_000));p.chmod(case['mode']|case['special'])
   paths[side][case['id']]=p;before[side][case['id']]=state(p);requests[side].append(dict(id=case['id'],path=str(p),new_mode=case['new_mode'],new_mtime_ms=case['new_mtime_ms']))
 cp=os.pathsep.join(map(str,sorted((REFERENCE/'bundle').glob('*.jar'))+sorted((REFERENCE/'app').glob('*.jar'))));classes=run/'classes';classes.mkdir();capture(run,'javac',['javac','-proc:none','-encoding','UTF-8','-cp',cp,'-d',str(classes),str(ROOT/'tools/java/W13Oracle.java')])
 replies={};after={}
 for side,cmd in [('original',['java','-Djava.awt.headless=true','-Duser.home='+str(run/'home'),'-cp',str(classes)+os.pathsep+cp,'W13Oracle']),('native',[str(ROOT/'target/debug/mucrab-compare'),'--adapter-w13'])]:
  payload=''.join(json.dumps(r,ensure_ascii=False)+'\n' for r in requests[side]);(run/(side+'-requests.jsonl')).write_text(payload);p=subprocess.run(cmd,input=payload,text=True,capture_output=True,timeout=60);(run/(side+'-stdout.jsonl')).write_text(p.stdout);(run/(side+'-stderr.log')).write_text(p.stderr);assert p.returncode==0,p.stderr[-3000:];replies[side]=[json.loads(l) for l in p.stdout.splitlines()];assert len(replies[side])==len(cases);after[side]={key:state(p) for key,p in paths[side].items()}
 unexpected=[];intentional=[]
 for case,a,b in zip(cases,replies['original'],replies['native']):
  key=case['id'];x=after['original'][key];y=after['native'][key];fields=[k for k in ['mode','mtime_ns','atime_ns','uid','gid','links','kind'] if x[k]!=y[k]]
  # Declared original differences must never excuse unrelated native changes.
  expected_native=before['native'][key].copy()
  if platform.system()=='Darwin' and case['kind']!='link':
   expected_native['atime_ns']=b['before']['accessed_ns']
  editable=case['kind']!='link' and bool(case['mode']&0o400)
  if editable:
   expected_native['mode']=case['special']|case['new_mode']
   expected_native['mtime_ns']=case['new_mtime_ms']*1_000_000
  # Reading target text can update the symlink's own atime; no edit was requested.
  if case['kind']=='link':expected_native['atime_ns']=y['atime_ns']
  if y!=expected_native or b['steps']!=[editable,editable]:unexpected.append(dict(id=key,native=y,expected_native=expected_native,steps=b['steps'],reason='Native mutation/preservation contract mismatch'))
  reasons=[]
  if platform.system()=='Darwin' and fields==['atime_ns'] and y['atime_ns']==expected_native['atime_ns']:reasons.append('Independent host access times differ; native preservation checked against its immediate pre-plan snapshot, retained in raw responses')
  if case['kind']=='link':reasons.append('Native lstat properties and link mutation rejection; original jobs can operate on targets')
  elif not case['mode']&0o400:reasons.append('Native combined plan fails before writing when a preserving time handle cannot be opened')
  if 'mode' in fields and x['mode']==((before['original'][key]['mode']&~a['before']['changeable'])|(case['new_mode']&a['before']['changeable'])):reasons.append('Original local provider exposes only a changeable permission mask of '+oct(a['before']['changeable'])+' in this isolated runtime; native supports all nine rwx bits')
  if case['special']:reasons.append('Native preserves special permission bits; original job receives the ordinary-bit replacement')
  if fields or a['steps']!=b['steps']:
   record=dict(id=key,fields=fields,original=x,native=y,original_steps=a['steps'],native_steps=b['steps'])
   (intentional if reasons else unexpected).append(dict(record,reasons=reasons))
  for side in ['original','native']:
   path=paths[side][key]
   if case['kind']!='link':path.chmod(0o700)
   if case['kind'] not in ['folder','link']:assert path.read_bytes()==b'x'*case['size']
 report=dict(status='passed' if not unexpected else 'failed',reserved=reserved,cases=len(cases),scope='Original ChangeFileAttributesJob.processFile permissions/date and native combined plans; filesystem mode/mtime/atime/owner/link-count read back; original lstat owner fields are not claimed',unexpected_differences=unexpected,intentional_differences=intentional,run=str(run.relative_to(ROOT)),sources={p:digest(ROOT/p) for p in ['tools/java/W13Oracle.java','crates/compare/src/w13.rs','tools/w13_w14_fixtures.py','tools/w13.py']})
 report['atime_boundary']='macOS: exact native pre-plan captured atime versus post-plan read-back; independent original/native host-access times are recorded separately' if platform.system()=='Darwin' else 'Exact frozen atime preservation and original/native read-back'
 report['fixture_root']=str(data);report['fixture_device']=data.stat().st_dev
 (run/'metadata-observations.json').write_text(json.dumps(dict(before=before,after=after),indent=2)+'\n')
 acquisition.run(verify_only=True);acquisition.write_json(evidence_directory('W13-properties-permissions')/('comparison-reserved.json' if reserved else 'comparison.json'),report);print(json.dumps(dict(status=report['status'],cases=len(cases),unexpected=len(unexpected),intentional=len(intentional),run=report['run']),indent=2));assert not unexpected,unexpected
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--reserved',action='store_true');p.add_argument('--fixture-root',type=Path);a=p.parse_args();main(a.reserved,a.fixture_root)
