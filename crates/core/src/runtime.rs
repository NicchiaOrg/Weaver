//! `<app local data>/runtime.json`: how the CLI finds the running app.
//!
//! The app writes it atomically at each launch (mode `0600`); the CLI reads it and connects to
//! `endpoint` with `secret`. The secret never goes to env vars, arguments, logs or `.trama/`.

use std::io;

use serde::{Deserialize, Serialize};

use crate::atomic;
use crate::paths::AppDirs;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeInfo {
    pub endpoint: String,
    pub secret: String,
    pub pid: u32,
    pub version: String,
}

// The secret must not leak through `{:?}` in logs.
impl std::fmt::Debug for RuntimeInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuntimeInfo")
            .field("endpoint", &self.endpoint)
            .field("secret", &"<redacted>")
            .field("pid", &self.pid)
            .field("version", &self.version)
            .finish()
    }
}

/// Atomically writes the runtime file with mode `0600` (Unix).
pub fn write(dirs: &AppDirs, info: &RuntimeInfo) -> io::Result<()> {
    let json = serde_json::to_vec_pretty(info).map_err(io::Error::other)?;
    atomic::write(&dirs.runtime_file(), &json, Some(0o600))
}

/// Reads the runtime file. `Ok(None)` when it does not exist; invalid content is an error.
pub fn read(dirs: &AppDirs) -> io::Result<Option<RuntimeInfo>> {
    match std::fs::read(dirs.runtime_file()) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info() -> RuntimeInfo {
        RuntimeInfo {
            endpoint: "/run/weaver/weaver-abcd1234.sock".into(),
            secret: "ab".repeat(32),
            pid: 42,
            version: "0.1.0".into(),
        }
    }

    #[test]
    fn round_trips_and_reads_none_when_missing() {
        let base = tempfile::tempdir().unwrap();
        let dirs = AppDirs::from_bases(base.path(), base.path(), None);
        assert_eq!(read(&dirs).unwrap(), None);
        write(&dirs, &info()).unwrap();
        assert_eq!(read(&dirs).unwrap(), Some(info()));
    }

    #[test]
    fn file_has_exactly_the_four_fields() {
        let base = tempfile::tempdir().unwrap();
        let dirs = AppDirs::from_bases(base.path(), base.path(), None);
        write(&dirs, &info()).unwrap();
        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dirs.runtime_file()).unwrap()).unwrap();
        let mut keys: Vec<_> = value.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        assert_eq!(keys, ["endpoint", "pid", "secret", "version"]);
    }

    #[cfg(unix)]
    #[test]
    fn file_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let base = tempfile::tempdir().unwrap();
        let dirs = AppDirs::from_bases(base.path(), base.path(), None);
        write(&dirs, &info()).unwrap();
        let mode = std::fs::metadata(dirs.runtime_file())
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn debug_output_hides_the_secret() {
        assert!(!format!("{:?}", info()).contains(&"ab".repeat(32)));
    }
}
