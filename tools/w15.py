#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Compare pinned provider list/streams with native SFTP over a real disposable server.

Skip is a native policy/integrity check; the original stream adapter is not an
oracle for CopyJob collision, trust, credentials UI or staged publication.
"""
import argparse,hashlib,json,os,subprocess,sys,time,uuid
from pathlib import Path
from datetime import datetime,timezone
import acquire_mucommander as acquisition
from w01 import ROOT,REFERENCE,digest,evidence_directory
from w15_prepare import prepare

def main(reserved,python):
 acquisition.run(verify_only=True)
 run=ROOT/'artifacts/runs'/('W15-'+datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')+'-'+uuid.uuid4().hex[:8]);prepare(run)
 serverlog=(run/'server.log').open('w');server=subprocess.Popen([python,'-B',str(ROOT/'tools/w15_server.py'),str(run)],stdout=serverlog,stderr=serverlog)
 try:
  for _ in range(100):
   if (run/'endpoint.json').exists():break
   assert server.poll() is None,'Disposable SFTP server unavailable; install pinned test-only Paramiko4.0.0';time.sleep(.05)
  endpoint=json.loads((run/'endpoint.json').read_text());fingerprint=subprocess.run(['ssh-keygen','-lf',str(run/'host.key.pub'),'-E','sha256'],capture_output=True,text=True,check=True).stdout.split()[1]
  cases=json.loads((ROOT/'fixtures/W15-sftp-connections'/('reserved.json'if reserved else'cases.json')).read_text());requests={};checks={}
  for side in ['original','native']:
   requests[side]=[];checks[side]={}
   for i,case in enumerate(cases):
    name=case['name'];remote=run/'remote'/side/case['id'];local=run/'cases'/side/case['id'];remote.mkdir(parents=True);local.mkdir(parents=True);data=bytes(j%251 for j in range(case['size']));upload=case['direction']=='upload';source,destination=(local/name,remote/name)if upload else(remote/name,local/name);source.write_bytes(data);os.utime(source,(1700000000+i,1700000000+i))
    if case['existing']:destination.write_bytes(b'prior destination')
    skip=case['existing']and case['policy']=='skip';checks[side][case['id']]=(source,destination,data,b'prior destination'if skip else data)
    common=dict(host=endpoint['host'],port=endpoint['port'],user='fixture',password='w15-disposable-only',fingerprint=fingerprint)
    if side=='native'or not skip:requests[side].append(dict(common,id=case['id'],operation=case['direction'],path=f'/{side}/{case["id"]}/{name}',local=str(local/name),overwrite=case['policy']=='overwrite'))
    requests[side].append(dict(common,id=case['id']+'-list',operation='list',path=f'/{side}/{case["id"]}'))
  jars=sorted(REFERENCE.glob('*/*.jar'));cp=os.pathsep.join(map(str,jars));classes=run/'classes';classes.mkdir();subprocess.run(['javac','-proc:none','-encoding','UTF-8','-cp',cp,'-d',str(classes),str(ROOT/'tools/java/W15Oracle.java')],check=True,capture_output=True)
  replies={}
  for side,cmd in [('original',['java','-Djava.awt.headless=true','-Duser.home='+str(run),' -cp'.strip(),str(classes)+os.pathsep+cp,'W15Oracle']),('native',[str(ROOT/'target/debug/mucrab-compare'),'--adapter-w15'])]:
   p=subprocess.run(cmd,input=''.join(json.dumps(v,ensure_ascii=False)+'\n'for v in requests[side]),text=True,capture_output=True,timeout=120);(run/(side+'-stdout.jsonl')).write_text(p.stdout);(run/(side+'-stderr.log')).write_text(p.stderr);assert p.returncode==0,p.stderr[-2000:];replies[side]={v['id']:v for v in map(json.loads,p.stdout.splitlines())};assert len(replies[side])==len(requests[side])
  differences=[];hashes={}
  for case in cases:
   key=case['id'];listings={}
   for side in ['original','native']:
    source,destination,data,expected=checks[side][key];assert source.read_bytes()==data,'Source changed';assert destination.read_bytes()==expected,(key,side,'Destination differs');hashes[side+'/'+key]=dict(source_sha256=digest(source),destination_sha256=digest(destination))
    result=replies[side].get(key)
    if result is not None and not result.get('ok'):differences.append(dict(id=key,side=side,result=result))
    entries=replies[side][key+'-list'].get('entries',[]);assert len(entries)==1,(key,side,entries);e=entries[0];stat=(run/'remote'/side/key/case['name']).lstat();assert e['modified_ms']==int(stat.st_mtime)*1000,(key,side,'Modified precision mismatch');listings[side]={k:e[k]for k in ['name','directory','link','size']}
   if listings['original']!=listings['native']:differences.append(dict(id=key,listings=listings))
  report=dict(status='passed'if not differences else'failed',reserved=reserved,cases=len(cases),stream_calls={k:sum(v['operation']!='list' for v in vals)for k,vals in requests.items()},listing_pairs=len(cases),unexpected_differences=differences,source_destination_hashes=hashes,run=str(run.relative_to(ROOT)),scope='Pinned SFTPFile provider listings and getInputStream/getOutputStream on actual disposable SFTP server; regular-file names/types/length and observed whole-second modified time; native Skip integrity separate from original stream semantics; no full original CopyJob/trust/UI equivalence',intentional_design_differences=['Native explicit first-use trust and changed-key refusal; recovered original UserInfo.promptYesNo returns true','Native staged writes and advertised POSIX rename; recovered original rename may delete destination first','Native does not follow links or support remote mutation beyond mixed Copy'],sources={p:digest(ROOT/p)for p in ['tools/w15.py','tools/java/W15Oracle.java','crates/compare/src/w15.rs','tools/w15_server.py']})
  out=evidence_directory('W15-sftp-connections')/('comparison-reserved.json'if reserved else'comparison.json');out.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(dict(status=report['status'],cases=len(cases),unexpected=len(differences),run=report['run']),indent=2));assert not differences
 finally:server.terminate();server.wait(timeout=5);serverlog.close()
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--reserved',action='store_true');p.add_argument('--server-python',default=sys.executable);a=p.parse_args();main(a.reserved,a.server_python)
