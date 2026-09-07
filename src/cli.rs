use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(name = "dirit", version)]
pub struct Args {
    pub paths: Vec<PathBuf>,

    #[arg(short, long)]
    pub recursive: bool,

    #[arg(long)]
    pub dry_run: bool,
}
