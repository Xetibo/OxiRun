use std::{fs, io::Read, path::PathBuf};

use iced_layershell::reexport::Anchor;
use toml::Table;

pub fn get_allowed_plugins(config: &Table) -> Vec<&str> {
    match config.get("plugins") {
        Some(toml::Value::Array(values)) => values
            .iter()
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

    toml::from_str(&read_config).expect("Could not deserialize config")
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

fn anchor_from_string(anchor_str: &str) -> Anchor {
    match anchor_str.to_lowercase().as_str() {
        "top" => Anchor::Top,
        "bottom" => Anchor::Bottom,
        "right" => Anchor::Right,
        "left" => Anchor::Left,
        _ => Anchor::Top,
    }
}

pub fn anchor_from_strings(anchor_strs: Vec<&str>) -> Anchor {
    let mut anchor = Anchor::empty();
    for anchor_str in anchor_strs {
        anchor |= anchor_from_string(anchor_str);
    }
    anchor
}
