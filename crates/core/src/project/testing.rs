//! Fixtures shared by the tests of this crate's project modules.

use std::fs;
use std::path::{Path, PathBuf};

use super::layout;

/// A live Task file.
pub(crate) const TASK: &str = "---\ncolumn: todo\ntechnical: false\nrefs:\n  - order\ncreated: 2026-11-02T07:00:01.000Z\n---\n# A task\n\nDone when.\n";

/// Writes `content` as `.weaver/<rel>` of the project at `root`, creating folders.
pub(crate) fn put(root: &Path, rel: &str, content: &str) -> PathBuf {
    let path = layout::absolute(root, rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, content).unwrap();
    path
}
