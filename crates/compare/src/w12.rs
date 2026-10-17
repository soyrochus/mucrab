// SPDX-License-Identifier: GPL-3.0-only
use mucrab_core::{SearchPlan, SearchQuery};
use serde::Deserialize;
use serde_json::json;
use std::{
    io::{self, BufRead},
    path::PathBuf,
};
#[derive(Deserialize)]
struct Request {
    id: String,
    root: PathBuf,
    name: String,
    pattern: String,
    case_sensitive: bool,
}
pub fn adapter() -> Result<(), Box<dyn std::error::Error>> {
    for line in io::stdin().lock().lines() {
        let r: Request = serde_json::from_str(&line?)?;
        let plan = SearchPlan::new(SearchQuery {
            root: r.root,
            pattern: r.pattern,
            case_sensitive: r.case_sensitive,
            ..Default::default()
        })?;
        println!(
            "{}",
            json!({"id":r.id,"matches":plan.matches_name(&r.name)})
        );
    }
    Ok(())
}
