// SPDX-License-Identifier: GPL-3.0-only
use mucrab_core::{PreviewContent, decode_preview};
use serde::Deserialize;
use serde_json::json;
use std::{
    fs,
    io::{self, BufRead},
    path::PathBuf,
};
#[derive(Deserialize)]
struct Request {
    id: String,
    path: PathBuf,
}
pub fn adapter() -> Result<(), Box<dyn std::error::Error>> {
    for line in io::stdin().lock().lines() {
        let r: Request = serde_json::from_str(&line?)?;
        let response = match decode_preview(&fs::read(&r.path)?, 16384) {
            Ok(PreviewContent::Text(t)) => {
                json!({"id":r.id,"text":t.text,"encoding":t.encoding,"lines":t.lines.len()})
            }
            Ok(PreviewContent::Image(i)) => {
                json!({"id":r.id,"width":i.width,"height":i.height,"rgba":i.rgba})
            }
            Err(e) => json!({"id":r.id,"error":e.to_string()}),
        };
        println!("{response}");
    }
    Ok(())
}
