// SPDX-License-Identifier: GPL-3.0-only
//! Explicitly fingerprint-pinned, test-only disposable-server adapter.
use mucrab_core::{CopyPolicy, JobControl, sftp::*};
use serde::Deserialize;
use serde_json::json;
use std::{
    io::{self, BufRead},
    path::PathBuf,
    sync::Arc,
};
#[derive(Deserialize)]
struct Request {
    id: String,
    host: String,
    port: u16,
    user: String,
    password: String,
    fingerprint: String,
    path: String,
    operation: String,
    local: Option<PathBuf>,
    #[serde(default)]
    overwrite: bool,
}
pub fn adapter() -> Result<(), Box<dyn std::error::Error>> {
    let mut cached: Option<Arc<RemoteSession>> = None;
    for line in io::stdin().lock().lines() {
        let r: Request = serde_json::from_str(&line?)?;
        let c = JobControl::default();
        if cached
            .as_ref()
            .is_none_or(|s| s.profile().port != r.port || s.profile().host != r.host)
        {
            let p = Profile {
                id: "fixture".into(),
                label: "Disposable test".into(),
                host: r.host.clone(),
                port: r.port,
                user: r.user,
                authentication: Authentication::Password,
                directory: "/".into(),
                remembered: false,
                credential_context: None,
            };
            let h = Handshake::connect(&p, &c)?;
            if h.identity().fingerprint != r.fingerprint {
                return Err("Test server fingerprint mismatch".into());
            }
            let identity = h.identity().clone();
            cached = Some(h.authenticate(p, &identity, &r.password, &c)?);
        }
        let session = cached.as_ref().expect("connected").clone();
        let location = RemoteLocation::new("fixture", &r.path)?;
        let response = match r.operation.as_str() {
            "list" => match session.list(&location, &c) {
                Ok(entries) => {
                    json!({"id":r.id,"entries":entries.into_iter().map(|e|json!({"name":e.name,"directory":e.is_directory,"link":e.is_symlink,"size":e.metadata.as_ref().map(|m|m.size),"modified_ms":e.metadata.as_ref().map(|m|m.modified_ms)})).collect::<Vec<_>>()})
                }
                Err(e) => json!({"id":r.id,"error":e.to_string()}),
            },
            "upload" | "download" => {
                let path = r.local.ok_or("Local path required")?;
                let name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .ok_or("Local leaf required")?
                    .to_owned();
                if location.path().rsplit('/').next() != Some(&name) {
                    return Err("Equivalent test leaf names required".into());
                }
                let local =
                    Endpoint::Local(path.parent().ok_or("Local parent required")?.to_owned());
                let remote = Endpoint::Remote {
                    session,
                    location: location.parent(),
                };
                let (source, destination) = if r.operation == "upload" {
                    (local, remote)
                } else {
                    (remote, local)
                };
                match TransferPlan::capture(
                    source,
                    destination,
                    vec![name],
                    if r.overwrite {
                        CopyPolicy::Overwrite
                    } else {
                        CopyPolicy::Skip
                    },
                    &c,
                )
                .and_then(|p| p.apply(&c))
                {
                    Ok(report) => {
                        json!({"id":r.id,"ok":report.issues.iter().all(|i|i.kind==mucrab_core::CopyIssueKind::Skipped),"issues":report.issues.iter().map(|i|i.message.clone()).collect::<Vec<_>>()})
                    }
                    Err(e) => json!({"id":r.id,"error":e.to_string()}),
                }
            }
            _ => return Err("Unknown SFTP test operation".into()),
        };
        println!("{}", serde_json::to_string(&response)?);
    }
    Ok(())
}
