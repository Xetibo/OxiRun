use optional_struct::{Applicable, optional_struct};
use serde::{Deserialize, Serialize};
use toml::Table;

#[optional_struct]
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

pub fn get_config(global_config: Table) -> Config {
    let default_config = Config::default();
    if let Some(config_value) = global_config.get("applications") {
        let config: OptionalConfig =
            // TODO how to not reserialize this?
            toml::from_str(&toml::to_string(config_value).expect("Could not reserialize"))
                .expect("Could not deserialize config");
        config.build(default_config)
    } else {
        default_config
    }
}
