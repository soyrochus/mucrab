// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

use mucrab_core::{
    BrowserPane, FileEntry, NameSorter, PaneEvent, SelectionSnapshot, SortMode, SortOptions,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    io::{self, BufRead},
    path::PathBuf,
};

#[derive(Deserialize)]
struct PaneInput {
    path: PathBuf,
    names: Vec<String>,
    parent: bool,
}

#[derive(Deserialize)]
struct Request {
    id: String,
    mode: SortMode,
    show_hidden: bool,
    panes: Vec<PaneInput>,
    events: Vec<Value>,
}

#[derive(Serialize)]
struct State {
    show_hidden: bool,
    panes: Vec<SelectionSnapshot>,
}

#[derive(Serialize)]
struct Response {
    id: String,
    status: &'static str,
    states: Vec<State>,
}

fn load(pane: &mut BrowserPane, input: &PaneInput, sorter: &NameSorter) -> io::Result<()> {
    let entries = input
        .names
        .iter()
        .map(|name| {
            let path = input.path.join(name);
            Ok(FileEntry {
                name: name.clone(),
                is_directory: path.is_dir(),
                is_symlink: path.symlink_metadata()?.is_symlink(),
            })
        })
        .collect::<io::Result<Vec<_>>>()?;
    pane.load_listing(input.path.clone(), entries, input.parent, sorter);
    Ok(())
}

fn snapshot(show_hidden: bool, panes: &[BrowserPane]) -> State {
    State {
        show_hidden,
        panes: panes
            .iter()
            .map(|p| p.selection.snapshot(&p.entries, &p.order))
            .collect(),
    }
}

pub fn adapter() -> Result<(), Box<dyn std::error::Error>> {
    let sorter = NameSorter::en_us()?;
    for line in io::stdin().lock().lines() {
        let request: Request = serde_json::from_str(&line?)?;
        if request.panes.len() != 2 {
            return Err("Exactly two panes required".into());
        }
        let mut show_hidden = request.show_hidden;
        let mut panes = Vec::new();
        for input in &request.panes {
            let mut pane = BrowserPane {
                show_hidden,
                options: SortOptions {
                    mode: request.mode,
                    ..Default::default()
                },
                ..Default::default()
            };
            load(&mut pane, input, &sorter)?;
            panes.push(pane);
        }
        let mut states = vec![snapshot(show_hidden, &panes)];
        for event in request.events {
            let op = event["op"].as_str().ok_or("Missing operation")?;
            if op == "toggle_hidden" {
                show_hidden = !show_hidden;
                for (pane, input) in panes.iter_mut().zip(&request.panes) {
                    pane.show_hidden = show_hidden;
                    load(pane, input, &sorter)?;
                }
            } else {
                let index = event["pane"].as_u64().ok_or("Missing pane")?;
                let index = usize::try_from(index)?;
                let pane = panes.get_mut(index).ok_or("Invalid pane")?;
                if op == "refresh" {
                    load(pane, &request.panes[index], &sorter)?;
                } else {
                    let command = if op == "focus_name" || op == "click_name" {
                        let name = event["name"].as_str().ok_or("Missing name")?;
                        let state = pane.selection.snapshot(&pane.entries, &pane.order);
                        let row = state
                            .visible
                            .iter()
                            .position(|n| n == name)
                            .ok_or("Name is not visible")?;
                        if op == "focus_name" {
                            PaneEvent::Focus { row }
                        } else {
                            PaneEvent::Click {
                                row,
                                ctrl: event.get("ctrl").map_or(Ok(false), |v| {
                                    v.as_bool().ok_or("Invalid Ctrl modifier")
                                })?,
                                shift: event.get("shift").map_or(Ok(false), |v| {
                                    v.as_bool().ok_or("Invalid Shift modifier")
                                })?,
                            }
                        }
                    } else {
                        serde_json::from_value::<PaneEvent>(event)?
                    };
                    if let PaneEvent::Sort { mode, .. } = command
                        && mode != request.mode
                    {
                        return Err("This suite freezes the shared sort mode per sequence".into());
                    }
                    pane.apply_event(&command, &sorter)?;
                }
            }
            states.push(snapshot(show_hidden, &panes));
        }
        println!(
            "{}",
            serde_json::to_string(&Response {
                id: request.id,
                status: "ok",
                states
            })?
        );
    }
    Ok(())
}
