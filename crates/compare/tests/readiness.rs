// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

use std::process::Command;

#[test]
fn unavailable_comparison_cannot_be_mistaken_for_a_passing_run() {
    for arguments in [vec![], vec!["--status"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_mucrab-compare"))
            .args(arguments)
            .output()
            .expect("comparison harness should start");
        assert_eq!(output.status.code(), Some(2));
        let report = String::from_utf8(output.stdout).expect("report should be UTF-8");
        assert!(report.contains(r#""status":"not_ready""#));
        assert!(report.contains(r#""cases_run":0"#));
    }
}
