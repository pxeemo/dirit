use std::collections::HashSet;
use std::io::Write;

struct Entry {
    id: usize,
    path: std::path::PathBuf,
}

struct Rename {
    from: std::path::PathBuf,
    to: std::path::PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let entries = std::fs::read_dir(".")?;
    let mut entries_vec = Vec::new();
    for (index, entry) in entries.enumerate() {
        let entry = entry?;
        entries_vec.push(Entry {
            id: index + 1,
            path: entry.path(),
        });
    }

    let edit_path = "/tmp/dirit.txt";
    let mut file = std::fs::File::create(edit_path)?;
    for entry in &entries_vec {
        writeln!(file, "{}\t{}", entry.id, entry.path.display())?;
    }

    let editor = std::env::var("EDITOR")?;
    std::process::Command::new(editor)
        .arg(edit_path)
        .status()?;

    let contents = std::fs::read_to_string(edit_path)?;
    let mut edited_entries = Vec::new();
    let mut paths = HashSet::new();
    for line in contents.lines() {
        let (id, path) = match line.split_once('\t') {
            Some(parts) => parts,
            None => continue,
        };
        let id: usize = id.parse()?;
        let path = std::path::PathBuf::from(path);
        if !paths.insert(path.clone()) {
            return Err(std::io::Error::other(format!(
                "Duplicate paths found: {}",
                path.display()
            )).into());
        }
        edited_entries.push(Entry { id, path });
    }

    let mut renames = Vec::new();
    for entry in &entries_vec {
        let edited = edited_entries.iter().find(|edited| edited.id == entry.id);
        match edited {
            Some(new) => {
                if new.path != entry.path {
                    renames.push(Rename {
                        from: entry.path.clone(),
                        to: new.path.clone(),
                    });
                }
            }
            None => println!("Delete: {}", entry.path.display()),
        }
    }

    for rename in &renames {
        println!(
            "Rename: {} -> {}",
            rename.from.display(),
            rename.to.display()
        );
    }

    Ok(())
}
