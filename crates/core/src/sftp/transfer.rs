// SPDX-License-Identifier: GPL-3.0-only
use super::*;
use crate::{CopyIssue, CopyIssueKind, CopyOutcome, CopyPolicy, CopyReport, PinnedDirectory};
use std::io::{Read, Write};
#[derive(Clone)]
pub enum Endpoint {
    Local(PathBuf),
    Remote {
        session: Arc<RemoteSession>,
        location: RemoteLocation,
    },
}
impl Endpoint {
    pub fn description(&self) -> String {
        match self {
            Self::Local(p) => p.display().to_string(),
            Self::Remote { location, .. } => location.display(),
        }
    }
    pub fn profile(&self) -> Option<&str> {
        match self {
            Self::Remote { location, .. } => Some(&location.profile),
            _ => None,
        }
    }
}
#[derive(Clone)]
struct Pin {
    name: String,
    local: Option<crate::properties::EntryIdentity>,
    remote: Option<wire::Attributes>,
    destination_local: Option<crate::properties::EntryIdentity>,
    destination_remote: Option<wire::Attributes>,
}
#[derive(Clone)]
pub struct TransferPlan {
    pub source: Endpoint,
    pub destination: Endpoint,
    pub policy: CopyPolicy,
    pins: Vec<Pin>,
    source_directory: Option<PinnedDirectory>,
    destination_directory: Option<PinnedDirectory>,
}
impl TransferPlan {
    /// Network pin capture belongs to a background preparation worker.
    pub fn capture(
        source: Endpoint,
        destination: Endpoint,
        names: Vec<String>,
        policy: CopyPolicy,
        c: &JobControl,
    ) -> io::Result<Self> {
        if !matches!(policy, CopyPolicy::Skip | CopyPolicy::Overwrite) {
            return Err(error("SFTP supports Skip and Overwrite only"));
        }
        if names.is_empty()
            || names.len() > 20000
            || matches!(
                (&source, &destination),
                (Endpoint::Local(_), Endpoint::Local(_))
                    | (Endpoint::Remote { .. }, Endpoint::Remote { .. })
            )
        {
            return Err(error("One local and one SFTP endpoint are required"));
        }
        let (local, remote) = match (&source, &destination) {
            (Endpoint::Local(p), Endpoint::Remote { session, location }) => {
                (p, (session, location))
            }
            (Endpoint::Remote { session, location }, Endpoint::Local(p)) => {
                (p, (session, location))
            }
            _ => unreachable!(),
        };
        if remote.1.profile != remote.0.profile.id
            || RemoteLocation::new(&remote.1.profile, &remote.1.path)? != *remote.1
        {
            return Err(error("Remote endpoint/profile mismatch"));
        }
        let pin = PinnedDirectory::new(local)?;
        let mut client = remote.0.client(c)?;
        client.ensure_directory(&remote.1.path)?;
        let mut pins = vec![];
        let mut seen = std::collections::HashSet::new();
        for name in names {
            check(c)?;
            if !crate::is_valid_leaf_name(&name) || !seen.insert(name.clone()) {
                return Err(error("Invalid or duplicate transfer name"));
            }
            let (local, remote) = match &source {
                Endpoint::Local(p) => (
                    Some(crate::properties::EntryIdentity::read(&p.join(&name))?),
                    None,
                ),
                Endpoint::Remote { location, .. } => (
                    None,
                    Some(
                        client
                            .lstat(&location.child(&name)?.path)?
                            .ok_or_else(|| error("Remote source disappeared"))?,
                    ),
                ),
            };
            let destination_local = match &destination {
                Endpoint::Local(dir) => {
                    match crate::properties::EntryIdentity::read(&dir.join(&name)) {
                        Ok(id) => Some(id),
                        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
                        Err(e) => return Err(e),
                    }
                }
                _ => None,
            };
            let destination_remote = match &destination {
                Endpoint::Remote { location, .. } => client.lstat(&location.child(&name)?.path)?,
                _ => None,
            };
            pins.push(Pin {
                name,
                local,
                remote,
                destination_local,
                destination_remote,
            });
        }
        let upload = matches!(source, Endpoint::Local(_));
        Ok(Self {
            source,
            destination,
            policy,
            pins,
            source_directory: upload.then_some(pin.clone()),
            destination_directory: (!upload).then_some(pin),
        })
    }
    pub fn names(&self) -> Vec<String> {
        self.pins.iter().map(|p| p.name.clone()).collect()
    }
    pub fn apply(&self, c: &JobControl) -> io::Result<CopyReport> {
        if let Some(p) = &self.source_directory {
            p.validate()?;
        }
        if let Some(p) = &self.destination_directory {
            p.validate()?;
        }
        let (session, remote, local, upload) = match (&self.source, &self.destination) {
            (Endpoint::Local(p), Endpoint::Remote { session, location }) => {
                (session, location, p, true)
            }
            (Endpoint::Remote { session, location }, Endpoint::Local(p)) => {
                (session, location, p, false)
            }
            _ => return Err(error("Invalid mixed transfer endpoints")),
        };
        let mut client = session.client(c)?;
        client.ensure_directory(&remote.path)?;
        let mut report = CopyReport::default();
        c.total(self.pins.len());
        let mut visited = 0;
        for pin in &self.pins {
            if !c.entry(
                &PathBuf::from(format!("{}/{}", remote.display(), pin.name)),
                None,
            ) {
                report.cancelled = true;
                break;
            }
            let result = (|| {
                if let Some(id) = &pin.local {
                    id.validate(&local.join(&pin.name))?;
                }
                if let Some(id) = &pin.remote
                    && client.lstat(&remote.child(&pin.name)?.path)?.as_ref() != Some(id)
                {
                    return Err(error("Remote source changed after review"));
                }
                if upload {
                    if client.lstat(&remote.child(&pin.name)?.path)? != pin.destination_remote {
                        return Err(error("Remote destination changed after review"));
                    }
                } else {
                    let current =
                        match crate::properties::EntryIdentity::read(&local.join(&pin.name)) {
                            Ok(id) => Some(id),
                            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
                            Err(e) => return Err(e),
                        };
                    if current != pin.destination_local {
                        return Err(error("Local destination changed after review"));
                    }
                }
                let mut walk = Walk {
                    client: &mut client,
                    control: c,
                    visited: &mut visited,
                    policy: self.policy,
                    upload,
                };
                walk.copy(&local.join(&pin.name), &remote.child(&pin.name)?.path, 0)
            })();
            let success = matches!(result, Ok(true));
            report.outcomes.push(CopyOutcome {
                name: pin.name.clone(),
                success,
            });
            match result {
                Ok(true) => {}
                Ok(false) => report.issues.push(CopyIssue {
                    path: PathBuf::from(remote.child(&pin.name)?.display()),
                    kind: CopyIssueKind::Skipped,
                    message: "Existing destination skipped; unfinished roots remain marked".into(),
                }),
                Err(e) => {
                    report.issues.push(CopyIssue {
                        path: PathBuf::from(remote.child(&pin.name)?.display()),
                        kind: if c.is_cancelled() {
                            CopyIssueKind::Cancelled
                        } else {
                            CopyIssueKind::Failed
                        },
                        message: e.to_string(),
                    });
                    if c.is_cancelled() {
                        report.cancelled = true;
                    }
                    break;
                }
            }
            c.completed();
        }
        report.cancelled |= c.is_cancelled();
        Ok(report)
    }
}
struct Walk<'a> {
    client: &'a mut wire::Client,
    control: &'a JobControl,
    visited: &'a mut usize,
    policy: CopyPolicy,
    upload: bool,
}
impl Walk<'_> {
    fn copy(&mut self, local: &Path, remote: &str, depth: usize) -> io::Result<bool> {
        check(self.control)?;
        *self.visited += 1;
        if depth > 256 || *self.visited > 100000 {
            return Err(error("SFTP tree limit reached; earlier work retained"));
        }
        let parent = remote
            .rsplit_once('/')
            .map(|(p, _)| if p.is_empty() { "/" } else { p })
            .ok_or_else(|| error("Remote parent required"))?;
        self.client.ensure_directory(parent)?;
        let remote_meta = self.client.lstat(remote)?;
        let local_meta = match fs::symlink_metadata(local) {
            Ok(m) => Some(m),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(e),
        };
        if local_meta
            .as_ref()
            .is_some_and(|m| m.is_symlink() || (!m.is_dir() && !m.is_file()))
            || remote_meta
                .as_ref()
                .is_some_and(|m| m.link() || (!m.directory() && !m.file()))
        {
            return Err(error(
                "Links and special files are unsupported in SFTP transfers",
            ));
        }
        let directory = if self.upload {
            local_meta
                .as_ref()
                .ok_or_else(|| error("Local source missing"))?
                .is_dir()
        } else {
            remote_meta
                .as_ref()
                .ok_or_else(|| error("Remote source missing"))?
                .directory()
        };
        if directory {
            if self.upload {
                match &remote_meta {
                    Some(m) if !m.directory() => {
                        return Err(error("Remote directory conflicts with file"));
                    }
                    Some(_) => {}
                    None => self.client.mkdir(remote)?,
                };
            } else {
                match &local_meta {
                    Some(m) if !m.is_dir() => {
                        return Err(error("Local directory conflicts with file"));
                    }
                    Some(_) => {}
                    None => fs::create_dir(local)?,
                };
            }
            let local_pin = PinnedDirectory::new(local)?;
            let children: Vec<String> = if self.upload {
                fs::read_dir(local)?
                    .take(20001)
                    .map(|e| {
                        e.and_then(|e| {
                            e.file_name()
                                .into_string()
                                .map_err(|_| error("Non-Unicode local filename unsupported"))
                        })
                    })
                    .collect::<io::Result<_>>()?
            } else {
                self.client
                    .list(remote, self.control)?
                    .into_iter()
                    .map(|(n, _)| n)
                    .collect()
            };
            if children.len() > 20000 {
                return Err(error("Transfer directory exceeds 20000 children"));
            }
            let mut complete = true;
            for name in children {
                if !crate::is_valid_leaf_name(&name) || name.chars().any(char::is_control) {
                    return Err(error("Invalid transfer leaf name"));
                }
                local_pin.validate()?;
                complete &= self.copy(
                    &local.join(&name),
                    &format!("{}/{name}", remote.trim_end_matches('/')),
                    depth + 1,
                )?;
            }
            return Ok(complete);
        }
        let existing = if self.upload {
            remote_meta.is_some()
        } else {
            local_meta.is_some()
        };
        if existing && self.policy == CopyPolicy::Skip {
            return Ok(false);
        }
        if existing
            && (if self.upload {
                remote_meta.as_ref().is_some_and(|m| !m.file())
            } else {
                local_meta.as_ref().is_some_and(|m| !m.is_file())
            })
        {
            return Err(error("File destination conflicts with directory"));
        }
        if self.upload && !self.client.posix_rename && self.policy == CopyPolicy::Overwrite {
            return Err(error(
                "Server does not advertise POSIX rename; overwrite refused before staging",
            ));
        }
        if self.upload {
            self.upload_file(local, remote, remote_meta.as_ref())?;
        } else {
            self.download_file(
                remote,
                local,
                remote_meta
                    .as_ref()
                    .ok_or_else(|| error("Remote source missing"))?,
                local_meta.as_ref(),
            )?;
        }
        Ok(true)
    }
    fn upload_file(
        &mut self,
        local: &Path,
        remote: &str,
        before_destination: Option<&wire::Attributes>,
    ) -> io::Result<()> {
        let identity = crate::properties::EntryIdentity::read(local)?;
        let mut input = fs::File::open(local)?;
        if crate::properties::EntryIdentity::from_metadata(local, &input.metadata()?)? != identity {
            return Err(error("Local source changed while opening"));
        }
        let len = input.metadata()?.len();
        self.control.entry(local, Some(len));
        let stage = format!(
            "{}.mucrab-stage-{}-{}",
            remote,
            std::process::id(),
            next_id()
        );
        let handle = self.client.open(&stage, true)?;
        let result = (|| {
            let mut offset = 0;
            let mut buffer = [0; 32768];
            loop {
                check(self.control)?;
                let n = input.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                offset += n as u64;
                if offset > len {
                    return Err(error("Local source grew during upload"));
                }
                self.client
                    .write(&handle, offset - n as u64, &buffer[..n])?;
                self.control.advance(n as u64);
            }
            if offset != len {
                return Err(error("Local source length changed during upload"));
            }
            identity.validate(local)?;
            Ok(())
        })();
        let closed = self.client.close(&handle);
        let result = result.and(closed).and_then(|_| {
            check(self.control)?;
            if self.client.lstat(&stage)?.and_then(|a| a.size) != Some(len) {
                return Err(error("Remote staged length mismatch"));
            }
            if self.client.lstat(remote)?.as_ref() != before_destination {
                return Err(error("Remote destination changed before publication"));
            }
            self.client
                .publish(&stage, remote, self.policy == CopyPolicy::Overwrite)
        });
        result.map_err(|e| error(format!("{e}. Owned remote stage may remain: {stage}")))
    }
    fn download_file(
        &mut self,
        remote: &str,
        local: &Path,
        source: &wire::Attributes,
        before_destination: Option<&fs::Metadata>,
    ) -> io::Result<()> {
        let len = source
            .size
            .ok_or_else(|| error("Remote source size unavailable"))?;
        self.control.entry(&PathBuf::from(remote), Some(len));
        let parent = local
            .parent()
            .ok_or_else(|| error("Local destination parent required"))?;
        let directory = PinnedDirectory::new(parent)?;
        let expected = before_destination
            .map(|m| crate::properties::EntryIdentity::from_metadata(local, m))
            .transpose()?;
        let stage = parent.join(format!(
            ".mucrab-stage-{}-{}",
            std::process::id(),
            next_id()
        ));
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut output = options.open(&stage)?;
        let handle = self.client.open(remote, false)?;
        let result = (|| {
            let mut offset = 0;
            loop {
                check(self.control)?;
                let data = self.client.read(&handle, offset)?;
                if data.is_empty() {
                    break;
                }
                offset += data.len() as u64;
                if offset > len {
                    return Err(error("Remote source grew during download"));
                }
                output.write_all(&data)?;
                self.control.advance(data.len() as u64);
            }
            if offset != len {
                return Err(error("Remote source length changed during download"));
            }
            output.sync_all()?;
            if self.client.lstat(remote)?.as_ref() != Some(source) {
                return Err(error("Remote source changed during download"));
            }
            Ok(())
        })();
        let closed = self.client.close(&handle);
        drop(output);
        result
            .and(closed)
            .and_then(|_| {
                check(self.control)?;
                directory.validate()?;
                match expected {
                    Some(id) => id.validate(local)?,
                    None => {
                        if fs::symlink_metadata(local).is_ok() {
                            return Err(error("Local destination appeared before publication"));
                        }
                    }
                }
                if self.policy == CopyPolicy::Skip {
                    fs::hard_link(&stage, local)?;
                    fs::remove_file(&stage)?;
                } else {
                    fs::rename(&stage, local)?;
                }
                Ok(())
            })
            .map_err(|e| {
                error(format!(
                    "{e}. Owned local stage may remain: {}",
                    stage.display()
                ))
            })
    }
}
/// A retained private copy for explicit external opening or W14 handoff.
pub fn download_open(
    session: &Arc<RemoteSession>,
    location: &RemoteLocation,
    c: &JobControl,
) -> io::Result<PathBuf> {
    if location.profile != session.profile.id
        || RemoteLocation::new(&location.profile, &location.path)? != *location
    {
        return Err(error("Remote Open endpoint/profile mismatch"));
    }
    let name = location
        .path
        .rsplit('/')
        .next()
        .ok_or_else(|| error("Remote leaf required"))?;
    if !crate::is_valid_leaf_name(name) {
        return Err(error("Invalid remote filename"));
    }
    if name.rsplit('.').next().is_some_and(|ext| {
        ["zip", "tar", "gz", "tgz", "bz2", "xz", "7z", "rar"]
            .iter()
            .any(|v| ext.eq_ignore_ascii_case(v))
    }) {
        return Err(error(
            "Remote archives are unsupported; use an explicit Copy to a local folder first",
        ));
    }
    let mut client = session.client(c)?;
    let source = client
        .lstat(&location.path)?
        .ok_or_else(|| error("Remote file missing"))?;
    if !source.file() || source.size.is_none_or(|n| n > 512 * 1024 * 1024) {
        return Err(error("Remote Open requires a regular file within512MiB"));
    }
    let root = std::env::temp_dir().join(format!(
        "mucrab-sftp-open-{}-{}",
        std::process::id(),
        next_id()
    ));
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(&root)?;
    let result = (|| {
        let path = root.join(name);
        let mut visited = 0;
        let mut walk = Walk {
            client: &mut client,
            control: c,
            visited: &mut visited,
            policy: CopyPolicy::Skip,
            upload: false,
        };
        let parent = location.parent();
        walk.client.ensure_directory(&parent.path)?;
        walk.download_file(&location.path, &path, &source, None)?;
        Ok(path)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&root);
    }
    result
}
