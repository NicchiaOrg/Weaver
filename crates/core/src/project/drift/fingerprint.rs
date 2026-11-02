//! The text fingerprint of the references of a Drift (`requirements/rules#drift`).
//!
//! SHA-256 (lowercase hex) over the references in order, each as a line `# <ref>` followed by the
//! text `trama model show <ref>` prints, which holds the element or section and every
//! Requirement section titled with it, LF-normalised. A reference that is gone adds `(missing)`.
//! This is the one function a dismissal and a prepared question both use: the fingerprint says
//! whether the Model and the Requirements a Drift concerns have changed since.

use crate::project::cards::Card;
use crate::project::image::ProjectImage;
use crate::project::model_view;
use crate::project::text;

/// The prefix of the fingerprint as it is stored in files.
pub const PREFIX: &str = "sha256:";

/// The text the fingerprint of `refs` is taken from.
pub fn text_of(image: &ProjectImage, refs: &[String]) -> String {
    let mut out = String::new();
    for reference in refs {
        out.push_str(&format!("# {reference}\n"));
        match model_view::show(image, Some(reference)) {
            Ok((view, _)) => {
                out.push_str(view.trim_end_matches('\n'));
                out.push('\n');
            }
            Err(_) => out.push_str("(missing)\n"),
        }
    }
    text::normalize(&out)
}

/// The fingerprint of `refs`: 64 lowercase hex characters.
pub fn of_refs(image: &ProjectImage, refs: &[String]) -> String {
    text::sha256_hex(&text_of(image, refs))
}

/// The fingerprint as a file stores it: `sha256:<hex>`.
pub fn stored_of_refs(image: &ProjectImage, refs: &[String]) -> String {
    format!("{PREFIX}{}", of_refs(image, refs))
}

/// The stored fingerprint of the references of `drift` in `image`.
pub fn stored_of(image: &ProjectImage, drift: &Card) -> String {
    stored_of_refs(image, &super::refs_of(drift))
}
