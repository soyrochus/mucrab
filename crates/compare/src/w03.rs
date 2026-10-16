// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
use mucrab_core::{CopyRequest, copy_entries};
use serde::Deserialize;
use std::io::{self, BufRead};

#[derive(Deserialize)]
struct Request {
    id: String,
    #[serde(flatten)]
    copy: CopyRequest,
}
pub fn adapter() -> Result<(), Box<dyn std::error::Error>> {
    for line in io::stdin().lock().lines() {
        let request: Request = serde_json::from_str(&line?)?;
        let report = copy_entries(&request.copy)?;
        println!(
            "{}",
            serde_json::json!({"id":request.id,"status":"ok","outcomes":report.outcomes,"state":"RUNNING"})
        );
    }
    Ok(())
}
