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

fn recursive_read_dir(dir: &Path) -> Result<HashSet<PathBuf>, Box<dyn std::error::Error>> {
    let mut paths = HashSet::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
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
    new_files: &[PathBuf],
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let suffix = &Uuid::new_v4().simple().to_string()[..8];
    let edit_path = std::env::temp_dir().join(format!("dirit{}", suffix));
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
    for path in new_files {
        writeln!(file, "{}", path.display())?;
    }
    Ok(PathBuf::from(edit_path))
}

fn parse_edited_entries(
    edit_path: &PathBuf,
) -> Result<(Vec<Entry>, Vec<PathBuf>), Box<dyn std::error::Error>> {
    let contents = std::fs::read_to_string(&edit_path)?;
    let mut entries = Vec::new();
    let mut paths = HashSet::new();
    let mut new_files = Vec::new();
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
        if id == 0 {
            new_files.push(path.clone());
        } else {
            entries.push(Entry { id, path });
        }
    }
    Ok((entries, new_files))
}

fn delete_files(paths: &[PathBuf], dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let has_trash = which::which("trash-put").is_ok();
    for path in paths {
        println!(
            "{}: {}",
            if has_trash { "Trash" } else { "Delete" },
            path.display()
        );
    }
    if dry_run || paths.is_empty() {
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

fn rename_files(renames: &mut [Rename], dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
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
    let suffix = &Uuid::new_v4().simple().to_string()[..8];
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
    if dry_run {
        return Ok(());
    }
    for (index, rename) in renames.iter_mut().enumerate() {
        let parent = rename.from.parent().unwrap_or(Path::new("."));
        let tempfile = parent.join(format!(".dirit-tmp{}-{}", suffix, index));
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

fn create_files(paths: &[PathBuf], dry_run: &bool) -> Result<(), Box<dyn std::error::Error>> {
    for path in paths {
        if !dry_run {
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
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut renames = Vec::new();
    let mut deletes = Vec::new();
    for entry in entries {
        let edited = edited_entries.iter().find(|edited| edited.id == entry.id);
        match edited {
            Some(new) => {
                if new.path != entry.path {
                    renames.push(Rename {
                        from: entry.path.clone(),
                        to: new.path.clone(),
                        temporary: None,
                        completed: false,
                    });
                }
            }
            None => deletes.push(entry.path.clone()),
        }
    }
    rename_files(&mut renames, dry_run)?;
    delete_files(&deletes, dry_run)?;
    Ok(())
}

fn process_path_args(
    args: &Args,
) -> Result<(Vec<PathBuf>, Vec<PathBuf>), Box<dyn std::error::Error>> {
    let mut paths = HashSet::<PathBuf>::new();
    let mut new_paths = HashSet::<PathBuf>::new();
    for path in &args.paths {
        if args.recursive && path.is_dir() && !path.is_symlink() {
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
            if args.recursive && path.is_dir() && !path.is_symlink() {
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

    let (edited_entries, new_files) = parse_edited_entries(&edit_path)?;
    create_files(&new_files, &args.dry_run)?;
    process_edited_entries(&entries, &edited_entries, args.dry_run)?;
    Ok(())
}
