//! Images of a request: which ones are accepted, how they are named, and the base64 the app
//! uses to move them to and from the interface.

/// At most this many bytes per image.
pub const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;

/// The image types Weaver recognises by their first bytes. The ones a design request accepts are
/// PNG, JPEG, GIF and WebP ([`ImageKind::is_request_format`]); BMP and ICO are only shown by the
/// File view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageKind {
    Png,
    Jpeg,
    Gif,
    Webp,
    Bmp,
    Ico,
}

impl ImageKind {
    /// The extension a saved image gets.
    pub fn extension(self) -> &'static str {
        match self {
            ImageKind::Png => "png",
            ImageKind::Jpeg => "jpg",
            ImageKind::Gif => "gif",
            ImageKind::Webp => "webp",
            ImageKind::Bmp => "bmp",
            ImageKind::Ico => "ico",
        }
    }

    pub fn mime(self) -> &'static str {
        match self {
            ImageKind::Png => "image/png",
            ImageKind::Jpeg => "image/jpeg",
            ImageKind::Gif => "image/gif",
            ImageKind::Webp => "image/webp",
            ImageKind::Bmp => "image/bmp",
            ImageKind::Ico => "image/x-icon",
        }
    }

    /// Whether a design request accepts this type: PNG, JPEG, GIF and WebP only.
    pub fn is_request_format(self) -> bool {
        !matches!(self, ImageKind::Bmp | ImageKind::Ico)
    }

    /// The type a file extension names (case-insensitive), `None` when it is not accepted.
    pub fn from_extension(extension: &str) -> Option<Self> {
        match extension.to_ascii_lowercase().as_str() {
            "png" => Some(ImageKind::Png),
            "jpg" | "jpeg" => Some(ImageKind::Jpeg),
            "gif" => Some(ImageKind::Gif),
            "webp" => Some(ImageKind::Webp),
            _ => None,
        }
    }

    /// The type of the content, from its first bytes (the extension is never trusted).
    pub fn detect(bytes: &[u8]) -> Option<Self> {
        if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
            Some(ImageKind::Png)
        } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            Some(ImageKind::Jpeg)
        } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
            Some(ImageKind::Gif)
        } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
            Some(ImageKind::Webp)
        } else if bytes.starts_with(b"BM") {
            Some(ImageKind::Bmp)
        } else if bytes.starts_with(&[0x00, 0x00, 0x01, 0x00]) {
            Some(ImageKind::Ico)
        } else {
            None
        }
    }
}

/// Why an image was refused. The message is the toast the interface shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ImageError {
    #[error("Only PNG, JPEG, GIF or WebP images up to 10 MB.")]
    NotAccepted,
}

/// Checks size and type of an image's content for a design request: PNG, JPEG, GIF or WebP.
pub fn check(bytes: &[u8]) -> Result<ImageKind, ImageError> {
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(ImageError::NotAccepted);
    }
    ImageKind::detect(bytes)
        .filter(|kind| kind.is_request_format())
        .ok_or(ImageError::NotAccepted)
}

/// `image-<n>.<ext>`: the name of the saved image.
pub fn image_name(number: u32, kind: ImageKind) -> String {
    format!("image-{number}.{}", kind.extension())
}

/// The next free number among the names of the files already in the images folder
/// (`image-<n>.<ext>`: the highest number plus one, 1 when there is none).
pub fn next_number<'a>(names: impl IntoIterator<Item = &'a str>) -> u32 {
    names
        .into_iter()
        .filter_map(|name| {
            let stem = name.strip_prefix("image-")?.split('.').next()?;
            stem.parse::<u32>().ok()
        })
        .max()
        .map_or(1, |n| n + 1)
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Standard base64 with padding.
pub fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// Decodes standard base64 (padding required, no whitespace); `None` when it is not valid.
pub fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let bytes = text.as_bytes();
    if !bytes.len().is_multiple_of(4) {
        return None;
    }
    let value = |c: u8| -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some(u32::from(c - b'A')),
            b'a'..=b'z' => Some(u32::from(c - b'a') + 26),
            b'0'..=b'9' => Some(u32::from(c - b'0') + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    };
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    let chunks = bytes.len() / 4;
    for (index, chunk) in bytes.chunks(4).enumerate() {
        let last = index + 1 == chunks;
        let padding = chunk.iter().rev().take_while(|c| **c == b'=').count();
        if padding > 2 || (padding > 0 && !last) {
            return None;
        }
        let mut n = 0u32;
        for (i, c) in chunk.iter().enumerate() {
            let v = if i >= 4 - padding { 0 } else { value(*c)? };
            n = (n << 6) | v;
        }
        out.push((n >> 16) as u8);
        if padding < 2 {
            out.push((n >> 8) as u8);
        }
        if padding < 1 {
            out.push(n as u8);
        }
    }
    Some(out)
}

