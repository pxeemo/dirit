use std::sync::OnceLock;

#[derive(Debug)]
pub struct Config {
    pub puid: String,
    pub dry_run: bool,
}

pub static CONFIG: OnceLock<Config> = OnceLock::new();

pub fn init(config: Config) {
    CONFIG.set(config).expect("Config already initialized")
}

pub fn get() -> &'static Config {
    CONFIG.get().expect("Config not initialized")
}
