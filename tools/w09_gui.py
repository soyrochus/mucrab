#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Isolated Linux default-application integration replay for W09."""
import json
import os
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
RUN = ROOT / 'artifacts/gui/W09-open-files'

def prepare():
    RUN.mkdir(parents=True, exist_ok=False)
    left = RUN / 'left'
    left.mkdir()
    (left / '- café & report.txt').write_text('Local W09 document\n')
    apps = RUN / 'data/applications'
    apps.mkdir(parents=True)
    (RUN / 'config').mkdir()
    handler = RUN / 'handler.py'
    handler.write_text('''#!/usr/bin/python3
import json, os, sys
from pathlib import Path
p = Path(sys.argv[1])
with open(os.environ['W09_LOG'], 'a') as f:
 f.write(json.dumps({'path': str(p), 'contents': p.read_text()}) + '\\n')
''')
    handler.chmod(0o700)
    (apps / 'w09.desktop').write_text(f'[Desktop Entry]\nType=Application\nName=W09 test handler\nExec={handler} %f\nMimeType=text/plain;\nTerminal=false\n')
    (RUN / 'config/mimeapps.list').write_text('[Default Applications]\ntext/plain=w09.desktop\n')
    spec = dict(argv=[str(ROOT / 'target/debug/mucrab'), str(left), str(ROOT / 'fixtures/W09/open.zip/nested')], cwd=str(ROOT), title='muCrab', output=str(RUN / 'native'), env=dict(XDG_CONFIG_HOME=str(RUN / 'config'), XDG_DATA_HOME=str(RUN / 'data'), XDG_CURRENT_DESKTOP='X-Generic', W09_LOG=str(RUN / 'opened.jsonl')))
    (RUN / 'launch.json').write_text(json.dumps(spec, indent=2))
    actions = [dict(op='click',x=80,y=162),dict(op='key',keys=['Return'],wait=1,capture='local-enter'),dict(op='click',x=650,y=162,count=2,wait=1,capture='archive-double-click'),dict(op='click',x=80,y=162),dict(op='click',x=16,y=12),dict(op='click',x=44,y=36,wait=1,capture='menu-open'),dict(op='click',x=16,y=12),dict(op='click',x=66,y=209),dict(op='await_exit')]
    (RUN / 'actions.json').write_text(json.dumps(actions, indent=2))
    print(RUN / 'launch.json')

def verify():
    records = [json.loads(line) for line in (RUN / 'opened.jsonl').read_text().splitlines()]
    assert len(records) == 3, records
    assert [r['contents'] for r in records] == ['Local W09 document\n','W09 archive document\n','Local W09 document\n'], records
    archive = Path(records[1]['path'])
    assert archive.name == '- café & report.txt' and archive.exists()
    assert archive.stat().st_mode & 0o777 == 0o600
    assert records[0]['path'] == records[2]['path']
    assert all(Path(r['path']).is_absolute() for r in records)
    report = dict(scope='Linux native Open via real xdg-open and isolated MIME registration', passed=True, requests=records, temporary_copy_survives_exit=True)
    dest=ROOT/'evidence/workflows/W09-open-files/native.json'
    dest.parent.mkdir(parents=True,exist_ok=True)
    dest.write_text(json.dumps(report,indent=2)+'\n')
    print('W09: Enter, archive double-click, File/Open, exact Unicode filenames and contents, retained copy after exit passed')

if __name__ == '__main__':
    {'prepare':prepare,'verify':verify}[sys.argv[1]]()
