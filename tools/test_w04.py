# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
import copy
import unittest
import w04

class MoveObservationTests(unittest.TestCase):
    def test_conservation_rejects_lost_bytes_and_links_and_accepts_mapped_names(self):
        request=dict(id='rename',names=['old'],new_name='new')
        before=dict(source=[dict(path='old',kind='file',size=3,sha256='abc')])
        after=dict(source=[],destination=[dict(path='new',kind='file',size=3,sha256='abc')])
        w04.assert_selected_content_preserved([request],[before],[after])
        after['destination'][0]['sha256']='bad'
        with self.assertRaises(ValueError):w04.assert_selected_content_preserved([request],[before],[after])
        before['source']=[dict(path='old',kind='symlink',target='target')];after['destination']=[dict(path='new',kind='file',size=3,sha256='abc')]
        with self.assertRaises(ValueError):w04.assert_selected_content_preserved([request],[before],[after])
    def test_date_mask_is_limited_to_input_selected_source_directories(self):
        item=dict(names=['old'],source=[dict(path='old',kind='directory'),dict(path='old/sub/file',kind='file'),dict(path='other',kind='directory')])
        self.assertEqual(w04.source_date_mask(item),['old','old/sub'])
    def test_opaque_inodes_do_not_hide_identity_relation_differences(self):
        expected=dict(source=[],destination=[],identities=[dict(name='file',identity_retained=True,destination_identity=[1,2])])
        actual=copy.deepcopy(expected);actual['identities'][0]['destination_identity']=[47,999]
        self.assertEqual(w04.comparable_after(expected),w04.comparable_after(actual))
        actual['identities'][0]['identity_retained']=False
        self.assertNotEqual(w04.comparable_after(expected),w04.comparable_after(actual))

if __name__=='__main__':unittest.main()
