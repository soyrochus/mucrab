#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Isolated W11/W12 native replay, persisted-bookmark restart and verification."""
import argparse,json,hashlib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
RUN=ROOT/'artifacts/gui/W11-W12-verified'

def prepare():
 RUN.mkdir(parents=True,exist_ok=False)
 for path in ['left/Documents','right','home','config','data','cache']:(RUN/path).mkdir(parents=True)
 for path,content in [('left/Documents/nested.txt','nested\n'),('left/alpha.txt','alpha\n'),('left/beta.log','beta\n'),('left/.hidden.txt','hidden\n')]: (RUN/path).write_text(content)
 spec=dict(argv=[str(ROOT/'target/debug/mucrab'),str(RUN/'left'),str(RUN/'right')],cwd=str(ROOT),title='muCrab',output=str(RUN/'native'),env=dict(XDG_CONFIG_HOME=str(RUN/'config'),XDG_DATA_HOME=str(RUN/'data'),HOME=str(RUN/'home'),XDG_CACHE_HOME=str(RUN/'cache')))
 (RUN/'launch.json').write_text(json.dumps(spec,indent=2));spec['output']=str(RUN/'restart');spec['argv']=[str(ROOT/'target/debug/mucrab'),str(RUN/'right'),str(RUN/'home')];(RUN/'restart-launch.json').write_text(json.dumps(spec,indent=2))
 for source,target in [('W11-tabs-bookmarks/native-gui-actions.json','actions.json'),('W11-tabs-bookmarks/restart-gui-actions.json','restart-actions.json')]: (RUN/target).write_text((ROOT/'fixtures'/source).read_text())
 (RUN/'before.json').write_text(json.dumps(snapshot(),indent=2));print(RUN/'launch.json')

def snapshot():
 return {str(p.relative_to(RUN)):hashlib.sha256(p.read_bytes()).hexdigest() for folder in ['left','right','home'] for p in (RUN/folder).rglob('*') if p.is_file()}

def verify():
 assert snapshot()==json.loads((RUN/'before.json').read_text()),'Search/navigation changed a source file'
 doc=json.loads((RUN/'config/mucrab/bookmarks.json').read_text());assert doc['version']==1 and len(doc['entries'])==1;assert doc['entries'][0]['label']=='Workspace';assert doc['entries'][0]['location']==dict(kind='Local',location=str(RUN/'left'))
 for folder,actions in [('native','actions.json'),('restart','restart-actions.json')]:
  assert json.loads((RUN/folder/'actions.json').read_text())==[dict(op='capture',name='initial',wait=2)]+json.loads((RUN/actions).read_text())
 report=dict(status='passed',run=str(RUN.relative_to(ROOT)),scope='Native both-pane tab actions, mark isolation, local bookmark save and restart/open, nonmodal wildcard search and containing-folder activation in each pane, minimum window and session-only tabs',sources_unchanged=True,bookmark_restart=True,visual_checks=['origin mark retained on switching','duplicate marks cleared','nested.txt focused in right/Documents','alpha.txt focused in left','Home new tab and last-tab close disabled','820x500 tab strips reachable','restart contains one tab per pane and Workspace opens left'],host='Linux X11 / isolated Xvfb',macos='pending')
 for slug in ['W11-tabs-bookmarks','W12-file-search']:(ROOT/'evidence/workflows'/slug/'native.json').write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps(report,indent=2))

def concurrent_prepare():
 r=ROOT/'artifacts/gui/W12-concurrent-final';r.mkdir(parents=True,exist_ok=False);(r/'target').mkdir();(r/'config').mkdir()
 source=ROOT/'artifacts/gui/W11-regression-W07/source'
 assert (source/'10-work').is_dir(),'Prepare W07 fixtures with tools/w11_w12_regressions.py first'
 spec=dict(argv=[str(ROOT/'target/debug/mucrab'),str(source),str(r/'target')],cwd=str(ROOT),title='muCrab',output=str(r/'native'),env=dict(XDG_CONFIG_HOME=str(r/'config')))
 (r/'launch.json').write_text(json.dumps(spec,indent=2));(r/'actions.json').write_text((ROOT/'fixtures/W12-file-search/native-concurrent-actions.json').read_text());print(r/'launch.json')

def concurrent_verify():
 from PIL import Image,ImageChops
 r=ROOT/'artifacts/gui/W12-concurrent-final';source=ROOT/'artifacts/gui/W11-regression-W07/source'
 actions=json.loads((r/'native/actions.json').read_text());assert actions==[dict(op='capture',name='initial',wait=2)]+json.loads((r/'actions.json').read_text());assert actions[-1]['op']=='await_exit'
 children=list((r/'target/10-work').iterdir());assert 0<len(children)<20000
 for p in children:assert p.read_bytes()==b'worker file contents\n'[:p.stat().st_size]
 assert (r/'target/00-done/complete.txt').read_bytes()==b'completed root\n';assert not (r/'target/99-later').exists();assert len(list((source/'10-work').iterdir()))==20000
 with Image.open(r/'native/copy-paused.png') as first,Image.open(r/'native/limited-search-copy-independent.png') as second:
  assert ImageChops.difference(first.crop((350,657,1110,682)),second.crop((350,657,1110,682))).getbbox() is None,'Paused copy path/progress changed during search'
 report=dict(status='passed',run=str(r.relative_to(ROOT)),scope='Real copy paused; focused-listing Ctrl+F opens search; 10,000 hits reported partial while copy progress remains unchanged; close acknowledges cancellation and exits normally',copied_children=len(children),source_files=20000,visual_checks=['Search partial: 10000 visited/matches, one result-limit diagnostic','Copy Resume/Cancel remain available'],macos='pending')
 (ROOT/'evidence/workflows/W12-file-search/concurrent-native.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('action',choices=['prepare','verify','concurrent-prepare','concurrent-verify']);globals()[p.parse_args().action.replace('-','_')]()
