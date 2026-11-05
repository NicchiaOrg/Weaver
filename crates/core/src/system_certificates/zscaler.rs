//! Whether Zscaler is installed (`requirements/tech#system-certificates`, "Zscaler turn-on").
//!
//! Only file metadata is looked at: no process, no content, no process list, no network. The file
//! system is injected, so tests run every system on any machine.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::omp::lookup::Os;

/// Where Zscaler's application is on macOS.
pub const MACOS_APP: &str = "/Applications/Zscaler/Zscaler.app";

/// `ProgramFiles` when the environment does not say.
pub const DEFAULT_PROGRAM_FILES: &str = r"C:\Program Files";

/// What the detection asks the file system. Metadata only.
pub trait Files {
    /// Whether something exists at `path`.
    fn exists(&self, path: &Path) -> bool;
    /// Whether `path` is a folder.
    fn is_dir(&self, path: &Path) -> bool;
}

/// The real file system.
pub struct SystemFiles;

impl Files for SystemFiles {
    fn exists(&self, path: &Path) -> bool {
        std::fs::metadata(path).is_ok()
    }

    fn is_dir(&self, path: &Path) -> bool {
        std::fs::metadata(path).is_ok_and(|metadata| metadata.is_dir())
    }
}

/// Whether Zscaler is installed: on macOS `/Applications/Zscaler/Zscaler.app` exists; on Windows
/// `<ProgramFiles>\Zscaler` is a folder, with `program_files` the `ProgramFiles` variable (else
/// `C:\Program Files`, also when the variable is empty); on Linux and any other system never.
pub fn installed(os: Os, files: &dyn Files, program_files: Option<&OsStr>) -> bool {
    match os {
        Os::MacOs => files.exists(Path::new(MACOS_APP)),
        Os::Windows => {
            let base = program_files
                .filter(|value| !value.is_empty())
                .map_or_else(|| PathBuf::from(DEFAULT_PROGRAM_FILES), PathBuf::from);
            files.is_dir(&base.join("Zscaler"))
        }
        Os::Linux => false,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    /// A file system that holds only what a test names.
    #[derive(Default)]
    struct Fake {
        files: BTreeSet<PathBuf>,
        folders: BTreeSet<PathBuf>,
    }

    impl Files for Fake {
        fn exists(&self, path: &Path) -> bool {
            self.files.contains(path) || self.folders.contains(path)
        }

        fn is_dir(&self, path: &Path) -> bool {
            self.folders.contains(path)
        }
    }

    fn with_folder(path: impl Into<PathBuf>) -> Fake {
        Fake {
            folders: BTreeSet::from([path.into()]),
            ..Fake::default()
        }
    }

    #[test]
    fn macos_looks_for_the_application_only() {
        assert!(installed(Os::MacOs, &with_folder(MACOS_APP), None));
        assert!(!installed(Os::MacOs, &Fake::default(), None));
        // The folder of the product without the application is not enough.
        assert!(!installed(
            Os::MacOs,
            &with_folder("/Applications/Zscaler"),
            None
        ));
    }

    #[test]
    fn windows_looks_for_a_folder_under_program_files_from_the_environment() {
        let custom = Path::new(r"D:\Programs").join("Zscaler");
        let fake = with_folder(custom);
        assert!(installed(
            Os::Windows,
            &fake,
            Some(OsStr::new(r"D:\Programs"))
        ));
        // The variable is the one that counts.
        assert!(!installed(Os::Windows, &fake, None));
    }

    #[test]
    fn windows_falls_back_to_c_program_files_without_the_variable_or_with_an_empty_one() {
        let fake = with_folder(Path::new(DEFAULT_PROGRAM_FILES).join("Zscaler"));
        assert!(installed(Os::Windows, &fake, None));
        assert!(installed(Os::Windows, &fake, Some(OsStr::new(""))));
        assert!(!installed(Os::Windows, &Fake::default(), None));
    }

    #[test]
    fn windows_needs_a_folder_not_a_file() {
        let path = Path::new(DEFAULT_PROGRAM_FILES).join("Zscaler");
        let fake = Fake {
            files: BTreeSet::from([path]),
            ..Fake::default()
        };
        assert!(!installed(Os::Windows, &fake, None));
    }

    #[test]
    fn linux_never_detects_it_whatever_the_file_system_holds() {
        let mut fake = with_folder(MACOS_APP);
        fake.folders
            .insert(Path::new(DEFAULT_PROGRAM_FILES).join("Zscaler"));
        fake.files.insert(PathBuf::from("/opt/zscaler"));
        assert!(!installed(Os::Linux, &fake, None));
    }

    #[test]
    fn each_system_ignores_the_other_systems_places() {
        let windows_only = with_folder(Path::new(DEFAULT_PROGRAM_FILES).join("Zscaler"));
        assert!(!installed(Os::MacOs, &windows_only, None));
        assert!(!installed(Os::Windows, &with_folder(MACOS_APP), None));
    }
}
