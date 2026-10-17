#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Configure/verify a native SFTP replay after w15_prepare.py + w15_server.py."""
import argparse,hashlib,json,stat
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
FIXTURE=ROOT/'fixtures/W15-sftp-connections/native-gui-actions.json'
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def configure(run):
 endpoint=json.loads((run/'endpoint.json').read_text());(run/'config-final/mucrab').mkdir(parents=True,exist_ok=False)
 profile=dict(id='server-1',label='SFTP server',host=endpoint['host'],port=endpoint['port'],user=endpoint['user'],authentication=dict(method='Password'),directory='/',remembered=False)
 (run/'config-final/mucrab/sftp.json').write_text(json.dumps(dict(version=1,profiles=[profile],trusted=[]),indent=2))
 (run/'data/applications').mkdir(parents=True,exist_ok=True);handler=run/'handler.py';handler.write_text('#!/usr/bin/python3\nimport json,sys\nfrom pathlib import Path\np=Path(sys.argv[1])\nwith open('+repr(str(run/'opened.jsonl'))+',"a") as f:f.write(json.dumps(dict(path=str(p),text=p.read_text()))+"\\n")\n');handler.chmod(0o700)
 (run/'data/applications/w15.desktop').write_text(f'[Desktop Entry]\nType=Application\nName=Disposable SFTP handler\nExec={handler} %f\nMimeType=text/plain;\nTerminal=false\n');(run/'config-final/mimeapps.list').write_text('[Default Applications]\ntext/plain=w15.desktop\n');(run/'local/upload.txt').write_bytes(b'Native upload from right pane\n')
 spec=json.loads((run/'launch.json').read_text());spec['output']=str(run/'native-final');spec['env']['XDG_CONFIG_HOME']=str(run/'config-final');(run/'final-launch.json').write_text(json.dumps(spec,indent=2));(run/'actions.json').write_text(FIXTURE.read_text());(run/'before.json').write_text(json.dumps({str(p.relative_to(run)):digest(p)for folder in ['remote','local']for p in (run/folder).rglob('*')if p.is_file()and not p.is_symlink()},indent=2));(run/'provenance.json').write_text(json.dumps(dict(binary_sha256=digest(ROOT/'target/debug/mucrab'),trace_sha256=digest(FIXTURE)),indent=2));print(run/'final-launch.json')
def verify(run):
 provenance=json.loads((run/'provenance.json').read_text());assert provenance['binary_sha256']==digest(ROOT/'target/debug/mucrab')and provenance['trace_sha256']==digest(FIXTURE)
 actions=json.loads((run/'native-final/actions.json').read_text());assert actions==[dict(op='capture',name='initial',wait=2)]+json.loads(FIXTURE.read_text());assert actions[-1]['op']=='await_exit'
 before=json.loads((run/'before.json').read_text());assert all(digest(run/p)==h for p,h in before.items());assert (run/'remote/upload.txt').read_bytes()==b'Native upload from right pane\n';assert (run/'local/folder/.hidden/data').read_bytes()==b'hidden';assert (run/'local/folder/file.bin').read_bytes()==(run/'remote/folder/file.bin').read_bytes()
 doc=json.loads((run/'config-final/mucrab/sftp.json').read_text());assert len(doc['trusted'])==1 and not doc['profiles'][0]['remembered'];raw=(run/'config-final/mucrab/sftp.json').read_text();assert 'w15-disposable-only'not in raw and 'w15-disposable-only'not in (run/'native-final/application.log').read_text();bookmarks=json.loads((run/'config-final/mucrab/bookmarks.json').read_text());assert bookmarks['entries'][0]['location']==dict(kind='Sftp',location=dict(profile='server-1',path='/'))
 opened=json.loads((run/'opened.jsonl').read_text().splitlines()[-1]);path=Path(opened['path']);assert path.exists()and path.name=='notes.txt'and path.stat().st_mode&0o777==0o600 and opened['text']=='SFTP document café\n';assert path.read_bytes()==(run/'remote/notes.txt').read_bytes()
 report=dict(status='passed',run=str(run.relative_to(ROOT)),**provenance,actions=len(actions),scope='Linux native first-use trust, session-only password auth, download Skip/Overwrite and upload Copy, hidden descendants, exact remote viewer clipboard, retained default-app opening, structured bookmark, tab duplication/close, disconnect/reconnect and surviving-tab refresh; normal exit',source_bytes_unchanged=True,config_and_app_log_no_secret=True,external_copy_retained=True,screenshot_sha256={p.name:digest(p)for p in (run/'native-final').glob('*.png')},macos='pending')
 (ROOT/'evidence/workflows/W15-sftp-connections/native.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(dict(status='passed',run=report['run'],actions=len(actions)),indent=2))
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('action',choices=['configure','verify']);p.add_argument('--run',type=Path,required=True);a=p.parse_args();run=a.run.resolve();(configure if a.action=='configure'else verify)(run)
