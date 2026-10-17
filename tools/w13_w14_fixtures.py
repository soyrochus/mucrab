#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Frozen executable metadata/viewer inputs; generated before Rust implementation."""
from pathlib import Path
import hashlib,io,json,random,struct,zlib
from PIL import Image
ROOT=Path(__file__).resolve().parents[1]

def properties(reserved=False):
 r=random.Random(13101791 if reserved else 131017)
 return [dict(id=('reserved-' if reserved else 'case-')+str(i),kind=r.choice(['file','folder','link','hardlink']),name=r.choice(['plain','café 中','space name','- leading'])+str(i),size=r.choice([0,1,17,1024]),mode=r.randrange(0o1000),special=r.choice([0,0o2000,0o1000]),mtime_ms=1_600_000_000_000+r.randrange(100000),new_mode=r.randrange(0o1000),new_mtime_ms=1_610_000_000_000+r.randrange(100000)) for i in range(40)]

def viewer(reserved=False):
 r=random.Random(14101791 if reserved else 141017)
 texts=['','plain\n','café 中 😀\r\nlast','one\rtwo\nthree\r\n','tabs\there\n','e\u0301 / é\n','needle\nneedle\n']
 rows=[dict(id=('reserved-' if reserved else 'case-')+str(i),text=r.choice(texts),encoding=r.choice(['utf8','utf8-bom','utf16-le','utf16-be'])) for i in range(40)]
 if not reserved:
  rows += [dict(id=key,error=True,recipe=key) for key in ['bad-utf8','unpaired-utf16','odd-utf16','nul-binary','text-over-limit','line-over-limit','lines-over-limit','truncated-png','oversized-png','truncated-jpeg','pdf','gif']]
 return rows

def write(path,payload):
 path.parent.mkdir(parents=True,exist_ok=True)
 if path.exists():assert path.read_bytes()==payload,'Frozen fixture differs: '+str(path)
 else:path.write_bytes(payload)

def freeze():
 for n,slug,fn in [(13,'properties-permissions',properties),(14,'built-in-viewer',viewer)]:
  for reserved in [False,True]:write(ROOT/'fixtures'/f'W{n}-{slug}'/('reserved.json' if reserved else 'cases.json'),(json.dumps(fn(reserved),ensure_ascii=False,indent=2)+'\n').encode())
 folder=ROOT/'fixtures/W14-built-in-viewer/images'
 for ext in ['png','jpg']:
  for orientation in range(1,9):
   img=Image.new('RGBA' if ext=='png' else 'RGB',(7,5));pixels=[]
   for y in range(5):
    for x in range(7):pixels.append((x*31,y*47,(x+y)*19,(x*43+y*29)%256) if ext=='png' else (x*31,y*47,(x+y)*19))
   img.putdata(pixels);buf=io.BytesIO();exif=Image.Exif();exif[274]=orientation
   img.save(buf,format='PNG' if ext=='png' else 'JPEG',exif=exif,**({'quality':95,'subsampling':0} if ext=='jpg' else {}));write(folder/f'{ext}-orientation-{orientation}.{ext}',buf.getvalue())
 for reserved in [False,True]:
  r=random.Random(14101791 if reserved else 141017)
  for i in range(12):
   w,h=r.randint(1,19),r.randint(1,19);img=Image.new('RGBA',(w,h));img.putdata([tuple(r.randrange(256) for _ in range(4)) for _ in range(w*h)]);b=io.BytesIO();img.save(b,format='PNG');write(folder/f'{"reserved" if reserved else "generated"}-{i}.png',b.getvalue())
 if __name__=='__main__':print('Frozen W13/W14 inputs verified')
if __name__=='__main__':freeze()
