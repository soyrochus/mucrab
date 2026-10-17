#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Fresh W07–W12 native regressions; preserve all preceding workflow receipts."""
import argparse,json
from pathlib import Path
import w07_gui,w08_gui,w09_gui,w10_gui,w11_w12_gui
from w11_w12_regressions import actions_for
ROOT=Path(__file__).resolve().parents[1]
PREFIX='W13-regression'
def adjust(actions):
 for a in actions:
  if a.get('op')=='click' and a.get('x') in [62,65,66] and a.get('y')==347:a['y']=399
 return actions

def prepare():
 for n,mod,fixture in [(7,w07_gui,'fixtures/W07-background-operations/native-gui-actions.json'),(8,w08_gui,'fixtures/W08-desktop-ui/native-gui-actions.json'),(9,w09_gui,None),(10,w10_gui,'fixtures/W10-everyday-file-management/native-gui-actions.json')]:
  mod.RUN=ROOT/f'artifacts/gui/{PREFIX}-W{n:02}'
  if n==7:
   actions=adjust(actions_for(n,mod,fixture));mod.FIXTURE=mod.RUN.parent/f'{PREFIX}-W07-actions.json';mod.FIXTURE.write_text(json.dumps(actions,indent=2))
  mod.prepare();target=mod.FIXTURE if n==7 else mod.RUN/'updated-actions.json'
  if n!=7:target.write_text(json.dumps(adjust(actions_for(n,mod,fixture)),indent=2))
  print('xvfb-run -a python3 -B tools/linux_gui.py',mod.RUN/'launch.json','--replay',target)
 w11_w12_gui.RUN=ROOT/f'artifacts/gui/{PREFIX}-W11-W12';w11_w12_gui.prepare()
 for name in ['actions.json','restart-actions.json']:
  p=w11_w12_gui.RUN/name;p.write_text(json.dumps(adjust(json.loads(p.read_text())),indent=2))
 print('Replay the W11/W12 launch and restart-launch with their actions.json and restart-actions.json')

def verify(evidence_slugs=None):
 results={};w07_gui.RUN=ROOT/f'artifacts/gui/{PREFIX}-W07';w07_gui.FIXTURE=ROOT/f'artifacts/gui/{PREFIX}-W07-actions.json';results['W07']=w07_gui.verify_filesystem();w08_gui.RUN=ROOT/f'artifacts/gui/{PREFIX}-W08';w08_gui.verify_filesystem();results['W08']=dict(status='passed',scope='Operation trees, unchanged archive and persisted appearance')
 r=ROOT/f'artifacts/gui/{PREFIX}-W09';records=list(map(json.loads,(r/'opened.jsonl').read_text().splitlines()));assert len(records)==3;assert [v['contents'] for v in records]==['Local W09 document\n','W09 archive document\n','Local W09 document\n'];assert Path(records[1]['path']).exists();results['W09']=dict(status='passed',dispatches=3)
 r=ROOT/f'artifacts/gui/{PREFIX}-W10-retry' if (ROOT/f'artifacts/gui/{PREFIX}-W10-retry').exists() else ROOT/f'artifacts/gui/{PREFIX}-W10';assert (r/'left/Created').is_dir() and (r/'right/RightCreated').is_dir();assert not (r/'right/New folder').exists();assert (r/'right/alpha.txt').read_text()=='alpha.txt\n';assert not (r/'left/to-delete.txt').exists();assert not (r/'left/to-trash.txt').exists();assert (r/'data/Trash/files/to-trash.txt').read_text()=='to-trash.txt\n';results['W10']=dict(status='passed',scope='Folder, clipboard, Trash and delete outcomes')
 for n in [7,8,9,10]:
  r=ROOT/f'artifacts/gui/{PREFIX}-W{n:02}';r=r.with_name(r.name+'-retry') if n==10 and r.with_name(r.name+'-retry').exists() else r;expected=json.loads((w07_gui.FIXTURE if n==7 else r/'updated-actions.json').read_text());actual=json.loads((r/'native/actions.json').read_text());assert actual==[dict(op='capture',name='initial',wait=2)]+expected;results[f'W{n:02}']['actions']=len(actual);results[f'W{n:02}']['run']=str(r.relative_to(ROOT))
 r=ROOT/f'artifacts/gui/{PREFIX}-W11-W12';w11_w12_gui.RUN=r;assert w11_w12_gui.snapshot()==json.loads((r/'before.json').read_text());doc=json.loads((r/'config/mucrab/bookmarks.json').read_text());assert doc['entries'][0]['label']=='Workspace' and doc['entries'][0]['location']['location']==str(r/'left')
 for folder,actions in [('native','actions.json'),('restart','restart-actions.json')]:assert json.loads((r/folder/'actions.json').read_text())==[dict(op='capture',name='initial',wait=2)]+json.loads((r/actions).read_text())
 results['W11-W12']=dict(status='passed',run=str(r.relative_to(ROOT)),scope='Tabs/Home/marks/bookmark restart and both-pane search result navigation; exact trace and unchanged source bytes')
 report=dict(status='passed',results=results,note='Fresh isolated fixtures; close-menu coordinates account for View/Properties. Earlier evidence is unchanged.')
 for slug in (evidence_slugs or ['W13-properties-permissions','W14-built-in-viewer']):(ROOT/'evidence/workflows'/slug/'regressions.json').write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps(report,indent=2))
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('action',choices=['prepare','verify']);p.add_argument('--prefix',default=PREFIX);args=p.parse_args();PREFIX=args.prefix;globals()[args.action]()
