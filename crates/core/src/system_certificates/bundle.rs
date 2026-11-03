//! The bundle: the PEM text built from DER certificates (`requirements/rules#system-certificates`,
//! "Certificates kept" and "No authority").
//!
//! Pure: no file, no clock, no system call.

use std::collections::HashSet;

use x509_parser::parse_x509_certificate;

use super::NO_AUTHORITY;

const BEGIN: &str = "-----BEGIN CERTIFICATE-----";
const END: &str = "-----END CERTIFICATE-----";
const LINE_LENGTH: usize = 64;

/// The text of a bundle and the number of certificates in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bundle {
    /// PEM `CERTIFICATE` blocks only, LF line endings, no keys.
    pub pem: String,
    pub count: usize,
}

/// Builds the bundle from the DER certificates the system trusts, then the public roots.
///
/// A certificate goes in only when it parses as X.509 and its basic constraints say it is a
/// certificate authority; the others are skipped without failing. Identical certificates (same
/// DER bytes) appear once, at their first place. When the certificates of the system give no
/// authority the result is [`NO_AUTHORITY`], even though the public roots alone would fill a
/// bundle.
pub fn build_bundle<S, P>(system: &[S], public: &[P]) -> Result<Bundle, String>
where
    S: AsRef<[u8]>,
    P: AsRef<[u8]>,
{
    let mut kept = Kept::default();
    let mut system_authorities = 0;
    for der in system {
        system_authorities += usize::from(kept.keep(der.as_ref()));
    }
    if system_authorities == 0 {
        return Err(NO_AUTHORITY.to_owned());
    }
    for der in public {
        kept.keep(der.as_ref());
    }
    Ok(Bundle {
        pem: kept.pem,
        count: kept.count,
    })
}

/// The certificates kept so far: the DER already seen, the PEM text and how many blocks it has.
#[derive(Default)]
struct Kept<'a> {
    seen: HashSet<&'a [u8]>,
    pem: String,
    count: usize,
}

impl<'a> Kept<'a> {
    /// Adds `der` at the end when it is new and an authority; says whether it was added.
    fn keep(&mut self, der: &'a [u8]) -> bool {
        if !self.seen.insert(der) || !is_authority(der) {
            return false;
        }
        push_block(&mut self.pem, der);
        self.count += 1;
        true
    }
}

/// How many certificates a bundle text holds: its `BEGIN CERTIFICATE` lines.
pub fn count_certificates(pem: &str) -> usize {
    pem.lines().filter(|line| line.trim_end() == BEGIN).count()
}

/// Whether `der` is an X.509 certificate whose basic constraints say it is an authority.
fn is_authority(der: &[u8]) -> bool {
    let Ok((_, certificate)) = parse_x509_certificate(der) else {
        return false;
    };
    matches!(certificate.basic_constraints(), Ok(Some(basic)) if basic.value.ca)
}

fn push_block(out: &mut String, der: &[u8]) {
    out.push_str(BEGIN);
    out.push('\n');
    let encoded = base64(der);
    for line in encoded.as_bytes().chunks(LINE_LENGTH) {
        out.push_str(std::str::from_utf8(line).expect("base64 is ASCII"));
        out.push('\n');
    }
    out.push_str(END);
    out.push('\n');
}

