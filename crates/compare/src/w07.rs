// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
use mucrab_core::*;
use serde::Deserialize;
use std::{
    io::{self, BufRead},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

#[derive(Deserialize)]
struct Request {
    id: String,
    action: String,
    mode: String,
    source_directory: PathBuf,
    destination_directory: PathBuf,
    names: Vec<String>,
    cancel_bytes: u64,
}

pub fn adapter() -> Result<(), Box<dyn std::error::Error>> {
    for line in io::stdin().lock().lines() {
        let request: Request = serde_json::from_str(&line?)?;
        let states = Arc::new(Mutex::new(vec![
            "NOT_STARTED".to_owned(),
            "RUNNING".to_owned(),
        ]));
        let observer_states = states.clone();
        let paused = Arc::new(AtomicBool::new(false));
        let did_pause = paused.clone();
        let mode = request.mode.clone();
        let threshold = request.cancel_bytes;
        let control = JobControl::with_observer(move |control, p| {
            let state = match p.state {
                JobState::Cancelling | JobState::Cancelled => "INTERRUPTED",
                JobState::Finished | JobState::Failed => "FINISHED",
                JobState::Running => "RUNNING",
                JobState::Paused => "PAUSED",
            };
            {
                let mut states = observer_states.lock().unwrap();
                if states.last().map(String::as_str) != Some(state) {
                    states.push(state.to_owned());
                }
            }
            if p.state != JobState::Running {
                return;
            }
            if mode == "pause"
                && p.current_file_bytes >= 65536
                && !did_pause.swap(true, Ordering::SeqCst)
            {
                control.set_paused(true);
            } else if ["stream", "nested", "pause"].contains(&mode.as_str())
                && p.current_file_bytes >= threshold
            {
                control.cancel();
            }
        });
        let wake_control = control.clone();
        let resumer = if request.mode == "pause" {
            Some(std::thread::spawn(move || {
                for _ in 0..1000 {
                    if wake_control.snapshot().state == JobState::Paused {
                        let before = wake_control.snapshot().transferred_bytes;
                        std::thread::sleep(Duration::from_millis(30));
                        let stable = before == wake_control.snapshot().transferred_bytes;
                        wake_control.set_paused(false);
                        return stable;
                    }
                    if wake_control.snapshot().state.is_terminal() {
                        return false;
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
                false
            }))
        } else {
            None
        };
        if request.mode == "before" {
            control.cancel();
        }
        let mut copy = CopyRequest {
            source_directory: request.source_directory.clone(),
            destination_directory: request.destination_directory.clone(),
            names: request.names.clone(),
            new_name: None,
            policy: if request.mode == "skip" {
                CopyPolicy::Skip
            } else {
                CopyPolicy::Overwrite
            },
        };
        let outcomes: Vec<CopyOutcome> = match request.action.as_str() {
            "copy" => copy_entries_with_control(&copy, &control)?.outcomes,
            "move" | "rename" => {
                let rename = request.action == "rename";
                if rename {
                    copy.new_name = Some("renamed.bin".into());
                }
                move_entries_with_control(
                    &MoveRequest {
                        transfer: copy,
                        rename_mode: rename,
                    },
                    &control,
                )?
                .outcomes
            }
            "batch" => {
                let plan = BatchRenamePlan::new(
                    &copy.source_directory,
                    &copy.names,
                    &BatchRenameOptions {
                        mask: "new_[N].[E]".into(),
                        ..Default::default()
                    },
                )?;
                apply_batch_rename_with_control(&plan, &control)?
                    .outcomes
                    .into_iter()
                    .map(|o| CopyOutcome {
                        name: o.name,
                        success: o.success,
                    })
                    .collect()
            }
            "extract" => {
                let plan = ZipExtractPlan::new(
                    &copy.source_directory.join(&copy.names[0]),
                    "",
                    None,
                    &copy.destination_directory,
                    copy.policy,
                )?;
                let report = extract_zip_with_control(&plan, &control)?;
                copy.names
                    .iter()
                    .map(|name| CopyOutcome {
                        name: name.clone(),
                        success: report.success,
                    })
                    .collect()
            }
            _ => return Err("Unknown W07 job action".into()),
        };
        control.finish(false);
        let pause_stable = resumer.is_none_or(|worker| worker.join().unwrap());
        let marked: Vec<_> = request
            .names
            .iter()
            .filter(|name| !outcomes.iter().any(|o| &o.name == *name && o.success))
            .collect();
        println!(
            "{}",
            serde_json::json!({"id":request.id,"state":if control.snapshot().state==JobState::Cancelled{"INTERRUPTED"}else{"FINISHED"},"transitions":states.lock().unwrap().join(","),"pause_stable":pause_stable,"marked":marked,"progress":control.snapshot()})
        );
    }
    Ok(())
}
