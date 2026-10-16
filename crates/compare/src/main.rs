// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

use mucrab_core::{FileEntry, NameSorter, SortOptions, list_directory};
mod w02;
mod w02b;
mod w02c;
use serde::{Deserialize, Serialize};
use std::{
    io::{self, BufRead},
    path::PathBuf,
    process::{Command, ExitCode},
};

#[derive(Deserialize)]
struct Request {
    id: String,
    operation: String,
    path: Option<PathBuf>,
    entries: Option<Vec<FileEntry>>,
    locale: String,
    #[serde(flatten)]
    options: SortOptions,
}

#[derive(Serialize)]
struct Response {
    id: String,
    status: &'static str,
    entries: Vec<FileEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pairs: Option<Vec<Vec<i8>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    order: Option<Vec<usize>>,
}

fn adapter() -> Result<(), Box<dyn std::error::Error>> {
    let sorter = NameSorter::en_us()?;
    for line in io::stdin().lock().lines() {
        let request: Request = serde_json::from_str(&line?)?;
        if request.locale != "en-US" {
            return Err("Only en-US is in W01 scope".into());
        }
        let result = match request.operation.as_str() {
            "compare" => Ok(request.entries.ok_or("Missing entries")?),
            "enumerate" => list_directory(&request.path.ok_or("Missing path")?),
            _ => return Err("Unknown adapter operation".into()),
        };
        let response = match result {
            Ok(entries) => {
                let pairs = entries
                    .iter()
                    .map(|left| {
                        entries
                            .iter()
                            .map(|right| match sorter.compare(left, right, request.options) {
                                std::cmp::Ordering::Less => -1,
                                std::cmp::Ordering::Equal => 0,
                                std::cmp::Ordering::Greater => 1,
                            })
                            .collect()
                    })
                    .collect();
                let order = sorter.sorted_indices(&entries, request.options);
                Response {
                    id: request.id,
                    status: "ok",
                    entries,
                    error: None,
                    pairs: Some(pairs),
                    order: Some(order),
                }
            }
            Err(_) => Response {
                id: request.id,
                status: "error",
                entries: vec![],
                error: Some("directory_unavailable"),
                pairs: None,
                order: None,
            },
        };
        println!("{}", serde_json::to_string(&response)?);
    }
    Ok(())
}

fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.as_slice() == ["--adapter-w02c"] {
        return match w02c::adapter() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("W02c adapter unavailable: {error}");
                ExitCode::from(2)
            }
        };
    }
    if arguments.as_slice() == ["--adapter-w02b"] {
        return match w02b::adapter() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("W02b adapter unavailable: {error}");
                ExitCode::from(2)
            }
        };
    }
    if arguments.as_slice() == ["--adapter-w02"] {
        return match w02::adapter() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("W02 adapter unavailable: {error}");
                ExitCode::from(2)
            }
        };
    }
    if arguments.as_slice() == ["--adapter"] {
        return match adapter() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("W01 adapter unavailable: {error}");
                ExitCode::from(2)
            }
        };
    }
    if arguments.as_slice() == ["--version"] || arguments.as_slice() == ["-V"] {
        println!("mucrab-compare {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    if arguments.as_slice() == ["--help"] || arguments.as_slice() == ["-h"] {
        println!(
            "muCrab comparison harness\n\nUsage: mucrab-compare --suite w01|w02|w02b|w02c [--reserved]\n       mucrab-compare --status\n\nRuns original Java binary behavior and the Rust adapter.\nRequires the acquired reference and a JDK. Status alone exits 2 with zero cases."
        );
        return ExitCode::SUCCESS;
    }
    if (arguments.len() == 2 || (arguments.len() == 3 && arguments[2] == "--reserved"))
        && arguments[0] == "--suite"
        && ["w01", "w02", "w02b", "w02c"]
            .iter()
            .any(|suite| arguments[1] == *suite)
    {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut command = Command::new("python3");
        command
            .arg("-B")
            .arg(root.join(if arguments[1] == "w02c" {
                "tools/w02c.py"
            } else if arguments[1] == "w02b" {
                "tools/w02b.py"
            } else if arguments[1] == "w02" {
                "tools/w02.py"
            } else {
                "tools/w01.py"
            }))
            .arg("compare")
            .arg("--rust-adapter")
            .arg(std::env::current_exe().expect("executable path"));
        if arguments.len() == 3 {
            command.arg("--reserved");
        }
        return match command.status() {
            Ok(status) => ExitCode::from(status.code().unwrap_or(2).try_into().unwrap_or(2)),
            Err(error) => {
                eprintln!("Comparison runner unavailable: {error}");
                ExitCode::from(2)
            }
        };
    }
    if arguments.is_empty() || arguments.as_slice() == ["--status"] {
        println!(
            r#"{{"status":"not_run","cases_run":0,"reason":"Run --suite w01, w02, w02b or w02c for a fresh scoped comparison"}}"#
        );
    } else {
        eprintln!("Unsupported arguments. Use mucrab-compare --help.");
    }
    ExitCode::from(2)
}