/// Standard base64 with padding.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        let sextet = |shift: u32| ALPHABET[(n >> shift & 0x3f) as usize] as char;
        out.push(sextet(18));
        out.push(sextet(12));
        out.push(if chunk.len() > 1 { sextet(6) } else { '=' });
        out.push(if chunk.len() > 2 { sextet(0) } else { '=' });
    }
    out
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// A DER certificate made by hand: only what `x509-parser` reads (a v3 certificate with the
    /// given common name as issuer and subject and, optionally, basic constraints).
    pub fn certificate(name: &str, authority: Option<bool>) -> Vec<u8> {
        fn der(tag: u8, content: &[u8]) -> Vec<u8> {
            let mut out = vec![tag];
            let length = content.len();
            if length < 128 {
                out.push(length as u8);
            } else if length < 256 {
                out.extend([0x81, length as u8]);
            } else {
                out.extend([0x82, (length >> 8) as u8, length as u8]);
            }
            out.extend(content);
            out
        }
        let sequence = |parts: &[Vec<u8>]| der(0x30, &parts.concat());
        let sha256_rsa = sequence(&[
            der(
                0x06,
                &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b],
            ),
            vec![0x05, 0x00],
        ]);
        let common_name = sequence(&[der(
            0x31,
            &sequence(&[der(0x06, &[0x55, 0x04, 0x03]), der(0x0c, name.as_bytes())]),
        )]);
        let validity = sequence(&[der(0x17, b"250101000000Z"), der(0x17, b"350101000000Z")]);
        let public_key = sequence(&[
            sequence(&[
                der(
                    0x06,
                    &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01],
                ),
                vec![0x05, 0x00],
            ]),
            der(0x03, &[0x00, 0x01, 0x02, 0x03]),
        ]);
        let mut fields = vec![
            der(0xa0, &der(0x02, &[0x02])),
            der(0x02, &[0x01]),
            sha256_rsa.clone(),
            common_name.clone(),
            validity,
            common_name,
            public_key,
        ];
        if let Some(ca) = authority {
            let basic = if ca {
                sequence(&[der(0x01, &[0xff])])
            } else {
                sequence(&[])
            };
            let extension = sequence(&[der(0x06, &[0x55, 0x1d, 0x13]), der(0x04, &basic)]);
            fields.push(der(0xa3, &sequence(&[extension])));
        }
        let to_be_signed = sequence(&fields);
        sequence(&[to_be_signed, sha256_rsa, der(0x03, &[0x00, 0xaa, 0xbb])])
    }

    fn blocks(pem: &str) -> Vec<String> {
        pem.split(END)
            .filter(|part| part.contains(BEGIN))
            .map(|part| part.trim().to_owned())
            .collect()
    }

    #[test]
    fn the_hand_made_certificates_are_read_as_x509() {
        assert!(is_authority(&certificate("A", Some(true))));
        assert!(!is_authority(&certificate("A", Some(false))));
        assert!(!is_authority(&certificate("A", None)));
    }

    #[test]
    fn base64_matches_the_standard_encoding() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64(&[0xfb, 0xff, 0xfe]), "+//+");
    }

    #[test]
    fn the_pem_has_64_character_lines_lf_endings_and_nothing_but_certificates() {
        let der = certificate("Long Authority Name For A Longer Certificate", Some(true));
        assert!(der.len() > 96, "{}", der.len());
        let bundle = build_bundle(std::slice::from_ref(&der), &Vec::<Vec<u8>>::new()).unwrap();
        assert_eq!(bundle.count, 1);
        assert!(!bundle.pem.contains('\r'));
        assert!(bundle.pem.ends_with("-----END CERTIFICATE-----\n"));
        let lines: Vec<&str> = bundle.pem.lines().collect();
        assert_eq!(lines[0], BEGIN);
        assert_eq!(lines[lines.len() - 1], END);
        let body = &lines[1..lines.len() - 1];
        assert!(body.len() >= 2);
        for line in &body[..body.len() - 1] {
            assert_eq!(line.len(), 64);
        }
        assert!(body.last().unwrap().len() <= 64);
        assert_eq!(body.concat(), base64(&der));
        assert!(!bundle.pem.contains("PRIVATE"));
    }

    #[test]
    fn certificates_with_the_same_der_appear_once_at_their_first_place() {
        let a = certificate("A", Some(true));
        let b = certificate("B", Some(true));
        let c = certificate("C", Some(true));
        let bundle =
            build_bundle(&[a.clone(), b.clone(), a.clone()], &[c.clone(), b.clone()]).unwrap();
        assert_eq!(bundle.count, 3);
        let expected: Vec<String> = [&a, &b, &c]
            .iter()
            .map(|der| {
                let one = build_bundle(&[(*der).clone()], &Vec::<Vec<u8>>::new()).unwrap();
                blocks(&one.pem).remove(0)
            })
            .collect();
        assert_eq!(blocks(&bundle.pem), expected);
    }

    #[test]
    fn unreadable_and_non_authority_certificates_are_skipped_without_failing() {
        let ca = certificate("CA", Some(true));
        let leaf = certificate("Leaf", Some(false));
        let no_constraints = certificate("Plain", None);
        let garbage = vec![0x30, 0x03, 0x01, 0x02];
        let bundle = build_bundle(
            &[
                garbage.clone(),
                leaf,
                ca.clone(),
                no_constraints,
                Vec::new(),
            ],
            &[garbage],
        )
        .unwrap();
        assert_eq!(bundle.count, 1);
        assert_eq!(bundle.pem.matches(BEGIN).count(), 1);
        assert_eq!(count_certificates(&bundle.pem), 1);
    }

    #[test]
    fn no_authority_in_the_system_fails_even_when_the_public_roots_would_fill_a_bundle() {
        let public = [certificate("Public", Some(true))];
        let leaf = certificate("Leaf", Some(false));
        assert_eq!(
            build_bundle(&Vec::<Vec<u8>>::new(), &public),
            Err("no certificate authority found".to_owned())
        );
        assert_eq!(
            build_bundle(&[leaf, vec![1, 2, 3]], &public),
            Err(NO_AUTHORITY.to_owned())
        );
    }

    #[test]
    fn the_public_roots_come_after_the_system_ones() {
        let system = [
            certificate("System 1", Some(true)),
            certificate("System 2", Some(true)),
        ];
        let public = [certificate("Public", Some(true))];
        let bundle = build_bundle(&system, &public).unwrap();
        assert_eq!(bundle.count, 3);
        let one = |der: &Vec<u8>| {
            blocks(
                &build_bundle(std::slice::from_ref(der), &Vec::<Vec<u8>>::new())
                    .unwrap()
                    .pem,
            )
            .remove(0)
        };
        assert_eq!(
            blocks(&bundle.pem),
            [one(&system[0]), one(&system[1]), one(&public[0])]
        );
    }

    #[test]
    fn a_public_root_the_system_already_has_is_not_repeated() {
        let shared = certificate("Shared", Some(true));
        let bundle =
            build_bundle(std::slice::from_ref(&shared), std::slice::from_ref(&shared)).unwrap();
        assert_eq!(bundle.count, 1);
    }

    #[test]
    fn the_real_public_roots_all_parse_and_are_authorities() {
        let roots: Vec<&[u8]> = webpki_root_certs::TLS_SERVER_ROOT_CERTS
            .iter()
            .map(|der| der.as_ref())
            .collect();
        assert!(roots.len() > 100);
        let system = [certificate("System", Some(true))];
        let bundle = build_bundle(&system, &roots).unwrap();
        assert_eq!(bundle.count, 1 + roots.len());
        assert_eq!(count_certificates(&bundle.pem), bundle.count);
    }

    #[test]
    fn certificates_are_counted_by_their_begin_lines() {
        assert_eq!(count_certificates(""), 0);
        assert_eq!(count_certificates("junk"), 0);
        let text = format!("{BEGIN}\nAAAA\n{END}\n{BEGIN}\nBBBB\n{END}\n");
        assert_eq!(count_certificates(&text), 2);
        assert_eq!(count_certificates(&text.replace('\n', "\r\n")), 2);
    }
}
