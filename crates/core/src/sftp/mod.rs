// SPDX-License-Identifier: GPL-3.0-only
//! Explicit SFTP session/provider boundary. No shell commands or implicit trust.
mod store;
mod transfer;
mod wire;
use crate::{FileEntry, FileMetadata, JobControl};
use base64::{Engine, engine::general_purpose::STANDARD_NO_PAD};
use ssh2::Session;
use std::{
    fs, io,
    net::{Shutdown, TcpStream, ToSocketAddrs},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
pub use store::*;
pub use transfer::*;
fn error(message: impl ToString) -> io::Error {
    io::Error::other(message.to_string())
}
fn next_id() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}
fn check(c: &JobControl) -> io::Result<()> {
    if c.checkpoint() {
        Ok(())
    } else {
        Err(error("SFTP operation cancelled"))
    }
}
pub struct Handshake {
    pub identity: HostIdentity,
    session: Session,
    socket: TcpStream,
}
pub struct RemoteSession {
    pub profile: Profile,
    pub identity: HostIdentity,
    session: Session,
    socket: TcpStream,
    disconnected: AtomicBool,
    gate: Arc<Mutex<()>>,
}
impl Handshake {
    pub fn connect(profile: &Profile, c: &JobControl) -> io::Result<Self> {
        profile.validate()?;
        check(c)?;
        // Name resolution is isolated on a bounded worker: a stalled resolver cannot
        // delay application cancellation indefinitely. No credentials enter this task.
        let host = profile.host.clone();
        let port = profile.port;
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let _ = tx.send(
                (host.as_str(), port)
                    .to_socket_addrs()
                    .map(|v| v.take(8).collect::<Vec<_>>()),
            );
        });
        let deadline = Instant::now() + Duration::from_secs(5);
        let addresses = loop {
            check(c)?;
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(v) => break v.map_err(|_| error("SFTP hostname resolution failed"))?,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) if Instant::now() < deadline => {}
                _ => return Err(error("SFTP hostname resolution timed out")),
            }
        };
        let mut socket = None;
        for a in addresses {
            check(c)?;
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            if let Ok(s) = TcpStream::connect_timeout(&a, remaining.min(Duration::from_millis(750)))
            {
                socket = Some(s);
                break;
            }
        }
        let socket = socket.ok_or_else(|| error("SFTP TCP connection failed or timed out"))?;
        socket.set_read_timeout(Some(Duration::from_secs(5)))?;
        socket.set_write_timeout(Some(Duration::from_secs(5)))?;
        let mut session = Session::new().map_err(|_| error("Cannot initialize SSH"))?;
        session.set_timeout(5000);
        session.set_tcp_stream(socket.try_clone()?);
        session
            .handshake()
            .map_err(|_| error("SSH handshake failed or timed out"))?;
        check(c)?;
        let (_, kind) = session
            .host_key()
            .ok_or_else(|| error("Server host key unavailable"))?;
        let hash = session
            .host_key_hash(ssh2::HashType::Sha256)
            .ok_or_else(|| error("SHA256 host fingerprint unavailable"))?;
        let identity = HostIdentity {
            endpoint: profile.endpoint(),
            algorithm: format!("{kind:?}"),
            fingerprint: format!("SHA256:{}", STANDARD_NO_PAD.encode(hash)),
        };
        Ok(Self {
            identity,
            session,
            socket,
        })
    }
    /// The caller must have explicitly verified/saved this exact host identity.
    pub fn authenticate(
        self,
        profile: Profile,
        verified: &HostIdentity,
        secret: &str,
        c: &JobControl,
    ) -> io::Result<Arc<RemoteSession>> {
        if self.identity != *verified || profile.endpoint() != verified.endpoint {
            return Err(error("Unverified server identity"));
        }
        check(c)?;
        let result = match &profile.authentication {
            Authentication::Password => self.session.userauth_password(&profile.user, secret),
            Authentication::PrivateKey(path) => self.session.userauth_pubkey_file(
                &profile.user,
                None,
                path,
                (!secret.is_empty()).then_some(secret),
            ),
        };
        result.map_err(|_| {
            error("SSH authentication failed; check user, authentication method and secret")
        })?;
        if !self.session.authenticated() {
            return Err(error("SSH authentication incomplete"));
        }
        check(c)?;
        Ok(Arc::new(RemoteSession {
            profile,
            identity: self.identity,
            session: self.session,
            socket: self.socket,
            disconnected: AtomicBool::new(false),
            gate: Arc::new(Mutex::new(())),
        }))
    }
}
impl RemoteSession {
    pub fn connected(&self) -> bool {
        !self.disconnected.load(Ordering::Acquire)
    }
    pub fn disconnect(&self) {
        self.disconnected.store(true, Ordering::Release);
        let _ = self.socket.shutdown(Shutdown::Both);
    }
    fn client(&self, c: &JobControl) -> io::Result<wire::Client> {
        if !self.connected() {
            return Err(error("SFTP session disconnected; reconnect first"));
        }
        wire::Client::new(&self.session, self.gate.clone(), c.clone())
    }
    pub fn list(&self, location: &RemoteLocation, c: &JobControl) -> io::Result<Vec<FileEntry>> {
        if location.profile != self.profile.id
            || RemoteLocation::new(&location.profile, &location.path)? != *location
        {
            return Err(error("Remote profile/session mismatch"));
        }
        let mut client = self.client(c)?;
        client.ensure_directory(&location.path)?;
        client.list(&location.path, c).map(|v| {
            v.into_iter()
                .map(|(name, m)| FileEntry {
                    name: name.clone(),
                    is_directory: m.directory(),
                    is_symlink: m.link(),
                    metadata: m.size.zip(m.mtime).and_then(|(size, time)| {
                        i64::try_from(size).ok().map(|size| FileMetadata {
                            size,
                            modified_ms: i64::from(time) * 1000,
                            extension: crate::filename_extension(&name).map(str::to_owned),
                        })
                    }),
                })
                .collect()
        })
    }
}

#[derive(Clone)]
pub struct RemoteNavigation {
    pub location: RemoteLocation,
    pub generation: u64,
    pub record: bool,
    pub cursor: Option<usize>,
}
pub fn session_secret(secret: String) -> zeroize::Zeroizing<String> {
    zeroize::Zeroizing::new(secret)
}

pub fn clear_secret(secret: &mut String) {
    use zeroize::Zeroize;
    secret.zeroize();
}
