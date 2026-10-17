# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
import copy
import json
import unittest

import w02b


class FilteringProtocolTests(unittest.TestCase):
    def setUp(self):
        pane = dict(names=[".hidden", "file"], parent=True)
        self.requests = [dict(id="filter", show_hidden=True, panes=[pane, pane], events=[dict(op="toggle_hidden")])]
        shown = dict(visible=["..", ".hidden", "file"], focus_row=1, focused=".hidden", marks=[False, True, False],
            marked=[".hidden"], selected=[".hidden"], marked_count=1)
        hidden = dict(visible=["..", "file"], focus_row=1, focused="file", marks=[False, False],
            marked=[], selected=["file"], marked_count=0)
        self.result = dict(id="filter", status="ok", states=[dict(show_hidden=True, panes=[shown, copy.deepcopy(shown)]),
            dict(show_hidden=False, panes=[hidden, copy.deepcopy(hidden)])])

    def decode(self):
        return w02b.decode(json.dumps(self.result), self.requests)

    def test_complete_two_pane_toggle_is_accepted(self):
        self.assertEqual(len(self.decode()[0]["states"]), 2)

    def test_rejects_missing_toggle_snapshot_or_pane(self):
        self.result["states"][1]["panes"].pop()
        with self.assertRaises(ValueError): self.decode()
        self.result["states"].pop()
        with self.assertRaises(ValueError): self.decode()
        with self.assertRaises(ValueError): w02b.decode("", [])

    def test_rejects_hidden_entries_in_either_filtered_pane(self):
        self.result["states"][1]["panes"][1] = self.result["states"][0]["panes"][1]
        with self.assertRaises(ValueError): self.decode()

    def test_rejects_stale_marks_focus_and_shared_setting(self):
        for update in (dict(marked=[".hidden"], selected=[".hidden"], marked_count=1),
                       dict(focus_row=2, focused=".hidden"), dict(marks=[True, False], marked=[".."], selected=[".."], marked_count=1)):
            result = copy.deepcopy(self.result)
            result["states"][1]["panes"][0].update(update)
            with self.assertRaises(ValueError): w02b.decode(json.dumps(result), self.requests)
        self.result["states"][1]["show_hidden"] = True
        with self.assertRaises(ValueError): self.decode()


if __name__ == "__main__":
    unittest.main()
