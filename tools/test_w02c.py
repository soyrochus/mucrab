# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
import copy
import json
import unittest
import w02c


class MetadataProtocolTests(unittest.TestCase):
    def setUp(self):
        entries = [dict(name=n, is_directory=False, is_symlink=False, metadata=dict(size=size, modified_ms=0, extension=None)) for n, size in (("a", 2), ("b", 1))]
        self.request = dict(id="metadata", kind="synthetic", mode="NATURAL", parent=False, entries=entries,
            events=[dict(op="sort", criterion="SIZE", ascending=True, directories_first=False)])
        selection = dict(visible=["a", "b"], focus_row=0, focused="a", marks=[False, False], marked=[], selected=["a"], marked_count=0)
        initial = dict(criterion="NAME", ascending=True, directories_first=True, mode="NATURAL", pairs=[[0,-1],[1,0]], order=[0,1], selection=selection)
        after = dict(criterion="SIZE", ascending=True, directories_first=False, mode="NATURAL", pairs=[[0,1],[-1,0]], order=[1,0],
            selection=dict(selection, visible=["b", "a"], focus_row=1))
        self.result = dict(id="metadata", status="ok", entries=entries, states=[initial, after])

    def decode(self): return w02c.decode(json.dumps(self.result), [self.request])

    def test_complete_metadata_sort_is_accepted(self):
        self.assertEqual(len(self.decode()[0]["states"]), 2)

    def test_rejects_missing_entries_or_transition(self):
        self.result["entries"] = []
        with self.assertRaises(ValueError): self.decode()
        with self.assertRaises(ValueError): w02c.decode("", [])
        self.result["entries"] = self.request["entries"]; self.result["states"].pop()
        with self.assertRaises(ValueError): self.decode()

    def test_rejects_wrong_permutation_metadata_and_focus(self):
        for change in (dict(order=[0,0]), dict(pairs=[[0,2],[1,0]]), dict(ascending=0)):
            result = copy.deepcopy(self.result); result["states"][1].update(change)
            with self.assertRaises(ValueError): w02c.decode(json.dumps(result), [self.request])
        self.result["states"][1]["selection"]["focused"] = "missing"
        with self.assertRaises(ValueError): self.decode()

    def test_pair_only_extremes_cannot_claim_ordered_lists(self):
        self.request["pairs_only"] = True
        with self.assertRaises(ValueError): self.decode()
        for state in self.result["states"]:
            state.update(order=None, selection=None, pairs=[[0,1],[1,0]])
        self.assertEqual(len(self.decode()[0]["states"]), 2)


if __name__ == "__main__": unittest.main()
