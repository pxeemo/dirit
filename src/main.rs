mod cli;
mod config;
mod editor;
mod fs;
mod model;
mod operations;

use clap::Parser;
use model::Entry;
use uuid::Uuid;

fn process_edited_entries(
    entries: &[Entry],
    edited_entries: &[Entry],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut renames = Vec::new();
    let mut removes = Vec::new();
    let mut copies = Vec::new();

    let new_paths: Vec<std::path::PathBuf> = edited_entries
        .iter()
        .filter(|e| e.id == 0)
        .map(|e| e.path.clone())
        .collect();

    for entry in entries {
        let matched_entries: Vec<&Entry> = edited_entries
            .iter()
            .filter(|e| e.id == entry.id)
            .collect();

        if matched_entries.is_empty() {
            removes.push(entry.path.clone());
            continue;
        }

        let mut found_original = false;

        for matched_entry in &matched_entries {
            if matched_entry.path == entry.path {
                found_original = true;
            } else {
                copies.push(model::Copy {
                    from: entry.path.clone(),
                    to: matched_entry.path.clone(),
                });
            }
        }

        if !found_original {
            let last_copy = copies.pop().unwrap();

            renames.push(model::Rename {
                from: last_copy.from,
                to: last_copy.to,
                temporary: None,
                completed: false,
            });
        }
    }

    // order is important
    operations::remove_paths(&removes)?;
    operations::copy_paths(&copies)?;
    operations::create_paths(&new_paths)?;
    operations::rename_paths(&mut renames)?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = cli::Args::parse();

    config::init(config::Config {
        puid: Uuid::new_v4().simple().to_string()[..8].to_string(),
        dry_run: args.dry_run,
    });

    let (paths, new_paths) = fs::process_path_args(&args)?;

    let entries: Vec<Entry> = paths
        .iter()
        .enumerate()
        .map(|(index, path)| Entry {
            id: index + 1,
            path: path.clone(),
        })
        .collect();

    let edit_path = editor::create_edit_file(&entries, &new_paths)?;

    editor::run_editor(&edit_path)?;

    let edited_entries = editor::parse_edited_entries(&edit_path)?;

    process_edited_entries(&entries, &edited_entries)?;

    let _ = std::fs::remove_file(edit_path);

    Ok(())
}