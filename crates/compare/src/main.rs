// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

//! Comparison-harness entry point. Java and Rust behavior adapters follow in
//! the next migration increment; this scaffold cannot report a passing run.

use std::process::ExitCode;

fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if let [argument] = arguments.as_slice() {
        if argument == "--version" || argument == "-V" {
            println!("mucrab-compare {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        if argument == "--help" || argument == "-h" {
            println!(
                "muCrab comparison harness\n\n\
                 Usage: mucrab-compare [--status | --help | --version]\n\n\
                 The Java and Rust behavior adapters are not implemented yet.\n\
                 Status and comparison attempts exit with code 2 while unavailable."
            );
            return ExitCode::SUCCESS;
        }
    }
    if !arguments.is_empty() && arguments.as_slice() != ["--status"] {
        eprintln!("Unsupported arguments. Use mucrab-compare --help.");
        return ExitCode::from(2);
    }
    println!(
        r#"{{"status":"not_ready","cases_run":0,"reason":"Java and Rust behavior adapters are not implemented"}}"#
    );
    ExitCode::from(2)
}
