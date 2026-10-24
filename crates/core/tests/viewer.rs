// SPDX-License-Identifier: GPL-3.0-only
//! Behavioural regression coverage for viewer using disposable fixtures.
mod support;
use mucrab_core::*;
use std::{
    fs,
    path::{Path, PathBuf},
};
static LEASE_TEST: std::sync::Mutex<()> = std::sync::Mutex::new(());
struct Tree(PathBuf);
impl Tree {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "mucrab-w14-{}-{}",
            std::process::id(),
            support::nonce()
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Tree {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
#[test]
fn frozen_text_cases_decode_exactly_with_newlines_and_wrapping_literal_find() {
    for payload in [
        include_str!("../../../fixtures/W14-built-in-viewer/cases.json"),
        include_str!("../../../fixtures/W14-built-in-viewer/reserved.json"),
    ] {
        let cases: Vec<serde_json::Value> = serde_json::from_str(payload).unwrap();
        for case in cases {
            if case["error"] == true {
                continue;
            }
            let text = case["text"].as_str().unwrap();
            let bytes = match case["encoding"].as_str().unwrap() {
                "utf8" => text.as_bytes().to_vec(),
                "utf8-bom" => [b"\xef\xbb\xbf".as_slice(), text.as_bytes()].concat(),
                encoding => {
                    let le = encoding == "utf16-le";
                    let mut bytes = if le {
                        vec![0xff, 0xfe]
                    } else {
                        vec![0xfe, 0xff]
                    };
                    for unit in text.encode_utf16() {
                        bytes.extend_from_slice(&if le {
                            unit.to_le_bytes()
                        } else {
                            unit.to_be_bytes()
                        });
                    }
                    bytes
                }
            };
            let doc = TextDocument::decode(&bytes).unwrap();
            assert_eq!(doc.text, text);
            assert_eq!(
                doc.lines
                    .iter()
                    .map(|r| &doc.text[r.clone()])
                    .collect::<String>(),
                text
            );
            for term in ["needle", "中", "é", "\r\n"] {
                let first = doc.find(term, 0, true);
                assert_eq!(first.as_ref().map(|r| r.start), text.find(term));
                if let Some(first) = first {
                    let last = doc.find(term, 0, false).unwrap();
                    assert_eq!(last.start, text.rfind(term).unwrap());
                    assert_eq!(&doc.text[first], term);
                }
            }
        }
    }
    let doc = TextDocument::decode(b"one\rtwo\nthree\r\n").unwrap();
    assert_eq!(doc.lines.len(), 3);
}
#[test]
fn binary_malformed_and_resource_limits_fail_without_replacement() {
    for bytes in [
        vec![0xff, 0],
        vec![0xff, 0xfe, 0x00],
        vec![0xff, 0xfe, 0x00, 0xd8],
        b"a\0b".to_vec(),
        vec![b'a'; 65537],
        vec![b'\n'; 100001],
        vec![b'a'; 16 * 1024 * 1024 + 1],
        b"%PDF-1.7".to_vec(),
        b"GIF89a".to_vec(),
        b"\xff\xd8bad".to_vec(),
        b"\x89PNG\r\n\x1a\ninvalid".to_vec(),
    ] {
        assert!(decode_preview(&bytes, 16384).is_err());
    }
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/W14-built-in-viewer/images/png-orientation-1.png");
    let mut bytes = fs::read(p).unwrap();
    assert!(decode_preview(&bytes, 4).is_err());
    bytes[40] ^= 0xff;
    assert!(decode_preview(&bytes, 16384).is_err());
}
fn original_coord(orientation: u8, x: u32, y: u32, w: u32, h: u32) -> (u32, u32) {
    match orientation {
        1 => (x, y),
        2 => (w - 1 - x, y),
        3 => (w - 1 - x, h - 1 - y),
        4 => (x, h - 1 - y),
        5 => (y, x),
        6 => (y, h - 1 - x),
        7 => (w - 1 - y, h - 1 - x),
        8 => (w - 1 - y, x),
        _ => unreachable!(),
    }
}
#[test]
fn png_exact_pixels_and_all_exif_orientations_and_jpeg_dimensions() {
    let folder =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/W14-built-in-viewer/images");
    for orientation in 1..=8 {
        for ext in ["png", "jpg"] {
            let bytes =
                fs::read(folder.join(format!("{ext}-orientation-{orientation}.{ext}"))).unwrap();
            let PreviewContent::Image(image) = decode_preview(&bytes, 16384).unwrap() else {
                panic!("image expected")
            };
            let (w, h) = if orientation <= 4 { (7, 5) } else { (5, 7) };
            assert_eq!((image.width, image.height), (w, h));
            assert_eq!(image.rgba.len(), w as usize * h as usize * 4);
            if ext == "png" {
                for y in 0..h {
                    for x in 0..w {
                        let (sx, sy) = original_coord(orientation, x, y, 7, 5);
                        let expected = [
                            (sx * 31) as u8,
                            (sy * 47) as u8,
                            ((sx + sy) * 19) as u8,
                            ((sx * 43 + sy * 29) % 256) as u8,
                        ];
                        let i = ((y * w + x) * 4) as usize;
                        assert_eq!(
                            &image.rgba[i..i + 4],
                            expected,
                            "orientation {orientation}, ({x},{y})"
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn local_source_revalidation_cancel_and_file_links_are_read_only() {
    let t = Tree::new();
    let path = t.0.join("file");
    fs::write(&path, b"original\r\n").unwrap();
    let request = ViewRequest::capture(EntryTarget::Local(path.clone())).unwrap();
    let preview = request.prepare(&JobControl::default(), 16384).unwrap();
    assert!(preview.lease.is_none());
    let PreviewContent::Text(doc) = preview.content else {
        panic!("text")
    };
    assert_eq!(doc.text, "original\r\n");
    let control = JobControl::default();
    control.cancel();
    assert!(request.prepare(&control, 16384).is_err());
    fs::write(&path, b"changed").unwrap();
    assert!(request.prepare(&JobControl::default(), 16384).is_err());
    #[cfg(unix)]
    {
        let link = t.0.join("link");
        std::os::unix::fs::symlink("file", &link).unwrap();
        let request = ViewRequest::capture(EntryTarget::Local(link.clone())).unwrap();
        assert!(request.prepare(&JobControl::default(), 16384).is_ok());
        fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink("missing", &link).unwrap();
        assert!(request.validate().is_err());
        assert!(ViewRequest::capture(EntryTarget::Local(link)).is_err());
    }
    assert_eq!(fs::read(path).unwrap(), b"changed");
}
#[test]
fn zip_viewer_lease_is_owned_and_drops_only_its_private_copy() {
    let _guard = LEASE_TEST.lock().unwrap();
    let archive = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/W09/open.zip");
    let before = fs::read(&archive).unwrap();
    let request = ViewRequest::capture(EntryTarget::Zip {
        location: ZipLocation {
            archive: archive.clone(),
            prefix: "nested/".into(),
        },
        name: "- café & report.txt".into(),
    })
    .unwrap();
    let preview = request.prepare(&JobControl::default(), 16384).unwrap();
    let directory = preview.lease.as_ref().unwrap().path().to_owned();
    assert!(directory.is_dir());
    assert!(preview.resolved_path.is_file());
    assert!(matches!(preview.content, PreviewContent::Text(_)));
    drop(preview);
    assert!(!directory.exists());
    assert_eq!(fs::read(archive).unwrap(), before);
}

#[test]
fn image_headers_encoded_limits_and_known_source_changes_are_rejected() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/W14-built-in-viewer/images/png-orientation-1.png");
    let bytes = fs::read(path).unwrap();
    for dimension in [20000u32, 8192] {
        let mut forged = bytes.clone();
        forged[16..20].copy_from_slice(&dimension.to_be_bytes());
        forged[20..24].copy_from_slice(&dimension.to_be_bytes());
        let crc = crc32fast::hash(&forged[12..29]);
        forged[29..33].copy_from_slice(&crc.to_be_bytes());
        assert!(decode_preview(&forged, 16384).is_err());
    }
    let jpeg = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/W14-built-in-viewer/images/jpg-orientation-1.jpg");
    let mut truncated = fs::read(jpeg).unwrap();
    truncated.truncate(truncated.len() / 2);
    truncated.extend_from_slice(b"\xff\xd9");
    assert!(decode_preview(&truncated, 16384).is_err());
    let t = Tree::new();
    let file = t.0.join("oversized");
    fs::File::create(&file)
        .unwrap()
        .set_len(32 * 1024 * 1024 + 1)
        .unwrap();
    let r = ViewRequest::capture(EntryTarget::Local(file)).unwrap();
    assert!(r.prepare(&JobControl::default(), 16384).is_err());
    let file = t.0.join("changes-during-read");
    fs::write(&file, vec![b'x'; 200000]).unwrap();
    let r = ViewRequest::capture(EntryTarget::Local(file.clone())).unwrap();
    let control = JobControl::with_observer(move |_, p| {
        if p.transferred_bytes == 65536 {
            fs::write(&file, b"replacement").unwrap();
        }
    });
    assert!(r.prepare(&control, 16384).is_err());
}
#[test]
fn failed_zip_crc_and_traversal_leave_no_viewer_lease_or_outside_file() {
    let _guard = LEASE_TEST.lock().unwrap();
    let leases = || {
        std::fs::read_dir(std::env::temp_dir())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with(&format!("mucrab-viewer-{}-", std::process::id()))
            })
            .map(|e| e.path())
            .collect::<std::collections::HashSet<_>>()
    };
    let before_leases = leases();
    use std::io::Write;
    let t = Tree::new();
    for (name, corrupt) in [("safe.txt", true), ("../escape.txt", false)] {
        let path = t.0.join(if corrupt { "crc.zip" } else { "unsafe.zip" });
        let mut zip = zip::ZipWriter::new(fs::File::create(&path).unwrap());
        zip.start_file(
            name,
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored),
        )
        .unwrap();
        zip.write_all(b"unique CRC payload").unwrap();
        zip.finish().unwrap();
        if corrupt {
            let mut data = fs::read(&path).unwrap();
            let i = data
                .windows(b"unique CRC payload".len())
                .position(|s| s == b"unique CRC payload")
                .unwrap();
            data[i] ^= 1;
            fs::write(&path, data).unwrap();
        }
        let before = fs::read(&path).unwrap();
        let request = ViewRequest::capture(EntryTarget::Zip {
            location: ZipLocation {
                archive: path.clone(),
                prefix: String::new(),
            },
            name: if corrupt { "safe.txt" } else { "escape.txt" }.into(),
        })
        .unwrap();
        assert!(request.prepare(&JobControl::default(), 16384).is_err());
        assert_eq!(fs::read(path).unwrap(), before);
        assert!(!t.0.join("escape.txt").exists());
    }
    assert_eq!(leases(), before_leases);
}
