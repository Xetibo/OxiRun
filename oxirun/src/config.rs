use std::{fs, io::Read, path::PathBuf};

use toml::Table;

pub fn get_allowed_plugins<'a>(config: &'a Table) -> Vec<&'a str> {
    match config.get("plugins") {
        Some(toml::Value::Array(values)) => values
            .into_iter()
            .filter_map(|value| match value {
                toml::Value::String(name) => Some(name.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    }
}

fn read_config(oxirun_config: &PathBuf) -> Table {
    let mut read_config = String::new();
    let mut file = fs::File::open(oxirun_config).expect("Could not open config file");
    let _ = file
        .read_to_string(&mut read_config)
        .expect("Could not read config file");
    let config = toml::from_str(&read_config).expect("Could not deserialize config");
    config
}

pub fn get_oxirun_dir() -> PathBuf {
    let base_dirs = xdg::BaseDirectories::new();
    let config_home = base_dirs.get_config_home();
    let oxirun_dir = config_home
        .expect("Could not get config home")
        .join("oxirun");
    if !oxirun_dir.is_dir() {
        std::fs::create_dir(&oxirun_dir).expect("Could not create config dir");
    }
    oxirun_dir
}

pub fn get_config() -> Table {
    let oxirun_dir = get_oxirun_dir();
    let oxirun_config = oxirun_dir.join("config.toml");
    if !oxirun_config.is_file() {
        Table::new()
    } else {
        read_config(&oxirun_config)
    }
}
