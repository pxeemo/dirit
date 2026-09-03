use clap::Parser;
use std::collections::HashSet;
use std::io::{IsTerminal, Read, Write};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "dirit", version)]
struct Args {
    paths: Vec<PathBuf>,
    #[arg(short, long)]
    recursive: bool,
}

struct Entry {
    id: usize,
    path: PathBuf,
}

struct Rename {
    from: PathBuf,
    to: PathBuf,
    temporary: PathBuf,
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
    let edit_path = std::env::temp_dir().join("dirit.txt");
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

fn delete_files(paths: &[PathBuf]) -> Result<(), Box<dyn std::error::Error>> {
    if !paths.is_empty()
        && std::process::Command::new("trash-put")
            .args(paths)
            .status()
            .is_ok()
    {
        for path in paths {
            println!("Trash: {}", path.display());
        }
    } else {
        for path in paths {
            if path.is_dir() {
                std::fs::remove_dir_all(&path)?;
            } else if path.exists() {
                std::fs::remove_file(&path)?;
            }
            println!("Delete: {}", path.display());
        }
    }
    Ok(())
}

fn rename_files(renames: &[Rename]) -> Result<(), Box<dyn std::error::Error>> {
    for rename in renames.iter() {
        std::fs::rename(&rename.from, &rename.temporary)?;
    }
    for rename in renames.iter() {
        if rename.to.exists() {
            std::fs::rename(&rename.temporary, &rename.from)?;
            return Err(format!("target path {} already exists", rename.to.display()).into());
        }
        std::fs::create_dir_all(&rename.to.parent().unwrap())?;
        std::fs::rename(&rename.temporary, &rename.to)?;
        println!(
            "Rename: {} -> {}",
            rename.from.display(),
            rename.to.display()
        );
    }
    Ok(())
}

fn create_files(paths: &[PathBuf]) -> Result<(), Box<dyn std::error::Error>> {
    for path in paths {
        if path.to_str().unwrap().ends_with("/") {
            std::fs::create_dir_all(&path)?;
        } else {
            std::fs::create_dir_all(&path.parent().unwrap())?;
            std::fs::File::create_new(path)?;
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
    let mut deletes = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let edited = edited_entries.iter().find(|edited| edited.id == entry.id);
        match edited {
            Some(new) => {
                if new.path != entry.path {
                    let parent = entry.path.parent().ok_or_else(|| {
                        std::io::Error::new(std::io::ErrorKind::InvalidInput, "Path has no parent")
                    })?;
                    renames.push(Rename {
                        from: entry.path.clone(),
                        to: new.path.clone(),
                        temporary: parent.join(format!(".dirit-temp{}", index + 1)),
                    });
                }
            }
            None => deletes.push(entry.path.clone()),
        }
    }
    rename_files(&renames)?;
    delete_files(&deletes)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut entries = Vec::new();
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
    for (index, path) in sorted_paths.iter().enumerate() {
        entries.push(Entry {
            id: index + 1,
            path: path.clone(),
        });
    }
    let edit_path = create_edit_file(&entries, &sorted_new_paths)?;
    // TODO: get editor properly
    let editor = std::env::var("EDITOR")?;
    std::process::Command::new(editor)
        .arg(&edit_path)
        .status()?;

    let (edited_entries, new_files) = parse_edited_entries(&edit_path)?;
    create_files(&new_files)?;
    process_edited_entries(&entries, &edited_entries)?;
    Ok(())
}