/// A link target safe inside `![](...)`: spaces and parentheses are percent-encoded.
pub fn link_escape(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for c in path.chars() {
        match c {
            ' ' => out.push_str("%20"),
            '(' => out.push_str("%28"),
            ')' => out.push_str("%29"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_types_are_recognised_by_their_content() {
        assert_eq!(
            ImageKind::detect(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0]),
            Some(ImageKind::Png)
        );
        assert_eq!(
            ImageKind::detect(&[0xFF, 0xD8, 0xFF, 0xE0]),
            Some(ImageKind::Jpeg)
        );
        assert_eq!(ImageKind::detect(b"GIF89a\x01"), Some(ImageKind::Gif));
        assert_eq!(ImageKind::detect(b"GIF87a\x01"), Some(ImageKind::Gif));
        assert_eq!(
            ImageKind::detect(b"RIFF\x00\x00\x00\x00WEBPVP8 "),
            Some(ImageKind::Webp)
        );
        assert_eq!(ImageKind::detect(b"RIFF\x00\x00\x00\x00WAVEfmt "), None);
        assert_eq!(
            ImageKind::detect(b"<svg xmlns='http://www.w3.org/2000/svg'/>"),
            None
        );
        assert_eq!(ImageKind::detect(b""), None);
    }

    #[test]
    fn a_name_extension_picks_a_type_only_for_the_accepted_ones() {
        assert_eq!(ImageKind::from_extension("PNG"), Some(ImageKind::Png));
        assert_eq!(ImageKind::from_extension("jpeg"), Some(ImageKind::Jpeg));
        assert_eq!(ImageKind::from_extension("jpg"), Some(ImageKind::Jpeg));
        assert_eq!(ImageKind::from_extension("svg"), None);
        assert_eq!(ImageKind::from_extension(""), None);
    }

    #[test]
    fn bmp_and_ico_are_recognised_by_content_but_no_request_accepts_them() {
        assert_eq!(ImageKind::detect(b"BM\x36\x00"), Some(ImageKind::Bmp));
        assert_eq!(ImageKind::detect(b"BA\x36\x00"), None);
        assert_eq!(
            ImageKind::detect(&[0x00, 0x00, 0x01, 0x00, 0x01]),
            Some(ImageKind::Ico)
        );
        assert_eq!(ImageKind::detect(&[0x00, 0x00, 0x02, 0x00]), None);
        assert_eq!(ImageKind::Bmp.mime(), "image/bmp");
        assert_eq!(ImageKind::Ico.mime(), "image/x-icon");
        assert!(!ImageKind::Bmp.is_request_format());
        assert!(!ImageKind::Ico.is_request_format());
        assert!(ImageKind::Webp.is_request_format());
        assert_eq!(check(b"BM\x36\x00"), Err(ImageError::NotAccepted));
        assert_eq!(
            check(&[0x00, 0x00, 0x01, 0x00, 0x01]),
            Err(ImageError::NotAccepted)
        );
        // A name never picks them either.
        assert_eq!(ImageKind::from_extension("bmp"), None);
        assert_eq!(ImageKind::from_extension("ico"), None);
    }

    #[test]
    fn size_and_type_are_checked() {
        let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(check(&png), Ok(ImageKind::Png));
        assert_eq!(check(b"nope"), Err(ImageError::NotAccepted));
        let mut big = png.to_vec();
        big.resize(MAX_IMAGE_BYTES, 0);
        assert_eq!(check(&big), Ok(ImageKind::Png));
        big.push(0);
        assert_eq!(check(&big), Err(ImageError::NotAccepted));
        assert_eq!(
            ImageError::NotAccepted.to_string(),
            "Only PNG, JPEG, GIF or WebP images up to 10 MB."
        );
    }

    #[test]
    fn the_next_number_is_the_highest_plus_one() {
        assert_eq!(next_number([]), 1);
        assert_eq!(next_number(["image-1.png", "image-2.jpg"]), 3);
        assert_eq!(
            next_number(["image-1.png", "image-7.gif", "notes.txt", "image-x.png"]),
            8
        );
        assert_eq!(image_name(3, ImageKind::Jpeg), "image-3.jpg");
    }

    #[test]
    fn base64_round_trips_every_length() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
        for len in 0..40usize {
            let data: Vec<u8> = (0..len).map(|i| (i * 37 + 11) as u8).collect();
            assert_eq!(base64_decode(&base64_encode(&data)).unwrap(), data, "{len}");
        }
        assert_eq!(base64_decode("Zm9vYmFy").unwrap(), b"foobar");
    }

    #[test]
    fn invalid_base64_is_refused() {
        for bad in ["Zm9", "Zm9v!", "Zg=a", "=Zg=", "Zg==Zm9v", "Z===", " Zm9v"] {
            assert_eq!(base64_decode(bad), None, "{bad}");
        }
    }

    #[test]
    fn link_targets_keep_markdown_intact() {
        assert_eq!(
            link_escape("../../my shots/a (1).png"),
            "../../my%20shots/a%20%281%29.png"
        );
    }
}
