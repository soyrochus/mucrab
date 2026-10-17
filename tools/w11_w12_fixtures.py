#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Executable W11/W12 fixture generator, frozen before Rust implementation."""
import json, random
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def tab_cases(reserved=False):
 r=random.Random(11101791 if reserved else 111017)
 cases=[dict(id='duplicate-isolation',events=['mark','duplicate','navigate','back','close']),dict(id='closed-origin',events=['mark','start-copy','duplicate','close-origin','finish-copy']),dict(id='same-path-origin',events=['mark','duplicate','mark','start-copy','switch','finish-copy']),dict(id='limits',events=['duplicate']*33+['close']*33),dict(id='bookmarks',events=['add-local','add-zip','rename','reorder','save','restart','open-missing'])]
 if reserved:cases=[]
 for i in range(24):cases.append(dict(id=('reserved-' if reserved else 'generated-')+str(i),events=[r.choice(['duplicate','switch','mark','navigate','back','close','hidden','refresh']) for _ in range(32)]))
 return cases

def search_cases(reserved=False):
 r=random.Random(12101791 if reserved else 121017)
 names=['alpha.txt','ALPHA.TXT','café.txt','中 文.md','a*b.txt','x.log','.hidden.txt','beta','😀.txt']
 patterns=['*','*.txt','a?pha.*','AL*','café.*','中*',r'a\*b.txt','?*','missing*']
 cases=[]
 for i in range(64):
  cases.append(dict(id=('reserved-' if reserved else 'generated-')+str(i),name=r.choice(names),pattern=r.choice(patterns),case_sensitive=r.choice([False,True]),size=r.choice([0,1,17,1024]),modified_ms=r.choice([-1,0,999,1000,2000]),min_size=r.choice([None,0,17]),max_size=r.choice([None,17,1024]),from_ms=r.choice([None,0,1000]),to_ms=r.choice([None,1000,2001])))
 return cases

def freeze():
 for n,slug,fn in [(11,'tabs-bookmarks',tab_cases),(12,'file-search',search_cases)]:
  folder=ROOT/'fixtures'/f'W{n}-{slug}';folder.mkdir(parents=True,exist_ok=True)
  for reserved in [False,True]:
   p=folder/('reserved.json' if reserved else 'cases.json');payload=json.dumps(fn(reserved),ensure_ascii=False,indent=2)+'\n'
   if p.exists():assert p.read_text()==payload,'Frozen fixtures differ'
   else:p.write_text(payload)
if __name__=='__main__':freeze()
