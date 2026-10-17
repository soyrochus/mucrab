#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
"""Prepare/check W07 real-file native replay at the stable 1120x720 geometry.

20,000 small files make an actual copy long enough to operate Pause/Resume and
Cancel without modifying application code or injecting delays. Exact copied
counts/timings are deliberately not fixtures. Preparation refuses overwrite.
"""
import argparse
import json
from pathlib import Path
from w01 import ROOT, digest

RUN = ROOT / 'artifacts/gui/W07-background'
FIXTURE = ROOT / 'fixtures/W07-background-operations/native-gui-actions.json'
COUNT = 20000
CONTENT = b'worker file contents\n'


def prepare():
    RUN.mkdir(parents=True, exist_ok=False)
    source, target = RUN / 'source', RUN / 'target'
    source.mkdir(); target.mkdir()
    for name in ['00-done', '10-work', '99-later', 'browse']:
        (source / name).mkdir()
    (source / '00-done/complete.txt').write_bytes(b'completed root\n')
    (source / '99-later/untouched.txt').write_bytes(b'later source\n')
    (source / 'browse/unrelated.txt').write_bytes(b'unrelated location\n')
    for index in range(COUNT):
        (source / '10-work' / f'file{index:05}.txt').write_bytes(CONTENT)
    launch = dict(argv=[str(ROOT / 'target/debug/mucrab'), str(source), str(target)],
                  cwd=str(ROOT), title='muCrab', output=str(RUN / 'native'),
                  env={'XDG_CONFIG_HOME': str(RUN / 'config')})
    (RUN / 'launch.json').write_text(json.dumps(launch, indent=2) + '\n')
    (RUN / 'provenance.json').write_text(json.dumps(dict(native_binary_sha256=digest(ROOT / 'target/debug/mucrab'), replay_sha256=digest(FIXTURE), file_count=COUNT), indent=2) + '\n')
    print(RUN / 'launch.json')


def verify_filesystem():
    source, target = RUN / 'source', RUN / 'target'
    provenance = json.loads((RUN / 'provenance.json').read_text())
    if digest(ROOT / 'target/debug/mucrab') != provenance['native_binary_sha256']:
        raise ValueError('Executable changed since replay preparation')
    if digest(FIXTURE) != provenance['replay_sha256']:
        raise ValueError('Replay changed since preparation')
    files = list((target / '10-work').iterdir())
    if not 0 < len(files) < COUNT:
        raise ValueError('No genuine partial folder copy retained')
    if (target / '99-later').exists() or (target / 'browse').exists():
        raise ValueError('Work after interrupted root was incorrectly started')
    if (target / '00-done/complete.txt').read_bytes() != b'completed root\n':
        raise ValueError('Earlier completed root missing or changed')
    partial_children = 0
    for path in files:
        data = path.read_bytes()
        if not path.is_file() or len(data) > len(CONTENT) or data != CONTENT[:len(data)]:
            raise ValueError('Child is not a valid source prefix: ' + str(path))
        partial_children += data != CONTENT
    if partial_children > 3:
        raise ValueError('More incomplete children than cancelled jobs')
    originals = list((source / '10-work').iterdir())
    if len(originals) != COUNT or any(path.read_bytes() != CONTENT for path in originals):
        raise ValueError('Source file changed or removed')
    if (source / '99-later/untouched.txt').read_bytes() != b'later source\n' or (source / 'browse/unrelated.txt').read_bytes() != b'unrelated location\n':
        raise ValueError('Later or unrelated source changed')
    result = dict(status='passed', destination_children=len(files), partial_children=partial_children, source_files=COUNT,
                  checks=['actual Pause/Resume/Cancel controls', 'copy roots and marks', 'second job disabled',
                          'browse and mark unrelated location while paused', 'unrelated marks retained on acknowledgement',
                          'window close wakes paused worker and exits normally before driver cleanup',
                          'earlier destination preserved, interrupted folder partial, later roots untouched', 'all source and completed-child bytes'])
    return result


def verify():
    result = verify_filesystem()
    expected = [dict(op='capture', name='initial', wait=2)] + json.loads(FIXTURE.read_text())
    actual = json.loads((RUN / 'native/actions.json').read_text())
    if actual != expected or actual[-1] != dict(op='await_exit', exit_code=0):
        raise ValueError('Replay incomplete; normal exit before driver cleanup must be acknowledged')
    result['actions'] = len(actual)
    (RUN / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['prepare', 'verify'])
    args = parser.parse_args()
    prepare() if args.action == 'prepare' else verify()
