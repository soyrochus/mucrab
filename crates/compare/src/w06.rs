// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
use mucrab_core::{CopyPolicy, ZipCatalog, ZipExtractPlan, extract_zip};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    io::{self, BufRead},
    path::PathBuf,
};

#[derive(Deserialize)]
struct Request {
    id: String,
    archive: PathBuf,
    destination: PathBuf,
    prefix: String,
    list_paths: Vec<String>,
    selected: Option<Vec<String>>,
    policy: CopyPolicy,
    extract: bool,
}
pub fn adapter() -> Result<(), Box<dyn std::error::Error>> {
    for line in io::stdin().lock().lines() {
        let request: Request = serde_json::from_str(&line?)?;
        let result = match ZipCatalog::open(&request.archive) {
            Err(_) => json!({"id": request.id, "status": "error", "error": "archive_unavailable"}),
            Ok(catalog) => {
                let listings: Vec<Value> = request
                    .list_paths
                    .iter()
                    .map(|path| match catalog.list(path) {
                        Ok(entries) => json!({"path":path,"status":"ok","entries":entries}),
                        Err(_) => json!({"path":path,"status":"error","entries":[]}),
                    })
                    .collect();
                let success = if request.extract {
                    let plan = ZipExtractPlan::new(
                        &request.archive,
                        &request.prefix,
                        request.selected.as_deref(),
                        &request.destination,
                        request.policy,
                    )?;
                    extract_zip(&plan)?.success
                } else {
                    false
                };
                json!({"id":request.id,"status":"ok","listings":listings,"extracted":request.extract,"success":success})
            }
        };
        println!("{}", serde_json::to_string(&result)?);
    }
    Ok(())
}
