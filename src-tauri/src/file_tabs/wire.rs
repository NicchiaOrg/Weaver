//! What the File view's tabs and the agent lock tell the UI: the results of the tab commands and
//! the two events. Every path is relative to the project root, with `/` separators.

use serde::Serialize;
use ts_rs::TS;

/// What kind of file a tab holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum FileKind {
    Text,
    /// Binary and not an image Weaver can show: no text goes to the UI.
    Binary,
    /// Binary and an image Weaver can show: `image` holds it, and no text goes to the UI.
    Image,
}

/// Why a tab's text cannot be changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ReadOnlyReason {
    /// More than 2 MB: read-only and without colours.
    TooLarge,
    /// Inside `.weaver/`.
    Weaver,
}

/// An image Weaver can show, as the UI keeps it for its tab only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FileImage {
    /// `data:<mime>;base64,<…>`.
    pub data_url: String,
    /// The size of the file in bytes.
    #[ts(type = "number")]
    pub size: u64,
}

/// A file as the tab read it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FileContent {
    pub kind: FileKind,
    /// The text with LF line endings and no byte order mark; `null` for a binary file.
    pub text: Option<String>,
    /// The SHA-256 of the raw bytes.
    pub hash: String,
    pub read_only: Option<ReadOnlyReason>,
    /// The image, for the kind `image`; `null` otherwise.
    pub image: Option<FileImage>,
    /// A binary file with an image signature that is larger than 10 MB: no bytes come with it.
    pub image_too_large: bool,
}

/// The answer of `file_open`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum FileOpenResult {
    /// The file was read and has a tab now.
    #[serde(rename_all = "camelCase")]
    Opened {
        path: String,
        #[serde(flatten)]
        content: FileContent,
        /// An agent is changing the file: the tab opens Locked (or Read-only with the line).
        locked: bool,
        /// The tab the limit of 10 closed to make room, if any.
        closed: Option<String>,
    },
    /// The file is open already, in the tab at `path`.
    #[serde(rename_all = "camelCase")]
    AlreadyOpen { path: String },
    /// Every tab has unsaved changes: nothing opened.
    Full,
}

/// The answer of `file_reload`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum FileReloadResult {
    /// The file as it is on disk now.
    Read {
        #[serde(flatten)]
        content: FileContent,
    },
    /// The file no longer exists.
    Gone,
}

/// The answer of `file_save`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum FileSaveResult {
    /// Written; the hash of what was written.
    Saved { hash: String },
    /// The file on disk is not the one the tab read (for `again`: something exists at the path).
    Changed,
    /// The file no longer exists.
    Gone,
    /// The write failed; nothing changed on disk.
    Failed { reason: String },
}

/// What happened to a tab's file on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum TabChange {
    /// `workspace.OPEN_FILE_CHANGED`: changed, or appeared again.
    Changed,
    /// `workspace.OPEN_FILE_DELETED`.
    Deleted,
}

/// The file of a tab changed, disappeared or reappeared on disk, by anything but the tab's own
/// save.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FileTabChanged {
    pub project_id: String,
    #[ts(type = "number")]
    pub generation: u64,
    pub path: String,
    pub change: TabChange,
}

/// The lock of a file open in a tab was taken (`locked`) or released.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FileLockChanged {
    pub project_id: String,
    #[ts(type = "number")]
    pub generation: u64,
    pub path: String,
    pub locked: bool,
    /// On a release, where the agent moved the file when omp said so: the tab already follows it.
    pub moved_to: Option<String>,
}
