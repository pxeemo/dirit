use std::io::Write;

struct Entry {
    id: usize,
    path: std::path::PathBuf,
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

    let mut file = std::fs::File::create("dirit.txt")?;
    for entry in &entries_vec {
        writeln!(file, "{}\t{}", entry.id, entry.path.display())?;
    }

    let editor = std::env::var("EDITOR")?;
    std::process::Command::new(editor)
        .arg("dirit.txt")
        .status()?;

    let contents = std::fs::read_to_string("dirit.txt")?;
    let mut editor_entries = Vec::new();
    for line in contents.lines() {
        let (id, path) = match line.split_once('\t') {
            Some(parts) => parts,
            None => continue,
        };
        let id: usize = id.parse()?;
        let path = std::path::PathBuf::from(path);
        editor_entries.push(Entry { id, path });
    }

    for entry in &editor_entries {
        println!("{}\t{}", entry.id, entry.path.display())
    }

    Ok(())
}
