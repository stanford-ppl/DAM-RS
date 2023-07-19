// This file defines the basic config that we'll be fetching from:
// $HOME
// $PWD
// $DAM_CONFIG_PATH (if set)

use std::{fs, path::PathBuf, sync::OnceLock};

use serde::{Deserialize, Serialize};

use crate::log_config::LogConfig;

static DEFAULT_CONFIG: OnceLock<Config> = OnceLock::new();

const FILE_NAME: &'static str = "dam-config.toml";

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Config {
    pub log_config: LogConfig,
}

impl Config {
    pub fn update(&mut self, other: Config) {
        if !other.log_config.validate() {
            panic!("Invalid Log Config!");
        }
        self.log_config.update(other.log_config);
    }
}

pub fn get_config() -> Config {
    DEFAULT_CONFIG
        .get_or_init(|| {
            let mut paths_to_check = vec![
                // Check $HOME
                home::home_dir(),
                // Check $PWD
                std::env::current_dir().map(Some).unwrap_or(None),
            ];

            match std::env::var("DAM_CONFIG_PATH") {
                Ok(path) => paths_to_check.push(Some(PathBuf::from(path))),
                Err(_) => {}
            }

            let mut config = Config::default();
            for path in paths_to_check.iter().flat_map(|x| x) {
                let data = fs::read_to_string(path.join(FILE_NAME));
                if let Ok(str) = data {
                    let read = toml::from_str::<Config>(&str);
                    match read {
                        Ok(read_config) => config.update(read_config),
                        Err(err) => panic!("Invalid input read from {:?} -- {}", path, err),
                    }
                }
            }
            config
        })
        .clone()
}
