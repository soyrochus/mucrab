#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Original text loader/JDK image decoder observations and independent pixel checks."""
import argparse,json,os,subprocess,uuid
from datetime import datetime,timezone
from pathlib import Path
from PIL import Image,ImageOps
import acquire_mucommander as acquisition
from w01 import ROOT,REFERENCE,capture,digest

def main(reserved):
 acquisition.run(verify_only=True);run=ROOT/'artifacts/runs'/('W14-'+datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')+'-'+uuid.uuid4().hex[:8]);run.mkdir(parents=True)
 cases=json.loads((ROOT/'fixtures/W14-built-in-viewer'/('reserved.json' if reserved else 'cases.json')).read_text());requests=[];expected={}
 for c in cases:
  if c.get('error'):continue
  text=c['text'];encoding=c['encoding'];p=run/(c['id']+'.txt');data={'utf8':lambda:text.encode(),'utf8-bom':lambda:b'\xef\xbb\xbf'+text.encode(),'utf16-le':lambda:b'\xff\xfe'+text.encode('utf-16-le'),'utf16-be':lambda:b'\xfe\xff'+text.encode('utf-16-be')}[encoding]();p.write_bytes(data);requests.append(dict(id=c['id'],path=str(p),encoding={'utf8':'UTF-8','utf8-bom':'UTF-8','utf16-le':'UTF-16LE','utf16-be':'UTF-16BE'}[encoding]));expected[c['id']]=dict(text=text)
 folder=ROOT/'fixtures/W14-built-in-viewer/images';images=sorted(folder.glob('reserved-*.png')) if reserved else sorted(p for p in folder.iterdir() if not p.name.startswith('reserved-'))
 for p in images:
  key=p.name;requests.append(dict(id=key,path=str(p)))
  with Image.open(p) as image:
   original=image.convert('RGBA');oriented=ImageOps.exif_transpose(image).convert('RGBA');expected[key]=dict(width=oriented.width,height=oriented.height,rgba=list(oriented.tobytes()),raw_width=original.width,raw_height=original.height,raw_rgba=list(original.tobytes()),orientation=image.getexif().get(274,1))
 cp=os.pathsep.join(map(str,sorted((REFERENCE/'bundle').glob('*.jar'))+sorted((REFERENCE/'app').glob('*.jar'))));classes=run/'classes';classes.mkdir();capture(run,'javac',['javac','-proc:none','-encoding','UTF-8','-cp',cp,'-d',str(classes),str(ROOT/'tools/java/W14Oracle.java')])
 payload=''.join(json.dumps(r,ensure_ascii=False)+'\n' for r in requests);(run/'requests.jsonl').write_text(payload);replies={}
 for side,cmd in [('original',['java','-Djava.awt.headless=true','-Duser.home='+str(run/'home'),'-cp',str(classes)+os.pathsep+cp,'W14Oracle']),('native',[str(ROOT/'target/debug/mucrab-compare'),'--adapter-w14'])]:
  p=subprocess.run(cmd,input=payload,text=True,capture_output=True,timeout=90);(run/(side+'-stdout.jsonl')).write_text(p.stdout);(run/(side+'-stderr.log')).write_text(p.stderr);assert p.returncode==0,p.stderr[-3000:];replies[side]={r['id']:r for r in map(json.loads,p.stdout.splitlines())};assert len(replies[side])==len(requests)
 unexpected=[];intentional=[];jpeg_max_delta=0
 for key,e in expected.items():
  a=replies['original'][key];b=replies['native'][key]
  if 'text' in e:
   if b.get('text')!=e['text']:unexpected.append(dict(id=key,native=b,expected=e))
   if a.get('text')!=e['text']:
    if a.get('text')==e['text'].replace('\r\n','\n').replace('\r','\n'):intentional.append(dict(id=key,reason='Original Swing document normalizes line endings; native preserves decoded CR/CRLF/LF for exact selection/copy'))
    else:unexpected.append(dict(id=key,original=a,expected=e))
  else:
   if (b.get('width'),b.get('height'))!=(e['width'],e['height']):unexpected.append(dict(id=key,native=b,reason='Oriented dimensions differ'));continue
   delta=max((abs(x-y) for x,y in zip(b.get('rgba',[]),e['rgba'])),default=999);assert len(b.get('rgba',[]))==len(e['rgba'])
   if key.endswith('.png') and delta:unexpected.append(dict(id=key,delta=delta))
   if key.endswith('.jpg'):
    jpeg_max_delta=max(jpeg_max_delta,delta)
    if delta>2:unexpected.append(dict(id=key,delta=delta))
   if (a.get('width'),a.get('height'))!=(e['raw_width'],e['raw_height']) or len(a.get('rgba',[]))!=len(e['raw_rgba']):unexpected.append(dict(id=key,original=a,reason='Original raw image decode failed'))
   elif max(abs(x-y) for x,y in zip(a['rgba'],e['raw_rgba']))>(2 if key.endswith('.jpg') else 0):unexpected.append(dict(id=key,reason='Original raw pixels disagree with reference'))
   if e['orientation']!=1:intentional.append(dict(id=key,reason='Native applies EXIF orientation; original ImageIO boundary exposes raw raster'))
 report=dict(status='passed' if not unexpected else 'failed',reserved=reserved,cases=len(expected),scope='Pinned TextViewer.loadDocument with actual TextEditorImpl; image boundary uses JDK ImageIO called by recovered ImageViewer, with independent Pillow orientation/pixels; no full original window parity',jpeg_max_channel_delta=jpeg_max_delta,unexpected_differences=unexpected,intentional_differences=intentional,run=str(run.relative_to(ROOT)),sources={p:digest(ROOT/p) for p in ['tools/java/W14Oracle.java','crates/compare/src/w14.rs','tools/w13_w14_fixtures.py']})
 acquisition.run(verify_only=True);acquisition.write_json(ROOT/'evidence/workflows/W14-built-in-viewer'/('comparison-reserved.json' if reserved else 'comparison.json'),report);print(json.dumps(dict(status=report['status'],cases=len(expected),unexpected=len(unexpected),intentional=len(intentional),jpeg_max_delta=jpeg_max_delta,run=report['run']),indent=2));assert not unexpected,unexpected
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--reserved',action='store_true');main(p.parse_args().reserved)
