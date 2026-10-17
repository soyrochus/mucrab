#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Run only under isolated Xvfb + DBus; never uses a personal clipboard/Trash."""
from datetime import datetime,timezone
import json,os,subprocess,uuid
from pathlib import Path
from w01 import ROOT,REFERENCE,digest

def main():
 display=os.environ.get('DISPLAY','');assert display.startswith(':') and int(display[1:].split('.')[0])>=99,'Use xvfb-run'
 assert 'DBUS_SESSION_BUS_ADDRESS' in os.environ,'Use dbus-run-session'
 run=ROOT/'artifacts/gui'/('W10-native-'+uuid.uuid4().hex[:8]);run.mkdir(parents=True)
 jars=sorted((REFERENCE/'bundle').glob('*.jar'))+sorted((REFERENCE/'app').glob('*.jar'));cp=os.pathsep.join(map(str,jars));classes=run/'classes';classes.mkdir()
 subprocess.run(['javac','-proc:none','-encoding','UTF-8','-cp',cp,'-d',str(classes),str(ROOT/'tools/java/W10ClipboardProbe.java')],check=True)
 env=dict(os.environ,XDG_DATA_HOME=str(run/'data'),W10_NATIVE_ROOT=str(run),W10_JAVA_CP=str(classes)+os.pathsep+cp);env.pop('WAYLAND_DISPLAY',None)
 # Services activated by DBus inherit its environment; start the isolated daemon
 # with this data root so its Trash backend sees the same disposable directory.
 subprocess.run(['dbus-update-activation-environment','XDG_DATA_HOME='+str(run/'data')],env=env,check=True)
 command=['cargo','test','-p','mucrab-desktop','linux_original_clipboard_interop_and_trash_restore','--locked','--offline','--','--ignored','--nocapture']
 p=subprocess.run(command,env=env,text=True,capture_output=True,timeout=60)
 (run/'stdout.log').write_text(p.stdout);(run/'stderr.log').write_text(p.stderr);print(p.stdout);print(p.stderr)
 receipt=dict(status='passed' if p.returncode==0 else 'failed',scope='Linux X11 file clipboard through original ClipboardSupport/AWT in both directions; GIO Trash metadata and native restore',run=str(run.relative_to(ROOT)),date=datetime.now(timezone.utc).isoformat(),original_clipboard_probe_sha256=digest(ROOT/'tools/java/W10ClipboardProbe.java'))
 if p.returncode==0:(ROOT/'evidence/workflows/W10-everyday-file-management/native-services.json').write_text(json.dumps(receipt,indent=2)+'\n')
 (run/'result.json').write_text(json.dumps(receipt,indent=2)+'\n');raise SystemExit(p.returncode)
if __name__=='__main__':main()
