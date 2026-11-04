//! CSPRNG helpers and endpoint naming for the CLI↔app channel.
//!
//! * Unix: a domain socket `weaver-<8 random>.sock` in a `0700` folder: `$XDG_RUNTIME_DIR/weaver/`
//!   when set, else `<app local data>/run/`, or `/tmp/weaver-<uid>/` when the socket path would
//!   exceed 100 bytes.
//! * Windows: a named pipe `\\.\pipe\weaver-<16 random>`.
//!
//! An endpoint is a plain string (a filesystem path or a pipe name), the same text stored in
//! `runtime.json` and passed as `WEAVER_ENDPOINT`.

use std::path::{Path, PathBuf};

use crate::paths::AppDirs;

/// Longest allowed socket path in bytes (Unix `sun_path` is 104 on macOS, 108 on Linux).
pub const SOCKET_PATH_LIMIT: usize = 100;

/// Prefix of Windows named pipes.
pub const PIPE_PREFIX: &str = r"\\.\pipe\";

/// `chars` random lowercase hex characters from the OS CSPRNG.
pub fn random_hex(chars: usize) -> String {
    let mut bytes = vec![0u8; chars.div_ceil(2)];
    getrandom::fill(&mut bytes).expect("the OS random number generator is unavailable");
    let mut hex = to_hex(&bytes);
    hex.truncate(chars);
    hex
}

/// The per-launch secret: 32 bytes from the OS CSPRNG, hex-encoded (64 characters).
pub fn new_secret() -> String {
    random_hex(64)
}

fn to_hex(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// Windows pipe name for the given 16 random characters.
pub fn windows_pipe_name(random16: &str) -> String {
    format!("{PIPE_PREFIX}weaver-{random16}")
}

/// Unix socket path for the given 8 random characters. Pure: nothing is created.
///
/// Folder choice: `$XDG_RUNTIME_DIR/weaver/`, else `<app local data>/run/`; when that path
/// would exceed [`SOCKET_PATH_LIMIT`] bytes, `/tmp/weaver-<uid>/`.
pub fn unix_socket_path(dirs: &AppDirs, random8: &str, uid: u32) -> PathBuf {
    let file = format!("weaver-{random8}.sock");
    let folder = match dirs.xdg_runtime_dir() {
        Some(xdg) => xdg.join("weaver"),
        None => dirs.run_dir(),
    };
    let candidate = folder.join(&file);
    if candidate.as_os_str().len() > SOCKET_PATH_LIMIT {
        Path::new("/tmp").join(format!("weaver-{uid}")).join(file)
    } else {
        candidate
    }
}

/// Creates a new endpoint for this launch.
///
/// On Unix the socket folder is created (or fixed) with mode `0700`; the socket file itself is
/// created by the listener. On Windows nothing is created.
pub fn new_endpoint(dirs: &AppDirs) -> std::io::Result<String> {
    #[cfg(unix)]
    {
        // SAFETY: `getuid` has no preconditions and cannot fail.
        let uid = unsafe { libc::getuid() };
        let path = unix_socket_path(dirs, &random_hex(8), uid);
        let folder = path.parent().expect("socket path has a folder");
        ensure_private_dir(folder)?;
        Ok(path.to_string_lossy().into_owned())
    }
    #[cfg(not(unix))]
    {
        let _ = dirs;
        Ok(windows_pipe_name(&random_hex(16)))
    }
}

#[cfg(unix)]
fn ensure_private_dir(folder: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(folder)?;
    // A pre-existing folder may carry looser permissions.
    std::fs::set_permissions(folder, std::fs::Permissions::from_mode(0o700))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_hex_has_the_requested_length_and_alphabet() {
        for chars in [1, 8, 16, 63, 64] {
            let text = random_hex(chars);
            assert_eq!(text.len(), chars);
            assert!(text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')));
        }
        assert_ne!(random_hex(32), random_hex(32));
    }

    #[test]
    fn secret_is_64_hex_characters() {
        assert_eq!(new_secret().len(), 64);
    }

    #[test]
    fn pipe_name_has_the_windows_prefix() {
        assert_eq!(
            windows_pipe_name("0123456789abcdef"),
            r"\\.\pipe\weaver-0123456789abcdef"
        );
    }

    // The XDG runtime folder is a Unix notion: `/run/user/7` is not an absolute path on Windows.
    #[cfg(unix)]
    #[test]
    fn socket_goes_to_xdg_runtime_when_set() {
        let dirs = AppDirs::from_bases(
            Path::new("/c"),
            Path::new("/d"),
            Some(Path::new("/run/user/7")),
        );
        assert_eq!(
            unix_socket_path(&dirs, "abcd1234", 7),
            Path::new("/run/user/7/weaver/weaver-abcd1234.sock")
        );
    }

    #[test]
    fn socket_falls_back_to_local_data_run() {
        let dirs = AppDirs::from_bases(Path::new("/c"), Path::new("/d"), None);
        assert_eq!(
            unix_socket_path(&dirs, "abcd1234", 7),
            Path::new("/d/dev.nicchia.weaver/run/weaver-abcd1234.sock")
        );
    }

    #[test]
    fn long_paths_fall_back_to_tmp_with_the_uid() {
        let long = format!("/{}", "x".repeat(90));
        let dirs = AppDirs::from_bases(Path::new("/c"), Path::new(&long), None);
        assert_eq!(
            unix_socket_path(&dirs, "abcd1234", 501),
            Path::new("/tmp/weaver-501/weaver-abcd1234.sock")
        );
    }

    #[cfg(unix)]
    #[test]
    fn new_endpoint_creates_a_private_folder() {
        use std::os::unix::fs::PermissionsExt;
        let base = tempfile::tempdir().unwrap();
        let dirs = AppDirs::from_bases(base.path(), base.path(), Some(base.path()));
        let endpoint = new_endpoint(&dirs).unwrap();
        let folder = Path::new(&endpoint).parent().unwrap();
        assert!(endpoint.len() <= SOCKET_PATH_LIMIT || folder.starts_with("/tmp"));
        assert_eq!(
            std::fs::metadata(folder).unwrap().permissions().mode() & 0o777,
            0o700
        );
        // On a long temp path the endpoint falls back to the shared /tmp/weaver-<uid>/, left in place.
    }
}
