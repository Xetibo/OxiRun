use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub max_entries: usize,
    pub terminal: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_entries: 7,
            terminal: String::from("kitty"),
        }
    }
}

fn create_default_config(oxirun_config: &PathBuf) -> Config {
    let default_config = Config::default();
    let default_config_toml = toml::to_string(&default_config).expect("Could not serialize config");
    fs::File::options()
        .create(true)
        .write(true)
        .open(oxirun_config)
        .expect("Could not create config file")
        .write(&default_config_toml.into_bytes())
        .expect("Could not write into config file");
    default_config
}

fn read_config(oxirun_config: &PathBuf) -> Config {
    let mut read_config = String::new();
    let mut file = fs::File::open(oxirun_config).expect("Could not open config file");
    let _ = file
        .read_to_string(&mut read_config)
        .expect("Could not read config file");
    let config = toml::from_str(&read_config).expect("Could not deserialize config");
    config
}

pub fn get_config() -> Config {
    let base_dirs = xdg::BaseDirectories::new().expect("Could not get base directories");
    let config_home = base_dirs.get_config_home();
    let oxirun_dir = config_home.join("oxirun");
    if !oxirun_dir.is_dir() {
        std::fs::create_dir(&oxirun_dir).expect("Could not create config dir");
    }
    let oxirun_config = oxirun_dir.join("config.toml");
    if !oxirun_config.is_file() {
        create_default_config(&oxirun_config)
    } else {
        read_config(&oxirun_config)
    }
}
