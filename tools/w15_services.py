#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Record real SFTP and strictly isolated native Secret Service checks."""
from datetime import datetime,timezone
import argparse,hashlib,json,os,re,subprocess,uuid
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def main(server):
 run=ROOT/'artifacts/runs'/('W15-services-'+datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')+'-'+uuid.uuid4().hex[:8]);run.mkdir(parents=True)
 for name in ['data','control','runtime']:(run/name).mkdir(mode=0o700)
 config=run/'bus.conf';config.write_text('<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN" "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd"><busconfig><type>session</type><listen>unix:tmpdir=/tmp</listen><auth>EXTERNAL</auth><policy context="default"><allow send_destination="*"/><allow eavesdrop="true"/><allow own="*"/></policy></busconfig>')
 cargo=['cargo','test','-p','mucrab-core','--test','sftp','--locked','--offline'];environment=dict(os.environ,MUCRAB_W15_RUN=str(server.resolve()));commands=[cargo+['real_sftp_','--','--ignored','--nocapture','--test-threads=1']]
 prefix=['dbus-run-session','--config-file='+str(config),'--','env','XDG_DATA_HOME='+str(run/'data'),'XDG_RUNTIME_DIR='+str(run/'runtime')]
 commands.append(prefix+['MUCRAB_W15_CREDENTIAL_FIXTURE=unavailable']+cargo+['isolated_native_credential','--','--ignored','--nocapture'])
 shell='printf %s w15-isolated-keyring-pass | gnome-keyring-daemon --unlock --components=secrets --control-directory "$1"; cargo test -p mucrab-core --test sftp --locked --offline isolated_native_credential -- --ignored --nocapture'
 commands.append(prefix+['MUCRAB_W15_CREDENTIAL_FIXTURE=available','bash','-c',shell,'w15',str(run/'control')]);checks=[];passed=0
 for i,cmd in enumerate(commands):
  p=subprocess.run(cmd,cwd=ROOT,env=environment,text=True,capture_output=True,timeout=180);out=run/f'{i}-stdout.log';err=run/f'{i}-stderr.log';out.write_text(p.stdout);err.write_text(p.stderr);assert p.returncode==0,p.stdout[-2500:]+p.stderr[-2000:];assert 'invalid permissions'not in p.stderr;passed+=sum(map(int,re.findall(r'test result: ok\. (\d+) passed;',p.stdout)));checks.append(dict(command=cmd,exit_code=p.returncode,stdout=str(out.relative_to(ROOT)),stderr=str(err.relative_to(ROOT))))
 report=dict(status='passed',host='Linux x86_64',run=str(run.relative_to(ROOT)),test_executions=passed,distinct_tests=3,checks=checks,binary_sha256=hashlib.sha256((ROOT/'target/debug/mucrab').read_bytes()).hexdigest(),scope='Real disposable SSH/SFTP password and plain/encrypted-key authentication;48 frozen transfers; hidden trees, strict names/link guards, stale destination, missing POSIX rename, paused read independence, canceled partial upload/download, 5s timeout, shutdown interruption and post-commit lost acknowledgment; isolated unavailable and unlocked Secret Service save/read/delete',macos_keychain='pending',credential_isolation='Dedicated bus with no service activation directory; explicit private0700 keyring/runtime/data; only disposable profile IDs, no user key files or personal credentials')
 (ROOT/'evidence/workflows/W15-sftp-connections/services.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(dict(status='passed',test_executions=passed,run=report['run']),indent=2))
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--server-run',type=Path,required=True);main(p.parse_args().server_run)
