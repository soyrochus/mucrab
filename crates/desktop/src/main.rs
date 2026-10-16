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
                 Usage: mucrab [LEFT_FOLDER [RIGHT_FOLDER]]\n\
                        mucrab [--help | --version]\n\n\
                 Opens the native two-pane local browser.\n\
                 Name ordering uses en-US; Show hidden files applies to both panes.\n\
                 F5 copies, F6 moves, F2 renames; Batch rename reviews masks and counters.\n\
                 Opening symbolic links from the listing is deferred."
            );
            return ExitCode::SUCCESS;
        }
    }
    if arguments.len() > 2
        || arguments
            .iter()
            .any(|arg| arg.to_string_lossy().starts_with('-'))
    {
        eprintln!("Expected up to two folder paths. Use mucrab --help.");
        return ExitCode::from(2);
    }
    match app::run(
        arguments
            .into_iter()
            .map(std::path::PathBuf::from)
            .collect(),
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Could not start muCrab: {error}");
            ExitCode::FAILURE
        }
    }
}
