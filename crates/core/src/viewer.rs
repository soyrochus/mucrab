// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
//! Bounded read-only preparation. Viewer leases never own user or W09 paths.
use crate::properties::{EntryIdentity, entry_kind};
use crate::{
    CopyPolicy, EntryKind, EntryTarget, JobControl, ZipExtractPlan, extract_zip_with_control,
};
use image::{ImageDecoder, ImageFormat, ImageReader};
use std::{
    fs,
    io::{self, Cursor, Read},
    ops::Range,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
pub const TEXT_BYTES: usize = 16 * 1024 * 1024;
pub const IMAGE_BYTES: usize = 32 * 1024 * 1024;
pub const RGBA_BYTES: u64 = 64 * 1024 * 1024;
fn invalid(s: impl ToString) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, s.to_string())
}
fn checkpoint(control: &JobControl) -> io::Result<()> {
    if control.is_cancelled() {
        Err(io::Error::new(io::ErrorKind::Interrupted, "View cancelled"))
    } else {
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct ViewRequest {
    pub target: EntryTarget,
    identity: EntryIdentity,
    resolved: Option<(PathBuf, EntryIdentity)>,
}
impl ViewRequest {
    pub fn capture(target: EntryTarget) -> io::Result<Self> {
        let (identity, resolved) = match &target {
            EntryTarget::Local(path) => {
                let identity = EntryIdentity::read(path)?;
                let resolved = fs::canonicalize(path)?;
                let m = fs::symlink_metadata(&resolved)?;
                if entry_kind(&m) != EntryKind::File {
                    return Err(invalid("View requires a regular file or a link to one"));
                }
                let target_id = EntryIdentity::from_metadata(&resolved, &m)?;
                (identity, Some((resolved, target_id)))
            }
            EntryTarget::Zip { location, name } => {
                if !crate::is_valid_leaf_name(name) {
                    return Err(invalid("One regular ZIP child is required"));
                }
                (EntryIdentity::read(&location.archive)?, None)
            }
        };
        Ok(Self {
            target,
            identity,
            resolved,
        })
    }
    pub fn validate(&self) -> io::Result<()> {
        match &self.target {
            EntryTarget::Local(path) => {
                self.identity.validate(path)?;
                let (resolved, id) = self.resolved.as_ref().expect("local target");
                if fs::canonicalize(path)? != *resolved {
                    return Err(invalid("Link target changed"));
                }
                id.validate(resolved)
            }
            EntryTarget::Zip { location, .. } => self.identity.validate(&location.archive),
        }
    }
    pub fn prepare(&self, control: &JobControl, gpu_limit: u32) -> io::Result<Preview> {
        checkpoint(control)?;
        self.validate()?;
        let mut lease = None;
        let path = match &self.target {
            EntryTarget::Local(_) => self.resolved.as_ref().expect("resolved file").0.clone(),
            EntryTarget::Zip { location, name } => {
                let owned = ViewerLease::new()?;
                let plan = ZipExtractPlan::new(
                    &location.archive,
                    &location.prefix,
                    Some(std::slice::from_ref(name)),
                    &owned.path,
                    CopyPolicy::Skip,
                )?;
                if plan.expanded_bytes() > IMAGE_BYTES as u64 {
                    return Err(invalid("ZIP member exceeds the 32 MiB preview limit"));
                }
                let entries = crate::ZipCatalog::open(&location.archive)?.list(&location.prefix)?;
                if !entries
                    .iter()
                    .any(|e| e.name == *name && !e.is_directory && !e.is_symlink)
                {
                    return Err(invalid("View requires a regular ZIP member"));
                }
                let report = extract_zip_with_control(&plan, control)?;
                if !report.success || report.cancelled {
                    return Err(invalid(format!(
                        "ZIP preview extraction failed: {:?}",
                        report.issues
                    )));
                }
                let path = owned.path.join(name);
                lease = Some(owned);
                path
            }
        };
        checkpoint(control)?;
        let identity = EntryIdentity::read(&path)?;
        let mut file = fs::File::open(&path)?;
        let metadata = file.metadata()?;
        if entry_kind(&metadata) != EntryKind::File
            || EntryIdentity::from_metadata(&path, &metadata)? != identity
        {
            return Err(invalid("Preview source changed while opening"));
        }
        if metadata.len() > IMAGE_BYTES as u64 {
            return Err(invalid(
                "Input exceeds the 32 MiB preview limit; open externally",
            ));
        }
        control.entry(&path, Some(metadata.len()));
        let mut bytes = Vec::with_capacity(usize::try_from(metadata.len()).map_err(invalid)?);
        let mut buf = [0u8; 65536];
        loop {
            checkpoint(control)?;
            let n = file.read(&mut buf)?;
            if n == 0 {
                break;
            }
            if bytes.len() + n > IMAGE_BYTES {
                return Err(invalid("Input grew beyond the preview limit"));
            }
            bytes.extend_from_slice(&buf[..n]);
            control.advance(n as u64);
        }
        identity.validate(&path)?;
        self.validate()?;
        checkpoint(control)?;
        let content = decode_preview(&bytes, gpu_limit)?;
        checkpoint(control)?;
        identity.validate(&path)?;
        self.validate()?;
        control.completed();
        Ok(Preview {
            content,
            resolved_path: path,
            lease,
        })
    }
}
#[derive(Debug)]
pub struct ViewerLease {
    path: PathBuf,
}
impl ViewerLease {
    fn new() -> io::Result<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        for _ in 0..100 {
            let path = std::env::temp_dir().join(format!(
                "mucrab-viewer-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            match builder.create(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e),
            }
        }
        Err(invalid("Cannot allocate a private viewer lease"))
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
}
impl Drop for ViewerLease {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
#[derive(Debug)]
pub struct Preview {
    pub content: PreviewContent,
    pub resolved_path: PathBuf,
    pub lease: Option<ViewerLease>,
}
#[derive(Debug)]
pub enum PreviewContent {
    Text(TextDocument),
    Image(PreviewImage),
}
#[derive(Debug)]
pub struct PreviewImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub orientation: String,
}
#[derive(Debug)]
pub struct TextDocument {
    pub text: String,
    pub encoding: &'static str,
    pub lines: Vec<Range<usize>>,
    pub max_chars: usize,
}
impl TextDocument {
    pub fn decode(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() > TEXT_BYTES {
            return Err(invalid("Text exceeds 16 MiB; open externally"));
        }
        let (text, encoding) = if bytes.starts_with(&[0xff, 0xfe])
            || bytes.starts_with(&[0xfe, 0xff])
        {
            if !bytes.len().is_multiple_of(2) {
                return Err(invalid("Odd-length UTF-16 input"));
            }
            let le = bytes[0] == 0xff;
            let units = bytes[2..]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| {
                    if le {
                        u16::from_le_bytes([b[0], b[1]])
                    } else {
                        u16::from_be_bytes([b[0], b[1]])
                    }
                })
                .collect::<Vec<_>>();
            (
                String::from_utf16(&units).map_err(invalid)?,
                if le { "UTF-16 LE BOM" } else { "UTF-16 BE BOM" },
            )
        } else {
            let (bytes, label) = if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
                (&bytes[3..], "UTF-8 BOM")
            } else {
                (bytes, "UTF-8")
            };
            (
                std::str::from_utf8(bytes)
                    .map_err(|_| invalid("Unsupported encoding or invalid UTF-8; open externally"))?
                    .to_owned(),
                label,
            )
        };
        if text
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\t' | '\r' | '\n'))
        {
            return Err(invalid(
                "Binary/control data is not supported by the text viewer",
            ));
        }
        let mut lines = Vec::new();
        let mut offset = 0;
        let mut max_chars = 0;
        let data = text.as_bytes();
        let mut cursor = 0;
        while cursor < data.len() {
            let end = match data[cursor] {
                b'\r' => Some(cursor + 1 + usize::from(data.get(cursor + 1) == Some(&b'\n'))),
                b'\n' => Some(cursor + 1),
                _ => None,
            };
            cursor += 1;
            if let Some(end) = end {
                let line = &text[offset..end];
                if line.len() > 65536 {
                    return Err(invalid("Decoded line exceeds 64 KiB; open externally"));
                }
                if lines.len() >= 100000 {
                    return Err(invalid("Text exceeds 100,000 lines; open externally"));
                }
                max_chars = max_chars.max(line.chars().count());
                lines.push(offset..end);
                offset = end;
                cursor = end;
            }
        }
        if offset < text.len() {
            let line = &text[offset..];
            if line.len() > 65536 {
                return Err(invalid("Decoded line exceeds 64 KiB; open externally"));
            }
            if lines.len() >= 100000 {
                return Err(invalid("Text exceeds 100,000 lines; open externally"));
            }
            max_chars = max_chars.max(line.chars().count());
            lines.push(offset..text.len());
        }
        if lines.is_empty() {
            lines.push(0..0);
        }
        Ok(Self {
            text,
            encoding,
            lines,
            max_chars,
        })
    }
    pub fn find(&self, term: &str, from: usize, forward: bool) -> Option<Range<usize>> {
        if term.is_empty() {
            return None;
        }
        let from = from.min(self.text.len());
        let from = (0..=from).rev().find(|i| self.text.is_char_boundary(*i))?;
        let offset = if forward {
            self.text[from..]
                .find(term)
                .map(|i| i + from)
                .or_else(|| self.text[..from].find(term))
        } else {
            self.text[..from]
                .rfind(term)
                .or_else(|| self.text[from..].rfind(term).map(|i| i + from))
        }?;
        Some(offset..offset + term.len())
    }
    pub fn line_at(&self, offset: usize) -> usize {
        self.lines
            .partition_point(|r| r.end <= offset)
            .min(self.lines.len() - 1)
    }
}
fn validate_png(bytes: &[u8]) -> io::Result<()> {
    let mut offset = 8usize;
    while offset < bytes.len() {
        let header = bytes
            .get(offset..offset + 8)
            .ok_or_else(|| invalid("Truncated PNG chunk"))?;
        let length = u32::from_be_bytes(header[..4].try_into().expect("four bytes")) as usize;
        let end = offset
            .checked_add(12)
            .and_then(|v| v.checked_add(length))
            .ok_or_else(|| invalid("PNG chunk overflow"))?;
        let chunk = bytes
            .get(offset + 4..end - 4)
            .ok_or_else(|| invalid("Truncated PNG chunk data"))?;
        let crc = bytes
            .get(end - 4..end)
            .ok_or_else(|| invalid("Truncated PNG CRC"))?;
        if crc32fast::hash(chunk) != u32::from_be_bytes(crc.try_into().expect("CRC length")) {
            return Err(invalid("PNG chunk CRC mismatch"));
        }
        if &header[4..8] == b"IEND" {
            return if length == 0 && end == bytes.len() {
                Ok(())
            } else {
                Err(invalid("Invalid PNG end/trailing data"))
            };
        }
        offset = end;
    }
    Err(invalid("Truncated PNG (missing IEND)"))
}
pub fn decode_preview(bytes: &[u8], gpu_limit: u32) -> io::Result<PreviewContent> {
    let format = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(ImageFormat::Png)
    } else if bytes.starts_with(b"\xff\xd8") {
        Some(ImageFormat::Jpeg)
    } else {
        None
    };
    let Some(format) = format else {
        if bytes.starts_with(b"%PDF")
            || bytes.starts_with(b"GIF87a")
            || bytes.starts_with(b"GIF89a")
            || bytes.starts_with(b"RIFF")
        {
            return Err(invalid("Unsupported preview format; open externally"));
        }
        return TextDocument::decode(bytes).map(PreviewContent::Text);
    };
    if bytes.len() > IMAGE_BYTES {
        return Err(invalid("Encoded image exceeds 32 MiB"));
    }
    if format == ImageFormat::Png {
        validate_png(bytes)?;
    }
    if format == ImageFormat::Jpeg && !bytes.ends_with(b"\xff\xd9") {
        return Err(invalid("Truncated JPEG (missing final EOI marker)"));
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(gpu_limit.min(16384));
    limits.max_image_height = Some(gpu_limit.min(16384));
    limits.max_alloc = Some(RGBA_BYTES);
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    reader.limits(limits);
    let mut decoder = reader.into_decoder().map_err(invalid)?;
    let (w, h) = decoder.dimensions();
    if w == 0
        || h == 0
        || u64::from(w) * u64::from(h) * 4 > RGBA_BYTES
        || decoder.total_bytes() > RGBA_BYTES
    {
        return Err(invalid(
            "Decoded image exceeds the 64 MiB sample/RGBA limit",
        ));
    }
    let orientation = decoder.orientation().map_err(invalid)?;
    let mut image = if format == ImageFormat::Jpeg {
        let options = zune_core::options::DecoderOptions::default()
            .set_strict_mode(true)
            .jpeg_set_out_colorspace(zune_core::colorspace::ColorSpace::RGB)
            .set_max_width(gpu_limit.min(16384) as usize)
            .set_max_height(gpu_limit.min(16384) as usize);
        let mut jpeg = zune_jpeg::JpegDecoder::new_with_options(
            zune_core::bytestream::ZCursor::new(bytes),
            options,
        );
        jpeg.decode_headers()
            .map_err(|e| invalid(format!("JPEG header: {e:?}")))?;
        if jpeg.dimensions() != Some((w as usize, h as usize)) {
            return Err(invalid("JPEG dimensions disagree"));
        }
        let pixels = jpeg
            .decode()
            .map_err(|e| invalid(format!("JPEG decoding failed: {e:?}")))?;
        let rgb = image::RgbImage::from_raw(w, h, pixels)
            .ok_or_else(|| invalid("Unexpected JPEG output length"))?;
        image::DynamicImage::ImageRgb8(rgb)
    } else {
        image::DynamicImage::from_decoder(decoder).map_err(invalid)?
    };
    image.apply_orientation(orientation);
    let rgba = image.into_rgba8();
    let (width, height) = rgba.dimensions();
    Ok(PreviewContent::Image(PreviewImage {
        width,
        height,
        rgba: rgba.into_raw(),
        orientation: if orientation == image::metadata::Orientation::NoTransforms {
            "Original orientation".into()
        } else {
            "Orientation corrected".into()
        },
    }))
}
