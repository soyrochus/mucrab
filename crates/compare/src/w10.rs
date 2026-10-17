// SPDX-License-Identifier: GPL-3.0-only
use mucrab_core::*;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    io::{self, BufRead},
    path::PathBuf,
};
#[derive(Deserialize)]
struct Request {
    id: String,
    operation: String,
    #[serde(default)]
    directory: PathBuf,
    #[serde(default)]
    destination: PathBuf,
    #[serde(default)]
    paths: Vec<PathBuf>,
    #[serde(default)]
    query: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    rewind: Option<usize>,
}
pub fn adapter() -> Result<(), Box<dyn std::error::Error>> {
    let sorter = NameSorter::en_us()?;
    for line in io::stdin().lock().lines() {
        let r: Request = serde_json::from_str(&line?)?;
        let response: Value = match r.operation.as_str() {
            "mkdir" => {
                let pin = PinnedDirectory::new(&r.directory)?;
                let outcomes = r
                    .paths
                    .iter()
                    .map(|p| {
                        pin.create_folder(
                            p.file_name().unwrap().to_str().unwrap(),
                            &JobControl::default(),
                        )
                        .unwrap_or(false)
                    })
                    .collect::<Vec<_>>();
                json!({"id":r.id,"outcomes":outcomes})
            }
            "delete" => {
                let report = DeletePlan::new(&r.paths, &[])?.apply(&JobControl::default(), None)?;
                json!({"id":r.id,"outcomes":report.outcomes.iter().map(|o|o.success).collect::<Vec<_>>()})
            }
            "copy" => {
                let report = PastePlan::new(&r.paths, &r.destination)?
                    .apply(CopyPolicy::Skip, &JobControl::default())?;
                json!({"id":r.id,"outcomes":report.outcomes.iter().map(|o|o.success).collect::<Vec<_>>()})
            }
            "quick" => {
                let mut p = BrowserPane::default();
                p.load_listing(
                    PathBuf::from("/synthetic"),
                    vec![FileEntry {
                        name: r.name,
                        ..Default::default()
                    }],
                    false,
                    &sorter,
                );
                json!({"id":r.id,"matches":p.find_prefix(&r.query,None)})
            }
            "history" => {
                let mut p = BrowserPane::default();
                for path in &r.paths {
                    if !p.open(path, &sorter) {
                        return Err("History fixture unavailable".into());
                    }
                }
                if let Some(index) = r.rewind {
                    if index >= p.history_len() {
                        return Err("Invalid history rewind".into());
                    }
                    for _ in 0..p.history_len() - 1 - index {
                        if !p.traverse_history(false, &sorter) {
                            return Err("History rewind unavailable".into());
                        }
                    }
                    if !p.open(&r.destination, &sorter) {
                        return Err("History branch unavailable".into());
                    }
                }
                json!({"id":r.id,"length":p.history_len(),"cursor":p.history_len()-1,"back":p.can_back(),"forward":p.can_forward()})
            }
            _ => return Err("Unknown W10 operation".into()),
        };
        println!("{}", response);
    }
    Ok(())
}
