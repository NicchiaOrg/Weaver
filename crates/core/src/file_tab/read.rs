//! Reading a file for a tab: the raw bytes are read once, and everything the tab keeps comes
//! from them (`requirements/rules#file-tab`).
//!
//! The functions here are pure, apart from [`read`], which reads the file once. They do not use
//! `project::text`, which is for `.weaver/` and changes line endings, byte order marks and final
//! line breaks.

use std::fs;
use std::io;
use std::path::Path;

use sha2::{Digest, Sha256};

/// How many bytes at the start of a file are searched for a NUL byte.
pub const BINARY_SCAN_BYTES: usize = 8_192;

/// The largest file a tab opens with colours and for editing: 2 MB.
pub const MAX_EDITABLE_BYTES: u64 = 2_097_152;

const BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// The line ending a file is written back with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    CrLf,
}

/// Why the text of a tab cannot be changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadOnly {
    /// Larger than [`MAX_EDITABLE_BYTES`]: also shown without colours.
    TooLarge,
    /// Inside `.weaver/`, which Weaver writes itself.
    Weaver,
}

/// What a tab keeps of a file as it read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRead {
    /// SHA-256 of the raw bytes, as lowercase hex.
    pub hash: String,
    /// Whether the bytes began with a UTF-8 byte order mark.
    pub bom: bool,
    pub line_ending: LineEnding,
    /// `None` for a binary file: no text goes to the UI.
    pub text: Option<String>,
    pub read_only: Option<ReadOnly>,
}

impl FileRead {
    pub fn is_binary(&self) -> bool {
        self.text.is_none()
    }
}

/// SHA-256 of raw bytes, as lowercase hex.
pub fn sha256_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// Whether `bytes` start with a UTF-8 byte order mark.
pub fn has_bom(bytes: &[u8]) -> bool {
    bytes.starts_with(&BOM)
}

/// A file is binary when its first 8,192 bytes hold a NUL byte, or when its bytes, after an
/// optional UTF-8 byte order mark, are not valid UTF-8.
pub fn is_binary(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(BINARY_SCAN_BYTES)];
    if head.contains(&0) {
        return true;
    }
    let body = bytes.strip_prefix(&BOM[..]).unwrap_or(bytes);
    std::str::from_utf8(body).is_err()
}

/// Whether a file of `len` bytes is too large to edit: more than 2 MB.
pub fn is_too_large(len: u64) -> bool {
    len > MAX_EDITABLE_BYTES
}

/// The line ending a file is written back with: the one used most, counting CRLF and lone LF;
/// LF on a tie or when there is no line break.
pub fn choose_line_ending(text: &str) -> LineEnding {
    let (mut crlf, mut lf) = (0usize, 0usize);
    let mut previous = 0u8;
    for &byte in text.as_bytes() {
        if byte == b'\n' {
            if previous == b'\r' {
                crlf += 1;
            } else {
                lf += 1;
            }
        }
        previous = byte;
    }
    if crlf > lf {
        LineEnding::CrLf
    } else {
        LineEnding::Lf
    }
}

/// The text the UI gets: no byte order mark, LF line endings.
pub fn to_ui_text(text: &str) -> String {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    text.replace("\r\n", "\n")
}

/// The bytes written for the text of a tab: its line ending, and the byte order mark when the file
/// had one. Nothing else changes, the final line break included.
pub fn to_disk_bytes(text: &str, line_ending: LineEnding, bom: bool) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(text.len() + 3);
    if bom {
        bytes.extend_from_slice(&BOM);
    }
    match line_ending {
        LineEnding::Lf => bytes.extend_from_slice(text.replace("\r\n", "\n").as_bytes()),
        LineEnding::CrLf => {
            bytes.extend_from_slice(text.replace("\r\n", "\n").replace('\n', "\r\n").as_bytes())
        }
    }
    bytes
}

/// What a tab keeps of `bytes`, read from a file of the given kind of place.
pub fn from_bytes(bytes: &[u8], inside_weaver: bool) -> FileRead {
    let hash = sha256_bytes(bytes);
    let bom = has_bom(bytes);
    if is_binary(bytes) {
        return FileRead {
            hash,
            bom,
            line_ending: LineEnding::Lf,
            text: None,
            read_only: None,
        };
    }
    // Valid UTF-8, checked by `is_binary`.
    let body = std::str::from_utf8(bytes).unwrap_or_default();
    let read_only = if is_too_large(bytes.len() as u64) {
        Some(ReadOnly::TooLarge)
    } else if inside_weaver {
        Some(ReadOnly::Weaver)
    } else {
        None
    };
    FileRead {
        hash,
        bom,
        line_ending: choose_line_ending(body),
        text: Some(to_ui_text(body)),
        read_only,
    }
}

