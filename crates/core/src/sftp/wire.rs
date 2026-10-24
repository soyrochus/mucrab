// SPDX-License-Identifier: GPL-3.0-only
//! Bounded SFTP v3 subset over an authenticated SSH subsystem channel.
use super::{Arc, JobControl, RemoteLocation, Session, check, error};
use std::{
    io::{self, Read, Write},
    time::Duration,
};
const PACKET: usize = 512 * 1024;
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Attributes {
    /// File size in bytes; compatibility listing metadata can use -1 for unknown size.
    pub size: Option<u64>,
    /// Natural or lexical name tie-breaking policy.
    pub mode: Option<u32>,
    pub mtime: Option<u32>,
}
impl Attributes {
    pub fn directory(&self) -> bool {
        self.mode.is_some_and(|v| v & 0o170000 == 0o040000)
    }
    pub fn file(&self) -> bool {
        self.mode.is_some_and(|v| v & 0o170000 == 0o100000)
    }
    pub fn link(&self) -> bool {
        self.mode.is_some_and(|v| v & 0o170000 == 0o120000)
    }
}
struct Packet {
    bytes: Vec<u8>,
    offset: usize,
}
impl Packet {
    fn take(&mut self, n: usize) -> io::Result<&[u8]> {
        let end = self
            .offset
            .checked_add(n)
            .ok_or_else(|| error("SFTP packet overflow"))?;
        let b = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| error("Truncated SFTP response"))?;
        self.offset = end;
        Ok(b)
    }
    fn u32(&mut self) -> io::Result<u32> {
        Ok(u32::from_be_bytes(
            self.take(4)?.try_into().expect("four bytes"),
        ))
    }
    fn u64(&mut self) -> io::Result<u64> {
        Ok(u64::from_be_bytes(
            self.take(8)?.try_into().expect("eight bytes"),
        ))
    }
    fn string(&mut self) -> io::Result<Vec<u8>> {
        let n = self.u32()? as usize;
        Ok(self.take(n)?.to_vec())
    }
    fn text(&mut self) -> io::Result<String> {
        String::from_utf8(self.string()?)
            .map_err(|_| error("Non-Unicode SFTP name/extension is unsupported"))
    }
    fn attrs(&mut self) -> io::Result<Attributes> {
        let flags = self.u32()?;
        if flags & !0x8000000f != 0 {
            return Err(error("Unsupported SFTP attribute flags"));
        }
        let size = if flags & 1 != 0 {
            Some(self.u64()?)
        } else {
            None
        };
        if flags & 2 != 0 {
            self.take(8)?;
        }
        let mode = if flags & 4 != 0 {
            Some(self.u32()?)
        } else {
            None
        };
        let mtime = if flags & 8 != 0 {
            self.u32()?;
            Some(self.u32()?)
        } else {
            None
        };
        if flags & 0x80000000 != 0 {
            let n = self.u32()?;
            if n > 128 {
                return Err(error("Too many SFTP extended attributes"));
            }
            for _ in 0..n {
                self.string()?;
                self.string()?;
            }
        }
        Ok(Attributes { size, mode, mtime })
    }
    fn status(&mut self) -> io::Result<u32> {
        self.u32()
    }
}
fn u32v(v: u32, b: &mut Vec<u8>) {
    b.extend_from_slice(&v.to_be_bytes());
}
fn string(s: &[u8], b: &mut Vec<u8>) {
    u32v(s.len() as u32, b);
    b.extend_from_slice(s);
}
pub(super) struct Client {
    channel: ssh2::Channel,
    next: u32,
    pub posix_rename: bool,
    gate: Arc<std::sync::Mutex<()>>,
    control: JobControl,
}
impl Client {
    pub fn new(
        session: &Session,
        gate: Arc<std::sync::Mutex<()>>,
        control: JobControl,
    ) -> io::Result<Self> {
        let lock = gate.clone();
        let _guard = lock_gate(&lock, &control)?;
        let mut channel = session
            .channel_session()
            .map_err(|_| error("Cannot open SSH channel"))?;
        channel
            .subsystem("sftp")
            .map_err(|_| error("Server SFTP subsystem unavailable"))?;
        let mut result = Self {
            channel,
            next: 0,
            posix_rename: false,
            gate,
            control,
        };
        result.send(&[1, 0, 0, 0, 3])?;
        let mut p = result.receive()?;
        if p.take(1)? != [2] || p.u32()? != 3 {
            return Err(error("Only SFTP version3 is supported"));
        }
        let mut count = 0;
        while p.offset < p.bytes.len() {
            count += 1;
            if count > 128 {
                return Err(error("Too many SFTP extensions"));
            }
            let name = p.text()?;
            let version = p.text()?;
            if name == "posix-rename@openssh.com" && version == "1" {
                result.posix_rename = true;
            }
        }
        Ok(result)
    }
    fn send(&mut self, b: &[u8]) -> io::Result<()> {
        if b.len() > PACKET {
            return Err(error("SFTP request exceeds packet limit"));
        }
        self.channel
            .write_all(&(b.len() as u32).to_be_bytes())
            .and_then(|_| self.channel.write_all(b))
            .map_err(|_| error("SFTP write failed or timed out"))
    }
    fn receive(&mut self) -> io::Result<Packet> {
        let mut n = [0; 4];
        self.channel
            .read_exact(&mut n)
            .map_err(|_| error("SFTP read failed or timed out"))?;
        let n = u32::from_be_bytes(n) as usize;
        if n == 0 || n > PACKET {
            return Err(error("Invalid or oversized SFTP packet"));
        }
        let mut bytes = vec![0; n];
        self.channel
            .read_exact(&mut bytes)
            .map_err(|_| error("SFTP packet read failed or timed out"))?;
        Ok(Packet { bytes, offset: 0 })
    }
    fn request(&mut self, kind: u8, args: &[u8]) -> io::Result<(u8, Packet)> {
        let lock = self.gate.clone();
        let _guard = lock_gate(&lock, &self.control)?;
        self.next = self
            .next
            .checked_add(1)
            .ok_or_else(|| error("SFTP request IDs exhausted"))?;
        let mut b = vec![kind];
        u32v(self.next, &mut b);
        b.extend_from_slice(args);
        self.send(&b)?;
        let mut p = self.receive()?;
        let kind = p.take(1)?[0];
        if p.u32()? != self.next {
            return Err(error("Unexpected SFTP response ID"));
        }
        Ok((kind, p))
    }
    fn ok(&mut self, kind: u8, args: &[u8]) -> io::Result<()> {
        let (t, mut p) = self.request(kind, args)?;
        if t != 101 || p.status()? != 0 {
            return Err(error("SFTP request refused by server"));
        }
        Ok(())
    }
    pub fn lstat(&mut self, path: &str) -> io::Result<Option<Attributes>> {
        let mut b = vec![];
        string(path.as_bytes(), &mut b);
        let (t, mut p) = self.request(7, &b)?;
        match t {
            105 => p.attrs().map(Some),
            101 => match p.status()? {
                2 => Ok(None),
                _ => Err(error(
                    "SFTP path cannot be inspected (denied, missing parent or server error)",
                )),
            },
            _ => Err(error("Unexpected SFTP attributes response")),
        }
    }
    pub fn ensure_directory(&mut self, path: &str) -> io::Result<()> {
        let loc = RemoteLocation::new("validation", path)?;
        let mut current = String::from("/");
        if !self.lstat("/")?.is_some_and(|a| a.directory()) {
            return Err(error("Remote root is not a directory"));
        }
        for part in loc.path().split('/').filter(|p| !p.is_empty()) {
            current = if current == "/" {
                format!("/{part}")
            } else {
                format!("{current}/{part}")
            };
            if !self.lstat(&current)?.is_some_and(|a| a.directory()) {
                return Err(error(
                    "Remote path component is missing, a link or not a directory",
                ));
            }
        }
        Ok(())
    }
    fn handle(&mut self, kind: u8, args: &[u8]) -> io::Result<Vec<u8>> {
        let (t, mut p) = self.request(kind, args)?;
        if t != 102 {
            return Err(error("SFTP open refused"));
        }
        let h = p.string()?;
        if h.is_empty() || h.len() > 256 {
            return Err(error("Invalid SFTP handle"));
        }
        Ok(h)
    }
    pub fn list(&mut self, path: &str, c: &JobControl) -> io::Result<Vec<(String, Attributes)>> {
        let mut b = vec![];
        string(path.as_bytes(), &mut b);
        let handle = self.handle(11, &b)?;
        let result = (|| {
            let mut result = vec![];
            let mut names = std::collections::HashSet::new();
            loop {
                check(c)?;
                let mut b = vec![];
                string(&handle, &mut b);
                let (t, mut p) = self.request(12, &b)?;
                if t == 101 && p.status()? == 1 {
                    break;
                }
                if t != 104 {
                    return Err(error("SFTP listing failed"));
                }
                let n = p.u32()? as usize;
                if n == 0 || n > 20002 {
                    return Err(error("Invalid SFTP listing batch"));
                }
                for _ in 0..n {
                    let name = p.text()?;
                    p.string()?;
                    let attrs = p.attrs()?;
                    if name == "." || name == ".." {
                        continue;
                    }
                    if !crate::is_valid_leaf_name(&name)
                        || name.chars().any(char::is_control)
                        || !names.insert(name.clone())
                    {
                        return Err(error("Unsupported or duplicate SFTP filename"));
                    }
                    if result.len() >= 20000 {
                        return Err(error("SFTP listing exceeds 20000 entries"));
                    }
                    result.push((name, attrs));
                }
            }
            Ok(result)
        })();
        let closed = self.close(&handle);
        let value = result?;
        closed?;
        Ok(value)
    }
    pub fn close(&mut self, handle: &[u8]) -> io::Result<()> {
        let mut b = vec![];
        string(handle, &mut b);
        self.ok(4, &b)
    }
    pub fn open(&mut self, path: &str, write: bool) -> io::Result<Vec<u8>> {
        let mut b = vec![];
        string(path.as_bytes(), &mut b);
        u32v(if write { 2 | 8 | 32 } else { 1 }, &mut b);
        u32v(if write { 4 } else { 0 }, &mut b);
        if write {
            u32v(0o600, &mut b);
        }
        self.handle(3, &b)
    }
    pub fn read(&mut self, handle: &[u8], offset: u64) -> io::Result<Vec<u8>> {
        let mut b = vec![];
        string(handle, &mut b);
        b.extend_from_slice(&offset.to_be_bytes());
        u32v(32768, &mut b);
        let (t, mut p) = self.request(5, &b)?;
        if t == 101 && p.status()? == 1 {
            return Ok(vec![]);
        }
        if t != 103 {
            return Err(error("SFTP file read failed"));
        }
        let data = p.string()?;
        if data.len() > 32768 {
            return Err(error("SFTP read exceeds requested chunk"));
        }
        Ok(data)
    }
    pub fn write(&mut self, handle: &[u8], offset: u64, data: &[u8]) -> io::Result<()> {
        if data.len() > 32768 {
            return Err(error("SFTP write exceeds chunk limit"));
        }
        let mut b = vec![];
        string(handle, &mut b);
        b.extend_from_slice(&offset.to_be_bytes());
        string(data, &mut b);
        self.ok(6, &b)
    }
    pub fn mkdir(&mut self, path: &str) -> io::Result<()> {
        let mut b = vec![];
        string(path.as_bytes(), &mut b);
        u32v(4, &mut b);
        u32v(0o700, &mut b);
        self.ok(14, &b)
    }
    pub fn publish(&mut self, stage: &str, destination: &str, overwrite: bool) -> io::Result<()> {
        let mut b = vec![];
        if overwrite {
            if !self.posix_rename {
                return Err(error(
                    "Server lacks advertised POSIX rename; existing destination preserved",
                ));
            }
            string(b"posix-rename@openssh.com", &mut b);
        }
        string(stage.as_bytes(), &mut b);
        string(destination.as_bytes(), &mut b);
        self.ok(if overwrite{200}else{18},&b).map_err(|_|error("SFTP publication failed or outcome is uncertain; inspect destination and owned stage before reviewing again"))
    }
}

