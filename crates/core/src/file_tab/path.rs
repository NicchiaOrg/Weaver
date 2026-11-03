//! Paths of the File view: the ones the UI sends are relative to the project root, with `/`
//! separators, and are checked here before any file is touched.
//!
//! A path is refused when it is absolute, has a `..` part, a NUL byte or a backslash, lies inside
//! `.git`, or when its canonical form (`dunce`: symbolic links resolved, no `\\?\` prefix on
//! Windows) leaves the canonical project root. A symbolic link whose target leaves the root is
//! refused like any other path outside it.

use std::fmt;
use std::io;
use std::path::{Component, Path, PathBuf};

/// Why a path of the File view was refused. `Display` is the reason shown to the developer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathRefusal {
    /// Absolute, with a `..` part, a NUL byte or a backslash.
    Invalid,
    /// Inside `.git`.
    InsideGit,
    /// Its canonical form leaves the project, by a symbolic link or otherwise.
    OutsideProject,
    /// The path could not be resolved (`io::ErrorKind` and its message).
    Unreadable(String),
}

impl fmt::Display for PathRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid => f.write_str("This path is not a path inside the project."),
            Self::InsideGit => f.write_str("Files inside .git are not shown or changed here."),
            Self::OutsideProject => f.write_str("This path leads outside the project."),
            Self::Unreadable(reason) => write!(f, "Could not resolve the path: {reason}"),
        }
    }
}

impl std::error::Error for PathRefusal {}

/// The parts of `rel`, checked lexically (nothing is read from disk): no absolute path, no `..`,
/// no NUL, no backslash, nothing inside `.git` (compared without case, because the folder is the
/// same one on case-insensitive file systems). `.` parts and empty parts are dropped; the empty
/// path is the project root.
pub fn parts(rel: &str) -> Result<Vec<&str>, PathRefusal> {
    if rel.contains(['\0', '\\']) || rel.starts_with('/') {
        return Err(PathRefusal::Invalid);
    }
    let mut parts = Vec::new();
    for part in rel.split('/') {
        match part {
            "" | "." => {}
            ".." => return Err(PathRefusal::Invalid),
            part => {
                // `C:` and the like are a drive on Windows.
                if Path::new(part)
                    .components()
                    .any(|c| matches!(c, Component::Prefix(_) | Component::RootDir))
                {
                    return Err(PathRefusal::Invalid);
                }
                if part.eq_ignore_ascii_case(".git") {
                    return Err(PathRefusal::InsideGit);
                }
                parts.push(part);
            }
        }
    }
    Ok(parts)
}

/// The path of `rel` under `root` as written, with its symbolic links not resolved, after the
/// lexical check of [`parts`]. `root` is the canonical project root.
pub fn joined(root: &Path, rel: &str) -> Result<PathBuf, PathRefusal> {
    let mut path = root.to_path_buf();
    path.extend(parts(rel)?);
    Ok(path)
}

/// The canonical form of `rel` under `root` (the canonical project root): symbolic links are
/// resolved, and a final part that does not exist yet is the canonical form of the deepest
/// existing folder plus the missing names. Refused when the result leaves the root or lies
/// inside `.git`.
pub fn resolve(root: &Path, rel: &str) -> Result<PathBuf, PathRefusal> {
    let path = joined(root, rel)?;
    let canonical = canonical_or_parent(&path)?;
    if !is_inside(root, &canonical) {
        return Err(PathRefusal::OutsideProject);
    }
    // A link inside the project may point into `.git`.
    if let Ok(inside) = canonical.strip_prefix(root) {
        if inside
            .components()
            .any(|c| c.as_os_str().eq_ignore_ascii_case(".git"))
        {
            return Err(PathRefusal::InsideGit);
        }
    }
    Ok(canonical)
}

/// The canonical form of `path`, or, when it does not exist, of its deepest existing ancestor
/// plus the names that follow.
pub fn canonical_or_parent(path: &Path) -> Result<PathBuf, PathRefusal> {
    let mut missing: Vec<&std::ffi::OsStr> = Vec::new();
    let mut existing = path;
    loop {
        match dunce::canonicalize(existing) {
            Ok(mut canonical) => {
                canonical.extend(missing.iter().rev());
                return Ok(canonical);
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let (Some(parent), Some(name)) = (existing.parent(), existing.file_name()) else {
                    return Err(PathRefusal::Unreadable(error.to_string()));
                };
                missing.push(name);
                existing = parent;
            }
            Err(error) => return Err(PathRefusal::Unreadable(error.to_string())),
        }
    }
}

