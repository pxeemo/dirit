use std::{
    collections::{BTreeMap, HashSet},
    io::Write,
    path::{Path, PathBuf},
};

use crate::{
    config,
    model::{EditedEntries, Entries},
};

pub fn create_edit_file(
    entries: &Entries,
    new_paths: &[PathBuf],
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let edit_path = std::env::temp_dir().join(format!(".dirit-{}", config::get().puid));

    let mut file = std::fs::File::create(&edit_path)?;
    let width = entries.len().to_string().len();

    for (id, path) in entries {
        writeln!(
            file,
            "{:0width$}\t{}{}",
            id,
            path.display(),
            if path.is_dir() { "/" } else { "" }
        )?;
    }

    for path in new_paths {
        writeln!(file, "{}", path.display())?;
    }

    Ok(PathBuf::from(edit_path))
}

pub fn parse_edited_entries(edit_path: &Path) -> Result<EditedEntries, Box<dyn std::error::Error>> {
    let contents = std::fs::read_to_string(edit_path)?;
    let mut entries: EditedEntries = BTreeMap::new();
    let mut paths = HashSet::new();

    for line in contents.lines() {
        let (id, path) = match line.split_once('\t') {
            Some(parts) => parts,
            None => ("0", line.trim()),
        };

        let id: usize = id.parse()?;
        let path = PathBuf::from(path);

        if !paths.insert(path.clone()) {
            return Err(std::io::Error::other(format!(
                "Duplicate paths found: {}",
                path.display()
            ))
            .into());
        }

        match entries.get_mut(&id) {
            Some(existing) => existing.push(path),
            None => {
                entries.insert(id, vec![path]);
            }
        }
    }

    Ok(entries)
}

pub fn run_editor(edit_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let editor = std::env::var("VISUAL")
        .or(std::env::var("EDITOR"))
        .or_else(|_| {
            for editor in ["nvim", "vim", "micro", "nano", "vi"] {
                if which::which(editor).is_ok() {
                    return Ok(editor.to_string());
                }
            }

            Err("$VISUAL and $EDITOR are empty and no suitable editor was found.")
        })?;

    let parts = shell_words::split(&editor)?;
    let (program, args) = parts.split_first().ok_or("editor is empty")?;

    std::process::Command::new(program)
        .args(args)
        .arg(edit_path)
        .status()?;

    Ok(())
}