// libssh2 blocking reads hold its session mutex. Serialize complete protocol
// exchanges, releasing the gate before pause/checkpoint and between file chunks.
fn lock_gate<'a>(
    gate: &'a std::sync::Mutex<()>,
    control: &JobControl,
) -> io::Result<std::sync::MutexGuard<'a, ()>> {
    loop {
        check(control)?;
        match gate.try_lock() {
            Ok(g) => return Ok(g),
            Err(std::sync::TryLockError::Poisoned(_)) => {
                return Err(error("SFTP session worker failed; reconnect"));
            }
            Err(std::sync::TryLockError::WouldBlock) => {
                std::thread::sleep(Duration::from_millis(10))
            }
        }
    }
}

#[cfg(any(test, feature = "test-util"))]
pub(crate) fn exercise_packet(bytes: &[u8]) -> io::Result<()> {
    if bytes.len() > PACKET {
        return Err(error("Oversized fuzz packet"));
    }
    let Some((&mode, bytes)) = bytes.split_first() else {
        return Ok(());
    };
    let mut packet = Packet {
        bytes: bytes.to_vec(),
        offset: 0,
    };
    match mode % 4 {
        0 => {
            packet.u32()?;
        }
        1 => {
            packet.u64()?;
        }
        2 => {
            packet.text()?;
        }
        _ => {
            packet.attrs()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod malformed_tests {
    use super::*;
    #[test]
    fn truncated_lengths_attributes_and_oversized_packets_are_errors() {
        for bytes in [
            vec![0],
            vec![1, 0, 1],
            vec![2, 255, 255, 255, 255],
            vec![3, 255, 255, 255, 255],
            vec![0; PACKET + 1],
        ] {
            assert!(
                exercise_packet(&bytes).is_err(),
                "invalid bounded packet length {}",
                bytes.len()
            );
        }
        for mode in 0..4 {
            for len in 0..64 {
                let mut bytes = vec![0u8; len + 1];
                bytes[0] = mode;
                let _ = exercise_packet(&bytes);
            }
        }
    }
}
