//! One `rename` that never replaces an existing entry (`requirements/tech#file-view`, **Rename and
//! move**): `renameat2(RENAME_NOREPLACE)` on Linux, `renamex_np(RENAME_EXCL)` on macOS and
//! `MoveFileExW` without `MOVEFILE_REPLACE_EXISTING` on Windows. A file system that does not know
//! the flag gets the check first and the plain `rename`, the best a system without the flag can do.
//!
//! The caller checks the name before; this is the last line, which closes the window between
//! that check and the rename.

use std::io;
use std::path::Path;

/// Renames `from` to `to`, failing with `AlreadyExists` when `to` exists. Both are on the same
/// volume or the rename fails (see [`is_cross_volume`]).
pub fn rename_no_replace(from: &Path, to: &Path) -> io::Result<()> {
    imp::rename_no_replace(from, to)
}

/// Whether `error` says the two paths are on different volumes.
pub fn is_cross_volume(error: &io::Error) -> bool {
    if error.kind() == io::ErrorKind::CrossesDevices {
        return true;
    }
    // EXDEV on Unix, ERROR_NOT_SAME_DEVICE on Windows.
    matches!(error.raw_os_error(), Some(18) if cfg!(unix))
        || matches!(error.raw_os_error(), Some(17) if cfg!(windows))
}

/// The plain check followed by `rename`, for the systems and file systems with no flag.
fn check_then_rename(from: &Path, to: &Path) -> io::Result<()> {
    match std::fs::symlink_metadata(to) {
        Ok(_) => Err(io::Error::from(io::ErrorKind::AlreadyExists)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => std::fs::rename(from, to),
        Err(error) => Err(error),
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use std::ffi::CString;
    use std::io;
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    const RENAME_NOREPLACE: libc::c_uint = 1;

    pub fn rename_no_replace(from: &Path, to: &Path) -> io::Result<()> {
        let (Ok(from_c), Ok(to_c)) = (
            CString::new(from.as_os_str().as_bytes()),
            CString::new(to.as_os_str().as_bytes()),
        ) else {
            return Err(io::Error::from(io::ErrorKind::InvalidInput));
        };
        // SAFETY: both pointers are NUL-terminated strings that live through the call.
        let result = unsafe {
            libc::renameat2(
                libc::AT_FDCWD,
                from_c.as_ptr(),
                libc::AT_FDCWD,
                to_c.as_ptr(),
                RENAME_NOREPLACE,
            )
        };
        if result == 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        // The kernel or the file system does not know the flag.
        if matches!(error.raw_os_error(), Some(libc::ENOSYS | libc::EINVAL)) {
            return super::check_then_rename(from, to);
        }
        Err(error)
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use std::ffi::CString;
    use std::io;
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    pub fn rename_no_replace(from: &Path, to: &Path) -> io::Result<()> {
        let (Ok(from_c), Ok(to_c)) = (
            CString::new(from.as_os_str().as_bytes()),
            CString::new(to.as_os_str().as_bytes()),
        ) else {
            return Err(io::Error::from(io::ErrorKind::InvalidInput));
        };
        // SAFETY: both pointers are NUL-terminated strings that live through the call.
        let result = unsafe { libc::renamex_np(from_c.as_ptr(), to_c.as_ptr(), libc::RENAME_EXCL) };
        if result == 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        // The volume does not support exclusive renames.
        if error.raw_os_error() == Some(libc::ENOTSUP) {
            return super::check_then_rename(from, to);
        }
        Err(error)
    }
}

#[cfg(windows)]
mod imp {
    use std::io;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows_sys::Win32::Storage::FileSystem::MoveFileExW;

    fn wide(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain(Some(0)).collect()
    }

    pub fn rename_no_replace(from: &Path, to: &Path) -> io::Result<()> {
        let (from, to) = (wide(from), wide(to));
        // No `MOVEFILE_REPLACE_EXISTING`, and no `MOVEFILE_COPY_ALLOWED`: a move to another
        // volume fails instead of copying.
        // SAFETY: both buffers are NUL-terminated and live through the call.
        let moved = unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 0) };
        if moved != 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
mod imp {
    use std::io;
    use std::path::Path;

    pub fn rename_no_replace(from: &Path, to: &Path) -> io::Result<()> {
        super::check_then_rename(from, to)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn an_existing_target_is_never_replaced_and_both_files_stay() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a"), "from").unwrap();
        fs::write(dir.path().join("b"), "target").unwrap();
        let error = rename_no_replace(&dir.path().join("a"), &dir.path().join("b")).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read_to_string(dir.path().join("a")).unwrap(), "from");
        assert_eq!(fs::read_to_string(dir.path().join("b")).unwrap(), "target");
    }

    #[test]
    fn a_free_name_is_taken_by_a_file_and_by_a_folder() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a"), "from").unwrap();
        fs::create_dir(dir.path().join("d")).unwrap();
        rename_no_replace(&dir.path().join("a"), &dir.path().join("b")).unwrap();
        rename_no_replace(&dir.path().join("d"), &dir.path().join("e")).unwrap();
        assert_eq!(fs::read_to_string(dir.path().join("b")).unwrap(), "from");
        assert!(dir.path().join("e").is_dir());
        assert!(!dir.path().join("a").exists() && !dir.path().join("d").exists());
    }

    #[test]
    fn an_existing_folder_target_is_not_replaced_either() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("d")).unwrap();
        fs::create_dir(dir.path().join("e")).unwrap();
        let error = rename_no_replace(&dir.path().join("d"), &dir.path().join("e")).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
    }

    #[test]
    fn the_check_then_rename_fallback_refuses_an_existing_target_too() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a"), "1").unwrap();
        fs::write(dir.path().join("b"), "2").unwrap();
        assert_eq!(
            check_then_rename(&dir.path().join("a"), &dir.path().join("b"))
                .unwrap_err()
                .kind(),
            io::ErrorKind::AlreadyExists
        );
        check_then_rename(&dir.path().join("a"), &dir.path().join("c")).unwrap();
        assert!(dir.path().join("c").exists());
    }
}
