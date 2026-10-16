// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

use mucrab_core::{
    BrowserPane, FileEntry, FileSelection, NameSorter, PaneEvent, SelectionSnapshot,
};
use serde::{Deserialize, Serialize};
use std::{
    io::{self, BufRead},
    path::PathBuf,
};

#[derive(Deserialize)]
struct Request {
    id: String,
    path: PathBuf,
    names: Vec<String>,
    parent: bool,
    events: Vec<PaneEvent>,
}

#[derive(Serialize)]
struct Response {
    id: String,
    status: &'static str,
    states: Vec<SelectionSnapshot>,
}

pub fn adapter() -> Result<(), Box<dyn std::error::Error>> {
    let sorter = NameSorter::en_us()?;
    for line in io::stdin().lock().lines() {
        let request: Request = serde_json::from_str(&line?)?;
        let entries = request
            .names
            .iter()
            .map(|name| {
                let path = request.path.join(name);
                let metadata = path.symlink_metadata()?;
                Ok(FileEntry {
                    name: name.clone(),
                    is_directory: path.is_dir(),
                    is_symlink: metadata.is_symlink(),
                })
            })
            .collect::<Result<Vec<_>, std::io::Error>>()?;
        let mut pane = BrowserPane {
            show_hidden: true,
            order: (0..entries.len()).collect(),
            selection: FileSelection::new(entries.len(), request.parent),
            entries,
            ..Default::default()
        };
        pane.order = sorter.sorted_indices(&pane.entries, pane.options);
        let mut states = vec![pane.selection.snapshot(&pane.entries, &pane.order)];
        for event in request.events {
            pane.apply_event(&event, &sorter)?;
            states.push(pane.selection.snapshot(&pane.entries, &pane.order));
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
