mod cli;
mod config;
mod editor;
mod fs;
mod model;
mod operations;

use clap::Parser;
use model::{EditedEntries, Entries};
use uuid::Uuid;

fn process_edited_entries(
    entries: &Entries,
    edited_entries: &EditedEntries,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut renames = Vec::new();
    let mut removes = Vec::new();
    let mut copies = Vec::new();

    let new_paths = edited_entries.get(&0).cloned().unwrap_or_default();

    for (entry_id, entry_paths) in entries {
        let edited_paths = edited_entries.get(&entry_id).cloned().unwrap_or_default();

        // if entry not found in edited entries, remove it
        if edited_paths.is_empty() {
            removes.push(entry_paths.clone());
            continue;
        }

        let mut found_original = false;

        for path in &edited_paths {
            if path == entry_paths {
                found_original = true;
            } else {
                copies.push(model::Copy {
                    from: entry_paths.clone(),
                    to: path.clone(),
                });
            }
        }

        /*
         * if the original path is found,
         * it means the entry is not renamed or copied
         *
         * otherwise, the entry is renamed
         * so it must be removed from the copies
         * and be added to the renames list after the loop
         */
        if !found_original {
            /*
             * if there were more than one copy
             * then the entries except the last one will be copied
             * and then the last entry will be renamed
             *
             * if there was only one copy,
             * then it won't be copied at all
             * and the entry will be renamed directly
             */
            let last_copy = copies.pop().unwrap();

            renames.push(model::Rename {
                from: last_copy.from,
                to: last_copy.to,
                temporary: None,
                completed: false,
            });
        }
    }

    // removes must happen before any other operations
    // copies must happen before renames
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

    let entries: Entries = paths
        .iter()
        .enumerate()
        .map(|(index, path)| (index + 1, path.clone()))
        .collect();

    let edit_path = editor::create_edit_file(&entries, &new_paths)?;

    editor::run_editor(&edit_path)?;

    let edited_entries = editor::parse_edited_entries(&edit_path)?;

    process_edited_entries(&entries, &edited_entries)?;

    let _ = std::fs::remove_file(edit_path);

    Ok(())
}