/// Reads the file once. `inside_weaver` tells that it lies inside `.weaver/`.
pub fn read(path: &Path, inside_weaver: bool) -> io::Result<FileRead> {
    Ok(from_bytes(&fs::read(path)?, inside_weaver))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_nul_byte_in_the_first_8192_bytes_makes_a_file_binary() {
        assert!(is_binary(b"ab\0cd"));
        let mut late = vec![b'a'; BINARY_SCAN_BYTES];
        late.push(0);
        assert!(!is_binary(&late), "a NUL after the scanned head is text");
        let mut inside = vec![b'a'; BINARY_SCAN_BYTES - 1];
        inside.push(0);
        assert!(is_binary(&inside));
    }

    #[test]
    fn bytes_that_are_not_utf8_make_a_file_binary_after_an_optional_bom() {
        assert!(is_binary(&[0xff, 0xfe, b'a']));
        assert!(is_binary(&[0xEF, 0xBB, 0xBF, 0xff]));
        assert!(!is_binary("caffè ☕".as_bytes()));
        assert!(!is_binary(&[0xEF, 0xBB, 0xBF, b'a']));
        assert!(!is_binary(b""));
    }

    #[test]
    fn a_file_is_too_large_only_above_two_mebibytes() {
        assert!(!is_too_large(2_097_152));
        assert!(is_too_large(2_097_153));
        assert!(!is_too_large(0));
    }

    #[test]
    fn the_line_ending_is_the_most_used_with_lf_on_a_tie() {
        assert_eq!(choose_line_ending("a\r\nb\r\nc\n"), LineEnding::CrLf);
        assert_eq!(choose_line_ending("a\nb\nc\r\n"), LineEnding::Lf);
        assert_eq!(choose_line_ending("a\r\nb\n"), LineEnding::Lf);
        assert_eq!(choose_line_ending("no break"), LineEnding::Lf);
        assert_eq!(choose_line_ending(""), LineEnding::Lf);
        assert_eq!(choose_line_ending("a\rb\rc"), LineEnding::Lf);
    }

    #[test]
    fn the_ui_text_has_no_bom_and_lf_endings() {
        assert_eq!(to_ui_text("\u{feff}a\r\nb\nc\r\n"), "a\nb\nc\n");
    }

    #[test]
    fn the_bytes_written_keep_the_ending_the_bom_and_the_final_break() {
        assert_eq!(to_disk_bytes("a\nb", LineEnding::CrLf, false), b"a\r\nb");
        assert_eq!(to_disk_bytes("a\nb\n", LineEnding::Lf, false), b"a\nb\n");
        assert_eq!(
            to_disk_bytes("a\nb\n", LineEnding::CrLf, true),
            b"\xEF\xBB\xBFa\r\nb\r\n"
        );
        assert_eq!(to_disk_bytes("", LineEnding::CrLf, false), b"");
        // A CRLF that slipped into the text is not doubled.
        assert_eq!(to_disk_bytes("a\r\nb", LineEnding::CrLf, false), b"a\r\nb");
    }

    #[test]
    fn a_read_keeps_the_hash_of_the_raw_bytes_the_bom_and_the_ending() {
        let bytes = b"\xEF\xBB\xBFone\r\ntwo\r\n";
        let read = from_bytes(bytes, false);
        assert_eq!(read.hash, sha256_bytes(bytes));
        assert!(read.bom);
        assert_eq!(read.line_ending, LineEnding::CrLf);
        assert_eq!(read.text.as_deref(), Some("one\ntwo\n"));
        assert_eq!(read.read_only, None);
    }

    #[test]
    fn a_binary_file_has_a_hash_and_no_text() {
        let read = from_bytes(&[1, 0, 2], false);
        assert!(read.is_binary());
        assert_eq!(read.hash, sha256_bytes(&[1, 0, 2]));
    }

    #[test]
    fn too_large_and_weaver_files_are_read_only() {
        let large = vec![b'a'; 2_097_153];
        assert_eq!(
            from_bytes(&large, false).read_only,
            Some(ReadOnly::TooLarge)
        );
        assert_eq!(from_bytes(b"x", true).read_only, Some(ReadOnly::Weaver));
        assert_eq!(from_bytes(b"x", false).read_only, None);
    }

    #[test]
    fn the_hash_is_the_sha256_of_the_bytes() {
        assert_eq!(
            sha256_bytes(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
