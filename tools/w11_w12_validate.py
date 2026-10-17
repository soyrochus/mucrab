#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Record actual W11/W12 workspace checks separately from original/native gates."""
from datetime import datetime,timezone
import hashlib,json,re,subprocess,uuid
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]

def main():
 run=ROOT/'artifacts/runs'/('W11-W12-validation-'+datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')+'-'+uuid.uuid4().hex[:8]);run.mkdir(parents=True)
 commands=[['cargo','fmt','--all','--','--check'],['cargo','build','--workspace','--all-features','--locked','--offline'],['cargo','test','--workspace','--all-features','--locked','--offline'],['cargo','clippy','--workspace','--all-targets','--all-features','--locked','--offline','--','-D','warnings'],['python3','-B','-m','unittest','discover','-s','tools','-p','test_*.py'],['python3','-B','tools/w11_w12_fixtures.py'],['openspec','validate','w11-tabs-bookmarks','--strict'],['openspec','validate','w12-file-search','--strict']]
 results=[];counts={}
 for i,cmd in enumerate(commands):
  p=subprocess.run(cmd,cwd=ROOT,capture_output=True,text=True,timeout=120);out=run/f'{i}-stdout.log';err=run/f'{i}-stderr.log';out.write_text(p.stdout);err.write_text(p.stderr);results.append(dict(command=cmd,exit_code=p.returncode,stdout=str(out.relative_to(ROOT)),stderr=str(err.relative_to(ROOT))))
  assert p.returncode==0,p.stdout[-4000:]+p.stderr[-4000:]
  if cmd[0:2]==['cargo','test']:
   counts['rust_passed']=sum(map(int,re.findall(r'test result: ok\. (\d+) passed;',p.stdout)));counts['rust_ignored']=sum(map(int,re.findall(r'test result: ok\. \d+ passed; \d+ failed; (\d+) ignored;',p.stdout)))
  if 'unittest' in cmd:counts['python_passed']=int(re.search(r'Ran (\d+) tests',p.stderr)[1])
 sources={str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for folder in ['crates/core/src','crates/core/tests','crates/desktop/src','crates/compare/src'] for p in (ROOT/folder).glob('*.rs')}
 for slug in ['W11-tabs-bookmarks','W12-file-search']:
  frozen=json.loads((ROOT/'evidence/workflows'/slug/'fixture-freeze.json').read_text())['files']
  assert all(hashlib.sha256((ROOT/p).read_bytes()).hexdigest()==h for p,h in frozen.items())
 report=dict(status='passed',date=datetime.now(timezone.utc).isoformat(),run=str(run.relative_to(ROOT)),counts=counts,checks=results,source_sha256=sources,binary_sha256=hashlib.sha256((ROOT/'target/debug/mucrab').read_bytes()).hexdigest(),frozen_generators_unchanged=True,host='Linux x86_64',separate_gates='Native receipts and original probes are separate; ignored W10 clipboard/Trash test is not counted as executed here; native macOS and broader original-window checks remain pending')
 for slug in ['W11-tabs-bookmarks','W12-file-search']:(ROOT/'evidence/workflows'/slug/'validation.json').write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps(dict(status=report['status'],counts=counts,run=report['run']),indent=2))
if __name__=='__main__':main()
