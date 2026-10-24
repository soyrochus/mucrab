// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
use mucrab_core::{BatchRenameOptions, BatchRenamePlan, apply_batch_rename};
use serde::Deserialize;
use std::{
    io::{self, BufRead},
    path::PathBuf,
};
#[derive(Deserialize)]
struct Request {
    id: String,
    directory: PathBuf,
    names: Vec<String>,
    options: Vec<BatchRenameOptions>,
    apply: bool,
}
pub fn adapter() -> Result<(), Box<dyn std::error::Error>> {
    for line in io::stdin().lock().lines() {
        let request: Request = serde_json::from_str(&line?)?;
        let mut plan = BatchRenamePlan::new(
            &request.directory,
            &request.names,
            &BatchRenameOptions::default(),
        )?;
        let mut previews = vec![plan.preview().clone()];
        for options in request.options {
            plan = BatchRenamePlan::new(&request.directory, &request.names, &options)?;
            previews.push(plan.preview().clone());
        }
        let unchanged_names: Vec<_> = plan
            .preview()
            .rows
            .iter()
            .filter(|row| row.name == row.new_name)
            .map(|row| row.name.clone())
            .collect();
        let applied =
            request.apply && plan.preview().rename_enabled && plan.preview().changed_count > 0;
        let outcomes = if applied {
            apply_batch_rename(&plan)?.outcomes
        } else {
            Vec::new()
        };
        println!(
            "{}",
            serde_json::json!({"id":request.id,"status":"ok","previews":previews,"applied":applied,"unchanged_names":unchanged_names,"outcomes":outcomes})
        );
    }
    Ok(())
}
