#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Record final W15 workspace checks, freezes/notices and actual source provenance."""
from datetime import datetime,timezone
import hashlib,json,re,subprocess,uuid
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 run=ROOT/'artifacts/runs'/('W15-validation-'+datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')+'-'+uuid.uuid4().hex[:8]);run.mkdir(parents=True)
 commands=[['cargo','fmt','--all','--','--check'],['cargo','build','--workspace','--all-features','--locked','--offline'],['cargo','test','--workspace','--all-features','--locked','--offline'],['cargo','clippy','--workspace','--all-targets','--all-features','--locked','--offline','--','-D','warnings'],['python3','-B','-m','unittest','discover','-s','tools','-p','test_*.py'],['python3','-B','tools/w15_fixtures.py'],['openspec','validate','w15-sftp-connections','--strict']]
 checks=[];counts={}
 for i,cmd in enumerate(commands):
  p=subprocess.run(cmd,cwd=ROOT,capture_output=True,text=True,timeout=180);out=run/f'{i}-stdout.log';err=run/f'{i}-stderr.log';out.write_text(p.stdout);err.write_text(p.stderr);assert p.returncode==0,p.stdout[-4000:]+p.stderr[-4000:];checks.append(dict(command=cmd,exit_code=p.returncode,stdout=str(out.relative_to(ROOT)),stderr=str(err.relative_to(ROOT))))
  if cmd[:2]==['cargo','test']:counts['rust_passed']=sum(map(int,re.findall(r'test result: ok\. (\d+) passed;',p.stdout)));counts['rust_ignored']=sum(map(int,re.findall(r'test result: ok\. \d+ passed; \d+ failed; (\d+) ignored;',p.stdout)))
  if 'unittest'in cmd:counts['python_passed']=int(re.search(r'Ran (\d+) tests',p.stderr)[1])
 out=ROOT/'evidence/workflows/W15-sftp-connections';freeze=json.loads((out/'fixture-freeze.json').read_text());errata=json.loads((out/'fixture-errata.json').read_text())
 for file,h in freeze['files'].items():assert digest(ROOT/file)==(errata['after_sha256']if file==errata['file']else h)
 notices=json.loads((ROOT/'third_party/sftp/manifest.json').read_text())
 for entry in notices:
  for file,h in entry['license_files'].items():assert digest(ROOT/'third_party/sftp'/(entry['name']+'-'+entry['version'])/file)==h
 sources={str(p.relative_to(ROOT)):digest(p)for folder in ['crates/core/src','crates/core/tests','crates/desktop/src','crates/compare/src']for p in (ROOT/folder).rglob('*.rs')}
 for name in ['Cargo.toml','Cargo.lock','crates/core/Cargo.toml','crates/desktop/Cargo.toml','.github/workflows/release.yml']:sources[name]=digest(ROOT/name)
 report=dict(status='passed',date=datetime.now(timezone.utc).isoformat(),host='Linux x86_64',run=str(run.relative_to(ROOT)),counts=counts,checks=checks,source_sha256=sources,binary_sha256=digest(ROOT/'target/debug/mucrab'),frozen_recipes_unchanged=True,server_errata_verified=True,notice_hashes_verified=True,separate_gates='Real protocol/native credential/native UI and earlier candidate regressions have separate receipts; macOS build/Keychain/UI and broader original/remote-active-job input coverage remain pending')
 (out/'validation.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(dict(status='passed',counts=counts,run=report['run']),indent=2))
if __name__=='__main__':main()
