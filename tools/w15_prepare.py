#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Prepare disposable SFTP keys/data; authentication constants are test fixtures."""
import argparse,json,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def prepare(run):
 run.mkdir(parents=True,exist_ok=False)
 for name in ['remote','local','config','data','cache']:(run/name).mkdir()
 for name,password in [('host',''),('client',''),('encrypted','w15-key-passphrase')]:subprocess.run(['ssh-keygen','-q','-m','PEM','-t','rsa','-b','2048','-N',password,'-f',str(run/(name+'.key'))],check=True)
 # Server uses its generated authorized key, never a user's SSH configuration.
 (run/'client.pub').write_text((run/'client.key.pub').read_text())
 for folder in ['remote/folder/.hidden','local/folder/.hidden']:(run/folder).mkdir(parents=True)
 for base in ['remote','local']:
  (run/base/'notes.txt').write_text('SFTP document café\n');(run/base/'folder/.hidden/data').write_bytes(b'hidden');(run/base/'folder/file.bin').write_bytes(bytes(range(256))*400);(run/base/'link').symlink_to('notes.txt')
 spec=dict(argv=[str(ROOT/'target/debug/mucrab'),str(run/'local'),str(run/'local')],cwd=str(ROOT),title='muCrab',output=str(run/'native'),env=dict(XDG_CONFIG_HOME=str(run/'config'),XDG_DATA_HOME=str(run/'data'),XDG_CACHE_HOME=str(run/'cache')))
 (run/'launch.json').write_text(json.dumps(spec,indent=2)+'\n');print(run)
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('run',type=Path);args=p.parse_args();prepare(args.run.resolve())
