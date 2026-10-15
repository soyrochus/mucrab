# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
import json
import unittest

import w02


class SelectionProtocolTests(unittest.TestCase):
    def setUp(self):
        self.requests = [dict(id="case", names=["file"], parent=True, events=[dict(op="next")])]
        initial = dict(visible=["..", "file"], focus_row=0, focused="..", marks=[False, False],
                       marked=[], selected=[], marked_count=0)
        after = dict(initial, focus_row=1, focused="file", selected=["file"])
        self.result = dict(id="case", status="ok", states=[initial, after])

    def test_rejects_missing_transition_and_zero_work(self):
        self.result["states"].pop()
        with self.assertRaises(ValueError):
            w02.decode(json.dumps(self.result), self.requests)
        with self.assertRaises(ValueError):
            w02.decode("", [])

    def test_rejects_parent_mark_and_inconsistent_count(self):
        self.result["states"][0].update(marks=[True, False], marked=[".."], selected=[".."], marked_count=1)
        with self.assertRaises(ValueError):
            w02.decode(json.dumps(self.result), self.requests)

    def test_rejects_focused_parent_as_effective_file_selection(self):
        self.result["states"][0]["selected"] = [".."]
        with self.assertRaises(ValueError):
            w02.decode(json.dumps(self.result), self.requests)


if __name__ == "__main__":
    unittest.main()
