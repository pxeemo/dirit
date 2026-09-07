use std::path::PathBuf;

pub struct Entry {
    pub id: usize,
    pub path: PathBuf,
}

pub struct Rename {
    pub from: PathBuf,
    pub to: PathBuf,
    pub temporary: Option<PathBuf>,
    pub completed: bool,
}

pub struct Copy {
    pub from: PathBuf,
    pub to: PathBuf,
}