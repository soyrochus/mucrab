#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Prepare fresh adapted W07–W10 native replays or verify their exact outcomes.

After prepare, replay each printed launch/actions pair with tools/linux_gui.py
under isolated xvfb-run, then verify. Earlier run directories are preserved.
"""
import argparse,json
from pathlib import Path
import w07_gui,w08_gui,w09_gui,w10_gui
ROOT=Path(__file__).resolve().parents[1]

def actions_for(n,mod,fixture):
 actions=json.loads((ROOT/fixture if fixture else mod.RUN/'actions.json').read_text())
 for a in actions:
  if n==7 and a.get('op')=='click' and a.get('x')==80 and a.get('y') in [162,185,208,231]:a['y']+=27
  if n==8:
   if a.get('op')=='click' and ((a.get('x') in [20,80,650] and a.get('y') in [162,230,231,232,277]) or (a.get('x')==370 and a.get('y')==113)):a['y']+=27
   if a.get('op')=='drag' and a.get('y')==113:a['y']+=27;a['to_y']+=27
   if a.get('op')=='click' and a.get('x')==198 and a.get('y')==71:a['y']=209
  if n==9 and a.get('op')=='click' and a.get('y')==162:a['y']+=27
  if n==10 and a.get('op')=='click' and a.get('x') in [80,650] and a.get('y') in [162,185,208,231,254]:a['y']+=27
 return actions

def prepare():
 for n,mod,fixture in [(7,w07_gui,'fixtures/W07-background-operations/native-gui-actions.json'),(8,w08_gui,'fixtures/W08-desktop-ui/native-gui-actions.json'),(9,w09_gui,None),(10,w10_gui,'fixtures/W10-everyday-file-management/native-gui-actions.json')]:
  mod.RUN=ROOT/f'artifacts/gui/W11-regression-W{n:02}'
  if n==7:
   actions=actions_for(n,mod,fixture);mod.FIXTURE=mod.RUN.parent/'W11-W07-actions.json';mod.FIXTURE.write_text(json.dumps(actions,indent=2))
  mod.prepare()
  target=mod.FIXTURE if n==7 else mod.RUN/'w11-actions.json'
  if n!=7:target.write_text(json.dumps(actions_for(n,mod,fixture),indent=2))
  print(f'xvfb-run -a python3 -B tools/linux_gui.py {mod.RUN}/launch.json --replay {target}')

def verify():
 root=ROOT;results={}
 w07_gui.RUN=root/'artifacts/gui/W11-regression-W07';w07_gui.FIXTURE=root/'artifacts/gui/W11-W07-actions.json'
 results['W07']=w07_gui.verify_filesystem()
 w08_gui.RUN=root/'artifacts/gui/W11-regression-W08';w08_gui.verify_filesystem();results['W08']=dict(status='passed',checks='copy/rename/move/batch/ZIP filesystem, archive digest and saved system theme')
 r=root/'artifacts/gui/W11-regression-W09';records=[json.loads(l) for l in (r/'opened.jsonl').read_text().splitlines()];assert len(records)==3;assert [v['contents'] for v in records]==['Local W09 document\n','W09 archive document\n','Local W09 document\n'];assert Path(records[1]['path']).exists();results['W09']=dict(status='passed',dispatches=3,retained_archive_copy=True)
 r=root/'artifacts/gui/W11-regression-W10'
 assert (r/'left/Created').is_dir() and (r/'right/RightCreated').is_dir();assert not (r/'right/New folder').exists();assert (r/'right/alpha.txt').read_text()=='alpha.txt\n';assert (r/'left/alpha.txt').read_text()=='alpha.txt\n';assert not (r/'left/to-delete.txt').exists();assert not (r/'left/to-trash.txt').exists();assert (r/'data/Trash/files/to-trash.txt').read_text()=='to-trash.txt\n';assert 'Path=' in (r/'data/Trash/info/to-trash.txt.trashinfo').read_text();results['W10']=dict(status='passed',checks='both-pane new folder, copy/Paste, Trash and permanent-delete filesystem outcomes')
 for n in [7,8,9,10]:
  r=root/f'artifacts/gui/W11-regression-W{n:02}';expected=json.loads((w07_gui.FIXTURE if n==7 else r/'w11-actions.json').read_text());actual=json.loads((r/'native/actions.json').read_text());assert actual==[dict(op='capture',name='initial',wait=2)]+expected
  results[f'W{n:02}']['actions']=len(actual);results[f'W{n:02}']['run']=str(r.relative_to(root))
 report=dict(status='passed',note='Earlier native traces adapted for tab-strip +27px and Window menu; fresh isolated fixtures; no earlier receipts overwritten',results=results)
 for slug in ['W11-tabs-bookmarks','W12-file-search']:(root/'evidence/workflows'/slug/'regressions.json').write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps(report,indent=2))

if __name__=="__main__":
 p=argparse.ArgumentParser();p.add_argument("action",choices=["prepare","verify"]);args=p.parse_args();globals()[args.action]()
