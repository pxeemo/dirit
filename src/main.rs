mod config;

use clap::Parser;
use std::collections::HashSet;
use std::io::{IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "dirit", version)]
struct Args {
    paths: Vec<PathBuf>,
    #[arg(short, long)]
    recursive: bool,
    #[arg(long)]
    dry_run: bool,
}

struct Entry {
    id: usize,
    path: PathBuf,
}

struct Rename {
    from: PathBuf,
    to: PathBuf,
    temporary: Option<PathBuf>,
    completed: bool,
}

struct Copy {
    from: PathBuf,
    to: PathBuf,
}

fn recursive_read_dir(dir: &Path) -> Result<HashSet<PathBuf>, Box<dyn std::error::Error>> {
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

fn get_dir_list(dir: &PathBuf) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let dir_list = std::fs::read_dir(dir)?;
    let mut paths = Vec::new();
    for entry in dir_list {
        let entry = entry?;
        paths.push(entry.path());
    }
    Ok(paths)
}

fn create_edit_file(
    entries: &[Entry],
    new_paths: &[PathBuf],
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let edit_path = std::env::temp_dir().join(format!(".dirit-{}", config::get().puid));
    let mut file = std::fs::File::create(&edit_path)?;
    let width = entries.len().to_string().len();
    for entry in entries {
        writeln!(
            file,
            "{:0width$}\t{}{}",
            entry.id,
            entry.path.display(),
            if entry.path.is_dir() { "/" } else { "" }
        )?;
    }
    for path in new_paths {
        writeln!(file, "{}", path.display())?;
    }
    Ok(PathBuf::from(edit_path))
}

fn parse_edited_entries(edit_path: &PathBuf) -> Result<Vec<Entry>, Box<dyn std::error::Error>> {
    let contents = std::fs::read_to_string(&edit_path)?;
    let mut entries = Vec::new();
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
        entries.push(Entry { id, path });
    }
    Ok(entries)
}

fn remove_paths(paths: &[PathBuf]) -> Result<(), Box<dyn std::error::Error>> {
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

fn rename_paths(renames: &mut [Rename]) -> Result<(), Box<dyn std::error::Error>> {
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
    let sources: HashSet<&PathBuf> = renames.iter().map(|r| &r.from).collect();
    for rename in renames.iter() {
        println!(
            "Rename: {} -> {}",
            rename.from.display(),
            rename.to.display()
        );
        if rename.to.exists() && !sources.contains(&rename.to) {
            return Err(format!("target path {} already exists", rename.to.display()).into());
        }
    }
    if config::get().dry_run {
        return Ok(());
    }
    for (index, rename) in renames.iter_mut().enumerate() {
        let parent = rename.from.parent().unwrap_or(Path::new("."));
        let tempfile = parent.join(format!(".dirit-{}-tmp{}", config::get().puid, index));
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
        match std::fs::rename(rename.temporary.as_ref().unwrap(), &rename.to) {
            Ok(()) => rename.completed = true,
            Err(e) => {
                rollback(renames);
                return Err(e.into());
            }
        }
    }
    Ok(())
}

fn copy_paths(copies: &[Copy]) -> Result<(), Box<dyn std::error::Error>> {
    for copy in copies {
        if !config::get().dry_run {
            std::fs::copy(&copy.from, &copy.to)?;
        }
        println!("Copy: {} -> {}", copy.from.display(), copy.to.display());
    }
    Ok(())
}

fn create_paths(paths: &[PathBuf]) -> Result<(), Box<dyn std::error::Error>> {
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

fn process_edited_entries(
    entries: &[Entry],
    edited_entries: &[Entry],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut renames = Vec::new();
    let mut removes = Vec::new();
    let mut copies = Vec::new();
    let new_paths: Vec<PathBuf> = edited_entries
        .iter()
        .filter(|e| e.id == 0)
        .map(|e| e.path.clone())
        .collect();

    for entry in entries {
        let matched_entries: Vec<&Entry> =
            edited_entries.iter().filter(|e| e.id == entry.id).collect();

        if matched_entries.is_empty() {
            removes.push(entry.path.clone());
            continue;
        }

        let mut found_original = false;
        for matched_entry in &matched_entries {
            if matched_entry.path == entry.path {
                found_original = true;
            } else {
                copies.push(Copy {
                    from: entry.path.clone(),
                    to: matched_entry.path.clone(),
                });
            }
        }
        if !found_original {
            let last_copy = copies.pop().unwrap();
            renames.push(Rename {
                from: last_copy.from,
                to: last_copy.to,
                temporary: None,
                completed: false,
            });
        }
    }

    // order is important
    remove_paths(&removes)?;
    copy_paths(&copies)?;
    create_paths(&new_paths)?;
    rename_paths(&mut renames)?;
    Ok(())
}

fn process_path_args(
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
            .map(|line| PathBuf::from(line))
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
    let mut sorted_paths = paths.iter().map(|p| p.clone()).collect::<Vec<PathBuf>>();
    sorted_paths.sort();
    let mut sorted_new_paths = new_paths
        .iter()
        .map(|p| p.clone())
        .collect::<Vec<PathBuf>>();
    sorted_new_paths.sort();
    Ok((sorted_paths, sorted_new_paths))
}

fn run_editor(edit_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
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
        .arg(&edit_path)
        .status()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    config::init(config::Config {
        puid: Uuid::new_v4().simple().to_string()[..8].to_string(),
        dry_run: args.dry_run,
    });

    let mut entries = Vec::new();
    let (paths, new_paths) = process_path_args(&args)?;
    for (index, path) in paths.iter().enumerate() {
        entries.push(Entry {
            id: index + 1,
            path: path.clone(),
        });
    }

    let edit_path = create_edit_file(&entries, &new_paths)?;
    run_editor(&edit_path)?;

    let edited_entries = parse_edited_entries(&edit_path)?;
    process_edited_entries(&entries, &edited_entries)?;

    let _ = std::fs::remove_file(edit_path);
    Ok(())
}
