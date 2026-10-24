// SPDX-License-Identifier: GPL-3.0-only
//! Validated non-secret profiles and exact host trust with native credential adapters.
use super::{Path, PathBuf, error, fs, io, next_id};
const ESCAPE: &percent_encoding::AsciiSet = &percent_encoding::CONTROLS
    .add(b' ')
    .add(b'%')
    .add(b'?')
    .add(b'#');
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, io::Write};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "method", content = "key_file", deny_unknown_fields)]
/// Non-secret authentication configuration; passwords and passphrases are supplied separately.
pub enum Authentication {
    #[doc = "Password authentication with a separately supplied session secret."]
    Password,
    #[doc = "A configured private-key file with a separately supplied optional passphrase."]
    PrivateKey(PathBuf),
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
/// Saved non-secret connection settings, validated before transport or persistence.
pub struct Profile {
    /// Stable identity used instead of a mutable display position.
    pub id: String,
    /// User-facing non-secret label, validated before persistence.
    pub label: String,
    /// Non-secret hostname or address; URLs and credential-bearing values are rejected.
    pub host: String,
    /// Nonzero SSH service port.
    pub port: u16,
    /// Non-secret account name, separate from passwords and passphrases.
    pub user: String,
    /// Password or private-key-file configuration, without the secret itself.
    pub authentication: Authentication,
    /// Initial absolute remote directory, validated with the saved-profile identifier.
    pub directory: String,
    #[serde(default)]
    /// Whether a native credential was explicitly saved for this profile.
    pub remembered: bool,
    #[serde(default)]
    /// Non-secret endpoint/account/authentication binding for the saved credential.
    pub credential_context: Option<String>,
}
impl Profile {
    /// Validate non-secret profile fields and the profile-backed initial directory.
    ///
    /// # Errors
    /// Returns invalid-input, changed-configuration or platform-service errors; no plaintext credential fallback is used.
    pub fn validate(&self) -> io::Result<()> {
        if !valid_id(&self.id)
            || self.label.trim().is_empty()
            || self.label.len() > 128
            || self.label.chars().any(char::is_control)
            || self.host.is_empty()
            || self.host.len() > 253
            || self.host.chars().any(|c| {
                c.is_control() || c.is_whitespace() || matches!(c, '/' | '@' | '%' | '[' | ']')
            })
            || self.port == 0
            || self.user.is_empty()
            || self.user.len() > 128
            || self
                .user
                .chars()
                .any(|c| c.is_control() || matches!(c, '@' | ':' | '/' | '\\'))
        {
            return Err(error("Invalid SFTP profile fields"));
        }
        if let Authentication::PrivateKey(p) = &self.authentication
            && (!p.is_absolute() || p.to_str().is_none_or(|s| s.contains('\0')))
        {
            return Err(error("Absolute Unicode private-key path required"));
        }
        RemoteLocation::new(&self.id, &self.directory)?;
        Ok(())
    }
    /// Encode the endpoint, account and authentication context binding a saved credential.
    pub fn credential_context_key(&self) -> String {
        serde_json::to_string(&(self.endpoint(), &self.user, &self.authentication))
            .expect("serializable profile context")
    }
    /// Check that remembered credentials belong to the current connection settings.
    pub fn saved_credential_matches(&self) -> bool {
        self.remembered
            && self.credential_context.as_deref() == Some(self.credential_context_key().as_str())
    }
    /// Compare profile identity and authentication settings without comparing any secret.
    pub fn same_connection(&self, other: &Self) -> bool {
        self.id == other.id
            && self.endpoint() == other.endpoint()
            && self.user == other.user
            && self.authentication == other.authentication
    }
    /// Format a normalized host and port identity used for host-key trust.
    pub fn endpoint(&self) -> String {
        format!("[{}]:{}", self.host.to_lowercase(), self.port)
    }
}
/// Validate a bounded ASCII saved-profile identifier rather than a host or credential URL.
pub fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}
/// A normalized absolute path belonging to a saved SFTP profile.
///
/// Serialized locations use the same validation as [`Self::new`].
///
/// ```compile_fail
/// use mucrab_core::sftp::RemoteLocation;
/// let mut location = RemoteLocation::new("server", "/")?;
/// location.path = "relative".into();
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(try_from = "LocationWire")]
pub struct RemoteLocation {
    profile: String,
    path: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LocationWire {
    profile: String,
    path: String,
}

impl TryFrom<LocationWire> for RemoteLocation {
    type Error = io::Error;
    fn try_from(value: LocationWire) -> Result<Self, Self::Error> {
        Self::new(&value.profile, &value.path)
    }
}

impl std::fmt::Display for RemoteLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.display())
    }
}
impl RemoteLocation {
    /// Saved profile identifier; never credentials or a host URL.
    pub fn profile(&self) -> &str {
        &self.profile
    }
    /// Normalized absolute remote path.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Normalize an absolute path and validate its saved-profile identifier.
    ///
    /// # Errors
    /// Returns invalid-input, changed-configuration or platform-service errors; no plaintext credential fallback is used.
    pub fn new(profile: &str, path: &str) -> io::Result<Self> {
        if !valid_id(profile)
            || !path.starts_with('/')
            || path.len() > 4096
            || path.chars().any(char::is_control)
            || path.contains('\\')
        {
            return Err(error("Invalid profile-backed remote path"));
        }
        let mut parts = vec![];
        for p in path.split('/') {
            match p {
                "" | "." => {}
                ".." => {
                    parts.pop();
                }
                p => parts.push(p),
            }
        }
        Ok(Self {
            profile: profile.into(),
            path: format!("/{}", parts.join("/")),
        })
    }
    /// Format a profile-backed URL with reserved path characters percent-encoded.
    pub fn display(&self) -> String {
        format!(
            "sftp://{}{}",
            self.profile,
            percent_encoding::utf8_percent_encode(&self.path, ESCAPE)
        )
    }
    /// Parse only saved-profile SFTP URLs and reject credentials or malformed encoding.
    ///
    /// # Errors
    /// Returns invalid-input, changed-configuration or platform-service errors; no plaintext credential fallback is used.
    pub fn parse(s: &str) -> io::Result<Self> {
        let rest = s
            .strip_prefix("sftp://")
            .ok_or_else(|| error("Use a saved SFTP profile"))?;
        let (id, path) = rest.split_once('/').unwrap_or((rest, ""));
        let bytes = path.as_bytes();
        for (i, b) in bytes.iter().enumerate() {
            if *b == b'%'
                && (bytes.get(i + 1).is_none_or(|b| !b.is_ascii_hexdigit())
                    || bytes.get(i + 2).is_none_or(|b| !b.is_ascii_hexdigit()))
            {
                return Err(error("Invalid remote percent encoding"));
            }
        }
        let path = percent_encoding::percent_decode_str(path)
            .decode_utf8()
            .map_err(|_| error("Invalid encoded remote path"))?;
        Self::new(id, &format!("/{path}"))
    }
    /// Join one validated direct-child name, enforcing the remote path length bound.
    ///
    /// # Errors
    /// Returns invalid-input, changed-configuration or platform-service errors; no plaintext credential fallback is used.
    pub fn child(&self, name: &str) -> io::Result<Self> {
        if !crate::is_valid_leaf_name(name) || name.chars().any(char::is_control) {
            return Err(error("Unsupported remote filename"));
        }
        Self::new(&self.profile, &format!("{}/{name}", self.path))
    }
    /// Return a normalized parent, keeping the remote root at the root.
    pub fn parent(&self) -> Self {
        let p = self.path.rsplit_once('/').map_or("", |(p, _)| p);
        Self {
            profile: self.profile.clone(),
            path: if p.is_empty() { "/".into() } else { p.into() },
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
/// Exact endpoint, algorithm and fingerprint accepted before authentication.
pub struct HostIdentity {
    /// Normalized host/port trust identity.
    pub endpoint: String,
    /// Reported SSH host-key algorithm identifier.
    pub algorithm: String,
    /// SHA-256 host-key fingerprint that must be verified before authentication.
    pub fingerprint: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    version: u32,
    profiles: Vec<Profile>,
    trusted: Vec<HostIdentity>,
}
impl Default for Document {
    fn default() -> Self {
        Self {
            version: 1,
            profiles: vec![],
            trusted: vec![],
        }
    }
}
#[derive(Debug)]
/// Bounded non-secret profiles and trusted hosts with checked atomic saves.
pub struct ConnectionStore {
    path: PathBuf,
    document: Document,
    baseline: Option<Vec<u8>>,
}
fn read(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
        Ok(m) => {
            if !m.is_file() || m.len() > 1024 * 1024 {
                return Err(error("SFTP config must be a regular file within 1 MiB"));
            }
            let b = fs::read(path)?;
            if b.len() > 1024 * 1024 {
                return Err(error("SFTP config exceeds 1 MiB"));
            }
            Ok(Some(b))
        }
    }
}
impl ConnectionStore {
    /// Load bounded non-secret connection/trust data without overwriting invalid input.
    ///
    /// # Errors
    /// Returns invalid-input, changed-configuration or platform-service errors; no plaintext credential fallback is used.
    pub fn load(path: PathBuf) -> io::Result<Self> {
        let baseline = read(&path)?;
        let document: Document = baseline
            .as_ref()
            .map(|b| serde_json::from_slice(b))
            .transpose()
            .map_err(|_| error("Invalid non-secret SFTP configuration"))?
            .unwrap_or_default();
        if document.version != 1 || document.profiles.len() > 128 || document.trusted.len() > 512 {
            return Err(error("Unsupported SFTP configuration version/count"));
        }
        let mut ids = HashSet::new();
        for p in &document.profiles {
            p.validate()?;
            if !ids.insert(&p.id) {
                return Err(error("Duplicate SFTP profile ID"));
            }
        }
        let mut endpoints = HashSet::new();
        for t in &document.trusted {
            if !endpoints.insert(&t.endpoint)
                || t.endpoint.len() > 300
                || t.algorithm.len() > 64
                || t.fingerprint.len() > 128
                || !t.fingerprint.starts_with("SHA256:")
                || t.endpoint
                    .chars()
                    .chain(t.algorithm.chars())
                    .chain(t.fingerprint.chars())
                    .any(char::is_control)
            {
                return Err(error("Invalid trusted host identity"));
            }
        }
        Ok(Self {
            path,
            document,
            baseline,
        })
    }
    /// Choose an unused bounded identifier within the loaded connection document.
    pub fn unused_profile_id(&self) -> String {
        (1..=129)
            .map(|n| format!("server-{n}"))
            .find(|id| self.profile(id).is_none())
            .expect("at most128 profiles")
    }
    /// Borrow saved profiles in their persisted order.
    pub fn profiles(&self) -> &[Profile] {
        &self.document.profiles
    }
    /// Borrow explicitly accepted host identities; changed keys require separate removal.
    pub fn trusted(&self) -> &[HostIdentity] {
        &self.document.trusted
    }
    /// Find a saved profile by its stable identifier.
    pub fn profile(&self, id: &str) -> Option<&Profile> {
        self.document.profiles.iter().find(|p| p.id == id)
    }
    /// Validate and atomically persist a profile after checking for external configuration changes.
    ///
    /// # Errors
    /// Returns invalid-input, changed-configuration or platform-service errors; no plaintext credential fallback is used.
    pub fn save_profile(&mut self, p: Profile) -> io::Result<()> {
        p.validate()?;
        let mut d = self.document.clone();
        if let Some(old) = d.profiles.iter_mut().find(|v| v.id == p.id) {
            *old = p;
        } else {
            if d.profiles.len() >= 128 {
                return Err(error("Maximum 128 profiles"));
            }
            d.profiles.push(p);
        }
        self.commit(d)
    }
    /// Persist profile removal without implicitly deleting native credential-store items.
    ///
    /// # Errors
    /// Returns invalid-input, changed-configuration or platform-service errors; no plaintext credential fallback is used.
    pub fn remove_profile(&mut self, id: &str) -> io::Result<()> {
        let mut d = self.document.clone();
        d.profiles.retain(|p| p.id != id);
        self.commit(d)
    }
    /// Return first-use or exact trusted-match status; changed host keys are errors.
    ///
    /// # Errors
    /// Returns invalid-input, changed-configuration or platform-service errors; no plaintext credential fallback is used.
    pub fn trust_status(&self, key: &HostIdentity) -> io::Result<bool> {
        match self
            .document
            .trusted
            .iter()
            .find(|k| k.endpoint == key.endpoint)
        {
            None => Ok(false),
            Some(old) if old == key => Ok(true),
            Some(_) => Err(error(
                "Host key changed. Connection refused before authentication. Manage trusted hosts separately.",
            )),
        }
    }
    /// Persist explicit acceptance of this exact host identity without replacing changed keys.
    ///
    /// # Errors
    /// Returns invalid-input, changed-configuration or platform-service errors; no plaintext credential fallback is used.
    pub fn trust(&mut self, key: HostIdentity) -> io::Result<()> {
        if self.trust_status(&key)? {
            return Ok(());
        }
        if self.document.trusted.len() >= 512 {
            return Err(error("Maximum 512 trusted hosts"));
        }
        let mut d = self.document.clone();
        d.trusted.push(key);
        self.commit(d)
    }
    /// Persist explicit removal of a trusted endpoint identity.
    ///
    /// # Errors
    /// Returns invalid-input, changed-configuration or platform-service errors; no plaintext credential fallback is used.
    pub fn forget_host(&mut self, endpoint: &str) -> io::Result<()> {
        let mut d = self.document.clone();
        d.trusted.retain(|k| k.endpoint != endpoint);
        self.commit(d)
    }
    fn commit(&mut self, d: Document) -> io::Result<()> {
        if read(&self.path)? != self.baseline {
            return Err(error(
                "SFTP configuration changed externally; reload before saving",
            ));
        }
        let parent = self
            .path
            .parent()
            .ok_or_else(|| error("Configuration parent unavailable"))?;
        fs::create_dir_all(parent)?;
        let p = parent.join(format!(".sftp-{}-{}", std::process::id(), next_id()));
        let result = (|| {
            let mut options = fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut f = options.open(&p)?;
            let bytes =
                serde_json::to_vec_pretty(&d).map_err(|_| error("Cannot encode SFTP config"))?;
            f.write_all(&bytes)?;
            f.sync_all()?;
            if read(&self.path)? != self.baseline {
                return Err(error("SFTP configuration changed during save"));
            }
            fs::rename(&p, &self.path)?;
            self.baseline = Some(bytes);
            self.document = d;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(p);
        }
        result
    }
}
/// No plaintext fallback; callers invoke native services only in read workers.
#[cfg(target_os = "macos")]
pub fn load_secret(profile: &str) -> io::Result<zeroize::Zeroizing<String>> {
    keyring::Entry::new("org.mucrab.sftp", profile)
        .and_then(|e| e.get_password())
        .map(zeroize::Zeroizing::new)
        .map_err(|_| error("Native credential unavailable; enter a session-only secret"))
}
#[cfg(target_os = "macos")]
/// Write a secret only to the platform credential service, never to profile JSON.
///
/// # Errors
/// Returns invalid-input, changed-configuration or platform-service errors; no plaintext credential fallback is used.
pub fn save_secret(profile: &str, secret: &str) -> io::Result<()> {
    keyring::Entry::new("org.mucrab.sftp", profile)
        .and_then(|e| e.set_password(secret))
        .map_err(|_| error("Native credential storage unavailable. Secret was not saved."))
}
#[cfg(target_os = "macos")]
/// Remove one saved native credential without changing other credential items.
///
/// # Errors
/// Returns invalid-input, changed-configuration or platform-service errors; no plaintext credential fallback is used.
pub fn delete_secret(profile: &str) -> io::Result<()> {
    let e = keyring::Entry::new("org.mucrab.sftp", profile)
        .map_err(|_| error("Native credential service unavailable"))?;
    match e.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err(error(
            "Could not remove native credential; profile retained",
        )),
    }
}

#[cfg(target_os = "linux")]
pub use linux_credentials::{delete_secret, load_secret, save_secret};
#[cfg(target_os = "linux")]
mod linux_credentials {
    use super::*;
    use dbus_secret_service::{EncryptionType, SecretService};
    use std::collections::HashMap;
    fn attrs(profile: &str) -> HashMap<&str, &str> {
        HashMap::from([
            ("application", "org.mucrab.sftp"),
            ("profile", profile),
            ("version", "1"),
        ])
    }
    fn service() -> io::Result<SecretService> {
        SecretService::connect_with_max_prompt_timeout(EncryptionType::Dh, 5)
            .map_err(|_| error("Secret Service unavailable; use a session-only secret"))
    }
    /// Read a native credential into zeroizing ownership; plaintext fallback is unavailable.
    ///
    /// # Errors
    /// Returns invalid-input, changed-configuration or platform-service errors; no plaintext credential fallback is used.
    pub fn load_secret(profile: &str) -> io::Result<zeroize::Zeroizing<String>> {
        let ss = service()?;
        let found = ss
            .search_items(attrs(profile))
            .map_err(|_| error("Native credential lookup failed"))?;
        if !found.locked.is_empty() || found.unlocked.len() != 1 {
            return Err(error(
                "Saved credential unavailable or locked; unlock your OS keyring or enter a session-only secret",
            ));
        }
        let bytes = zeroize::Zeroizing::new(
            found.unlocked[0]
                .get_secret()
                .map_err(|_| error("Native credential read failed"))?,
        );
        if bytes.len() > 65536 {
            return Err(error("Saved credential exceeds supported size"));
        }
        String::from_utf8(bytes.to_vec())
            .map(zeroize::Zeroizing::new)
            .map_err(|_| error("Saved credential is not UTF-8"))
    }
    /// Write a secret only to the platform credential service, never to profile JSON.
    ///
    /// # Errors
    /// Returns invalid-input, changed-configuration or platform-service errors; no plaintext credential fallback is used.
    pub fn save_secret(profile: &str, secret: &str) -> io::Result<()> {
        let ss = service()?;
        let found = ss
            .search_items(attrs(profile))
            .map_err(|_| error("Native credential lookup failed"))?;
        if !found.locked.is_empty() || found.unlocked.len() > 1 {
            return Err(error(
                "Native credential is locked or ambiguous; secret was not saved",
            ));
        }
        if let Some(item) = found.unlocked.first() {
            return item
                .set_secret(secret.as_bytes(), "text/plain")
                .map_err(|_| error("Native credential save failed"));
        }
        let collection = ss.get_default_collection().map_err(|_| {
            error("Default OS keyring unavailable; use session-only authentication")
        })?;
        if collection
            .is_locked()
            .map_err(|_| error("Cannot inspect OS keyring"))?
        {
            return Err(error(
                "OS keyring is locked. Unlock it separately or use session-only authentication.",
            ));
        }
        collection
            .create_item(
                "muCrab SFTP",
                attrs(profile),
                secret.as_bytes(),
                false,
                "text/plain",
            )
            .map(|_| ())
            .map_err(|_| error("Native credential save failed; no plaintext fallback"))
    }
    /// Remove one saved native credential without changing other credential items.
    ///
    /// # Errors
    /// Returns invalid-input, changed-configuration or platform-service errors; no plaintext credential fallback is used.
    pub fn delete_secret(profile: &str) -> io::Result<()> {
        let ss = service()?;
        let found = ss
            .search_items(attrs(profile))
            .map_err(|_| error("Native credential lookup failed"))?;
        if !found.locked.is_empty() || found.unlocked.len() > 1 {
            return Err(error(
                "Native credential is locked or ambiguous; profile retained",
            ));
        }
        if let Some(item) = found.unlocked.first() {
            item.delete()
                .map_err(|_| error("Could not remove native credential; profile retained"))?;
        }
        Ok(())
    }
}
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
/// Read a native credential into zeroizing ownership; plaintext fallback is unavailable.
///
/// # Errors
/// Returns invalid-input, changed-configuration or platform-service errors; no plaintext credential fallback is used.
pub fn load_secret(_: &str) -> io::Result<zeroize::Zeroizing<String>> {
    Err(error(
        "Native credential storage unavailable on this platform",
    ))
}
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
/// Write a secret only to the platform credential service, never to profile JSON.
///
/// # Errors
/// Returns invalid-input, changed-configuration or platform-service errors; no plaintext credential fallback is used.
pub fn save_secret(_: &str, _: &str) -> io::Result<()> {
    Err(error(
        "Native credential storage unavailable on this platform",
    ))
}
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
/// Remove one saved native credential without changing other credential items.
///
/// # Errors
/// Returns invalid-input, changed-configuration or platform-service errors; no plaintext credential fallback is used.
pub fn delete_secret(_: &str) -> io::Result<()> {
    Err(error(
        "Native credential storage unavailable on this platform",
    ))
}
