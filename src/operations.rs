use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use crate::{config, model::{Copy, Rename}};

pub fn remove_paths(
    paths: &[PathBuf],
) -> Result<(), Box<dyn std::error::Error>> {
    let has_trash = which::which("trash-put").is_ok();

    for path in paths {
        println!(
            "{}: {}",
            if has_trash { "Trash" } else { "Remove" },
            path.display()
        );
    }

    if config::get().dry_run || paths.is_empty() {
        return Ok(());
    }

    if has_trash {
        std::process::Command::new("trash-put")
            .args(paths)
            .status()?;
    } else {
        for path in paths {
            if path.is_dir() {
                std::fs::remove_dir_all(&path)?;
            } else if path.exists() {
                std::fs::remove_file(&path)?;
            }
        }
    }

    Ok(())
}

pub fn rename_paths(
    renames: &mut [Rename],
) -> Result<(), Box<dyn std::error::Error>> {
    fn rollback(renames: &[Rename]) {
        for rename in renames {
            // TODO: don't ignore rollback errors
            if rename.completed {
                let _ = std::fs::rename(&rename.to, &rename.from);
            } else if let Some(temporary) = &rename.temporary {
                let _ = std::fs::rename(temporary, &rename.from);
            }
        }
    }

    let sources: HashSet<&PathBuf> =
        renames.iter().map(|r| &r.from).collect();

    for rename in renames.iter() {
        println!(
            "Rename: {} -> {}",
            rename.from.display(),
            rename.to.display()
        );

        if rename.to.exists() && !sources.contains(&rename.to) {
            return Err(
                format!("target path {} already exists", rename.to.display())
                    .into(),
            );
        }
    }

    if config::get().dry_run {
        return Ok(());
    }

    for (index, rename) in renames.iter_mut().enumerate() {
        let parent = rename.from.parent().unwrap_or(Path::new("."));

        let tempfile = parent.join(format!(
            ".dirit-{}-tmp{}",
            config::get().puid,
            index
        ));

        match std::fs::rename(&rename.from, &tempfile) {
            Ok(_) => rename.temporary = Some(tempfile),
            Err(e) => {
                rollback(renames);
                return Err(e.into());
            }
        }
    }

    for rename in renames.iter_mut() {
        if let Some(parent) = rename.to.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                rollback(renames);
                return Err(e.into());
            }
        }

        match std::fs::rename(
            rename.temporary.as_ref().unwrap(),
            &rename.to,
        ) {
            Ok(()) => rename.completed = true,
            Err(e) => {
                rollback(renames);
                return Err(e.into());
            }
        }
    }

    Ok(())
}

pub fn copy_paths(
    copies: &[Copy],
) -> Result<(), Box<dyn std::error::Error>> {
    for copy in copies {
        if !config::get().dry_run {
            std::fs::copy(&copy.from, &copy.to)?;
        }

        println!(
            "Copy: {} -> {}",
            copy.from.display(),
            copy.to.display()
        );
    }

    Ok(())
}

pub fn create_paths(
    paths: &[PathBuf],
) -> Result<(), Box<dyn std::error::Error>> {
    for path in paths {
        if !config::get().dry_run {
            if path.to_str().unwrap().ends_with("/") {
                std::fs::create_dir_all(&path)?;
            } else {
                std::fs::create_dir_all(&path.parent().unwrap())?;
                std::fs::File::create_new(path)?;
            }
        }

        println!("Create: {}", path.display());
    }

    Ok(())
}
