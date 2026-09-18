use std::{
    collections::BTreeSet,
    io::{IsTerminal, Read},
    path::{Path, PathBuf},
};

use crate::{cli::Args, utils::shrink_home};

pub fn recursive_read_dir(dir: &Path) -> Result<BTreeSet<PathBuf>, Box<dyn std::error::Error>> {
    let mut paths = BTreeSet::new();

    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            eprintln!("Permission denied: {}", dir.display());
            return Ok(paths);
        }
        Err(e) => return Err(Box::new(e)),
    };

    for entry in entries {
        let entry = entry?;
        let path = shrink_home(&entry.path());

        if path.is_dir() && !path.is_symlink() {
            paths.extend(recursive_read_dir(&path)?);
        } else {
            paths.insert(path);
        }
    }

    Ok(paths)
}

pub fn get_dir_list(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let dir_list = std::fs::read_dir(dir)?;
    let mut paths = Vec::new();

    for entry in dir_list {
        let entry = entry?;
        paths.push(entry.path());
    }

    Ok(paths)
}

pub fn process_path_args(
    args: &Args,
) -> Result<(BTreeSet<PathBuf>, BTreeSet<PathBuf>), Box<dyn std::error::Error>> {
    let mut paths = BTreeSet::<PathBuf>::new();
    let mut new_paths = BTreeSet::<PathBuf>::new();

    for path in &args.paths {
        if args.recursive && path.is_dir() {
            paths.extend(recursive_read_dir(path)?);
        } else if path.exists() {
            paths.insert(path.clone());
        } else {
            new_paths.insert(path.clone());
        }
    }

    if !std::io::stdin().is_terminal() {
        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer)?;

        let stdin_paths = buffer.lines().map(PathBuf::from).collect::<Vec<_>>();

        for path in stdin_paths {
            if args.recursive && path.is_dir() {
                paths.extend(recursive_read_dir(&path)?);
            } else if path.exists() {
                paths.insert(path.clone());
            } else {
                new_paths.insert(path);
            }
        }
    } else if args.paths.is_empty() {
        if args.recursive {
            paths.extend(recursive_read_dir(&PathBuf::from("."))?);
        } else {
            paths.extend(get_dir_list(&PathBuf::from("."))?);
        }
    }

    Ok((paths, new_paths))
}
