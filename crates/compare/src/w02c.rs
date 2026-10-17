// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

use mucrab_core::{
    BrowserPane, FileEntry, NameSorter, PaneEvent, SelectionSnapshot, SortCriterion, SortMode,
    SortOptions, list_directory,
};
use serde::{Deserialize, Serialize};
use std::{
    io::{self, BufRead},
    path::PathBuf,
};

#[derive(Deserialize)]
struct Request {
    id: String,
    operation: String,
    path: PathBuf,
    mode: SortMode,
    parent: bool,
    #[serde(default)]
    pairs_only: bool,
    entries: Option<Vec<FileEntry>>,
    events: Vec<Event>,
}

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum Event {
    Sort {
        criterion: SortCriterion,
        ascending: bool,
        directories_first: bool,
    },
    Focus {
        row: usize,
    },
    Mark {
        row: usize,
    },
}

#[derive(Serialize)]
struct State {
    #[serde(flatten)]
    options: SortOptions,
    pairs: Vec<Vec<i8>>,
    order: Option<Vec<usize>>,
    selection: Option<SelectionSnapshot>,
}

#[derive(Serialize)]
struct Response {
    id: String,
    status: &'static str,
    entries: Vec<FileEntry>,
    states: Vec<State>,
}

fn state(
    sorter: &NameSorter,
    entries: &[FileEntry],
    options: SortOptions,
    pane: Option<&BrowserPane>,
) -> State {
    State {
        options,
        pairs: entries
            .iter()
            .map(|left| {
                entries
                    .iter()
                    .map(|right| match sorter.compare(left, right, options) {
                        std::cmp::Ordering::Less => -1,
                        std::cmp::Ordering::Equal => 0,
                        std::cmp::Ordering::Greater => 1,
                    })
                    .collect()
            })
            .collect(),
        order: pane.map(|p| p.order.clone()),
        selection: pane.map(|p| p.selection.snapshot(&p.entries, &p.order)),
    }
}

pub fn adapter() -> Result<(), Box<dyn std::error::Error>> {
    let sorter = NameSorter::en_us()?;
    for line in io::stdin().lock().lines() {
        let request: Request = serde_json::from_str(&line?)?;
        if request.operation == "enumerate" {
            let entries = list_directory(&request.path)?;
            println!(
                "{}",
                serde_json::json!({"id": request.id, "status":"ok", "entries": entries})
            );
            continue;
        }
        if request.operation != "run" {
            return Err("Unknown metadata operation".into());
        }
        let entries = request.entries.ok_or("Missing metadata entries")?;
        for entry in &entries {
            let metadata = entry.metadata.as_ref().ok_or("Missing metadata")?;
            if !request.pairs_only && !metadata.is_sortable() {
                return Err("Metadata outside sortable range; use pair-only coverage".into());
            }
        }
        let mut options = SortOptions {
            mode: request.mode,
            ..Default::default()
        };
        let mut pane = if request.pairs_only {
            None
        } else {
            let mut pane = BrowserPane {
                options,
                show_hidden: true,
                ..Default::default()
            };
            pane.load_listing(request.path, entries.clone(), request.parent, &sorter);
            Some(pane)
        };
        let mut states = vec![state(&sorter, &entries, options, pane.as_ref())];
        for event in request.events {
            match event {
                Event::Sort {
                    criterion,
                    ascending,
                    directories_first,
                } => {
                    options = SortOptions {
                        criterion,
                        ascending,
                        directories_first,
                        mode: request.mode,
                    };
                    if let Some(pane) = &mut pane {
                        pane.apply_event(
                            &PaneEvent::Sort {
                                criterion,
                                ascending,
                                directories_first,
                                mode: request.mode,
                            },
                            &sorter,
                        )?;
                    }
                }
                Event::Focus { row } | Event::Mark { row } => {
                    let pane = pane
                        .as_mut()
                        .ok_or("Pair-only cases cannot claim selection")?;
                    pane.apply_event(&PaneEvent::Focus { row }, &sorter)?;
                    if matches!(event, Event::Mark { .. }) {
                        pane.selection.set_marked(row, true, &pane.order);
                    }
                }
            }
            states.push(state(&sorter, &entries, options, pane.as_ref()));
        }
        println!(
            "{}",
            serde_json::to_string(&Response {
                id: request.id,
                status: "ok",
                entries,
                states
            })?
        );
    }
    Ok(())
}
