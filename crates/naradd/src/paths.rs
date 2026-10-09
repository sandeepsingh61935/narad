//! XDG paths for Narad data (ARCHITECTURE §5).

use std::fs;
use std::path::PathBuf;

/// `~/.local/share/narad` or `$XDG_DATA_HOME/narad`.
pub fn resolve_data_dir() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        return Some(PathBuf::from(xdg).join("narad"));
    }
    std::env::var("HOME").ok().map(|home| {
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("narad")
    })
}

/// History SQLite database path (`narad.db` under the data dir).
pub fn resolve_db_path() -> Option<PathBuf> {
    resolve_data_dir().map(|dir| dir.join("narad.db"))
}

/// Create the data directory with `0700` when missing (NFR-S-2).
pub fn ensure_data_dir() -> std::io::Result<PathBuf> {
    let dir = resolve_data_dir().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "XDG data path unavailable")
    })?;
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;
        }
    }
    Ok(dir)
}
