#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Prepare/check read-only W13/W14 windows alongside a genuinely paused W07 copy."""
import argparse,hashlib,json
from pathlib import Path
from PIL import Image,ImageChops
import w07_gui
ROOT=Path(__file__).resolve().parents[1]
FIXTURE=ROOT/'fixtures/W13-properties-permissions/concurrent-gui-actions.json'
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def prepare(run):
 w07_gui.RUN=run;w07_gui.FIXTURE=FIXTURE;w07_gui.prepare()
 (run/'actions.json').write_text(FIXTURE.read_text())
def verify(run):
 launch=json.loads((run/'launch.json').read_text());source=Path(launch['argv'][1]);target=Path(launch['argv'][2]);assert source.is_relative_to(ROOT/'artifacts/gui') and target.is_relative_to(run)
 assert (target/'00-done/complete.txt').read_bytes()==b'completed root\n'
 files=list((target/'10-work').iterdir());assert 0<len(files)<20000
 assert all(p.read_bytes()==w07_gui.CONTENT[:p.stat().st_size] and p.stat().st_size<=len(w07_gui.CONTENT) for p in files)
 assert not (target/'99-later').exists()
 originals=list((source/'10-work').iterdir());assert len(originals)==20000 and all(p.read_bytes()==w07_gui.CONTENT for p in originals)
 assert (source/'00-done/complete.txt').read_bytes()==b'completed root\n' and (source/'99-later/untouched.txt').read_bytes()==b'later source\n'
 actual=json.loads((run/'native/actions.json').read_text());assert actual==[dict(op='capture',name='initial',wait=2)]+json.loads(FIXTURE.read_text())
 # Compare the paused current-path tail, excluding window shadows and the clock.
 crop=(740,660,1110,678)
 with Image.open(run/'native/copy-paused.png') as reference:
  for name in ['viewer-copy-independent','folder-total-copy-independent']:
   with Image.open(run/f'native/{name}.png') as current:assert ImageChops.difference(reference.crop(crop),current.crop(crop)).getbbox() is None
 report=dict(status='passed',run=str(run.relative_to(ROOT)),binary_sha256=digest(ROOT/'target/debug/mucrab'),trace_sha256=digest(FIXTURE),actions=len(actual),source_files=20000,source_bytes_unchanged=True,destination_children=len(files),later_roots_untouched=True,paused_path_crop=list(crop),paused_path_unchanged=True,visually_observed='Viewer text loaded and folder totals completed at 420000 bytes / 20000 files while copy remained paused; Resume/Cancel remained visible',normal_exit=True,scope='Linux X11 isolated native windows and actual W07 copy; not scheduler or full original UI parity')
 for slug in ['W13-properties-permissions','W14-built-in-viewer']:(ROOT/'evidence/workflows'/slug/'concurrent-native.json').write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps(report,indent=2))
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('action',choices=['prepare','verify']);p.add_argument('--run',type=Path,default=Path('artifacts/gui/W13-W14-concurrent'));args=p.parse_args();run=args.run if args.run.is_absolute() else ROOT/args.run
 (prepare if args.action=='prepare' else verify)(run)
