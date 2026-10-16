// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
use mucrab_core::{MoveRequest, move_entries};
use serde::Deserialize;
use std::io::{self, BufRead};
#[derive(Deserialize)]
struct Request {
    id: String,
    #[serde(flatten)]
    operation: MoveRequest,
}
pub fn adapter() -> Result<(), Box<dyn std::error::Error>> {
    for line in io::stdin().lock().lines() {
        let request: Request = serde_json::from_str(&line?)?;
        let report = move_entries(&request.operation)?;
        println!(
            "{}",
            serde_json::json!({"id":request.id,"status":"ok","state":"RUNNING","outcomes":report.outcomes})
        );
    }
    Ok(())
}
