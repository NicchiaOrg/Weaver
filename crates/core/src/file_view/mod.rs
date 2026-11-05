//! The File view's tree: the Machine `file-view` (`files` Feature), without the tabs.
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`names`] | The names a new entry may have |
//! | [`copy_in`] | Copying files and folders dropped from outside into the project |
//! | [`listing`] | One folder at a time: names and kinds, `.git` and what git ignores left out |
//! | [`rename`] | A rename that never replaces an existing entry, on every system |
//! | [`ops`] | Create, rename, move and delete, with every refusal checked when they run |
//! | [`machine`] | The state of a window's view (Closed, Open, Releasing), its listings and watches |
//!
//! Everything here is synchronous `std` I/O on paths relative to the project root, with `/`; the
//! app runs it on the blocking pool and sends the results to the UI. Paths are checked with
//! [`crate::file_tab::path`] before anything is touched.

pub mod copy_in;
pub mod listing;
pub mod machine;
pub mod names;
pub mod ops;
pub mod rename;

use std::fmt;

use serde::Serialize;
use ts_rs::TS;

/// The folder (relative to the project root) whose files Weaver writes by itself.
pub const WEAVER_FOLDER: &str = ".weaver";

/// Why an action of the File view was refused. `Display` is the reason shown to the developer,
/// in an error toast. A refused action changes nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal(String);

impl Refusal {
    pub fn new(reason: impl Into<String>) -> Self {
        Self(reason.into())
    }

    pub fn reason(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Refusal {}

impl From<crate::file_tab::path::PathRefusal> for Refusal {
    fn from(refusal: crate::file_tab::path::PathRefusal) -> Self {
        Self(refusal.to_string())
    }
}

/// What an entry of a folder is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum FileEntryKind {
    File,
    Folder,
    /// A symbolic link; whether its target lies in the project is told when it is opened.
    Link,
}

/// One entry of a listed folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    /// Relative to the project root, with `/`.
    pub path: String,
    pub kind: FileEntryKind,
}

/// A listed folder: its path (empty for the project root) and its entries, folders first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FolderListing {
    pub path: String,
    pub entries: Vec<FileEntry>,
}

/// `parent/name`, or `name` for the root.
pub fn join_rel(parent: &str, name: &str) -> String {
    let parent = parent.trim_matches('/');
    if parent.is_empty() {
        name.to_owned()
    } else {
        format!("{parent}/{name}")
    }
}

/// The folder part of a relative path (empty for an entry of the root) and its last name.
pub fn split_rel(rel: &str) -> (&str, &str) {
    match rel.rsplit_once('/') {
        Some((parent, name)) => (parent, name),
        None => ("", rel),
    }
}

/// Whether `path` is `folder` or lies inside it (relative paths; the empty folder is the root and
/// holds everything).
pub fn is_within(folder: &str, path: &str) -> bool {
    folder.is_empty()
        || path == folder
        || path
            .strip_prefix(folder)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// Whether the file system compares names without regard to case (macOS and Windows).
pub const fn case_insensitive_names() -> bool {
    cfg!(any(windows, target_os = "macos"))
}

/// `a` and `b` as the file system compares them: without regard to case on macOS and Windows.
pub fn same_name(a: &str, b: &str) -> bool {
    if case_insensitive_names() {
        a.to_lowercase() == b.to_lowercase()
    } else {
        a == b
    }
}
