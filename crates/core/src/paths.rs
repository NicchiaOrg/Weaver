//! App folders, derived from the Tauri identifier so the CLI finds them with the app closed.
//!
//! The app and the CLI both call [`AppDirs::resolve`]; the layout below is the single source
//! of truth for every file Weaver keeps outside a project.
//!
//! ```text
//! <config>/dev.nicchia.weaver/            settings, recent projects (later Tasks)
//! <local data>/dev.nicchia.weaver/
//!   runtime.json                          endpoint, secret, pid, version of the running app
//!   bin/weaver                            copy of the bundled `weaver-cli` (only file in this folder)
//!   locks/<project hash>.lock             advisory lock of a project
//!   run/                                  socket folder when XDG_RUNTIME_DIR is not set (Unix)
//!   system-certificates.pem               the bundle of System Certificates (all windows and projects)
//!   omp-install/                          marker files of omp's installer: `<nonce>.code`
//! ```
//!
//! `<config>` and `<local data>` are what Tauri's `app_config_dir` and `app_local_data_dir`
//! resolve to (`dirs::config_dir()` and `dirs::data_local_dir()` plus the identifier).

use std::io;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// Tauri identifier of the app (`src-tauri/tauri.conf.json`).
pub const APP_IDENTIFIER: &str = "dev.nicchia.weaver";

/// Name of the runtime file inside the local data folder.
pub const RUNTIME_FILE_NAME: &str = "runtime.json";

/// Length in hex characters of a project hash.
pub const PROJECT_HASH_LEN: usize = 16;

#[derive(Debug, thiserror::Error)]
pub enum PathsError {
    #[error("the system does not provide a {0} folder")]
    MissingBase(&'static str),
}

/// The folders of the app. Plain data: nothing is created on disk by building one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppDirs {
    config: PathBuf,
    local_data: PathBuf,
    xdg_runtime: Option<PathBuf>,
}

impl AppDirs {
    /// Resolves the folders of the current user from the operating system.
    pub fn resolve() -> Result<Self, PathsError> {
        let config = dirs::config_dir().ok_or(PathsError::MissingBase("config"))?;
        let local = dirs::data_local_dir().ok_or(PathsError::MissingBase("local data"))?;
        let xdg = if cfg!(unix) {
            std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from)
        } else {
            None
        };
        Ok(Self::from_bases(&config, &local, xdg.as_deref()))
    }

    /// Builds the folders from explicit bases (what the OS would return). An empty or
    /// relative `xdg_runtime` counts as not set.
    pub fn from_bases(
        config_base: &Path,
        local_data_base: &Path,
        xdg_runtime: Option<&Path>,
    ) -> Self {
        Self {
            config: config_base.join(APP_IDENTIFIER),
            local_data: local_data_base.join(APP_IDENTIFIER),
            xdg_runtime: xdg_runtime
                .filter(|p| p.is_absolute())
                .map(Path::to_path_buf),
        }
    }

    /// `<config>/dev.nicchia.weaver`.
    pub fn config_dir(&self) -> &Path {
        &self.config
    }

    /// `<local data>/dev.nicchia.weaver`.
    pub fn local_data_dir(&self) -> &Path {
        &self.local_data
    }

    /// `$XDG_RUNTIME_DIR`, when set to an absolute path (Unix only).
    pub fn xdg_runtime_dir(&self) -> Option<&Path> {
        self.xdg_runtime.as_deref()
    }

    /// `<local data>/runtime.json`.
    pub fn runtime_file(&self) -> PathBuf {
        self.local_data.join(RUNTIME_FILE_NAME)
    }

    /// `<local data>/bin`: holds only the `weaver` copy.
    pub fn bin_dir(&self) -> PathBuf {
        self.local_data.join("bin")
    }

    /// `<local data>/locks`.
    pub fn locks_dir(&self) -> PathBuf {
        self.local_data.join("locks")
    }

    /// `<local data>/locks/<project hash>.lock`.
    pub fn lock_file(&self, project_hash: &str) -> PathBuf {
        self.locks_dir().join(format!("{project_hash}.lock"))
    }

    /// `<local data>/system-certificates.pem`: the System Certificates bundle, one for every
    /// window and project.
    pub fn system_certificates_file(&self) -> PathBuf {
        self.local_data
            .join(crate::system_certificates::BUNDLE_FILE_NAME)
    }

    /// `<local data>/omp-install`: where the installer of omp leaves its exit code, one
    /// `<nonce>.code` file for each `INSTALL_OMP` (`omp.INSTALL_FAILED`).
    pub fn omp_install_dir(&self) -> PathBuf {
        self.local_data.join("omp-install")
    }

    /// `<local data>/run`: socket folder when `XDG_RUNTIME_DIR` is not set.
    pub fn run_dir(&self) -> PathBuf {
        self.local_data.join("run")
    }
}

