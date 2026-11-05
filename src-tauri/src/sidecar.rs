//! The bundled `weaver-cli`, a Tauri `externalBin` sidecar.
//!
//! At each start Weaver copies it into `<app local data>/bin/` as `weaver` when its content
//! differs, so the path stays stable even under AppImage (where the bundled copy lives in a
//! changing mount), and removes the `trama` left there by earlier versions of Weaver. That folder
//! holds only `weaver` and goes first on the PATH of every omp Weaver starts.

use std::io;
use std::path::{Path, PathBuf};

use weaver_core::atomic;
use weaver_core::paths::AppDirs;

/// File name of the CLI inside the bundle, next to the executable of the app.
pub const BUNDLED_FILE_NAME: &str = if cfg!(windows) {
    "weaver-cli.exe"
} else {
    "weaver-cli"
};

/// File name of the CLI inside `<app local data>/bin/`: the name the agent types.
pub const FILE_NAME: &str = if cfg!(windows) {
    "weaver.exe"
} else {
    "weaver"
};

/// File name that earlier versions of Weaver put in `<app local data>/bin/`.
const PREVIOUS_FILE_NAME: &str = if cfg!(windows) { "trama.exe" } else { "trama" };

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncOutcome {
    /// The copy was missing or different and has been written.
    Installed,
    /// The copy already has the same content.
    UpToDate,
    /// The bundled file is missing or empty (a development build without the real sidecar).
    SourceMissing,
}

/// Where the bundled `weaver-cli` sits: next to the app executable (Tauri strips the target triple).
pub fn bundled_path() -> io::Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let folder = exe
        .parent()
        .ok_or_else(|| io::Error::other("the executable has no folder"))?;
    Ok(folder.join(BUNDLED_FILE_NAME))
}

/// Copies `source` to `<bin_dir>/weaver` atomically when the content differs, and removes the
/// `trama` of earlier versions from `bin_dir`.
pub fn sync(source: &Path, bin_dir: &Path) -> io::Result<SyncOutcome> {
    match std::fs::remove_file(bin_dir.join(PREVIOUS_FILE_NAME)) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    let bytes = match std::fs::read(source) {
        Ok(bytes) if !bytes.is_empty() => bytes,
        Ok(_) => return Ok(SyncOutcome::SourceMissing),
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(SyncOutcome::SourceMissing),
        Err(e) => return Err(e),
    };
    let target = bin_dir.join(FILE_NAME);
    match std::fs::read(&target) {
        Ok(existing) if existing == bytes => return Ok(SyncOutcome::UpToDate),
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    atomic::write(&target, &bytes, Some(0o755))?;
    Ok(SyncOutcome::Installed)
}

/// Installs the bundled `weaver-cli` as `weaver` into the app's bin folder. Blocking: call it from the blocking pool.
pub fn install(dirs: &AppDirs) -> io::Result<SyncOutcome> {
    sync(&bundled_path()?, &dirs.bin_dir())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn copies_when_missing_then_is_up_to_date_then_copies_again_when_it_differs() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("bundled");
        let bin = dir.path().join("bin");
        std::fs::write(&source, b"v1").unwrap();

        assert_eq!(sync(&source, &bin).unwrap(), SyncOutcome::Installed);
        assert_eq!(std::fs::read(bin.join(FILE_NAME)).unwrap(), b"v1");
        assert_eq!(sync(&source, &bin).unwrap(), SyncOutcome::UpToDate);

        std::fs::write(&source, b"v2").unwrap();
        assert_eq!(sync(&source, &bin).unwrap(), SyncOutcome::Installed);
        assert_eq!(std::fs::read(bin.join(FILE_NAME)).unwrap(), b"v2");
        assert_eq!(
            names(&bin),
            vec![FILE_NAME.to_owned()],
            "the folder holds only weaver"
        );
    }

    #[test]
    fn the_trama_of_earlier_versions_is_removed_and_the_folder_holds_only_weaver() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("bundled");
        let bin = dir.path().join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::write(&source, b"v1").unwrap();
        std::fs::write(bin.join(PREVIOUS_FILE_NAME), b"old").unwrap();

        assert_eq!(sync(&source, &bin).unwrap(), SyncOutcome::Installed);
        assert_eq!(names(&bin), vec![FILE_NAME.to_owned()]);

        // Also when the copy is already up to date, or the bundled file is missing.
        std::fs::write(bin.join(PREVIOUS_FILE_NAME), b"old").unwrap();
        assert_eq!(sync(&source, &bin).unwrap(), SyncOutcome::UpToDate);
        assert_eq!(names(&bin), vec![FILE_NAME.to_owned()]);
        std::fs::write(bin.join(PREVIOUS_FILE_NAME), b"old").unwrap();
        assert_eq!(
            sync(&dir.path().join("none"), &bin).unwrap(),
            SyncOutcome::SourceMissing
        );
        assert_eq!(names(&bin), vec![FILE_NAME.to_owned()]);
    }

    #[test]
    fn a_missing_or_empty_source_changes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("bin");
        assert_eq!(
            sync(&dir.path().join("none"), &bin).unwrap(),
            SyncOutcome::SourceMissing
        );
        let empty = dir.path().join("empty");
        std::fs::write(&empty, b"").unwrap();
        assert_eq!(sync(&empty, &bin).unwrap(), SyncOutcome::SourceMissing);
        assert!(!bin.exists());
    }

    #[cfg(unix)]
    #[test]
    fn the_copy_is_executable() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("bundled");
        std::fs::write(&source, b"#!/bin/sh\n").unwrap();
        sync(&source, &dir.path().join("bin")).unwrap();
        let mode = std::fs::metadata(dir.path().join("bin").join(FILE_NAME))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o111, 0o111);
    }
}
