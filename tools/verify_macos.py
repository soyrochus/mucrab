#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
"""Run fresh macOS workspace checks and original-binary comparison suites.

Supply a writable directory on a genuinely different mounted filesystem.
W07 opens a disposable original application window; explicitly opt in to it.
Native input and release packaging are separate acceptance checks.
"""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import time
import uuid

from w01 import ROOT, digest

SUITES = ('w01', 'w02', 'w02b', 'w02c', 'w03', 'w04', 'w05', 'w06', 'w07')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cross-device-root', type=Path, required=True)
    parser.add_argument('--allow-reference-gui', action='store_true')
    parser.add_argument('--skip-checks', action='store_true')
    parser.add_argument('--suites', nargs='*', choices=SUITES, default=list(SUITES))
    args = parser.parse_args()
    if platform.system() != 'Darwin' or os.geteuid() == 0:
        parser.error('Run as an unprivileged macOS user')
    cross = args.cross_device_root.resolve(strict=True)
    if not cross.is_dir() or cross.stat().st_dev == ROOT.stat().st_dev:
        parser.error('The temporary directory must be on a real second filesystem')
    if 'w07' in args.suites and not args.allow_reference_gui:
        parser.error('W07 requires --allow-reference-gui for its disposable reference window')
    env = dict(os.environ, TMPDIR=str(cross), MUCRAB_MACOS_REFERENCE_GUI='1' if args.allow_reference_gui else '0')
    env['PATH'] = str(Path(sys.executable).parent) + os.pathsep + env['PATH']
    run = ROOT / 'artifacts/macos-verification' / (datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ') + '-' + uuid.uuid4().hex[:8])
    run.mkdir(parents=True)
    checks = []

    def check(label, command, expected=0):
        start = time.monotonic()
        log = run / (label + '.log')
        with log.open('w') as stream:
            process = subprocess.run(command, cwd=ROOT, env=env, stdout=stream, stderr=subprocess.STDOUT)
        row = dict(check=label, argv=command, exit_code=process.returncode, expected_exit_code=expected,
                   status='passed' if process.returncode == expected else 'failed', seconds=round(time.monotonic()-start, 2),
                   log=log.relative_to(ROOT).as_posix(), log_sha256=digest(log))
        checks.append(row)
        (run / 'checks.json').write_text(json.dumps(checks, indent=2) + '\n')
        print(label + ': ' + row['status'], flush=True)

    if not args.skip_checks:
        for label, command in [
            ('reference', [sys.executable, '-B', 'tools/acquire_mucommander.py', '--verify-only']),
            ('analysis', [sys.executable, '-B', 'tools/analysis_tools.py', 'probe']),
            ('python-tests', [sys.executable, '-B', '-m', 'unittest', 'discover', '-s', 'tools', '-p', 'test_*.py', '-v']),
            ('build', ['cargo', 'build', '--workspace', '--all-features', '--locked', '--offline']),
            ('format', ['cargo', 'fmt', '--all', '--', '--check']),
            ('clippy', ['cargo', 'clippy', '--workspace', '--all-targets', '--all-features', '--locked', '--offline', '--', '-D', 'warnings']),
            ('rust-tests', ['cargo', 'test', '--workspace', '--all-features', '--locked', '--offline']),
            ('version', ['target/debug/mucrab', '--version']),
            ('help', ['target/debug/mucrab', '--help']),
        ]:
            check(label, command)
        check('readiness-not-run', ['target/debug/mucrab-compare', '--status'], expected=2)
    for suite in args.suites:
        for reserved in (False, True):
            check(suite + ('-reserved' if reserved else '-predefined'),
                  ['target/debug/mucrab-compare', '--suite', suite] + (['--reserved'] if reserved else []))
    files = [ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', ROOT / '.github/workflows/release.yml'] + list((ROOT / 'crates').rglob('*.rs')) + list((ROOT / 'crates').rglob('Cargo.toml')) + list((ROOT / 'tools').glob('*.py')) + list((ROOT / 'tools/java').glob('*.java')) + list((ROOT / 'fixtures').rglob('*.json'))
    report = dict(status='passed' if checks and all(row['status'] == 'passed' for row in checks) else 'failed',
                  scope='Automated macOS checks and selected original-binary suites; native input and release acceptance recorded separately',
                  host=dict(system=platform.system(), architecture=platform.machine(), platform=platform.platform(), python=sys.version, uid=os.geteuid()),
                  source_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
                  source_inputs=[dict(path=p.relative_to(ROOT).as_posix(), sha256=digest(p)) for p in sorted(files)],
                  source_device=ROOT.stat().st_dev, temporary_device=cross.stat().st_dev,
                  raw_run=run.relative_to(ROOT).as_posix(), checks=checks)
    (run / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    destination = ROOT / 'evidence/platforms' / ('macos-' + platform.machine())
    destination.mkdir(parents=True, exist_ok=True)
    name = 'validation.json' if not args.skip_checks and tuple(args.suites) == SUITES else 'validation-selected.json'
    (destination / name).write_text(json.dumps(report, indent=2) + '\n')
    print('Report: ' + str(destination / name))
    return int(report['status'] != 'passed')


if __name__ == '__main__':
    raise SystemExit(main())
