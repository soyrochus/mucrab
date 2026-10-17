#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
"""Compare original CopyJob and Rust on separate equivalent disposable trees."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import random
import stat
import subprocess
import sys
import uuid

import acquire_mucommander as acquisition
from w01 import ROOT, REFERENCE, capture, digest
from w02c import java_capture

SPEC = ROOT / 'fixtures/W03-recursive-copy/cases.json'
EVIDENCE = ROOT / 'evidence/workflows/W03-recursive-copy'
SOURCE = ROOT / 'tools/java/W03Oracle.java'


def generated(spec, reserved):
    rng = random.Random(spec['reserved_seed'] if reserved else spec['generated_seed'])
    for i in range(spec['reserved_sets'] if reserved else spec['generated_sets']):
        source = [dict(path='tree', kind='directory'), dict(path='tree/empty', kind='directory')]
        destination = [dict(path='tree', kind='directory'), dict(path='tree/retained', kind='file', size=31, pattern=240)]
        for j in range(rng.randrange(3, 9)):
            name = 'tree/' + rng.choice(['file', 'é', '.hidden', 'with space']) + str(j)
            source.append(dict(path=name, kind='file', size=rng.choice([0, 1, 17, 4096, 65537]), pattern=rng.randrange(251),
                mode=rng.choice([0o444, 0o600, 0o644, 0o751]), modified_ms=rng.choice([-1, 0, 946684800000, 1577836800123])))
            if rng.choice([True, False]): destination.append(dict(path=name, kind='file', size=rng.choice([0, 17, 4096]), pattern=250,
                modified_ms=rng.choice([0, 1577836800123, 1893456000000])))
        yield dict(id=('reserved' if reserved else 'generated') + '-' + str(i), source=source, destination=destination, names=['tree'])


def cases(spec, reserved):
    items = list(generated(spec, reserved)) if reserved else spec['cases'] + list(generated(spec, False))
    for item in items:
        for policy in item.get('policies', spec['policies']):
            yield dict(item, id=item['id'] + '-' + policy.lower(), policy=policy)


def materialize_tree(folder, entries, spec):
    folder.mkdir(parents=True)
    paths = {}
    for entry in entries:
        path = folder / entry['path']
        if path.is_absolute() and not path.is_relative_to(folder): raise ValueError('Fixture escapes root')
        if '..' in Path(entry['path']).parts: raise ValueError('Unsafe fixture path')
        path.parent.mkdir(parents=True, exist_ok=True)
        if entry['kind'] == 'directory': path.mkdir(exist_ok=True)
        elif entry['kind'] == 'symlink': path.symlink_to(entry['target'])
        else:
            pattern = bytes((i + entry.get('pattern', 3)) % 251 for i in range(251))
            path.write_bytes((pattern * ((entry['size'] + 250) // 251))[:entry['size']])
        paths[path] = entry
    all_paths = sorted(folder.rglob('*'), key=lambda p: len(p.parts), reverse=True)
    for path in all_paths:
        if path.is_symlink(): continue
        entry = paths.get(path, {})
        os.chmod(path, entry.get('mode', spec['default_directory_mode'] if path.is_dir() else spec['default_file_mode']))
        stamp = entry.get('modified_ms', spec['default_modified_ms']) * 1_000_000
        os.utime(path, ns=(stamp, stamp))


def snapshot(folder, ignored_dates=()):
    """Inspect only fixture paths; restore access bits immediately after inspection.

    Permission lifts let the independent inspector hash denied fixtures. They
    occur outside either copy invocation; ctime/atime are deliberately unmeasured.
    Symlinks are recorded with readlink and are never followed by this inspector.
    """
    result = []
    def visit(path):
        info = path.lstat(); rel = path.relative_to(folder).as_posix()
        kind = 'symlink' if stat.S_ISLNK(info.st_mode) else 'directory' if stat.S_ISDIR(info.st_mode) else 'file'
        record = dict(path=rel, kind=kind, mode=stat.S_IMODE(info.st_mode))
        if kind == 'symlink': record['target'] = os.readlink(path)
        else:
            if rel not in ignored_dates: record['modified_ms'] = info.st_mtime_ns // 1_000_000
            mode = stat.S_IMODE(info.st_mode)
            try:
                os.chmod(path, mode | (0o500 if kind == 'directory' else 0o400))
                if kind == 'file': record.update(size=info.st_size, sha256=hashlib.sha256(path.read_bytes()).hexdigest())
                else:
                    for child in sorted(path.iterdir()): visit(child)
            finally: os.chmod(path, mode)
        result.append(record)
    for child in sorted(folder.iterdir()): visit(child)
    return sorted(result, key=lambda e: e['path'])


def prepare(run, side, items, spec):
    requests, observations = [], []
    for item in items:
        folder = run / side / item['id']
        materialize_tree(folder / 'source', item['source'], spec)
        materialize_tree(folder / 'destination', item['destination'], spec)
        destination = folder / item.get('destination_use', 'destination')
        request = dict(id=item['id'], source_directory=str(folder / 'source'), destination_directory=str(destination),
            names=item['names'], policy=item['policy'], new_name=item.get('new_name'))
        requests.append(request)
        observations.append(dict(id=item['id'], source=snapshot(folder / 'source'), destination=snapshot(destination)))
        # Confirm the error fixture denies access under the actual executing uid.
        for tree in ('source', 'destination'):
            for entry in item[tree]:
                if entry.get('mode') == 0 and os.access(folder / tree / entry['path'], os.R_OK):
                    raise ValueError('Access-denied fixture is readable; no privileged pass allowed')
    return requests, observations


def decode(text, requests):
    results = [json.loads(line) for line in text.splitlines()]
    if not requests or not any(r['names'] for r in requests) or [r.get('id') for r in results] != [r['id'] for r in requests]:
        raise ValueError('Missing, reordered or zero-work copy execution')
    for result, request in zip(results, requests):
        if result.get('status') != 'ok' or result.get('state') != 'RUNNING': raise ValueError('Incomplete copy method execution')
        outcomes = result.get('outcomes', [])
        if [o.get('name') for o in outcomes] != request['names'] or any(type(o.get('success')) is not bool for o in outcomes):
            raise ValueError('Missing or malformed copy outcomes')
    return results


def observe(requests, items):
    return [dict(id=r['id'], source=snapshot(Path(r['source_directory'])),
        destination=snapshot(Path(r['destination_directory']), item.get('ignore_destination_mtime', []))) for r, item in zip(requests, items)]


def run_suite(compare, reserved, rust_adapter):
    if platform.system() != 'Linux' or os.geteuid() == 0: raise ValueError('W03 first verification requires unprivileged Linux')
    acquisition.run(verify_only=True)
    run_id = datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ') + '-' + uuid.uuid4().hex[:8]
    run = ROOT / 'artifacts/runs' / ('W03-' + run_id); run.mkdir(parents=True)
    spec = json.loads(SPEC.read_text()); items = list(cases(spec, reserved))
    requests, before = prepare(run, 'original', items, spec)
    (run / 'requests.jsonl').write_text(''.join(json.dumps(r) + '\n' for r in requests))
    acquisition.write_json(run / 'original-before.json', before)
    jars = sorted((REFERENCE / 'bundle').glob('*.jar')) + sorted((REFERENCE / 'app').glob('*.jar'))
    cp = os.pathsep.join(map(str, jars)); classes = run / 'classes'; classes.mkdir()
    _, compiler = capture(run, 'javac', ['javac', '-proc:none', '-encoding', 'UTF-8', '-cp', cp, '-d', str(classes), str(SOURCE)])
    output, java = java_capture(run, ['java', '--add-opens=java.base/java.io=ALL-UNNAMED', '-Djava.awt.headless=true',
        '-Duser.home=' + str(run / 'home'), '-cp', str(classes) + os.pathsep + cp, 'W03Oracle'], '', timeout=90)
    original = decode(output, requests); after = observe(requests, items)
    acquisition.write_json(run / 'original-after.json', after)
    commands = dict(javac=compiler, java=java); differences = []
    for old, new in zip(before, after):
        if old['source'] != new['source']: raise ValueError('Original changed source fixture: ' + old['id'])
    if compare:
        native_requests, native_before = prepare(run, 'rust', items, spec)
        if before != native_before: raise ValueError('Original/Rust initial trees differ')
        text = ''.join(json.dumps(r) + '\n' for r in native_requests)
        (run / 'rust-requests.jsonl').write_text(text)
        output, commands['rust'] = capture(run, 'rust', [str(rust_adapter), '--adapter-w03'], text)
        native = decode(output, native_requests); native_after = observe(native_requests, items)
        acquisition.write_json(run / 'rust-after.json', native_after)
        for expected, actual, old, reference, observed in zip(original, native, native_before, after, native_after):
            if expected != actual: differences.append(dict(id=expected['id'], component='outcomes', expected=expected, actual=actual))
            if old['source'] != observed['source']: differences.append(dict(id=expected['id'], component='source_changed'))
            if reference['destination'] != observed['destination']:
                differences.append(dict(id=expected['id'], component='destination_tree', expected=reference['destination'], actual=observed['destination']))
    acquisition.run(verify_only=True)
    files = [SPEC, SOURCE, ROOT / 'tools/w03.py', ROOT / 'tools/w02c.py', ROOT / 'tools/w01.py', ROOT / 'Cargo.lock', ROOT / 'Cargo.toml']
    if (EVIDENCE / 'contract.md').exists(): files.append(EVIDENCE / 'contract.md')
    files += sorted((ROOT / 'crates').rglob('*.rs')) + sorted((ROOT / 'crates').rglob('Cargo.toml'))
    report = dict(schema_version=1,workflow='W03-recursive-copy',run_id=run_id,suite='reserved' if reserved else 'predefined',
        status=('differences' if differences else 'passed') if compare else 'oracle_ready',
        scope='Original CopyJob.processFile recursion/collision/transfer/getters; auto-skip errors; filesystem snapshots; lifecycle/UI excluded',
        source_revision=subprocess.check_output(['git','rev-parse','HEAD'],text=True,cwd=ROOT).strip(),
        source_status=subprocess.check_output(['git','status','--porcelain'],text=True,cwd=ROOT).splitlines(),
        source_inputs=[dict(path=p.relative_to(ROOT).as_posix(),sha256=digest(p)) for p in files],
        host=dict(system=platform.system(),architecture=platform.machine(),platform=platform.platform(),uid=os.geteuid(),umask='022'),
        classpath=[dict(path=p.relative_to(ROOT).as_posix(),sha256=digest(p)) for p in jars],
        java_adapter_classes=[dict(path=p.relative_to(ROOT).as_posix(),sha256=digest(p)) for p in sorted(classes.glob('*.class'))],
        commands=commands, fixture_seed=spec['reserved_seed'] if reserved else spec['generated_seed'],
        oracle_cases=len(items),cases_run=len(items) if compare else 0,top_level_outcomes=sum(len(r['names']) for r in requests) if compare else 0,
        source_integrity_checks=len(items) * (2 if compare else 1),destination_tree_checks=len(items) if compare else 0,
        destination_file_checks=sum(e['kind']=='file' for state in after for e in state['destination']) if compare else 0,
        destination_symlink_checks=sum(e['kind']=='symlink' for state in after for e in state['destination']) if compare else 0,
        differences=differences,skipped=[],reference_unchanged=True,raw_run=run.relative_to(ROOT).as_posix(),
        request_sha256=digest(run/'requests.jsonl'),original_after_sha256=digest(run/'original-after.json'),
        rust_adapter=dict(path=str(rust_adapter).replace(str(ROOT),'{repository}'),sha256=digest(rust_adapter)) if compare else None,
        reproduce='target/debug/mucrab-compare --suite w03' + (' --reserved' if reserved else ''))
    acquisition.write_json(run/'report.json',report)
    acquisition.write_json(EVIDENCE/(('comparison-' if compare else 'oracle-')+report['suite']+'.json'),report)
    print('{}: {} copy cases, {} differences; {}'.format(report['status'],len(items),len(differences),run.relative_to(ROOT)))
    return int(bool(differences))


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('action',choices=['oracle','compare'])
    parser.add_argument('--reserved',action='store_true');parser.add_argument('--rust-adapter',type=Path,default=ROOT/'target/debug/mucrab-compare')
    args=parser.parse_args(); old_mask=os.umask(0o022)
    try: return run_suite(args.action=='compare',args.reserved,args.rust_adapter.resolve())
    except (OSError,ValueError,KeyError,TypeError,subprocess.SubprocessError) as error:
        print('W03 unavailable: '+str(error),file=sys.stderr);return 2
    finally: os.umask(old_mask)


if __name__=='__main__': sys.exit(main())
