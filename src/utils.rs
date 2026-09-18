
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
    let home = std::env::var_os("HOME");
    if let Some(home) = home {
        let home = Path::new(&home);
        if let Ok(relative) = path.strip_prefix(home) {
            PathBuf::from("~").join(relative)
        } else {
            path.to_path_buf()
        }
    } else {
        path.to_path_buf()
    }
}
