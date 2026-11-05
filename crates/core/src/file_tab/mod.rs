//! The files open in the tabs of the File view: the checked paths, the reading of a file, its
//! saving, and the rules of the tabs (Machine `file-tab`).
//!
//! | Module    | Responsibility |
//! |-----------|----------------|
//! | [`path`]  | Paths relative to the project root, checked before any file is touched |
//! | [`read`]  | What a tab keeps of a file: hash of the raw bytes, byte order mark, line ending, text for the UI; binary, size and read-only rules |
//! | [`save`]  | Saving with the check against the hash the tab read, `Overwrite`, `Save again`; the per-file lock |
//! | [`limit`] | The limit of 10 tabs, and what a watch event on a tab's file means |
//!
//! Nothing here touches `.weaver/` the way `project::text` does: that module normalises line
//! endings and the final line break of the files Weaver writes, and a file of the developer's
//! code must never be normalised.

pub mod limit;
pub mod path;
pub mod read;
pub mod save;
