#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Fresh earlier native workflows under W15, preserving historical receipts."""
import argparse
from pathlib import Path
import w13_w14_regressions as earlier,w13_w14_gui as properties
ROOT=Path(__file__).resolve().parents[1]
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('action',choices=['prepare','verify']);a=p.parse_args();earlier.PREFIX='W15-regression';properties.RUN=ROOT/'artifacts/gui/W15-regression-W13-W14'
 if a.action=='prepare':earlier.prepare();properties.prepare();print('Replay the W13/W14 launch with its actions.json')
 else:earlier.verify(evidence_slugs=['W15-sftp-connections']);properties.verify(evidence_slugs=['W15-sftp-connections'],receipt_name='regression-properties-viewer.json')
