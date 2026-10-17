# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
import json
from pathlib import Path
import os
import tempfile
import unittest
from unittest.mock import patch
import w03

class CopyProtocolTests(unittest.TestCase):
    def test_macos_filesystem_comparison_keeps_contents_exact_and_raw_names(self):
        original = [dict(path='e\u0301.txt', kind='file', sha256='original')]
        native = [dict(path='é.txt', kind='file', sha256='original')]
        with patch.object(w03.platform, 'system', return_value='Darwin'):
            self.assertEqual(w03.comparable_tree(original), w03.comparable_tree(native))
            self.assertEqual(original[0]['path'], 'e\u0301.txt')
            native[0]['sha256'] = 'changed'
            self.assertNotEqual(w03.comparable_tree(original), w03.comparable_tree(native))
            with self.assertRaises(ValueError):
                w03.comparable_tree(original + native)

    def test_linux_filesystem_comparison_retains_distinct_unicode_spellings(self):
        with patch.object(w03.platform, 'system', return_value='Linux'):
            self.assertNotEqual(w03.filesystem_name('e\u0301'), w03.filesystem_name('é'))

    def setUp(self):
        self.request=dict(id='copy',names=['a','b'])
        self.result=dict(id='copy',status='ok',state='RUNNING',outcomes=[dict(name='a',success=True),dict(name='b',success=False)])
    def test_complete_copy_outcomes_are_accepted(self):
        self.assertEqual(w03.decode(json.dumps(self.result),[self.request]),[self.result])
    def test_zero_missing_extra_or_reordered_work_cannot_pass(self):
        for text,requests in [('',[]),('',[self.request]),(json.dumps(self.result)+'\n'+json.dumps(self.result),[self.request])]:
            with self.assertRaises(ValueError):w03.decode(text,requests)
        self.result['outcomes'].reverse()
        with self.assertRaises(ValueError):w03.decode(json.dumps(self.result),[self.request])
    def test_non_boolean_or_incomplete_outcomes_are_unavailable(self):
        self.result['outcomes'][0]['success']=1
        with self.assertRaises(ValueError):w03.decode(json.dumps(self.result),[self.request])
        self.result['outcomes'][0]['success']=True;self.result['state']='FINISHED'
        with self.assertRaises(ValueError):w03.decode(json.dumps(self.result),[self.request])
    def test_inspector_hashes_denied_files_and_restores_modes_without_following_links(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);(root/'locked').write_bytes(b'fixture bytes');os.chmod(root/'locked',0)
            (root/'link').symlink_to('missing')
            try:
                snapshot=w03.snapshot(root)
                self.assertEqual((root/'locked').stat().st_mode&0o777,0)
                self.assertEqual(snapshot[1]['mode'],0);self.assertEqual(snapshot[1]['size'],13)
                self.assertEqual(snapshot[0]['target'],'missing')
                self.assertEqual(snapshot,w03.snapshot(root))
            finally:os.chmod(root/'locked',0o600)

if __name__=='__main__':unittest.main()
