// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

use std::process::ExitCode;

mod app;

fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if let [argument] = arguments.as_slice() {
        if argument == "--version" || argument == "-V" {
            println!("muCrab {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        if argument == "--help" || argument == "-h" {
            println!(
                "muCrab — Two claws, two panes.\n\n\
                 Usage: mucrab [--help | --version]\n\n\
                 With no arguments, opens the native desktop preview.\n\
                 File browsing and file operations are not implemented yet."
            );
            return ExitCode::SUCCESS;
        }
    }
    if !arguments.is_empty() {
        eprintln!("Unsupported arguments. Use mucrab --help.");
        return ExitCode::from(2);
    }
    match app::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Could not start muCrab: {error}");
            ExitCode::FAILURE
        }
    }
}
