// SPDX-License-Identifier: GPL-3.0-only
use mucrab_core::*;
use serde::Deserialize;
use serde_json::json;
use std::{
    io::{self, BufRead},
    path::PathBuf,
};
#[derive(Deserialize)]
struct Request {
    id: String,
    path: PathBuf,
    new_mode: u32,
    new_mtime_ms: i64,
}
pub fn adapter() -> Result<(), Box<dyn std::error::Error>> {
    for line in io::stdin().lock().lines() {
        let r: Request = serde_json::from_str(&line?)?;
        let s = PropertySnapshot::capture(EntryTarget::Local(r.path))?;
        let accessed_ns = s
            .accessed
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos());
        let before = json!({"kind":format!("{:?}",s.kind),"size":s.size,"mode":s.mode,"uid":s.uid,"gid":s.gid,"links":s.links,"accessed_ns":accessed_ns});
        let result = s
            .metadata_plan(Some(r.new_mode), time_from_millis(r.new_mtime_ms))
            .and_then(|p| p.apply(&JobControl::default()));
        let (steps, error) = match result {
            Ok(report) => (
                report.steps.iter().map(|s| s.changed).collect::<Vec<_>>(),
                None,
            ),
            Err(e) => (vec![false, false], Some(e.to_string())),
        };
        println!(
            "{}",
            json!({"id":r.id,"before":before,"steps":steps,"error":error})
        );
    }
    Ok(())
}
