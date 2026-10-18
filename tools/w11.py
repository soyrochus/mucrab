#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Supplemental original tab factory/bookmark value probe, not a frozen UI trace."""
import json,os,subprocess,uuid
from datetime import datetime,timezone
import acquire_mucommander as acquisition
from w01 import ROOT,REFERENCE,capture,digest,evidence_directory

def main():
 acquisition.run(verify_only=True)
 run=ROOT/'artifacts/runs'/('W11-probe-'+datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')+'-'+uuid.uuid4().hex[:8]);run.mkdir(parents=True);root=run/'root';child=root/'child';child.mkdir(parents=True)
 cp=os.pathsep.join(map(str,sorted((REFERENCE/'bundle').glob('*.jar'))+sorted((REFERENCE/'app').glob('*.jar'))));classes=run/'classes';classes.mkdir()
 capture(run,'javac',['javac','-proc:none','-encoding','UTF-8','-cp',cp,'-d',str(classes),str(ROOT/'tools/java/W11Oracle.java')])
 result={}
 for side,cmd in [('original',['java','-Djava.awt.headless=true','-Duser.home='+str(run/'home'),'-cp',str(classes)+os.pathsep+cp,'W11Oracle']),('native',[str(ROOT/'target/debug/mucrab-compare'),'--probe-w11'])]:
  p=subprocess.run(cmd+[str(root),str(child)],text=True,capture_output=True,timeout=30);(run/(side+'-stdout.json')).write_text(p.stdout);(run/(side+'-stderr.log')).write_text(p.stderr);assert p.returncode==0,p.stderr;result[side]=json.loads(p.stdout)
 differences=[k for k in result['original'] if k in result['native'] and result['original'][k]!=result['native'][k]]
 assert differences==['duplicate_history_length'],differences
 report=dict(status='passed',scope='Supplemental Default/ClonedFileTableTabFactory and Bookmark value methods; not full tab UI/job/persistence parity and not frozen before implementation',run=str(run.relative_to(ROOT)),results=result,intentional_differences={'duplicate_history_length':'Java factory starts a new empty history; W11 independently copies existing history'},original_only_observation='Bookmark.equals compares labels only; this is not a comparison against native storage validation, which is tested separately',sources={p:digest(ROOT/p) for p in ['tools/java/W11Oracle.java','crates/compare/src/w11.rs']})
 acquisition.run(verify_only=True);acquisition.write_json(evidence_directory('W11-tabs-bookmarks')/'original-probe.json',report);print(json.dumps(report,indent=2))
if __name__=='__main__':main()
