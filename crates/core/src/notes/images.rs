//! The images of the notes sheet: which files are accepted, how they are named, copying them into
//! `.weaver/notes/`, reading one for display and moving the removed ones to the trash
//! (`requirements/rules#notes-view`, `requirements/tech#notes-view`).
//!
//! The decisions are pure functions ([`classify`], [`choose_name`], [`pasted_name`]); the rest is
//! synchronous `std` I/O on the canonical project root. The commands of the app take image names,
//! never paths: [`check_name`] is the one place a name is checked before `.weaver/notes/<name>` is
//! built from it. Nothing here deletes a file: the trash only receives images the developer
//! removed, one at a time.

use std::borrow::Cow;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use super::{notes_dir, SHEET_FILE};
use crate::assets;
use crate::file_view::{same_name, Refusal};
use crate::project::layout::{self, NOTES_DIR};
use crate::project::requests::id::LocalTime;
use crate::project::requests::image::{base64_encode, ImageKind};

/// An image is accepted up to this many bytes, the limit included: 10 MB.
pub const MAX_IMAGE_BYTES: u64 = 10 * 1024 * 1024;

/// What a refusal calls a pasted file that has no name.
pub const PASTED_LABEL: &str = "pasted image";

/// What a refusal calls a dropped file whose name cannot be read.
const UNNAMED_LABEL: &str = "dropped file";

/// The extensions of the image kinds of `requirements/ui#asset-view`, lowercase. A file with one of
/// them that is not accepted is refused; any other file is left out.
pub const IMAGE_EXTENSIONS: [&str; 12] = [
    "png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "ico", "tif", "tiff", "heic", "avif",
];

/// How many bytes at the start of a file are enough to recognise the signature of an accepted
/// image (WebP needs 12).
const HEAD_BYTES: usize = 16;

/// Why an image was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// Larger than [`MAX_IMAGE_BYTES`].
    TooLarge,
    /// Not a PNG, JPEG, GIF or WebP, by its signature.
    NotAccepted,
}

/// What [`classify`] decides about one file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// An accepted image of this kind.
    Accept(ImageKind),
    Refuse(Why),
    /// Not an image: no message.
    LeftOut,
}

/// What [`add`] did with one source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The image was copied under this name.
    Added { name: String },
    /// Refused; `source` is the name of the file, or [`PASTED_LABEL`].
    Refused { source: String, why: Why },
    /// The copy failed with this reason.
    Failed { source: String, message: String },
    /// Not an image (a folder, a link, any other file): nothing happens.
    LeftOut,
}

/// One image to add: the bytes of a pasted file, or the path of a file dropped from outside.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// A pasted file: its own name when it has one and its bytes. A file whose bytes could not be
    /// decoded carries the reason instead.
    Pasted {
        name: Option<String>,
        bytes: Result<Vec<u8>, String>,
    },
    /// A dropped file, by its absolute path. It is read with `symlink_metadata` and never
    /// followed when it is a link.
    Path(PathBuf),
}

// --- pure decisions -----------------------------------------------------------------------------

/// `Ok` when `name` may name an image of the notes folder: a name of the asset rules (no
/// separator, not empty) that does not start with `.` and is not the sheet.
pub fn check_name(name: &str) -> Result<(), Refusal> {
    assets::check_name(name)?;
    if same_name(name, SHEET_FILE) {
        return Err(Refusal::new(format!(
            "\"{name}\" is the sheet, not an image."
        )));
    }
    Ok(())
}

/// Whether `name` ends with the extension of an image kind (case-insensitive).
pub fn has_image_extension(name: &str) -> bool {
    name.rsplit_once('.').is_some_and(|(stem, extension)| {
        !stem.is_empty()
            && IMAGE_EXTENSIONS
                .iter()
                .any(|known| known.eq_ignore_ascii_case(extension))
    })
}

/// The accepted kind (PNG, JPEG, GIF or WebP) that the first bytes are the signature of.
pub fn accepted_kind(head: &[u8]) -> Option<ImageKind> {
    ImageKind::detect(head).filter(|kind| kind.is_request_format())
}

/// Decides one file from its length, its first bytes and its own name, when it has one:
///
/// * an accepted signature is added, up to [`MAX_IMAGE_BYTES`] inclusive and refused above;
/// * any other content under the extension of an image kind is refused as not accepted;
/// * anything else is not an image and is left out.
pub fn classify(len: u64, head: &[u8], name: Option<&str>) -> Verdict {
    match accepted_kind(head) {
        Some(_) if len > MAX_IMAGE_BYTES => Verdict::Refuse(Why::TooLarge),
        Some(kind) => Verdict::Accept(kind),
        None if name.is_some_and(has_image_extension) => Verdict::Refuse(Why::NotAccepted),
        None => Verdict::LeftOut,
    }
}

