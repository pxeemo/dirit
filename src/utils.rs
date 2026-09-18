
use std::path::{Path, PathBuf};

pub fn expand_tilde(path: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    if path.starts_with('~') {
        path.replacen('~', home.as_str(), 1)
    } else {
        path.to_string()
    }
}

pub fn shrink_home(path: &Path) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    if path.starts_with(&home) {
        PathBuf::from(path.to_string_lossy().replacen(&home, "~", 1))
    } else {
        path.to_path_buf()
    }
}