/// Whether `path` is `root` or lies under it; both canonical. Without regard to case on Windows.
pub fn is_inside(root: &Path, path: &Path) -> bool {
    if cfg!(windows) {
        fold(path).starts_with(fold(root))
    } else {
        path.starts_with(root)
    }
}

/// Whether two canonical paths name the same file: equal, without regard to case on Windows.
pub fn same_file(a: &Path, b: &Path) -> bool {
    if cfg!(windows) {
        fold(a) == fold(b)
    } else {
        a == b
    }
}

fn fold(path: &Path) -> PathBuf {
    PathBuf::from(path.to_string_lossy().to_lowercase())
}

/// The path of `canonical` relative to `root`, with `/` separators; `None` outside the root.
pub fn relative(root: &Path, canonical: &Path) -> Option<String> {
    if !is_inside(root, canonical) {
        return None;
    }
    let tail = if cfg!(windows) {
        // The prefix has the same length whatever its case.
        let (root, canonical) = (root.to_string_lossy(), canonical.to_string_lossy());
        PathBuf::from(canonical.get(root.len()..)?.to_owned())
    } else {
        canonical.strip_prefix(root).ok()?.to_path_buf()
    };
    let parts: Vec<_> = tail
        .components()
        .filter_map(|c| match c {
            Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    Some(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        (dir, root)
    }

    #[test]
    fn absolute_paths_dot_dot_backslashes_and_nul_are_invalid() {
        for rel in ["/etc/passwd", "a/../b", "..", "a\\b", "a\0b"] {
            assert_eq!(parts(rel), Err(PathRefusal::Invalid), "{rel}");
        }
        if cfg!(windows) {
            assert_eq!(parts("C:/x"), Err(PathRefusal::Invalid));
        }
    }

    #[test]
    fn anything_inside_git_is_refused_whatever_the_case() {
        for rel in [".git", ".git/config", "a/.GIT/x", ".Git"] {
            assert_eq!(parts(rel), Err(PathRefusal::InsideGit), "{rel}");
        }
        assert_eq!(parts(".gitignore").unwrap(), vec![".gitignore"]);
    }

    #[test]
    fn the_empty_path_is_the_root_and_dots_are_dropped() {
        assert_eq!(parts("").unwrap(), Vec::<&str>::new());
        assert_eq!(parts("./a//b/.").unwrap(), vec!["a", "b"]);
        let (_dir, root) = project();
        assert_eq!(resolve(&root, "").unwrap(), root);
    }

    #[test]
    fn a_file_that_does_not_exist_yet_resolves_under_its_existing_folder() {
        let (_dir, root) = project();
        std::fs::create_dir(root.join("src")).unwrap();
        assert_eq!(
            resolve(&root, "src/new/file.rs").unwrap(),
            root.join("src").join("new").join("file.rs")
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_link_leaving_the_root_is_refused_and_one_inside_resolves_to_its_target() {
        let (_dir, root) = project();
        let (_other, outside) = project();
        std::fs::write(outside.join("secret"), b"x").unwrap();
        std::fs::write(root.join("real"), b"x").unwrap();
        std::os::unix::fs::symlink(outside.join("secret"), root.join("out")).unwrap();
        std::os::unix::fs::symlink(root.join("real"), root.join("in")).unwrap();
        std::fs::create_dir(root.join(".git")).unwrap();
        std::os::unix::fs::symlink(root.join(".git"), root.join("to-git")).unwrap();
        assert_eq!(resolve(&root, "out"), Err(PathRefusal::OutsideProject));
        assert_eq!(resolve(&root, "in").unwrap(), root.join("real"));
        assert_eq!(resolve(&root, "to-git"), Err(PathRefusal::InsideGit));
    }

    #[test]
    fn relative_paths_use_slashes_and_leave_nothing_outside() {
        let root = PathBuf::from(if cfg!(windows) { r"C:\p" } else { "/p" });
        let file = root.join("a").join("b.txt");
        assert_eq!(relative(&root, &file).as_deref(), Some("a/b.txt"));
        assert_eq!(relative(&root, &root).as_deref(), Some(""));
        let outside = PathBuf::from(if cfg!(windows) { r"C:\q\x" } else { "/q/x" });
        assert_eq!(relative(&root, &outside), None);
    }

    #[test]
    fn windows_comparisons_ignore_case() {
        let (a, b) = (Path::new("/Proj/File"), Path::new("/proj/file"));
        assert_eq!(same_file(a, b), cfg!(windows));
        assert!(same_file(a, a));
    }
}