/// `pasted-YYYY-MM-DD-HHMMSS.<ext>`, with the extension of the signature.
pub fn pasted_name(time: &LocalTime, kind: ImageKind) -> String {
    format!(
        "pasted-{:04}-{:02}-{:02}-{:02}{:02}{:02}.{}",
        time.year,
        time.month,
        time.day,
        time.hour,
        time.minute,
        time.second,
        kind.extension()
    )
}

/// `name` with `-<n>` before its extension (`a.png` → `a-2.png`; a name with no extension gets it
/// at the end).
pub fn numbered(name: &str, n: u32) -> String {
    match name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => format!("{stem}-{n}.{extension}"),
        _ => format!("{name}-{n}"),
    }
}

/// The name a new image gets: its own name when it has one and it is a valid name, otherwise the
/// date and time; then `-2`, `-3`… until `taken` (which compares names as the file system does)
/// no longer holds.
pub fn choose_name(
    own: Option<&str>,
    time: &LocalTime,
    kind: ImageKind,
    taken: &dyn Fn(&str) -> bool,
) -> String {
    let base = match own {
        Some(name) if check_name(name).is_ok() => name.to_owned(),
        _ => pasted_name(time, kind),
    };
    if !taken(&base) {
        return base;
    }
    (2u32..)
        .map(|n| numbered(&base, n))
        .find(|candidate| !taken(candidate))
        .expect("an unbounded range always yields a free name")
}

// --- copying in ---------------------------------------------------------------------------------

/// Reads the first bytes of `file` and then the rest, never more than [`MAX_IMAGE_BYTES`] bytes in
/// all plus one (which tells that the file grew past the limit).
fn read_rest(file: File, mut bytes: Vec<u8>) -> io::Result<Vec<u8>> {
    let room = (MAX_IMAGE_BYTES + 1).saturating_sub(bytes.len() as u64);
    (&file).take(room).read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// A source read far enough to decide it.
struct Opened<'a> {
    /// The name shown in a refusal.
    label: String,
    /// The file's own name, when it has one.
    own: Option<String>,
    len: u64,
    head: Vec<u8>,
    /// What is left to read once the file is accepted.
    rest: Rest<'a>,
}

enum Rest<'a> {
    Bytes(&'a [u8]),
    File(File),
}

/// Opens one source. `Err(Outcome)` is the end of it: left out, or failed.
fn open(source: &Source) -> Result<Opened<'_>, Outcome> {
    match source {
        Source::Pasted { name, bytes } => {
            let label = name.clone().unwrap_or_else(|| PASTED_LABEL.to_owned());
            let bytes = bytes.as_ref().map_err(|message| Outcome::Failed {
                source: label.clone(),
                message: message.clone(),
            })?;
            Ok(Opened {
                label,
                own: name.clone(),
                len: bytes.len() as u64,
                head: bytes[..bytes.len().min(HEAD_BYTES)].to_vec(),
                rest: Rest::Bytes(bytes),
            })
        }
        Source::Path(path) => {
            let own = path
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned);
            let label = own.clone().unwrap_or_else(|| {
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| UNNAMED_LABEL.to_owned())
            });
            let failed = |error: io::Error| Outcome::Failed {
                source: label.clone(),
                message: error.to_string(),
            };
            let meta = fs::symlink_metadata(path).map_err(failed)?;
            if !meta.file_type().is_file() {
                return Err(Outcome::LeftOut);
            }
            let file = File::open(path).map_err(failed)?;
            let mut head = Vec::with_capacity(HEAD_BYTES);
            (&file)
                .take(HEAD_BYTES as u64)
                .read_to_end(&mut head)
                .map_err(failed)?;
            Ok(Opened {
                label,
                own,
                len: meta.len(),
                head,
                rest: Rest::File(file),
            })
        }
    }
}

/// The names in the notes folder, the sheet's included.
fn names_in(dir: &Path) -> io::Result<Vec<String>> {
    let mut names = vec![SHEET_FILE.to_owned()];
    for entry in fs::read_dir(dir)? {
        if let Ok(name) = entry?.file_name().into_string() {
            names.push(name);
        }
    }
    Ok(names)
}

/// Creates `dir/name` with `create_new` and writes `bytes` in it. A file that could not be
/// written completely is removed again.
fn create(dir: &Path, name: &str, bytes: &[u8]) -> io::Result<()> {
    let path = dir.join(name);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    let written = file.write_all(bytes).and_then(|()| file.flush());
    drop(file);
    if let Err(error) = written {
        let _ = fs::remove_file(&path);
        return Err(error);
    }
    Ok(())
}

