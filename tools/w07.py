#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
"""Compare real original full-job lifecycle with Rust controlled operations.

Run on an isolated Xvfb display. All mutations use separate disposable fixtures;
original code is loaded from an intact runtime copy, never the reference tree.
"""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import random
import re
import shutil
import subprocess
import tempfile
import uuid
import xml.etree.ElementTree as ET
import zipfile

import acquire_mucommander as acquisition
from w01 import ROOT, REFERENCE, capture, digest
from w03 import snapshot

SPEC = ROOT / 'fixtures/W07-background-operations/cases.json'
EVIDENCE = ROOT / 'evidence/workflows/W07-background-operations'


def cases(spec, reserved):
    if not reserved:
        return spec['cases']
    rng = random.Random(spec['reserved_seed'])
    templates = [row for row in spec['cases'] if row['mode'] in ('before', 'stream', 'pause', 'finish')]
    return [dict(rng.choice(templates), id='reserved-' + str(i), large_size=rng.choice([524288, 786432, 1048576]), cancel_bytes=rng.choice([131072, 196608])) for i in range(12)]


def payload(size):
    return (bytes(range(251)) * ((size + 250) // 251))[:size]


def prepare(run, external, side, items, spec):
    requests = []
    for item in items:
        folder = run / side / item['id']
        source = folder / 'source'
        source.mkdir(parents=True)
        destination = external / side / item['id'] if item.get('cross_device') else folder / 'destination'
        destination.mkdir(parents=True)
        size = item.get('large_size', spec['large_size']) if item['mode'] in ('stream', 'pause', 'nested') else 65537
        for name, data in [('first.bin', b'earlier completed\n'), ('large.bin', payload(size)), ('later.bin', b'untouched later\n')]:
            (source / name).write_bytes(data)
        names = ['first.bin', 'large.bin', 'later.bin']
        if item['mode'] in ('nested', 'error'):
            (source / 'tree').mkdir()
            (source / 'large.bin').rename(source / 'tree/large.bin')
            if item['mode'] == 'error':
                (source / 'tree/large.bin').chmod(0)
                if os.access(source / 'tree/large.bin', os.R_OK):
                    raise ValueError('Denied source is readable; no privileged pass')
            names = ['first.bin', 'tree', 'later.bin']
        if item['action'] == 'extract':
            with zipfile.ZipFile(source / 'sample.zip', 'w', compression=zipfile.ZIP_DEFLATED) as archive:
                for name in names:
                    info = zipfile.ZipInfo(name, (2020, 1, 1, 0, 0, 0))
                    info.compress_type = zipfile.ZIP_DEFLATED
                    archive.writestr(info, (source / name).read_bytes())
            for name in names: (source / name).unlink()
            names = ['sample.zip']
        if item['action'] in ('rename', 'batch'):
            destination = source
            if item['action'] == 'rename': names = ['first.bin']
        if item['mode'] == 'skip': (destination / 'first.bin').write_bytes(b'existing collision\n')
        if item.get('cross_device') and source.stat().st_dev == destination.stat().st_dev:
            raise ValueError('A real cross-device move filesystem is required')
        requests.append(dict(id=item['id'], action=item['action'], mode=item['mode'], source_directory=str(source), destination_directory=str(destination), names=names, cancel_bytes=item.get('cancel_bytes', spec['cancel_bytes'])))
    return requests


def tree(path):
    # Runtime directory clocks and partial-file dates/modes are not W07 targets.
    return [{key: value for key, value in row.items() if key not in ('modified_ms', 'mode')} for row in snapshot(path)]


def observe(requests):
    return [dict(id=r['id'], source=tree(Path(r['source_directory'])), destination=tree(Path(r['destination_directory']))) for r in requests]


def comparable(request, state):
    if request['mode'] not in ('stream', 'pause', 'nested'):
        return state
    result = dict(state)
    result['destination'] = []
    for record in state['destination']:
        row = dict(record)
        if row['kind'] == 'file' and Path(row['path']).name == 'large.bin':
            destination = Path(request['destination_directory']) / row['path']
            partial = destination.read_bytes()
            if request['action'] == 'extract':
                with zipfile.ZipFile(Path(request['source_directory']) / 'sample.zip') as archive: source = archive.read('large.bin')
            else: source = (Path(request['source_directory']) / row['path']).read_bytes()
            if not 0 < len(partial) < len(source) or partial != source[:len(partial)]:
                raise ValueError('Cancellation must retain a nonempty proper source prefix: ' + request['id'])
            row.pop('size'); row.pop('sha256'); row['valid_partial_prefix'] = True
        result['destination'].append(row)
    if not any(row.get('valid_partial_prefix') for row in result['destination']):
        raise ValueError('Cancellation did not reach a real streaming transfer: ' + request['id'])
    return result


def run_suite(adapter, reserved):
    if os.geteuid() == 0: raise ValueError('Run unprivileged; permission errors must be real')
    if not re.fullmatch(r':[1-9]\d+(?:\.0)?', os.environ.get('DISPLAY', '')):
        raise ValueError('Use xvfb-run -a for an isolated original application; never the live desktop')
    acquisition.run(verify_only=True)
    spec = json.loads(SPEC.read_text()); items = cases(spec, reserved)
    if not items: raise ValueError('Zero W07 cases is not a pass')
    run = ROOT / 'artifacts/runs' / ('W07-' + datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ') + '-' + uuid.uuid4().hex[:8])
    run.mkdir(parents=True)
    with tempfile.TemporaryDirectory(prefix='mucrab-w07-cross-') as temporary:
        external = Path(temporary)
        original_requests = prepare(run, external, 'original', items, spec)
        native_requests = prepare(run, external, 'rust', items, spec)
        before_original = observe(original_requests); before_native = observe(native_requests)
        if before_original != before_native: raise ValueError('Initial original/native trees differ')
        acquisition.write_json(run / 'before.json', before_original)
        (run / 'original-requests.tsv').write_text(''.join('\t'.join([r['id'], r['action'], r['mode'], r['source_directory'], r['destination_directory'], ','.join(r['names']), str(r['cancel_bytes'])]) + '\n' for r in original_requests))
        (run / 'rust-requests.jsonl').write_text(''.join(json.dumps(r) + '\n' for r in native_requests))
        runtime = run / 'reference'; shutil.copytree(REFERENCE, runtime)
        if any(digest(path) != digest(runtime / path.relative_to(REFERENCE)) for path in REFERENCE.rglob('*') if path.is_file()):
            raise ValueError('Reference runtime copy differs')
        profile = run / 'profile'; profile.mkdir()
        ET.parse(ROOT / 'fixtures/W01-local-browse-sort/reference-preferences.xml').write(profile / 'preferences.xml', encoding='utf-8', xml_declaration=True)
        classes = run / 'classes'; classes.mkdir()
        sources = [ROOT / 'tools/java/W07GuiProbe.java', ROOT / 'tools/java/W02bGuiProbe.java']
        _, compiler = capture(run, 'javac', ['javac', '-proc:none', '-d', str(classes)] + list(map(str, sources)))
        manifest = run / 'agent.mf'; manifest.write_text('Manifest-Version: 1.0\nPremain-Class: W07GuiProbe\n\n')
        agent = run / 'agent.jar'
        _, jar = capture(run, 'jar', ['jar', '--create', '--file', str(agent), '--manifest', str(manifest), '-C', str(classes), '.'])
        (run / 'original-probe').mkdir()
        command = ['java', '-javaagent:' + str(agent) + '=' + str(run / 'original-probe'), '-Duser.timezone=UTC', '-Duser.home=' + str(run / 'home'), '--add-opens=java.base/java.net=ALL-UNNAMED', '--add-opens=java.base/java.io=ALL-UNNAMED', '--add-opens=java.desktop/javax.swing.plaf.basic=ALL-UNNAMED', '-cp', str(runtime / 'mucommander-1.6.2.jar'), 'com.mucommander.main.muCommander', '--preferences', str(profile), original_requests[0]['source_directory'], original_requests[0]['destination_directory']]
        _, java = capture(run, 'java', command)
        original = [json.loads((run / 'original-probe' / (r['id'] + '.json')).read_text()) for r in original_requests]
        original_after = observe(original_requests)
        native_text, rust = capture(run, 'rust', [str(adapter), '--adapter-w07'], (run / 'rust-requests.jsonl').read_text())
        native = [json.loads(line) for line in native_text.splitlines()]
        if [row['id'] for row in native] != [r['id'] for r in native_requests]: raise ValueError('Incomplete native lifecycle execution')
        native_after = observe(native_requests)
        differences = []
        for index, (old, new, item) in enumerate(zip(original, native, items)):
            interrupted = item['mode'] in ('before', 'stream', 'pause', 'nested')
            expected_state = 'INTERRUPTED' if interrupted else 'FINISHED'
            transitions = 'NOT_STARTED,RUNNING,' + ('PAUSED,RUNNING,' if item['mode'] == 'pause' else '') + expected_state
            for value in (old, new):
                if value['state'] != expected_state or value['transitions'] != transitions or not value['pause_stable']:
                    differences.append(dict(id=item['id'], kind='lifecycle', expected=transitions, actual=value))
            if sorted(old['marked']) != sorted(new['marked']): differences.append(dict(id=item['id'], kind='marks', original=old['marked'], rust=new['marked']))
            left = comparable(original_requests[index], original_after[index]); right = comparable(native_requests[index], native_after[index])
            if left != right: differences.append(dict(id=item['id'], kind='trees', original=left, rust=right))
            if item['action'] in ('copy', 'extract') and original_after[index]['source'] != before_original[index]['source']:
                raise ValueError('Original copy/extraction changed source contents')
            if item['action'] in ('copy', 'extract') and native_after[index]['source'] != before_native[index]['source']:
                raise ValueError('Native copy/extraction changed source contents')
        acquisition.write_json(run / 'original-after.json', original_after)
        acquisition.write_json(run / 'rust-after.json', native_after)
        acquisition.write_json(run / 'original-results.json', original)
        acquisition.write_json(run / 'rust-results.json', native)
        acquisition.write_json(run / 'differences.json', differences)
    acquisition.run(verify_only=True)
    report = dict(workflow='W07', status='passed' if not differences else 'differences', scope='Real original start/run/interrupt/pause, full-job trees/marks, cooperative native control', cases_run=len(items), reserved=reserved, difference_count=len(differences), raw_run=run.relative_to(ROOT).as_posix(), fixture_sha256=digest(SPEC), sources=[dict(path=p.relative_to(ROOT).as_posix(), sha256=digest(p)) for p in sources], rust_adapter_sha256=digest(adapter), reference_unchanged=True, commands=dict(javac=compiler, jar=jar, java=java, rust=rust), normalization='Runtime clocks omitted; partial lengths/hashes compared as valid nonempty proper source prefixes, not equal byte counts', excluded='Original scheduler concurrency/remote jobs and exact timing; native panic/close guarantees tested separately')
    acquisition.write_json(run / 'report.json', report)
    acquisition.write_json(EVIDENCE / ('comparison-reserved.json' if reserved else 'comparison-predefined.json'), report)
    print(json.dumps({key: report[key] for key in ('status', 'cases_run', 'reserved', 'difference_count', 'raw_run')}))
    return 1 if differences else 0


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['compare'])
    parser.add_argument('--rust-adapter', type=Path, required=True)
    parser.add_argument('--reserved', action='store_true')
    args = parser.parse_args()
    raise SystemExit(run_suite(args.rust_adapter.resolve(), args.reserved))
