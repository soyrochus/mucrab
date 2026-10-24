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
pub use store::{
    Authentication, ConnectionStore, HostIdentity, Profile, RemoteLocation, delete_secret,
    load_secret, save_secret, valid_id,
};
pub use transfer::{Endpoint, TransferPlan, download_open};
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
/// Unauthenticated SSH transport exposing host identity before any secret is sent.
pub struct Handshake {
    /// Non-secret host identity associated with the transport or authenticated session.
    identity: HostIdentity,
    session: Session,
    socket: TcpStream,
}
/// An authenticated explicitly disconnectable session; Debug omits transport internals.
pub struct RemoteSession {
    /// Saved profile configuration or identifier; secret material is supplied separately.
    profile: Profile,
    /// Non-secret host identity associated with the transport or authenticated session.
    identity: HostIdentity,
    session: Session,
    socket: TcpStream,
    disconnected: AtomicBool,
    gate: Arc<Mutex<()>>,
}
impl Handshake {
    /// Exact host identity reported by this transport; callers cannot replace it.
    pub fn identity(&self) -> &HostIdentity {
        &self.identity
    }

    /// Resolve and establish bounded SSH transport, returning its unverified host identity.
    ///
    /// # Errors
    /// Returns connection, authentication, cancellation, trust-mismatch or protocol errors without secret values.
    pub fn connect(profile: &Profile, c: &JobControl) -> io::Result<Self> {
        profile.validate()?;
        check(c)?;
        // Name resolution is isolated from the event loop: a stalled resolver cannot
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
    /// Connection settings frozen at successful authentication.
    pub fn profile(&self) -> &Profile {
        &self.profile
    }
    /// Host identity verified before this session authenticated.
    pub fn identity(&self) -> &HostIdentity {
        &self.identity
    }

    /// Test whether this authenticated session has been explicitly disconnected.
    pub fn connected(&self) -> bool {
        !self.disconnected.load(Ordering::Acquire)
    }
    /// Mark the session disconnected and interrupt its transport without implicit reconnect.
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
    /// Enumerate a validated remote folder through the authenticated SFTP session.
    ///
    /// # Errors
    /// Returns connection, authentication, cancellation, trust-mismatch or protocol errors without secret values.
    pub fn list(&self, location: &RemoteLocation, c: &JobControl) -> io::Result<Vec<FileEntry>> {
        if location.profile() != self.profile.id
            || RemoteLocation::new(location.profile(), location.path())? != *location
        {
            return Err(error("Remote profile/session mismatch"));
        }
        let mut client = self.client(c)?;
        client.ensure_directory(location.path())?;
        client.list(location.path(), c).map(|v| {
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

#[derive(Clone, Debug)]
/// A generation-bound request distinct from the last committed remote listing.
pub struct RemoteNavigation {
    /// Structured provider location rather than an unvalidated display URL.
    pub location: RemoteLocation,
    /// Monotonic request identity used to reject superseded results.
    pub generation: u64,
    /// Whether successful navigation appends to the successful-location history.
    pub record: bool,
    /// Optional existing history position to commit after successful navigation.
    pub cursor: Option<usize>,
}
/// Move a session secret into ownership that zeroizes its allocation on drop.
pub fn session_secret(secret: String) -> zeroize::Zeroizing<String> {
    zeroize::Zeroizing::new(secret)
}

/// Zeroize a mutable secret before its editor or request is discarded.
pub fn clear_secret(secret: &mut String) {
    use zeroize::Zeroize;
    secret.zeroize();
}

#[cfg(feature = "test-util")]
pub(crate) use wire::exercise_packet;

impl std::fmt::Debug for Handshake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handshake")
            .field("identity", &self.identity)
            .finish_non_exhaustive()
    }
}
impl std::fmt::Debug for RemoteSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteSession")
            .field("profile", &self.profile.id)
            .field("identity", &self.identity)
            .field("connected", &self.connected())
            .finish_non_exhaustive()
    }
}
