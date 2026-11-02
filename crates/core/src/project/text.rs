//! The one function that reads project text, and the one that writes it.
//!
//! On read a BOM is dropped and CRLF becomes LF, before the text is parsed, hashed or
//! compared. On write the content is UTF-8 without BOM, LF-only, ends with a newline and goes
//! to disk atomically.

use std::fs;
use std::io;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::atomic;

/// Why a file could not be read as text.
#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("the file is not valid UTF-8")]
    NotUtf8,
}

/// Drops a leading BOM and turns every CRLF into LF.
pub fn normalize(text: &str) -> String {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    if text.contains("\r\n") {
        text.replace("\r\n", "\n")
    } else {
        text.to_owned()
    }
}

/// Reads `path` as normalized text (see [`normalize`]). Every read of a project file goes
/// through this function.
pub fn read_text(path: &Path) -> Result<String, ReadError> {
    decode(&fs::read(path)?)
}

/// Decodes bytes as normalized text.
pub fn decode(bytes: &[u8]) -> Result<String, ReadError> {
    let text = std::str::from_utf8(bytes).map_err(|_| ReadError::NotUtf8)?;
    Ok(normalize(text))
}

/// SHA-256 of normalized text, as lowercase hex.
pub fn sha256_hex(normalized: &str) -> String {
    let digest = Sha256::digest(normalized.as_bytes());
    let mut hex = String::with_capacity(64);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// The bytes written for `content`: normalized, with exactly one final newline.
pub fn file_content(content: &str) -> String {
    let mut text = normalize(content);
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text
}

/// Writes `content` to `path` as UTF-8 without BOM, LF-only and with a final newline,
/// atomically.
pub fn write_text(path: &Path, content: &str) -> io::Result<()> {
    atomic::write(path, file_content(content).as_bytes(), None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_drops_the_bom_and_turns_crlf_into_lf() {
        assert_eq!(normalize("\u{feff}a\r\nb\r\n"), "a\nb\n");
        assert_eq!(normalize("a\nb"), "a\nb");
        assert_eq!(normalize("a\rb"), "a\rb");
        assert_eq!(normalize("a\u{feff}b"), "a\u{feff}b");
    }

    #[test]
    fn hashes_agree_across_line_endings_and_bom() {
        let lf = normalize("one\ntwo\n");
        let crlf = normalize("\u{feff}one\r\ntwo\r\n");
        assert_eq!(sha256_hex(&lf), sha256_hex(&crlf));
        assert_eq!(
            sha256_hex(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn written_content_has_one_final_newline() {
        assert_eq!(file_content("a"), "a\n");
        assert_eq!(file_content("a\n"), "a\n");
        assert_eq!(file_content("a\r\nb"), "a\nb\n");
        assert_eq!(file_content("\u{feff}a"), "a\n");
    }

    #[test]
    fn read_removes_bom_and_crlf_and_write_is_lf_without_bom() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f.md");
        fs::write(&path, b"\xEF\xBB\xBFline one\r\nline two\r\n").unwrap();
        assert_eq!(read_text(&path).unwrap(), "line one\nline two\n");

        write_text(&path, &read_text(&path).unwrap()).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"line one\nline two\n");
    }

    #[test]
    fn invalid_utf8_is_an_error_not_a_lossy_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.md");
        fs::write(&path, [0x66, 0xFF, 0xFE]).unwrap();
        assert!(matches!(read_text(&path), Err(ReadError::NotUtf8)));
        assert!(matches!(
            read_text(&dir.path().join("missing")),
            Err(ReadError::Io(_))
        ));
    }
}
