//! The developer's assets: the files he keeps in `.weaver/assets/` for the agent (the Machine
//! `asset-view`, `assets` Feature).
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`list`] | The assets of a project: names and sizes from one `read_dir`, never opening a file |
//! | [`copy_in`] | Copying files dropped from outside into the assets folder, all or nothing |
//! | [`ops`] | Rename, delete to the trash and the paths of a drag out, each checked when it runs |
//!
//! An asset is a regular file directly in `.weaver/assets/` whose name does not start with `.`.
//! The folder is flat, versioned with the project (no `.gitignore`) and created when the first
//! asset is copied in, never earlier. Weaver reads the content of an asset only to copy it in.
//!
//! Everything here is synchronous `std` I/O on the canonical project root; the app runs it on the
//! blocking pool. The commands take asset names, never paths: [`check_name`] is the one place a
//! name is checked before `.weaver/assets/<name>` is built from it.

pub mod copy_in;
pub mod list;
pub mod ops;
pub mod tags;

use std::path::{Path, PathBuf};

use serde::Serialize;
use ts_rs::TS;

use crate::file_view::{names, Refusal};
use crate::project::layout;

/// An asset is larger than the limit above 10 MB (10 × 1024 × 1024 bytes), the figure of the
/// request images. It is only a warning and a mark: nothing is ever refused for its size.
pub const SIZE_LIMIT: u64 = 10 * 1024 * 1024;

/// Whether an asset of `size` bytes is larger than the limit.
pub const fn exceeds_limit(size: u64) -> bool {
    size > SIZE_LIMIT
}

/// One asset as the list holds it: its name and its size in bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub name: String,
    #[ts(type = "number")]
    pub size: u64,
}

impl Asset {
    /// Whether it is larger than the size limit.
    pub fn is_large(&self) -> bool {
        exceeds_limit(self.size)
    }
}

/// `<project root>/.weaver/assets`.
pub fn assets_dir(root: &Path) -> PathBuf {
    layout::weaver_dir(root).join(layout::ASSETS_DIR)
}

/// The refusal for a name that starts with a dot.
fn dot_name() -> Refusal {
    Refusal::new("Names starting with \".\" are not allowed.")
}

/// The refusal for a name already used by an asset.
pub fn taken(name: &str) -> Refusal {
    Refusal::new(format!("An asset named \"{name}\" already exists."))
}

/// `Ok` when `name` may name an asset (`requirements/rules#asset-view`): the rules of the File
/// view for names (no separator, not empty, `.` and `..` refused, the Windows rules on Windows)
/// and no leading dot.
pub fn check_name(name: &str) -> Result<(), Refusal> {
    validate_name(name, cfg!(windows))
}

/// [`check_name`] with the Windows rules applied when `windows` is true, so the tests check them
/// on any system.
pub fn validate_name(name: &str, windows: bool) -> Result<(), Refusal> {
    names::validate(name, windows)?;
    if name.starts_with('.') {
        return Err(dot_name());
    }
    Ok(())
}

/// Whether two names are the same name for the file system of this system (case-insensitive on
/// macOS and Windows).
pub fn same_name(a: &str, b: &str) -> bool {
    crate::file_view::same_name(a, b)
}

/// The assets in the order of the grid: by name without regard to case, then by name.
pub fn sort(assets: &mut [Asset]) {
    assets.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.name.cmp(&b.name))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_size_limit_is_exactly_ten_mebibytes() {
        assert_eq!(SIZE_LIMIT, 10_485_760);
        assert!(!exceeds_limit(SIZE_LIMIT));
        assert!(exceeds_limit(SIZE_LIMIT + 1));
        assert!(!Asset {
            name: "a".into(),
            size: 0
        }
        .is_large());
        assert!(Asset {
            name: "a".into(),
            size: SIZE_LIMIT + 1
        }
        .is_large());
    }

    #[test]
    fn a_name_follows_the_file_view_rules_and_never_starts_with_a_dot() {
        for windows in [false, true] {
            assert!(validate_name("logo.png", windows).is_ok());
            assert!(validate_name("my notes.txt", windows).is_ok());
            assert!(validate_name("", windows).is_err());
            assert!(validate_name("a/b", windows).is_err());
            assert!(validate_name("a\\b", windows).is_err());
            assert!(validate_name("..", windows).is_err());
            assert_eq!(
                validate_name(".env", windows).unwrap_err().to_string(),
                "Names starting with \".\" are not allowed."
            );
        }
        assert!(validate_name("a:b", true).is_err());
        assert!(validate_name("a:b", false).is_ok());
        assert!(validate_name("CON.txt", true).is_err());
    }

    #[test]
    fn the_grid_order_ignores_case() {
        let mut assets: Vec<Asset> = ["b.png", "A.png", "a.png", "C.txt"]
            .into_iter()
            .map(|name| Asset {
                name: name.into(),
                size: 1,
            })
            .collect();
        sort(&mut assets);
        let names: Vec<&str> = assets.iter().map(|asset| asset.name.as_str()).collect();
        assert_eq!(names, ["A.png", "a.png", "b.png", "C.txt"]);
    }
}