/// The canonical form of a project root (symlinks resolved, no Windows `\\?\` prefix): the
/// form the project hash is computed from and the one the CLI sends to the app.
pub fn canonical_root(root: &Path) -> io::Result<PathBuf> {
    dunce::canonicalize(root)
}

/// First 16 hex characters of the SHA-256 of the canonical project root.
///
/// The root is canonicalised with symlinks resolved and the Windows `\\?\` prefix stripped
/// (`dunce`); on Windows the comparison is case-insensitive, so the text is lowercased.
pub fn project_hash(root: &Path) -> io::Result<String> {
    Ok(project_hash_of_canonical(&dunce::canonicalize(root)?))
}

/// Same as [`project_hash`] for a path that is already canonical.
pub fn project_hash_of_canonical(canonical_root: &Path) -> String {
    let text = canonical_root.to_string_lossy();
    let text = if cfg!(windows) {
        text.to_lowercase()
    } else {
        text.into_owned()
    };
    let digest = Sha256::digest(text.as_bytes());
    let mut hex = String::with_capacity(PROJECT_HASH_LEN);
    for byte in digest.iter().take(PROJECT_HASH_LEN / 2) {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folders_are_the_identifier_under_each_base() {
        let dirs = AppDirs::from_bases(Path::new("/cfg"), Path::new("/data"), None);
        assert_eq!(dirs.config_dir(), Path::new("/cfg/dev.nicchia.weaver"));
        assert_eq!(dirs.local_data_dir(), Path::new("/data/dev.nicchia.weaver"));
        assert_eq!(
            dirs.runtime_file(),
            Path::new("/data/dev.nicchia.weaver/runtime.json")
        );
        assert_eq!(dirs.bin_dir(), Path::new("/data/dev.nicchia.weaver/bin"));
        assert_eq!(
            dirs.system_certificates_file(),
            Path::new("/data/dev.nicchia.weaver/system-certificates.pem")
        );
        assert_eq!(
            dirs.omp_install_dir(),
            Path::new("/data/dev.nicchia.weaver/omp-install")
        );
        assert_eq!(
            dirs.lock_file("0123456789abcdef"),
            Path::new("/data/dev.nicchia.weaver/locks/0123456789abcdef.lock")
        );
    }

    #[test]
    fn resolve_is_stable_between_calls_so_cli_and_app_agree() {
        let a = AppDirs::resolve().expect("resolve");
        let b = AppDirs::resolve().expect("resolve");
        assert_eq!(a, b);
        assert!(a.local_data_dir().ends_with(APP_IDENTIFIER));
        assert!(a.config_dir().ends_with(APP_IDENTIFIER));
    }

    #[test]
    fn relative_or_missing_xdg_runtime_is_ignored() {
        let rel = AppDirs::from_bases(Path::new("/c"), Path::new("/d"), Some(Path::new("run")));
        assert_eq!(rel.xdg_runtime_dir(), None);
        let abs = AppDirs::from_bases(
            Path::new("/c"),
            Path::new("/d"),
            Some(Path::new("/run/user/1")),
        );
        assert_eq!(abs.xdg_runtime_dir(), Some(Path::new("/run/user/1")));
    }

    #[test]
    fn project_hash_is_16_hex_of_sha256() {
        // `printf '/tmp/project' | shasum -a 256` starts with f630ad93b344dd6b.
        let hash = project_hash_of_canonical(Path::new("/tmp/project"));
        assert_eq!(hash, "f630ad93b344dd6b");
        assert_eq!(hash.len(), PROJECT_HASH_LEN);
        assert!(hash
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()));
        let again = project_hash_of_canonical(Path::new("/tmp/project"));
        assert_eq!(hash, again);
        assert_ne!(hash, project_hash_of_canonical(Path::new("/tmp/other")));
    }

    #[test]
    fn project_hash_follows_symlinks_and_equals_the_canonical_hash() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real");
        std::fs::create_dir(&real).unwrap();
        let canonical = dunce::canonicalize(&real).unwrap();
        assert_eq!(
            project_hash(&real).unwrap(),
            project_hash_of_canonical(&canonical)
        );
        #[cfg(unix)]
        {
            let link = dir.path().join("link");
            std::os::unix::fs::symlink(&real, &link).unwrap();
            assert_eq!(project_hash(&link).unwrap(), project_hash(&real).unwrap());
        }
    }
}
