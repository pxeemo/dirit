use std::{collections::BTreeMap, path::PathBuf};

pub type Entries = BTreeMap<usize, PathBuf>;
pub type EditedEntries = BTreeMap<usize, Vec<PathBuf>>;

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
