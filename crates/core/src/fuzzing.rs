// SPDX-License-Identifier: GPL-3.0-only
//! Opt-in parser boundaries: no sockets, credentials, filesystem writes or GUI.
use std::io;

/// Exercise one bounded SFTP packet primitive, selected by the first input byte.
///
/// # Errors
/// Returns malformed/truncated/oversized-input errors; no network is involved.
pub fn sftp_packet(bytes: &[u8]) -> io::Result<()> {
    crate::sftp::exercise_packet(bytes)
}

/// Validate a decoded ZIP entry path using the production traversal/depth checks.
///
/// # Errors
/// Returns errors for unsupported paths or invalid UTF-8 without extracting files.
pub fn zip_path(bytes: &[u8]) -> io::Result<()> {
    if bytes.len() > 65_557 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Oversized path input",
        ));
    }
    let path =
        std::str::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    crate::archives::validate_path(path)
}

/// Parse a bounded ZIP directory tail using the production header checks.
///
/// # Errors
/// Returns an error if the end record, directory size or offsets are inconsistent.
pub fn zip_directory(bytes: &[u8]) -> io::Result<()> {
    crate::archives::directory_tail(bytes, bytes.len() as u64).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_inputs_are_bounded_and_side_effect_free() {
        for path in [
            b"../outside".as_slice(),
            b"/absolute",
            b"a//b",
            b"bad\\path",
            b"bad\0path",
            b"a/./b",
        ] {
            assert!(zip_path(path).is_err());
        }
        assert!(zip_path(b"folder/notes.txt").is_ok());
        for length in 0..128 {
            assert!(zip_directory(&vec![0; length]).is_err());
        }
        assert!(zip_directory(&vec![0; 65_558]).is_err());
        let mut record = vec![0u8; 22];
        record[..4].copy_from_slice(b"PK\x05\x06");
        assert!(zip_directory(&record).is_ok());
        record[4] = 1;
        assert!(zip_directory(&record).is_err());
    }
}
