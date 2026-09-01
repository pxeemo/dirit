use clap::Parser;
use std::collections::HashSet;
use std::io::IsTerminal;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "dirit", version)]
struct Args {
    paths: Vec<PathBuf>,
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
    temporary: PathBuf,
}

fn get_entries(dir: &PathBuf, offset_id: usize) -> Result<Vec<Entry>, Box<dyn std::error::Error>> {
    let dir_list = std::fs::read_dir(dir)?;
    let mut entries = Vec::new();
    for (index, entry) in dir_list.enumerate() {
        let entry = entry?;
        entries.push(Entry {
            id: index + offset_id + 1,
            path: entry.path(),
        });
    }
    Ok(entries)
}

fn create_edit_file(entries: &[Entry]) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let edit_path = "/tmp/dirit.txt";
    let mut file = std::fs::File::create(edit_path)?;
    for entry in entries.iter() {
        if entry.id == 0 {
            writeln!(file, "{}", entry.path.display())?;
        } else {
            writeln!(file, "{}\t{}", entry.id, entry.path.display())?;
        }
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
    // TODO: fallback to rm
    for path in paths {
        let status = std::process::Command::new("trash-put")
            .arg(&path)
            .status()?;

        if !status.success() {
            return Err(format!("failed to trash {}", path.display()).into());
        }
        println!("Delete: {}", path.display());
    }
    Ok(())
}

fn rename_files(renames: &[Rename]) -> Result<(), Box<dyn std::error::Error>> {
    for rename in renames.iter() {
        std::fs::rename(&rename.from, &rename.temporary)?;
    }
    for rename in renames.iter() {
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
        std::fs::create_dir_all(&path.parent().unwrap())?;
        std::fs::File::create_new(path)?;
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
        if entry.id == 0 {
            continue;
        }
        let edited = edited_entries.iter().find(|edited| edited.id == entry.id);
        match edited {
            Some(new) => {
                if new.path != entry.path {
                    renames.push(Rename {
                        from: entry.path.clone(),
                        to: new.path.clone(),
                        temporary: PathBuf::from(format!(".dirit-temp{}", index + 1)),
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
    let mut last_id = 0;
    for path in &args.paths {
        if path.is_dir() {
            let new_entries = get_entries(&path, last_id)?;
            last_id += new_entries.len();
            entries.extend(new_entries);
        } else {
            entries.push(Entry {
                id: if path.exists() {
                    last_id += 1;
                    last_id
                } else {
                    0
                },
                path: path.clone(),
            });
        }
    }
    if !std::io::stdin().is_terminal() {
        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer)?;
        let paths = buffer
            .lines()
            .map(|line| PathBuf::from(line))
            .collect::<Vec<_>>();
        for path in paths {
            entries.push(Entry {
                id: if path.exists() {
                    last_id += 1;
                    last_id
                } else {
                    0
                },
                path: path.clone(),
            });
        }
    } else if args.paths.is_empty() {
        entries.extend(get_entries(&PathBuf::from("."), last_id)?);
    }
    let edit_path = create_edit_file(&entries)?;
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
