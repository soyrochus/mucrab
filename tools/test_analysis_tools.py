# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
"""Checks that readiness cannot be reported for the wrong inputs or failures."""

import unittest

import analysis_tools as tools


class AnalysisToolTests(unittest.TestCase):
    def test_node_policy_includes_only_supported_release_lines(self):
        for version in ("v22.19.0", "24.11.0", "v24.12.0", "26.0.0"):
            self.assertTrue(tools.supported_node(version), version)
        for version in ("v20.19.0", "v22.18.0", "v24.10.9", "v25.0.0", "unavailable"):
            self.assertFalse(tools.supported_node(version), version)

    def test_unrelated_android_failures_cannot_pass_the_format_boundary_probe(self):
        error = {
            "code": "capability_unavailable",
            "category": "unsupported_provider",
            "details": {
                "provider_id": "jadx", "operation": "inspect_android_package",
                "reason": "Missing REA_JADX_MCP_JAR",
            },
        }
        with self.assertRaisesRegex(ValueError, "other than the expected APK-only"):
            tools.android_boundary(error)
        error["details"]["reason"] = "Target example.jar is zip; provide one standalone APK."
        self.assertTrue(tools.android_boundary(error)["expected_rejection"])

    def test_wrong_jar_or_class_digest_cannot_pass_inventory_probe(self):
        evidence = {
            "subject": {"digest": {"sha256": "wrong-jar"}},
            "normalized_result": {"substeps": []},
        }
        with self.assertRaisesRegex(ValueError, "different JAR"):
            tools.inventory_facts(evidence, "expected-jar", "Example.class", "expected-class")
        evidence["subject"]["digest"]["sha256"] = "expected-jar"
        evidence["normalized_result"]["substeps"] = [{
            "operation": "inventory_artifact",
            "evidence": {"normalized_result": {
                "occurrences": [{"logical_path": "Example.class", "artifact_id": "example", "hash_status": "verified"}],
                "nodes": [{"artifact_id": "example", "sha256": "wrong-class"}],
            }},
        }]
        with self.assertRaisesRegex(ValueError, "class fingerprint differs"):
            tools.inventory_facts(evidence, "expected-jar", "Example.class", "expected-class")


if __name__ == "__main__":
    unittest.main()
