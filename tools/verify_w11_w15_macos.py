#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Fresh Mac checks for W11–W15; retain Linux receipts and every failed attempt.

Use the test-only Python environment containing Paramiko 4.0.0 and Pillow.
Native input and isolated Keychain acceptance have separate receipts.
"""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import sys
import time
import uuid

from w01 import ROOT, digest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cross-device-root', type=Path, required=True)
    args = parser.parse_args()
    if platform.system() != 'Darwin' or os.geteuid() == 0:
        parser.error('Run on macOS as an unprivileged user')
    cross = args.cross_device_root.resolve(strict=True)
    if cross.stat().st_dev == ROOT.stat().st_dev:
        parser.error('A genuine second filesystem is required')
    run = ROOT / 'artifacts/macos-verification' / (
        'W11-W15-' + datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ') + '-' + uuid.uuid4().hex[:8])
    run.mkdir(parents=True)
    destination = ROOT / 'evidence/platforms' / ('macos-' + platform.machine())
    environment = dict(os.environ, TMPDIR=str(cross))
    environment['PATH'] = str(Path(sys.executable).parent) + os.pathsep + environment['PATH']
    checks = []
    counts = {}

    def check(label, command, timeout=300, expected=0):
        started = time.monotonic()
        log = run / (label + '.log')
        with log.open('w') as stream:
            try:
                result = subprocess.run(command, cwd=ROOT, env=environment,
                                        stdout=stream, stderr=subprocess.STDOUT, timeout=timeout)
                code = result.returncode
            except subprocess.TimeoutExpired:
                code = 'timeout'
        output = log.read_text(errors='replace')
        row = dict(check=label, argv=command, exit_code=code, expected_exit_code=expected,
                   status='passed' if code == expected else 'failed',
                   seconds=round(time.monotonic() - started, 2),
                   log=log.relative_to(ROOT).as_posix(), log_sha256=digest(log))
        checks.append(row)
        (run / 'checks.json').write_text(json.dumps(checks, indent=2) + '\n')
        if label == 'rust-tests':
            counts['rust_passed'] = sum(map(int, re.findall(r'test result: ok\. (\d+) passed;', output)))
            counts['rust_ignored'] = sum(map(int, re.findall(r'test result: ok\. \d+ passed; \d+ failed; (\d+) ignored;', output)))
        if label == 'python-tests' and (match := re.search(r'Ran (\d+) tests', output)):
            counts['python_passed'] = int(match[1])
        print(label + ': ' + row['status'], flush=True)

    py = sys.executable
    for label, command in [
        ('reference', [py, '-B', 'tools/acquire_mucommander.py', '--verify-only']),
        ('format', ['cargo', 'fmt', '--all', '--', '--check']),
        ('build', ['cargo', 'build', '--workspace', '--all-features', '--locked', '--offline']),
        ('rust-tests', ['cargo', 'test', '--workspace', '--all-features', '--locked', '--offline']),
        ('clippy', ['cargo', 'clippy', '--workspace', '--all-targets', '--all-features', '--locked', '--offline', '--', '-D', 'warnings']),
        ('python-tests', [py, '-B', '-m', 'unittest', 'discover', '-s', 'tools', '-p', 'test_*.py']),
        ('version', ['target/debug/mucrab', '--version']),
        ('help', ['target/debug/mucrab', '--help']),
    ]:
        check(label, command)
    for slug in ['w11-tabs-bookmarks', 'w12-file-search', 'w13-properties-permissions',
                 'w14-built-in-viewer', 'w15-sftp-connections']:
        check(slug + '-spec', ['openspec', 'validate', slug, '--strict'])
    check('w11-original-probe', [py, '-B', 'tools/w11.py'])
    for suite in ['w12', 'w13', 'w14', 'w15']:
        for reserved in [False, True]:
            check(suite + ('-reserved' if reserved else '-predefined'),
                  [py, '-B', 'tools/' + suite + '.py'] + (['--reserved'] if reserved else [])
                  + (['--fixture-root', str(cross / 'w13-fixtures')] if suite == 'w13' else []))
    files = [ROOT / 'Cargo.toml', ROOT / 'Cargo.lock']
    for folder, pattern in [('crates', '*.rs'), ('crates', 'Cargo.toml'),
                            ('tools', '*.py'), ('tools/java', '*.java'), ('fixtures', '*.json')]:
        files.extend((ROOT / folder).rglob(pattern))
    report = dict(status='passed' if all(c['status'] == 'passed' for c in checks) else 'failed',
                  date=datetime.now(timezone.utc).isoformat(),
                  scope='Current Mac workspace and scoped original/native W11–W15 comparisons; native UI and credential services are separate',
                  host=dict(system=platform.system(), architecture=platform.machine(), platform=platform.platform(), python=sys.version, uid=os.geteuid()),
                  source_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
                  source_inputs={p.relative_to(ROOT).as_posix(): digest(p) for p in sorted(set(files))},
                  binary_sha256=digest(ROOT / 'target/debug/mucrab'), counts=counts,
                  source_device=ROOT.stat().st_dev, temporary_device=cross.stat().st_dev,
                  mount_table=subprocess.check_output(['/sbin/mount'], text=True),
                  raw_run=run.relative_to(ROOT).as_posix(), checks=checks)
    (run / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    (destination / 'w11-w15-validation.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(status=report['status'], counts=counts, run=report['raw_run']), indent=2))
    return int(report['status'] != 'passed')


if __name__ == '__main__':
    raise SystemExit(main())
