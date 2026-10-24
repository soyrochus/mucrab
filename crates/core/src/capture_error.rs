// SPDX-License-Identifier: GPL-3.0-only
//! Actionable stale-capture failures without flattening underlying I/O errors.
use std::{
    fmt,
    path::{Path, PathBuf},
};

/// A previously reviewed filesystem identity no longer matches its source.
///
/// Operations retain `io::Result` and wrap this value in an `InvalidInput` error.
/// Callers can inspect `io::Error::get_ref` and downcast to distinguish a stale
/// review from ordinary permission or storage errors without matching text.
#[derive(Debug)]
pub struct CaptureChanged {
    path: PathBuf,
}
impl CaptureChanged {
    pub(crate) fn new(path: &Path) -> Self {
        Self {
            path: path.to_owned(),
        }
    }
    /// Source requiring a fresh capture or review.
    pub fn path(&self) -> &Path {
        &self.path
    }
}
impl fmt::Display for CaptureChanged {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "captured entry changed; refresh and review again: {}",
            self.path.display()
        )
    }
}
impl std::error::Error for CaptureChanged {}
