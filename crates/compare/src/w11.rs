// SPDX-License-Identifier: GPL-3.0-only
use mucrab_core::{Bookmark, BrowserPane, NameSorter, PaneLocation, TabStack};
use serde_json::json;
use std::path::PathBuf;
pub fn probe(args: &[std::ffi::OsString]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() != 2 {
        return Err("W11 probe needs root and child".into());
    }
    let root = PathBuf::from(&args[0]);
    let child = PathBuf::from(&args[1]);
    let sorter = NameSorter::en_us()?;
    let mut p = BrowserPane::default();
    if !p.open(&root, &sorter) || !p.open(&child, &sorter) {
        return Err("Probe paths unavailable".into());
    }
    let mut tabs = TabStack::new(p);
    let origin = tabs.active().id;
    tabs.duplicate(&sorter)?;
    let copied = tabs.active().pane.history_len();
    let same = tabs.active().pane.location() == Some(&child);
    tabs.active_mut().pane.open(&root, &sorter);
    tabs.select(origin);
    let b = Bookmark {
        id: 1,
        label: "Original".into(),
        location: PaneLocation::Local(root),
    };
    let mut copy = b.clone();
    copy.label = "Renamed".into();
    println!(
        "{}",
        json!({"same_location":same,"independent_history":tabs.active().pane.history_len()==2,"duplicate_history_length":copied,"bookmark_clone_independent":b.label=="Original"})
    );
    Ok(())
}
