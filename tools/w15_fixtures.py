#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Deterministic protocol fixtures, frozen before W15 production Rust changes."""
import hashlib,json,random
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def generate(seed,count):
 r=random.Random(seed);return [dict(id=f'{seed}-{i}',name=f'{i:02}-'+r.choice(['file.txt','.hidden','café.txt','data.bin']),size=r.choice([0,1,32767,32768,65537,131072]),policy=r.choice(['skip','overwrite']),existing=r.choice([False,True]),direction=r.choice(['upload','download'])) for i in range(count)]
def main():
 folder=ROOT/'fixtures/W15-sftp-connections';folder.mkdir(parents=True,exist_ok=True)
 fixtures={'cases.json':generate(15001,32),'reserved.json':generate(15002,16),'protocol.json':dict(version=1,chunk_bytes=32768,packet_bytes=524288,listing_entries=20000,depth=256,tree_entries=100000,download_bytes=536870912,timeout_ms=5000,scenarios=['unknown-key-no-auth','decline-no-trust','matching-key','changed-key-refusal','password','invalid-password','private-key','encrypted-key-passphrase','unavailable-secret-store','isolated-native-secret-roundtrip','listing-names-types-seconds','hidden-tree','symlink-source','symlink-destination-component','non-unicode-name','denied','no-posix-overwrite-refusal','skip-existing','cancel-partial-stage','source-change','network-loss-before-commit','ambiguous-commit-no-retry','tab-close-pinned-job','stale-listing-discard','disconnect','retained-open-copy'])}
 for name,value in fixtures.items():
  data=(json.dumps(value,indent=2,ensure_ascii=False)+'\n').encode();p=folder/name
  if p.exists():assert p.read_bytes()==data,'Frozen fixture drift: '+str(p)
  else:p.write_bytes(data)
if __name__=='__main__':main()
