use std::{
    collections::HashSet,
    io::{IsTerminal, Read},
    path::{Path, PathBuf},
};

use crate::cli::Args;

pub fn recursive_read_dir(
    dir: &Path,
) -> Result<HashSet<PathBuf>, Box<dyn std::error::Error>> {
    let mut paths = HashSet::new();

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
        let path = entry.path();

        if path.is_dir() && !path.is_symlink() {
            paths.extend(recursive_read_dir(&path)?);
        } else {
            paths.insert(path.to_path_buf());
        }
    }

    Ok(paths)
}

pub fn get_dir_list(
    dir: &PathBuf,
) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
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
) -> Result<(Vec<PathBuf>, Vec<PathBuf>), Box<dyn std::error::Error>> {
    let mut paths = HashSet::<PathBuf>::new();
    let mut new_paths = HashSet::<PathBuf>::new();

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

        let stdin_paths = buffer
            .lines()
            .map(PathBuf::from)
            .collect::<Vec<_>>();

        for path in stdin_paths {
            if args.recursive && path.is_dir() {
                paths.extend(recursive_read_dir(&path)?);
            } else if path.exists() {
                paths.insert(path);
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

    let mut sorted_paths =
        paths.iter().map(|p| p.clone()).collect::<Vec<PathBuf>>();

    sorted_paths.sort();

    let mut sorted_new_paths =
        new_paths.iter().map(|p| p.clone()).collect::<Vec<PathBuf>>();

    sorted_new_paths.sort();

    Ok((sorted_paths, sorted_new_paths))
}