/// The notes folder once an image is accepted: created, and the names it holds.
struct Folder {
    dir: PathBuf,
    taken: Vec<String>,
}

fn add_one(root: &Path, source: &Source, time: &LocalTime, folder: &mut Option<Folder>) -> Outcome {
    let Opened {
        label,
        own,
        len,
        head,
        rest,
    } = match open(source) {
        Ok(opened) => opened,
        Err(outcome) => return outcome,
    };
    let kind = match classify(len, &head, own.as_deref()) {
        Verdict::Accept(kind) => kind,
        Verdict::Refuse(why) => return Outcome::Refused { source: label, why },
        Verdict::LeftOut => return Outcome::LeftOut,
    };
    let failed = |error: io::Error| Outcome::Failed {
        source: label.clone(),
        message: error.to_string(),
    };
    let bytes: Cow<'_, [u8]> = match rest {
        Rest::Bytes(bytes) => Cow::Borrowed(bytes),
        Rest::File(file) => match read_rest(file, head) {
            Ok(bytes) if bytes.len() as u64 > MAX_IMAGE_BYTES => {
                return Outcome::Refused {
                    source: label,
                    why: Why::TooLarge,
                }
            }
            Ok(bytes) => Cow::Owned(bytes),
            Err(error) => return failed(error),
        },
    };
    if folder.is_none() {
        let dir = match layout::ensure_local_folder(root, NOTES_DIR) {
            Ok(dir) => dir,
            Err(error) => return failed(error),
        };
        match names_in(&dir) {
            Ok(taken) => *folder = Some(Folder { dir, taken }),
            Err(error) => return failed(error),
        }
    }
    let folder = folder.as_mut().expect("the folder was just made");
    loop {
        let name = choose_name(own.as_deref(), time, kind, &|candidate| {
            folder.taken.iter().any(|name| same_name(name, candidate))
        });
        match create(&folder.dir, &name, &bytes) {
            Ok(()) => {
                folder.taken.push(name.clone());
                return Outcome::Added { name };
            }
            // Made by someone else since the folder was listed: the next free name.
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                folder.taken.push(name);
            }
            Err(error) => return failed(error),
        }
    }
}

/// Adds each of `sources` to `.weaver/notes/` of the project at `root` (the canonical root), in
/// the order given, each decided on its own: a refusal or a failure does not stop the others. The
/// folder, with its `.gitignore`, is made by the first image that is added and never before. A
/// copy never replaces a file and never changes an original. The date and time of a name come
/// from `time` (local time).
pub fn add(root: &Path, sources: &[Source], time: &LocalTime) -> Vec<Outcome> {
    let mut folder = None;
    sources
        .iter()
        .map(|source| add_one(root, source, time, &mut folder))
        .collect()
}

// --- reading for display ------------------------------------------------------------------------

/// `data:<mime>;base64,<…>` for the image `name` of the notes folder, from one read of the file.
/// Only a regular file (a link is never followed) that is a PNG, JPEG, GIF or WebP by its
/// signature, at most [`MAX_IMAGE_BYTES`], is read.
pub fn read_data_url(root: &Path, name: &str) -> Result<String, Refusal> {
    check_name(name)?;
    let path = notes_dir(root).join(name);
    let unreadable = |error: io::Error| match error.kind() {
        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory => {
            Refusal::new(format!("\"{name}\" does not exist."))
        }
        _ => Refusal::new(format!("\"{name}\" cannot be read: {error}")),
    };
    let meta = fs::symlink_metadata(&path).map_err(unreadable)?;
    if !meta.file_type().is_file() {
        return Err(Refusal::new(format!("\"{name}\" is not an image.")));
    }
    if meta.len() > MAX_IMAGE_BYTES {
        return Err(Refusal::new(format!(
            "\"{name}\" is larger than 10 MB and cannot be shown."
        )));
    }
    let file = File::open(&path).map_err(unreadable)?;
    let bytes = read_rest(file, Vec::new()).map_err(unreadable)?;
    match accepted_kind(&bytes) {
        Some(_) if bytes.len() as u64 > MAX_IMAGE_BYTES => Err(Refusal::new(format!(
            "\"{name}\" is larger than 10 MB and cannot be shown."
        ))),
        Some(kind) => Ok(format!(
            "data:{};base64,{}",
            kind.mime(),
            base64_encode(&bytes)
        )),
        None => Err(Refusal::new(format!(
            "\"{name}\" is not a PNG, JPEG, GIF or WebP image."
        ))),
    }
}

// --- trash --------------------------------------------------------------------------------------

