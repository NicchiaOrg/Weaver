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

use crate::project::requests::image::{base64_encode, ImageKind, MAX_IMAGE_BYTES};

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

/// An image Weaver can show (`requirements/rules#file-tab`), as the UI gets it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageRead {
    /// `data:<mime>;base64,<…>`.
    pub data_url: String,
    /// The size of the file in bytes.
    pub size: u64,
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
    /// The image a binary file is, when Weaver can show it. Handed to the UI at once: a tab keeps
    /// none of it.
    pub image: Option<ImageRead>,
    /// A binary file with an image signature that is larger than 10 MB.
    pub image_too_large: bool,
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

/// The image a binary file is, when its first bytes are the signature of one Weaver can show and it
/// is at most 10 MB; the second value tells a file with such a signature that is larger.
fn image_of(bytes: &[u8]) -> (Option<ImageRead>, bool) {
    match ImageKind::detect(bytes) {
        Some(kind) if bytes.len() <= MAX_IMAGE_BYTES => (
            Some(ImageRead {
                data_url: format!("data:{};base64,{}", kind.mime(), base64_encode(bytes)),
                size: bytes.len() as u64,
            }),
            false,
        ),
        Some(_) => (None, true),
        None => (None, false),
    }
}

/// What a tab keeps of a file as it read it, from `bytes`, of the given kind of place.
pub fn from_bytes(bytes: &[u8], inside_weaver: bool) -> FileRead {
    let hash = sha256_bytes(bytes);
    let bom = has_bom(bytes);
    if is_binary(bytes) {
        let (image, image_too_large) = image_of(bytes);
        return FileRead {
            hash,
            bom,
            line_ending: LineEnding::Lf,
            text: None,
            read_only: None,
            image,
            image_too_large,
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
        image: None,
        image_too_large: false,
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

    /// A binary file that starts with `signature`, `len` bytes long.
    fn with_signature(signature: &[u8], len: usize) -> Vec<u8> {
        let mut bytes = signature.to_vec();
        bytes.resize(len, 0);
        bytes
    }

    #[test]
    fn a_binary_file_with_an_image_signature_is_an_image_of_that_type() {
        let cases: [(&[u8], &str); 6] = [
            (
                &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A],
                "image/png",
            ),
            (&[0xFF, 0xD8, 0xFF, 0xE0], "image/jpeg"),
            (b"GIF87a", "image/gif"),
            (b"GIF89a", "image/gif"),
            (b"RIFF\0\0\0\0WEBP", "image/webp"),
            (b"BM", "image/bmp"),
        ];
        for (signature, mime) in cases {
            let bytes = with_signature(signature, 40);
            let read = from_bytes(&bytes, false);
            let image = read.image.as_ref().expect(mime);
            assert!(read.is_binary() && read.text.is_none(), "{mime}");
            assert!(!read.image_too_large, "{mime}");
            assert_eq!(image.size, 40, "{mime}");
            assert_eq!(
                image.data_url,
                format!("data:{mime};base64,{}", base64_encode(&bytes)),
                "{mime}"
            );
        }
        let ico = from_bytes(&with_signature(&[0, 0, 1, 0], 40), false);
        assert!(ico
            .image
            .unwrap()
            .data_url
            .starts_with("data:image/x-icon;base64,"));
    }

    #[test]
    fn an_image_is_shown_up_to_ten_mebibytes_and_binary_with_a_reason_above() {
        let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        let at_limit = from_bytes(&with_signature(&png, MAX_IMAGE_BYTES), false);
        assert_eq!(at_limit.image.map(|image| image.size), Some(10_485_760));
        assert!(!at_limit.image_too_large);

        let over = from_bytes(&with_signature(&png, MAX_IMAGE_BYTES + 1), false);
        assert!(over.is_binary());
        assert_eq!(over.image, None);
        assert!(over.image_too_large);
    }

    #[test]
    fn any_other_binary_file_and_any_text_file_is_never_an_image() {
        let other = from_bytes(&[1, 0, 2, 3], false);
        assert!(other.is_binary() && other.image.is_none() && !other.image_too_large);
        // The signature is looked at only for binary files: this one is text.
        let text = from_bytes(b"BM is not a bitmap here", false);
        assert!(!text.is_binary() && text.image.is_none() && !text.image_too_large);
        let svg = from_bytes(b"<svg xmlns='http://www.w3.org/2000/svg'/>", false);
        assert!(svg.image.is_none());
    }
}
