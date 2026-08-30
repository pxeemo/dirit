use std::fs::DirEntry;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let entries = std::fs::read_dir(".")?;
    let entries_vec = entries.collect::<Result<Vec<DirEntry>, std::io::Error>>()?;
    let mut file = std::fs::File::create("dirit.txt")?;
    for (index, entry) in entries_vec.iter().enumerate() {
        writeln!(file, "{}\t{}", index + 1, entry.path().display())?;
    }

    let editor = std::env::var("EDITOR")?;
    std::process::Command::new(editor)
        .arg("dirit.txt")
        .status()?;

    let contents = std::fs::read_to_string("dirit.txt")?;
    println!("{contents}");

    Ok(())
}
