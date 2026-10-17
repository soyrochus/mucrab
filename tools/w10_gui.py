#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Isolated W10 native replay preparation and exact result verification."""
import json
from pathlib import Path
import sys
ROOT=Path(__file__).resolve().parents[1]
RUN=ROOT/'artifacts/gui/W10-everyday'

def prepare():
 RUN.mkdir(parents=True,exist_ok=False)
 for name in ['left/Documents','right','config','data']: (RUN/name).mkdir(parents=True)
 for name in ['alpha.txt','beta.txt','to-delete.txt','to-trash.txt']:(RUN/'left'/name).write_text(name+'\n')
 spec=dict(argv=[str(ROOT/'target/debug/mucrab'),str(RUN/'left'),str(RUN/'right')],cwd=str(ROOT),title='muCrab',output=str(RUN/'native'),env=dict(XDG_CONFIG_HOME=str(RUN/'config'),XDG_DATA_HOME=str(RUN/'data')))
 (RUN/'launch.json').write_text(json.dumps(spec,indent=2))
 actions=json.loads((ROOT/'fixtures/W10-everyday-file-management/native-gui-actions.json').read_text())
 (RUN/'actions.json').write_text(json.dumps(actions,indent=2))
 print(RUN/'launch.json')

def verify():
 assert (RUN/'left/Created').is_dir()
 assert (RUN/'right/RightCreated').is_dir()
 assert not (RUN/'right/New folder').exists()
 assert (RUN/'right/alpha.txt').read_text()=='alpha.txt\n'
 assert (RUN/'left/alpha.txt').read_text()=='alpha.txt\n'
 assert not (RUN/'left/to-delete.txt').exists()
 assert not (RUN/'left/to-trash.txt').exists()
 trash=RUN/'data/Trash'
 assert (trash/'files/to-trash.txt').read_text()=='to-trash.txt\n'
 assert 'Path=' in (trash/'info/to-trash.txt.trashinfo').read_text()
 actual=json.loads((RUN/'native/actions.json').read_text())
 expected=[dict(op='capture',name='initial',wait=2)]+json.loads((ROOT/'fixtures/W10-everyday-file-management/native-gui-actions.json').read_text())
 assert actual==expected
 receipt=dict(status='passed',scope='Both-pane W10 native input, history, quick-find, clipboard Paste, Trash/permanent-delete reviews and exact filesystem outcomes',actions=len(actual),run=str(RUN.relative_to(ROOT)))
 (ROOT/'evidence/workflows/W10-everyday-file-management/native.json').write_text(json.dumps(receipt,indent=2)+'\n')
 print('W10 GUI filesystem and native Trash outcomes passed')
if __name__=='__main__':{'prepare':prepare,'verify':verify}[sys.argv[1]]()