/// An image the trash refused (or that could not be sent there) and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    pub name: String,
    pub reason: String,
}

/// What [`trash`] did.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Trashed {
    /// The images that went to the trash, in the order asked.
    pub trashed: Vec<String>,
    /// The ones that stayed, with the reason. An image already gone is neither.
    pub refused: Vec<Refused>,
}

pub use crate::file_view::ops::system_trash;

/// Sends each of `names` to the trash on its own with `trash` (the `trash` crate in the app, a
/// fake in the tests): the ones the trash refuses stay, and the others are not put back. A name
/// that is not a name of [`check_name`], or that is not a regular file of the folder, is refused;
/// nothing else is ever touched, and no file is deleted any other way.
pub fn trash(
    root: &Path,
    names: &[String],
    trash: &dyn Fn(&Path) -> Result<(), String>,
) -> Trashed {
    let dir = notes_dir(root);
    let mut done = Trashed::default();
    let mut seen: Vec<&str> = Vec::with_capacity(names.len());
    for name in names {
        if seen.contains(&name.as_str()) {
            continue;
        }
        seen.push(name);
        let refuse = |reason: String| Refused {
            name: name.clone(),
            reason,
        };
        if let Err(refusal) = check_name(name) {
            done.refused.push(refuse(refusal.reason().to_owned()));
            continue;
        }
        let image = dir.join(name);
        match fs::symlink_metadata(&image) {
            Ok(meta) if meta.file_type().is_file() => match trash(&image) {
                Ok(()) => done.trashed.push(name.clone()),
                Err(reason) => done.refused.push(refuse(reason)),
            },
            Ok(_) => done
                .refused
                .push(refuse(format!("\"{name}\" is not an image."))),
            // Already gone: nothing to put in the trash.
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
                ) => {}
            Err(error) => done
                .refused
                .push(refuse(format!("it cannot be read: {error}"))),
        }
    }
    done
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::sheet;
    use crate::paths::canonical_root;

    fn time() -> LocalTime {
        LocalTime {
            year: 2026,
            month: 11,
            day: 4,
            hour: 9,
            minute: 5,
            second: 7,
        }
    }

    const PNG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

    /// A PNG signature followed by zeros, `len` bytes in all.
    fn png(len: usize) -> Vec<u8> {
        let mut bytes = PNG.to_vec();
        bytes.resize(len, 0);
        bytes
    }

    fn pasted(name: Option<&str>, bytes: Vec<u8>) -> Source {
        Source::Pasted {
            name: name.map(str::to_owned),
            bytes: Ok(bytes),
        }
    }

    fn project() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = canonical_root(dir.path()).unwrap();
        (dir, root)
    }

    fn names(root: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(notes_dir(root))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    fn added(outcome: &Outcome) -> &str {
        match outcome {
            Outcome::Added { name } => name,
            other => panic!("not added: {other:?}"),
        }
    }

    // --- pure: names ----------------------------------------------------------------------------

    #[test]
    fn a_valid_own_name_is_kept() {
        let name = choose_name(Some("diagram.png"), &time(), ImageKind::Png, &|_| false);
        assert_eq!(name, "diagram.png");
        let spaced = choose_name(Some("my photo.JPG"), &time(), ImageKind::Jpeg, &|_| false);
        assert_eq!(spaced, "my photo.JPG");
    }

    #[test]
    fn an_invalid_own_name_gives_the_date_and_time() {
        for own in [".hidden.png", "a/b.png", "a\\b.png", "", "notes.md"] {
            let name = choose_name(Some(own), &time(), ImageKind::Png, &|_| false);
            assert_eq!(name, "pasted-2026-11-04-090507.png", "{own:?}");
        }
    }

    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn the_sheet_name_in_another_case_is_invalid_where_the_file_system_ignores_case() {
        let name = choose_name(Some("NOTES.MD"), &time(), ImageKind::Png, &|_| false);
        assert_eq!(name, "pasted-2026-11-04-090507.png");
    }

    #[test]
    #[cfg(not(any(windows, target_os = "macos")))]
    fn the_sheet_name_in_another_case_is_kept_where_the_file_system_tells_case_apart() {
        let name = choose_name(Some("NOTES.MD"), &time(), ImageKind::Png, &|_| false);
        assert_eq!(name, "NOTES.MD");
    }

    #[test]
    fn no_own_name_gives_the_date_and_time_with_the_extension_of_the_signature() {
        for (kind, extension) in [
            (ImageKind::Png, "png"),
            (ImageKind::Jpeg, "jpg"),
            (ImageKind::Gif, "gif"),
            (ImageKind::Webp, "webp"),
        ] {
            assert_eq!(
                choose_name(None, &time(), kind, &|_| false),
                format!("pasted-2026-11-04-090507.{extension}")
            );
        }
        assert_eq!(
            pasted_name(&time(), ImageKind::Jpeg),
            "pasted-2026-11-04-090507.jpg"
        );
    }

    #[test]
    fn a_taken_name_gets_a_number_before_the_extension() {
        let used = ["a.png", "a-2.png"];
        let taken = |candidate: &str| used.contains(&candidate);
        assert_eq!(
            choose_name(Some("a.png"), &time(), ImageKind::Png, &taken),
            "a-3.png"
        );
        assert_eq!(numbered("a.png", 2), "a-2.png");
        assert_eq!(numbered("archive", 2), "archive-2");
        assert_eq!(numbered("a.b.png", 5), "a.b-5.png");
        let stamp = |candidate: &str| candidate == "pasted-2026-11-04-090507.png";
        assert_eq!(
            choose_name(None, &time(), ImageKind::Png, &stamp),
            "pasted-2026-11-04-090507-2.png"
        );
    }

    #[test]
    fn names_are_compared_as_the_file_system_compares_them() {
        // The same comparison with both file system rules, whatever system the test runs on.
        let insensitive = |candidate: &str| candidate.to_lowercase() == "photo.png";
        assert_eq!(
            choose_name(Some("PHOTO.png"), &time(), ImageKind::Png, &insensitive),
            "PHOTO-2.png"
        );
        let sensitive = |candidate: &str| candidate == "photo.png";
        assert_eq!(
            choose_name(Some("PHOTO.png"), &time(), ImageKind::Png, &sensitive),
            "PHOTO.png"
        );
    }

    #[test]
    fn names_with_a_separator_a_leading_dot_or_the_sheet_name_are_refused() {
        for bad in [
            "a/b.png",
            "a\\b.png",
            "../x.png",
            "/etc/passwd",
            ".gitignore",
            ".hidden.png",
            "",
            ".",
            "..",
            "notes.md",
        ] {
            assert!(check_name(bad).is_err(), "{bad:?}");
        }
        assert!(check_name("a.png").is_ok());
        assert!(check_name("my photo.png").is_ok());
    }

    // --- pure: kinds and limit ------------------------------------------------------------------

    #[test]
    fn png_jpeg_gif_and_webp_are_accepted_by_their_signature() {
        let webp = *b"RIFF\0\0\0\0WEBP";
        for (head, kind) in [
            (&PNG[..], ImageKind::Png),
            (&[0xFF, 0xD8, 0xFF, 0xE0][..], ImageKind::Jpeg),
            (&b"GIF89a"[..], ImageKind::Gif),
            (&b"GIF87a"[..], ImageKind::Gif),
            (&webp[..], ImageKind::Webp),
        ] {
            assert_eq!(classify(100, head, Some("x")), Verdict::Accept(kind));
            assert_eq!(accepted_kind(head), Some(kind));
        }
        // The extension is never trusted.
        assert_eq!(
            classify(100, &PNG, Some("x.txt")),
            Verdict::Accept(ImageKind::Png)
        );
        assert_eq!(classify(100, &PNG, None), Verdict::Accept(ImageKind::Png));
    }

    #[test]
    fn bmp_and_ico_are_not_accepted() {
        assert_eq!(accepted_kind(b"BM\0\0"), None);
        assert_eq!(accepted_kind(&[0, 0, 1, 0]), None);
    }

    #[test]
    fn the_limit_is_ten_megabytes_included() {
        let limit = MAX_IMAGE_BYTES;
        assert_eq!(limit, 10 * 1024 * 1024);
        assert_eq!(
            classify(limit, &PNG, Some("a.png")),
            Verdict::Accept(ImageKind::Png)
        );
        assert_eq!(
            classify(limit + 1, &PNG, Some("a.png")),
            Verdict::Refuse(Why::TooLarge)
        );
    }

    #[test]
    fn a_file_of_an_image_kind_that_is_not_accepted_is_refused_and_any_other_is_left_out() {
        for name in [
            "a.svg", "a.BMP", "a.ico", "a.tif", "a.tiff", "a.heic", "a.avif", "a.png", "a.JPG",
        ] {
            assert_eq!(
                classify(100, b"not an image", Some(name)),
                Verdict::Refuse(Why::NotAccepted),
                "{name}"
            );
        }
        for name in ["a.txt", "a", ".png", "a.png.txt", "notes.md"] {
            assert_eq!(
                classify(100, b"not an image", Some(name)),
                Verdict::LeftOut,
                "{name}"
            );
        }
        assert_eq!(classify(100, b"text", None), Verdict::LeftOut);
        // Too large, but not an image either way: left out when it has no image extension.
        assert_eq!(
            classify(MAX_IMAGE_BYTES + 1, b"text", Some("a.zip")),
            Verdict::LeftOut
        );
    }

    // --- against folders ------------------------------------------------------------------------

    #[test]
    fn the_first_image_creates_the_folder_with_its_gitignore_and_nothing_else_does() {
        let (_dir, root) = project();
        let outcomes = add(
            &root,
            &[pasted(Some("notes.txt"), b"text".to_vec())],
            &time(),
        );
        assert_eq!(outcomes, [Outcome::LeftOut]);
        assert!(!notes_dir(&root).exists());

        let outcomes = add(&root, &[pasted(Some("a.png"), png(32))], &time());
        assert_eq!(added(&outcomes[0]), "a.png");
        assert_eq!(names(&root), [".gitignore", "a.png"]);
        assert_eq!(
            fs::read(notes_dir(&root).join(".gitignore")).unwrap(),
            b"*\n"
        );
        assert_eq!(fs::read(notes_dir(&root).join("a.png")).unwrap(), png(32));
    }

    #[test]
    fn a_pasted_file_without_a_name_is_named_by_the_date_and_time() {
        let (_dir, root) = project();
        let outcomes = add(
            &root,
            &[pasted(None, png(16)), pasted(None, png(16))],
            &time(),
        );
        assert_eq!(added(&outcomes[0]), "pasted-2026-11-04-090507.png");
        assert_eq!(added(&outcomes[1]), "pasted-2026-11-04-090507-2.png");
    }

    #[test]
    fn a_copy_never_replaces_a_file() {
        let (_dir, root) = project();
        fs::create_dir_all(notes_dir(&root)).unwrap();
        fs::write(notes_dir(&root).join("a.png"), b"mine").unwrap();
        let outcomes = add(
            &root,
            &[
                pasted(Some("a.png"), png(16)),
                pasted(Some("a.png"), png(24)),
            ],
            &time(),
        );
        assert_eq!(added(&outcomes[0]), "a-2.png");
        assert_eq!(added(&outcomes[1]), "a-3.png");
        assert_eq!(fs::read(notes_dir(&root).join("a.png")).unwrap(), b"mine");
        assert_eq!(fs::read(notes_dir(&root).join("a-3.png")).unwrap(), png(24));
    }

    #[test]
    fn an_image_named_like_the_sheet_never_replaces_it() {
        let (_dir, root) = project();
        sheet::write(&root, "# keep me\n").unwrap();
        let outcomes = add(&root, &[pasted(Some("notes.md"), png(16))], &time());
        assert_ne!(added(&outcomes[0]), "notes.md");
        assert_eq!(
            fs::read_to_string(notes_dir(&root).join("notes.md")).unwrap(),
            "# keep me\n"
        );
    }

    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn names_that_differ_by_case_are_taken_where_the_file_system_ignores_case() {
        let (_dir, root) = project();
        let outcomes = add(
            &root,
            &[
                pasted(Some("Photo.png"), png(16)),
                pasted(Some("PHOTO.PNG"), png(16)),
            ],
            &time(),
        );
        assert_eq!(added(&outcomes[0]), "Photo.png");
        assert_eq!(added(&outcomes[1]), "PHOTO-2.PNG");
    }

    #[test]
    fn dropped_files_are_copied_and_the_originals_stay() {
        let (_dir, root) = project();
        let outside = tempfile::tempdir().unwrap();
        let original = outside.path().join("shot.png");
        fs::write(&original, png(40)).unwrap();
        let outcomes = add(&root, &[Source::Path(original.clone())], &time());
        assert_eq!(added(&outcomes[0]), "shot.png");
        assert_eq!(fs::read(&original).unwrap(), png(40));
        assert_eq!(
            fs::read(notes_dir(&root).join("shot.png")).unwrap(),
            png(40)
        );
    }

    #[test]
    fn folders_and_missing_files_and_other_files_are_left_out_or_failed_and_the_rest_is_added() {
        let (_dir, root) = project();
        let outside = tempfile::tempdir().unwrap();
        let folder = outside.path().join("pics.png");
        fs::create_dir(&folder).unwrap();
        let text = outside.path().join("readme.txt");
        fs::write(&text, "text").unwrap();
        let svg = outside.path().join("logo.svg");
        fs::write(&svg, "<svg/>").unwrap();
        let good = outside.path().join("good.png");
        fs::write(&good, png(20)).unwrap();
        let missing = outside.path().join("gone.png");

        let outcomes = add(
            &root,
            &[
                Source::Path(folder),
                Source::Path(text),
                Source::Path(svg),
                Source::Path(missing),
                Source::Path(good),
            ],
            &time(),
        );
        assert_eq!(outcomes[0], Outcome::LeftOut);
        assert_eq!(outcomes[1], Outcome::LeftOut);
        assert_eq!(
            outcomes[2],
            Outcome::Refused {
                source: "logo.svg".into(),
                why: Why::NotAccepted
            }
        );
        assert!(matches!(
            &outcomes[3],
            Outcome::Failed { source, .. } if source == "gone.png"
        ));
        assert_eq!(added(&outcomes[4]), "good.png");
        assert_eq!(names(&root), [".gitignore", "good.png"]);
    }

    #[cfg(unix)]
    #[test]
    fn a_link_is_left_out_and_never_followed() {
        let (_dir, root) = project();
        let outside = tempfile::tempdir().unwrap();
        let target = outside.path().join("real.png");
        fs::write(&target, png(20)).unwrap();
        let link = outside.path().join("link.png");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        let outcomes = add(&root, &[Source::Path(link)], &time());
        assert_eq!(outcomes, [Outcome::LeftOut]);
        assert!(!notes_dir(&root).exists());
    }

    #[test]
    fn an_image_over_the_limit_is_refused_and_one_at_the_limit_is_added() {
        let (_dir, root) = project();
        let outside = tempfile::tempdir().unwrap();
        let at = outside.path().join("at.png");
        fs::write(&at, png(MAX_IMAGE_BYTES as usize)).unwrap();
        let over = outside.path().join("over.png");
        fs::write(&over, png(MAX_IMAGE_BYTES as usize + 1)).unwrap();
        let outcomes = add(
            &root,
            &[
                Source::Path(over),
                Source::Path(at),
                pasted(Some("big.png"), png(MAX_IMAGE_BYTES as usize + 1)),
            ],
            &time(),
        );
        assert_eq!(
            outcomes[0],
            Outcome::Refused {
                source: "over.png".into(),
                why: Why::TooLarge
            }
        );
        assert_eq!(added(&outcomes[1]), "at.png");
        assert_eq!(
            outcomes[2],
            Outcome::Refused {
                source: "big.png".into(),
                why: Why::TooLarge
            }
        );
        assert_eq!(names(&root), [".gitignore", "at.png"]);
    }

    #[test]
    fn a_refused_pasted_file_without_a_name_is_called_a_pasted_image() {
        let (_dir, root) = project();
        let outcomes = add(
            &root,
            &[pasted(None, png(MAX_IMAGE_BYTES as usize + 1))],
            &time(),
        );
        assert_eq!(
            outcomes[0],
            Outcome::Refused {
                source: PASTED_LABEL.into(),
                why: Why::TooLarge
            }
        );
        assert!(!notes_dir(&root).exists());
    }

    #[test]
    fn a_pasted_file_that_could_not_be_decoded_fails_alone() {
        let (_dir, root) = project();
        let outcomes = add(
            &root,
            &[
                Source::Pasted {
                    name: Some("a.png".into()),
                    bytes: Err("not base64".into()),
                },
                pasted(Some("b.png"), png(16)),
            ],
            &time(),
        );
        assert_eq!(
            outcomes[0],
            Outcome::Failed {
                source: "a.png".into(),
                message: "not base64".into()
            }
        );
        assert_eq!(added(&outcomes[1]), "b.png");
    }

    #[test]
    fn a_copy_that_fails_reports_its_reason_and_leaves_nothing() {
        let (_dir, root) = project();
        // `.weaver/notes` is a file: the folder cannot be made.
        fs::create_dir_all(layout::weaver_dir(&root)).unwrap();
        fs::write(notes_dir(&root), "in the way").unwrap();
        let outcomes = add(&root, &[pasted(Some("a.png"), png(16))], &time());
        assert!(matches!(
            &outcomes[0],
            Outcome::Failed { source, .. } if source == "a.png"
        ));
        assert_eq!(fs::read(notes_dir(&root)).unwrap(), b"in the way");
    }

    // --- reading for display --------------------------------------------------------------------

    #[test]
    fn an_image_is_read_as_a_data_url_with_the_mime_type_of_its_signature() {
        let (_dir, root) = project();
        add(&root, &[pasted(Some("a.png"), png(16))], &time());
        // The extension is not the type.
        fs::write(notes_dir(&root).join("b.png"), b"GIF89a....").unwrap();
        let url = read_data_url(&root, "a.png").unwrap();
        assert_eq!(
            url,
            format!("data:image/png;base64,{}", base64_encode(&png(16)))
        );
        assert!(read_data_url(&root, "b.png")
            .unwrap()
            .starts_with("data:image/gif;base64,"));
    }

    #[test]
    fn a_name_that_is_missing_or_not_an_accepted_image_is_an_error() {
        let (_dir, root) = project();
        add(&root, &[pasted(Some("a.png"), png(16))], &time());
        fs::write(notes_dir(&root).join("t.png"), b"text").unwrap();
        fs::write(notes_dir(&root).join("m.bmp"), b"BM....").unwrap();
        fs::create_dir(notes_dir(&root).join("dir.png")).unwrap();
        for name in [
            "gone.png",
            "t.png",
            "m.bmp",
            "dir.png",
            "notes.md",
            ".gitignore",
        ] {
            assert!(read_data_url(&root, name).is_err(), "{name}");
        }
        let missing_folder = project();
        assert!(read_data_url(&missing_folder.1, "a.png").is_err());
    }

    #[test]
    fn a_name_with_a_separator_never_reads_outside_the_folder() {
        let (_dir, root) = project();
        fs::write(root.join("secret.png"), png(16)).unwrap();
        sheet::write(&root, "x").unwrap();
        for name in [
            "../../secret.png",
            "../notes/../../secret.png",
            "/secret.png",
        ] {
            assert!(read_data_url(&root, name).is_err(), "{name}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_link_in_the_folder_is_never_followed_when_reading() {
        let (_dir, root) = project();
        fs::create_dir_all(notes_dir(&root)).unwrap();
        fs::write(root.join("secret.png"), png(16)).unwrap();
        std::os::unix::fs::symlink(root.join("secret.png"), notes_dir(&root).join("l.png"))
            .unwrap();
        assert!(read_data_url(&root, "l.png").is_err());
    }

    #[test]
    fn an_image_over_the_limit_is_not_read_for_display() {
        let (_dir, root) = project();
        fs::create_dir_all(notes_dir(&root)).unwrap();
        fs::write(
            notes_dir(&root).join("big.png"),
            png(MAX_IMAGE_BYTES as usize + 1),
        )
        .unwrap();
        fs::write(
            notes_dir(&root).join("ok.png"),
            png(MAX_IMAGE_BYTES as usize),
        )
        .unwrap();
        assert!(read_data_url(&root, "big.png").is_err());
        assert!(read_data_url(&root, "ok.png").is_ok());
    }

    // --- trash ----------------------------------------------------------------------------------

    fn strings(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    /// A trash that deletes what it takes, as the real one removes it from the folder.
    fn fake_trash(refuse: &'static str) -> impl Fn(&Path) -> Result<(), String> {
        move |path: &Path| {
            if path.file_name().unwrap() == refuse {
                Err("the trash is full".to_owned())
            } else {
                fs::remove_file(path).map_err(|error| error.to_string())
            }
        }
    }

    #[test]
    fn the_trash_takes_each_image_on_its_own_and_a_refusal_stays() {
        let (_dir, root) = project();
        add(
            &root,
            &[
                pasted(Some("a.png"), png(16)),
                pasted(Some("b.png"), png(16)),
                pasted(Some("c.png"), png(16)),
                pasted(Some("kept.png"), png(16)),
            ],
            &time(),
        );
        sheet::write(&root, "text").unwrap();
        let done = trash(
            &root,
            &strings(&["a.png", "b.png", "a.png", "c.png"]),
            &fake_trash("b.png"),
        );
        assert_eq!(done.trashed, ["a.png", "c.png"]);
        assert_eq!(
            done.refused,
            [Refused {
                name: "b.png".into(),
                reason: "the trash is full".into()
            }]
        );
        assert_eq!(
            names(&root),
            [".gitignore", "b.png", "kept.png", "notes.md"]
        );
    }

    #[test]
    fn the_trash_refuses_names_that_are_not_images_of_the_folder_and_touches_nothing_else() {
        let (_dir, root) = project();
        sheet::write(&root, "text").unwrap();
        fs::create_dir(notes_dir(&root).join("dir.png")).unwrap();
        fs::write(root.join("outside.png"), png(16)).unwrap();
        let ask = strings(&[
            "notes.md",
            ".gitignore",
            "../../outside.png",
            "a/b.png",
            "dir.png",
        ]);
        let done = trash(&root, &ask, &fake_trash(""));
        assert!(done.trashed.is_empty());
        assert_eq!(done.refused.len(), ask.len());
        assert!(root.join("outside.png").exists());
        assert_eq!(names(&root), [".gitignore", "dir.png", "notes.md"]);
    }

    #[test]
    fn an_image_that_is_already_gone_is_neither_trashed_nor_refused() {
        let (_dir, root) = project();
        let done = trash(&root, &strings(&["gone.png"]), &fake_trash(""));
        assert_eq!(done, Trashed::default());
    }
}
