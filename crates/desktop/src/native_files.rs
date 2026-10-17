// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
//! Native file clipboard and Trash adapters. No filename is interpreted as code.
use std::{
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use url::Url;
const LIMIT: usize = 1024 * 1024;
fn error(message: impl Into<String>) -> io::Error {
    io::Error::other(message.into())
}

pub fn encode_paths(paths: &[PathBuf]) -> io::Result<String> {
    if paths.is_empty() || paths.len() > 10000 {
        return Err(error("Select between 1 and 10000 local files"));
    }
    let text = paths
        .iter()
        .map(|p| {
            Url::from_file_path(p)
                .map(|u| format!("{u}\r\n"))
                .map_err(|()| error("Absolute local path required"))
        })
        .collect::<io::Result<String>>()?;
    if text.len() > LIMIT {
        return Err(error("File clipboard exceeds 1 MiB"));
    }
    Ok(text)
}
pub fn decode_paths(text: &str) -> io::Result<Vec<PathBuf>> {
    if text.len() > LIMIT {
        return Err(error("File clipboard exceeds 1 MiB"));
    }
    let mut paths = Vec::new();
    for line in text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let bytes = line.as_bytes();
        for (i, b) in bytes.iter().enumerate() {
            if *b == b'%'
                && (i + 2 >= bytes.len()
                    || !bytes[i + 1].is_ascii_hexdigit()
                    || !bytes[i + 2].is_ascii_hexdigit())
            {
                return Err(error("Malformed file URL escape"));
            }
        }
        let u = Url::parse(line).map_err(|e| error(e.to_string()))?;
        if u.scheme() != "file"
            || u.query().is_some()
            || u.fragment().is_some()
            || u.port().is_some()
            || u.host_str()
                .is_some_and(|h| h != "localhost" && !h.is_empty())
        {
            return Err(error(
                "Only local file URLs without query or fragment are supported",
            ));
        }
        let p = u
            .to_file_path()
            .map_err(|()| error("Invalid local file URL"))?;
        if !p.is_absolute() || p.to_string_lossy().contains('\0') {
            return Err(error("Invalid clipboard path"));
        }
        if !paths.contains(&p) {
            paths.push(p);
        }
        if paths.len() > 10000 {
            return Err(error("Clipboard has more than 10000 files"));
        }
    }
    if paths.is_empty() {
        return Err(error("Clipboard contains no local files"));
    }
    Ok(paths)
}
// Bound command execution, input and captured output; run only on a worker.
fn run(mut command: Command, input: Option<String>) -> io::Result<String> {
    command
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = command.spawn()?;
    let mut output = child.stdout.take().unwrap();
    let stdin = child.stdin.take();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = (|| {
            if let (Some(mut stdin), Some(input)) = (stdin, input) {
                stdin.write_all(input.as_bytes())?;
            }
            let mut bytes = Vec::new();
            output
                .by_ref()
                .take((LIMIT + 1) as u64)
                .read_to_end(&mut bytes)?;
            if bytes.len() > LIMIT {
                return Err(error("Native clipboard output exceeds 1 MiB"));
            }
            String::from_utf8(bytes).map_err(|e| error(e.to_string()))
        })();
        let _ = tx.send(result);
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = child.try_wait()? {
            if !status.success() {
                return Err(error(format!(
                    "Native file service failed ({status}); check desktop support and installed tools"
                )));
            }
            return rx
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .map_err(|_| error("Native clipboard read timed out"))?;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error("Native file service timed out"));
        }
        thread::sleep(Duration::from_millis(10));
    }
}
#[cfg(target_os = "linux")]
pub fn write_clipboard(paths: &[PathBuf]) -> io::Result<()> {
    crate::x11_clipboard::publish(encode_paths(paths)?)
}
#[cfg(target_os = "linux")]
pub fn read_clipboard() -> io::Result<Vec<PathBuf>> {
    let c =
        if std::env::var_os("DISPLAY").is_none() && std::env::var_os("WAYLAND_DISPLAY").is_some() {
            let mut c = Command::new("wl-paste");
            c.args(["--no-newline", "--type", "text/uri-list"]);
            c
        } else {
            let mut c = Command::new("xclip");
            c.args(["-selection", "clipboard", "-t", "text/uri-list", "-o"]);
            c
        };
    decode_paths(&run(c, None)?)
}
#[cfg(target_os = "linux")]
pub fn trash(path: &Path) -> io::Result<()> {
    // GIO preserves restore metadata and reports unsupported mounts; never --force.
    let mut c = Command::new("gio");
    c.arg("trash").arg("--").arg(path);
    let status = c
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(error(format!(
            "Trash failed ({status}); source retained where possible. Permanent deletion was not attempted."
        )))
    }
}
#[cfg(target_os = "macos")]
fn jxa(script: &str, args: &[&std::ffi::OsStr]) -> io::Result<String> {
    let mut c = Command::new("/usr/bin/osascript");
    c.args(["-l", "JavaScript", "-e", script]);
    c.args(args);
    run(c, None)
}
#[cfg(target_os = "macos")]
pub fn trash(path: &Path) -> io::Result<()> {
    jxa("ObjC.import('Foundation'); function run(a) { var e=Ref(); if (!$.NSFileManager.defaultManager.trashItemAtURLResultingItemURLError($.NSURL.fileURLWithPath(a[0]),null,e)) throw Error(ObjC.unwrap(e[0].localizedDescription)); return ''; }",&[path.as_os_str()]).map(|_|())
}
#[cfg(target_os = "macos")]
pub fn write_clipboard(paths: &[PathBuf]) -> io::Result<()> {
    encode_paths(paths)?;
    let args: Vec<_> = paths.iter().map(|p| p.as_os_str()).collect();
    jxa("ObjC.import('AppKit'); function run(a) { var p=$.NSPasteboard.generalPasteboard; p.clearContents; var u=a.map(function(x){return $.NSURL.fileURLWithPath(x);}); if (!p.writeObjects(u)) throw Error('Cannot publish file clipboard'); p.addTypesOwner([$.NSPasteboardTypeString],null); if(!p.setStringForType(u.map(function(x){return ObjC.unwrap(x.absoluteString);}).join('\\r\\n'),$.NSPasteboardTypeString)) throw Error('Cannot publish clipboard text representation'); return ''; }",&args).map(|_|())
}
#[cfg(target_os = "macos")]
pub fn read_clipboard() -> io::Result<Vec<PathBuf>> {
    let text = jxa(
        "ObjC.import('AppKit'); function run() { var p=$.NSPasteboard.generalPasteboard; var u=p.readObjectsForClassesOptions([$.NSURL],$({NSPasteboardURLReadingFileURLsOnlyKey:true})); if(!u) throw Error('No local files in clipboard'); var r=[];for(var i=0;i<u.count;i++)r.push(ObjC.unwrap(u.objectAtIndex(i).absoluteString));return r.join('\\r\\n'); }",
        &[],
    )?;
    decode_paths(&text)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_urls_round_trip_exact_names_and_deduplicate() {
        let p = PathBuf::from("/tmp/- café 中 & % #.txt");
        let text = encode_paths(std::slice::from_ref(&p)).unwrap();
        assert!(text.contains("%25") && text.contains("%23"));
        assert_eq!(
            decode_paths(&format!("# comment\r\n{text}{text}")).unwrap(),
            vec![p]
        );
    }
    #[test]
    fn reject_remote_mixed_malformed_and_ambiguous_payloads() {
        for s in [
            "",
            "https://example.com/file",
            "file:///tmp/a\nftp://example.com/b",
            "file:///tmp/a%00b",
            "file:///tmp/a?query",
            "file:///tmp/a#fragment",
            "file://server/tmp/a",
            "file:///tmp/%ZZ",
            "file:///tmp/%a",
        ] {
            assert!(decode_paths(s).is_err(), "{s}");
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod native_tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    #[test]
    #[ignore = "Requires an isolated Xvfb/DBus session and disposable XDG_DATA_HOME"]
    fn linux_original_clipboard_interop_and_trash_restore() {
        let root =
            PathBuf::from(std::env::var_os("W10_NATIVE_ROOT").expect("Use tools/w10_native.py"));
        assert!(
            root.is_absolute()
                && root
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("W10-native-")
        );
        assert_eq!(
            std::env::var_os("XDG_DATA_HOME").unwrap(),
            root.join("data")
        );
        let java_cp = std::env::var("W10_JAVA_CP").unwrap();
        fs_create(&root);
        let path = root.join("- café 中 & % #.txt");
        std::fs::write(&path, "native clipboard document").unwrap();
        write_clipboard(std::slice::from_ref(&path)).unwrap();
        let external = Command::new("java")
            .args([
                "-Djava.awt.headless=false",
                "-cp",
                &java_cp,
                "W10ClipboardProbe",
                "read",
            ])
            .stderr(Stdio::null())
            .output()
            .unwrap();
        assert!(external.status.success());
        assert_eq!(
            String::from_utf8(external.stdout).unwrap().trim_end(),
            path.to_str().unwrap()
        );
        let source = root.join("external document.txt");
        std::fs::write(&source, "external clipboard").unwrap();
        let mut j = Command::new("java")
            .args([
                "-Djava.awt.headless=false",
                "-cp",
                &java_cp,
                "W10ClipboardProbe",
                "write",
            ])
            .arg(&source)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut ready = String::new();
        BufReader::new(j.stdout.take().unwrap())
            .read_line(&mut ready)
            .unwrap();
        assert_eq!(ready.trim(), "ready");
        let read = read_clipboard();
        let _ = j.stdin.take().unwrap().write_all(b"quit\n");
        let _ = j.wait();
        assert_eq!(read.unwrap(), vec![source]);
        let trash_path = root.join("restorable.txt");
        std::fs::write(&trash_path, "restore exact bytes").unwrap();
        trash(&trash_path).unwrap();
        assert!(!trash_path.exists());
        let data = root.join("data/Trash");
        assert_eq!(
            std::fs::read_to_string(data.join("files/restorable.txt")).unwrap(),
            "restore exact bytes"
        );
        assert!(
            std::fs::read_to_string(data.join("info/restorable.txt.trashinfo"))
                .unwrap()
                .contains("Path=")
        );
        let restored = Command::new("gio")
            .args(["trash", "--restore", "trash:///restorable.txt"])
            .status()
            .unwrap();
        assert!(restored.success());
        assert_eq!(
            std::fs::read_to_string(&trash_path).unwrap(),
            "restore exact bytes"
        );
    }
    fn fs_create(path: &Path) {
        std::fs::create_dir_all(path.join("data")).unwrap();
    }
}
