#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Compare the frozen W12 filename cases against the pinned predicate only."""
import argparse,json,os,subprocess,uuid
from datetime import datetime,timezone
import acquire_mucommander as acquisition
from w01 import ROOT,REFERENCE,capture,digest,evidence_directory

def main(reserved):
 acquisition.run(verify_only=True)
 cases=json.loads((ROOT/'fixtures/W12-file-search'/('reserved.json' if reserved else 'cases.json')).read_text())
 run=ROOT/'artifacts/runs'/('W12-'+datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')+'-'+uuid.uuid4().hex[:8]);run.mkdir(parents=True)
 root=run/'tree';root.mkdir()
 for c in cases:(root/c['name']).touch()
 payload=''.join(json.dumps(dict(c,root=str(root)),ensure_ascii=False)+'\n' for c in cases)
 (run/'requests.jsonl').write_text(payload)
 cp=os.pathsep.join(map(str,sorted((REFERENCE/'bundle').glob('*.jar'))+sorted((REFERENCE/'app').glob('*.jar'))));classes=run/'classes';classes.mkdir()
 capture(run,'javac',['javac','-proc:none','-encoding','UTF-8','-cp',cp,'-d',str(classes),str(ROOT/'tools/java/W12Oracle.java')])
 responses={};observed_names={}
 for side,cmd in [('original',['java','-Djava.awt.headless=true','-Duser.home='+str(run/'home'),'-cp',str(classes)+os.pathsep+cp,'W12Oracle']),('native',[str(ROOT/'target/debug/mucrab-compare'),'--adapter-w12'])]:
  if side=='native':
   observed_names={r['id']:r.pop('observed_name') for r in responses['original']}
   payload=''.join(json.dumps(dict(c,root=str(root),name=observed_names[c['id']]),ensure_ascii=False)+'\n' for c in cases)
   (run/'native-requests.jsonl').write_text(payload)
  p=subprocess.run(cmd,input=payload,text=True,capture_output=True,timeout=60);(run/(side+'-stdout.jsonl')).write_text(p.stdout);(run/(side+'-stderr.log')).write_text(p.stderr)
  assert p.returncode==0,p.stderr[-3000:];responses[side]=[json.loads(l) for l in p.stdout.splitlines()];assert len(responses[side])==len(cases)
 differences=[];intentional=[]
 for case,a,b in zip(cases,responses['original'],responses['native']):
  if a!=b:
   if '\\' in case['pattern']:intentional.append(dict(case=case,original=a,native=b,reason='W12 backslash escapes the next scalar; original wildcard conversion treats backslash literally before expanding the star'))
   else:differences.append(dict(case=case,original=a,native=b))
 report=dict(status='passed' if not differences else 'failed',cases=len(cases),reserved=reserved,scope='Pinned SearchBuilder.createFilenamePredicate only; metadata/traversal/GUI checked separately, no full original search-job parity claim',unexpected_differences=differences,intentional_differences=intentional,run=str(run.relative_to(ROOT)),sources={p:digest(ROOT/p) for p in ['tools/java/W12Oracle.java','crates/compare/src/w12.rs','tools/w11_w12_fixtures.py']})
 report['observed_filename_inputs']={c['id']:dict(requested=c['name'],observed=observed_names[c['id']]) for c in cases if c['name']!=observed_names[c['id']]}
 report['filename_boundary']='Both predicates receive the original provider getName() value; raw spellings remain in original stdout and native requests. macOS traversal separately normalizes to NFC like pane listings.'
 acquisition.run(verify_only=True);acquisition.write_json(evidence_directory('W12-file-search')/('comparison-reserved.json' if reserved else 'comparison.json'),report);print(json.dumps(report,indent=2));assert not differences
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--reserved',action='store_true');main(p.parse_args().reserved)
