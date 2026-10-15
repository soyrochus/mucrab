# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors

import json
import unittest

from w01 import decode_responses


class ProtocolTests(unittest.TestCase):
    def test_incomplete_and_reordered_outputs_are_unavailable(self):
        for text in ("", '{"id":"other","status":"error","error":"directory_unavailable"}'):
            with self.assertRaises(ValueError):
                decode_responses(text, [{"id": "expected"}])

    def test_malformed_permutations_and_signs_cannot_pass(self):
        valid = {"id": "case", "status": "ok", "entries": [{}, {}],
                 "pairs": [[0, -1], [1, 0]], "order": [0, 1]}
        for changes in ({"order": [0, 0]}, {"pairs": [[0, -1], [1, 9]]}, {"pairs": [[0], [1]]}):
            with self.assertRaises(ValueError):
                decode_responses(json.dumps(dict(valid, **changes)), [{"id": "case"}])

    def test_error_is_an_explicit_outcome_not_an_empty_listing(self):
        response = {"id": "case", "status": "error", "error": "directory_unavailable"}
        self.assertEqual(decode_responses(json.dumps(response), [{"id": "case"}]), [response])
        with self.assertRaises(ValueError):
            decode_responses(json.dumps(dict(response, error="unknown")), [{"id": "case"}])


if __name__ == "__main__":
    unittest.main()
