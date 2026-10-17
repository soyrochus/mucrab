# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
import json
from pathlib import Path
import tempfile
import unittest

import w07


class LifecycleProtocolTests(unittest.TestCase):
    def test_reserved_cases_are_deterministic_disjoint_and_have_real_work(self):
        spec = json.loads(w07.SPEC.read_text())
        predefined, reserved = w07.cases(spec, False), w07.cases(spec, True)
        self.assertEqual(reserved, w07.cases(spec, True))
        self.assertEqual(len(predefined), 18)
        self.assertEqual(len(reserved), 12)
        self.assertFalse({row['id'] for row in predefined} & {row['id'] for row in reserved})
        self.assertTrue(all(row['action'] in ('copy', 'move', 'rename', 'batch', 'extract') for row in reserved))
        self.assertTrue(all(row['large_size'] > row['cancel_bytes'] > 0 for row in reserved))

    def test_partial_comparison_rejects_empty_complete_and_corrupt_destinations(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source, dest = root / 'source', root / 'dest'
            source.mkdir(); dest.mkdir()
            (source / 'large.bin').write_bytes(b'abcdefgh')
            request = dict(id='prefix', mode='stream', action='copy', source_directory=str(source), destination_directory=str(dest))
            for invalid in (b'', b'abcdefgh', b'wrong'):
                (dest / 'large.bin').write_bytes(invalid)
                with self.assertRaises(ValueError):
                    w07.comparable(request, dict(destination=w07.tree(dest)))
            (dest / 'large.bin').write_bytes(b'abc')
            result = w07.comparable(request, dict(destination=w07.tree(dest)))
            self.assertEqual(result['destination'], [dict(path='large.bin', kind='file', valid_partial_prefix=True)])

    def test_streaming_without_a_partial_file_is_never_a_pass(self):
        request = dict(id='missing', mode='stream')
        with self.assertRaises(ValueError):
            w07.comparable(request, dict(destination=[]))


if __name__ == '__main__':
    unittest.main()
