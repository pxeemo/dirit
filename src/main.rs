use std::collections::HashSet;
use std::io::Write;
use std::path::PathBuf;

struct Entry {
    id: usize,
    path: PathBuf,
}

struct Rename {
    from: PathBuf,
    to: PathBuf,
    temporary: PathBuf,
}

fn get_entries() -> Result<Vec<Entry>, Box<dyn std::error::Error>> {
    let dir_list = std::fs::read_dir(".")?;
    let mut entries = Vec::new();
    for (index, entry) in dir_list.enumerate() {
        let entry = entry?;
        entries.push(Entry {
            id: index + 1,
            path: entry.path(),
        });
    }
    Ok(entries)
}

fn create_edit_file(entries: &[Entry]) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let edit_path = "/tmp/dirit.txt";
    let mut file = std::fs::File::create(edit_path)?;
    for entry in entries.iter() {
        writeln!(file, "{}\t{}", entry.id, entry.path.display())?;
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
            None => {
                let line = PathBuf::from(line.trim());
                std::fs::create_dir_all(&line.parent().unwrap())?;
                std::fs::File::create_new(&line)?;
                println!("Create: {}", &line.display());
                continue;
            }
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
    let entries = get_entries()?;
    let edit_path = create_edit_file(&entries)?;
    // TODO: get editor properly
    let editor = std::env::var("EDITOR")?;
    std::process::Command::new(editor)
        .arg(&edit_path)
        .status()?;

    let edited_entries = parse_edited_entries(&edit_path)?;
    process_edited_entries(&entries, &edited_entries)?;
    Ok(())
}
