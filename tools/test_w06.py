# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
import copy
import json
import unittest

import w06


class ZipProtocolTests(unittest.TestCase):
    def sample(self):
        request = dict(id="zip", list_paths=["", "folder/"], extract=True)
        entry = dict(name="folder", is_directory=True, is_symlink=False,
                     metadata=dict(size=0, modified_ms=1591517350639, extension=None))
        result = dict(id="zip", status="ok", extracted=True, success=True,
                      listings=[dict(path="", status="ok", entries=[entry]), dict(path="folder/", status="ok", entries=[])])
        return request, result

    def test_complete_zip_listings_and_explicit_errors_are_distinct_from_missing_work(self):
        request, result = self.sample()
        self.assertEqual(w06.decode(json.dumps(result), [request]), [result])
        for requests, text in (([], ""), ([request], ""), ([request], json.dumps(result) + "\n" + json.dumps(result))):
            with self.assertRaises(ValueError):
                w06.decode(text, requests)
        bad = copy.deepcopy(result)
        bad["listings"].reverse()
        with self.assertRaises(ValueError):
            w06.decode(json.dumps(bad), [request])
        error = dict(id="zip", status="error", error="archive_unavailable")
        self.assertEqual(w06.decode(json.dumps(error), [request]), [error])

    def test_malformed_flags_and_incomplete_metadata_are_rejected(self):
        request, result = self.sample()
        for key, value in (("success", 1), ("extracted", "true"), ("listings", [])):
            bad = copy.deepcopy(result)
            bad[key] = value
            with self.assertRaises(ValueError):
                w06.decode(json.dumps(bad), [request])
        for key, value in (("size", True), ("modified_ms", "time"), ("extension", 17)):
            bad = copy.deepcopy(result)
            bad["listings"][0]["entries"][0]["metadata"][key] = value
            with self.assertRaises(ValueError):
                w06.decode(json.dumps(bad), [request])

    def test_date_normalization_keeps_raw_data_and_other_fields_exact(self):
        _, result = self.sample()
        normalized = w06.comparable(result)
        self.assertEqual(normalized["listings"][0]["entries"][0]["metadata"]["modified_ms"], 1591517350000)
        self.assertEqual(result["listings"][0]["entries"][0]["metadata"]["modified_ms"], 1591517350639)
        bad = copy.deepcopy(result)
        bad["listings"][0]["entries"][0]["metadata"]["size"] = 1
        self.assertNotEqual(w06.comparable(bad), normalized)


if __name__ == "__main__":
    unittest.main()
