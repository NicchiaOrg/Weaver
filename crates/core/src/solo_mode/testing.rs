//! Real git repositories in temporary folders, for the tests of Solo mode and of what builds on it.
//! Nothing here reads or writes the developer's home: git runs with no global or system config.

use std::path::{Path, PathBuf};

use crate::paths;

/// A temporary folder that holds the repositories of one test.
pub struct Sandbox {
    pub dir: tempfile::TempDir,
}

impl Sandbox {
    pub fn new() -> Self {
        Self {
            dir: tempfile::tempdir().unwrap(),
        }
    }

    /// The canonical path of a folder `name` of the sandbox, made when missing.
    pub fn folder(&self, name: &str) -> PathBuf {
        let path = self.dir.path().join(name);
        std::fs::create_dir_all(&path).unwrap();
        paths::canonical_root(&path).unwrap()
    }

    /// A git repository in the folder `name`; returns its canonical root.
    pub fn repo(&self, name: &str) -> PathBuf {
        let root = self.folder(name);
        git(&root, &["init", "-q"]);
        root
    }
}

/// Runs real git in `dir` and returns what it printed; it must succeed.
pub fn git(dir: &Path, args: &[&str]) -> String {
    let null = if cfg!(windows) { "NUL" } else { "/dev/null" };
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", null)
        .env("GIT_CONFIG_SYSTEM", null)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Puts `content` in `rel` (with `/`) under `root`, making the folders.
pub fn put(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

/// Tracks `rel` (already written) in the index of the repository at `repo`.
pub fn track(repo: &Path, rel: &str) {
    git(repo, &["add", "-f", "--", rel]);
}

/// A first commit, so that a linked worktree can be added.
pub fn commit(repo: &Path) {
    git(
        repo,
        &[
            "-c",
            "user.name=Weaver Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "first",
        ],
    );
}

/// Whether git ignores `rel` in the repository at `repo`, by the rules of its exclusions.
pub fn ignored(repo: &Path, rel: &str) -> bool {
    let null = if cfg!(windows) { "NUL" } else { "/dev/null" };
    std::process::Command::new("git")
        .args(["check-ignore", "-q", "--no-index", "--", rel])
        .current_dir(repo)
        .env("GIT_CONFIG_GLOBAL", null)
        .env("GIT_CONFIG_SYSTEM", null)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .status()
        .unwrap()
        .success()
}
