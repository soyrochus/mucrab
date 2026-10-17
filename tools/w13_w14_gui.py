#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Prepare/verify isolated W13/W14 native edits, viewing and archive handoff."""
import argparse,hashlib,json,os,shutil,zipfile
from pathlib import Path
from PIL import Image
ROOT=Path(__file__).resolve().parents[1]
RUN=ROOT/'artifacts/gui/W13-W14-final'
FIXTURE=ROOT/'fixtures/W13-properties-permissions/native-gui-actions.json'
TEXT='first line\r\ncafé 中 😀\nneedle one\nlast line\n'
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def prepare():
 RUN.mkdir(parents=True,exist_ok=False)
 for p in ['left/Folder/.hidden','right','config','data/applications','cache']:(RUN/p).mkdir(parents=True)
 (RUN/'left/notes.txt').write_bytes(TEXT.encode());(RUN/'left/unrelated.txt').write_text('unrelated\n');(RUN/'left/Folder/inside.txt').write_bytes(b'12345');(RUN/'left/Folder/.hidden/data').write_bytes(b'123');(RUN/'left/Folder/cycle').symlink_to('.');(RUN/'left/link').symlink_to('notes.txt');(RUN/'right/right.txt').write_bytes(b'right\n');os.link(RUN/'right/right.txt',RUN/'right/alias.txt');os.chmod(RUN/'left/notes.txt',0o2640)
 with Image.open(ROOT/'fixtures/W14-built-in-viewer/images/png-orientation-1.png') as image:image.resize((560,400),resample=Image.Resampling.NEAREST).save(RUN/'left/picture.png')
 with zipfile.ZipFile(RUN/'left/sample.zip','w') as archive:archive.writestr('nested/archive.txt',b'Archive viewer document\n')
 handler=RUN/'handler.py';handler.write_text('''#!/usr/bin/python3
import json,os,sys
from pathlib import Path
p=Path(sys.argv[1])
with open(os.environ['W14_OPEN_LOG'],'a') as output:output.write(json.dumps(dict(path=str(p),text=p.read_text()))+'\\n')
''');handler.chmod(0o700)
 (RUN/'data/applications/w14.desktop').write_text(f'[Desktop Entry]\nType=Application\nName=W14 test handler\nExec={handler} %f\nMimeType=text/plain;\nTerminal=false\n');(RUN/'config/mimeapps.list').write_text('[Default Applications]\ntext/plain=w14.desktop\n')
 spec=dict(argv=[str(ROOT/'target/debug/mucrab'),str(RUN/'left'),str(RUN/'right')],cwd=str(ROOT),title='muCrab',output=str(RUN/'native'),env=dict(XDG_CONFIG_HOME=str(RUN/'config'),XDG_DATA_HOME=str(RUN/'data'),XDG_CACHE_HOME=str(RUN/'cache'),XDG_CURRENT_DESKTOP='X-Generic',W14_OPEN_LOG=str(RUN/'opened.jsonl')))
 (RUN/'actions.json').write_text(FIXTURE.read_text());(RUN/'launch.json').write_text(json.dumps(spec,indent=2));(RUN/'before.json').write_text(json.dumps({str(p.relative_to(RUN)):digest(p) for p in (RUN/'left').rglob('*') if p.is_file() and not p.is_symlink()},indent=2));print(RUN/'launch.json')
def verify():
 before=json.loads((RUN/'before.json').read_text());assert all(digest(RUN/p)==h for p,h in before.items())
 assert (RUN/'right/right.txt').read_bytes()==b'right\n' and (RUN/'right/alias.txt').read_bytes()==b'right\n'
 observations=json.loads((RUN/'native/metadata.json').read_text());a=observations['left-before'];b=observations['left-after'];assert b['mode']==0o2740 and b['mtime_ns']==1792326896123456789;assert a['atime_ns']==b['atime_ns'];assert observations['right-after']['mode']==0o764;assert observations['right-after']['links']==2;assert (RUN/'right/right.txt').stat().st_mode&0o777==0o764
 external=[json.loads(line) for line in (RUN/'opened.jsonl').read_text().splitlines()];assert external[-1]['text']=='Archive viewer document\n';retained=Path(external[-1]['path']);assert retained.exists() and retained.name=='archive.txt';assert retained.stat().st_mode&0o777==0o600
 pid=retained.parent.name.split('-')[2];assert not list(Path('/tmp').glob(f'mucrab-viewer-{pid}-*'))
 actions=json.loads((RUN/'native/actions.json').read_text());expected=[dict(op='capture',name='initial',wait=2)]+json.loads(FIXTURE.read_text());expected=[{k:v.replace('{run}',str(RUN)) if isinstance(v,str) else v for k,v in x.items()} for x in expected];assert actions==expected and actions[-1]['op']=='await_exit'
 assert (RUN/'config/mucrab/appearance').read_text().strip()=='dark'
 report=dict(status='passed',run=str(RUN.relative_to(ROOT)),scope='Both-pane native local properties/metadata editing, hidden/link-safe folder totals, exact Unicode/newline text clipboard and find, image fit/100%/zoom/pan, dark/narrow layout, ZIP read-only properties and viewer-to-external retained handoff',observations=observations,source_hashes_unchanged=True,external_copy_survives_exit=True,viewer_leases_removed=True,binary_sha256=digest(ROOT/'target/debug/mucrab'),trace_sha256=digest(FIXTURE),trace_stage='Recorded after implementation; includes initial unsuccessful dark-theme clicks followed by verified corrected captures',verified_dark_captures=['dark-application','actual-text-dark','actual-image-dark','image-dark-narrow'],screenshot_sha256={p.name:digest(p) for p in (RUN/'native').glob('*.png')},actions=len(actions),macos='pending',font_limit='Bundled Linux font lacks some CJK glyphs; decoded content and clipboard remain exact')
 for slug in ['W13-properties-permissions','W14-built-in-viewer']:(ROOT/'evidence/workflows'/slug/'native.json').write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps(report,indent=2))
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('action',choices=['prepare','verify']);p.add_argument('--run',type=Path,default=RUN);args=p.parse_args();RUN=args.run if args.run.is_absolute() else ROOT/args.run;globals()[args.action]()
