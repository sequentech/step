use std::fs;
use std::path::PathBuf;
use std::env;
use serde_json;
use crate::commands::configure::ConfigData;


pub fn get_config_dir() -> PathBuf {
    let exe_path = env::current_exe().expect("Failed to get current executable path");
    exe_path.parent().expect("Failed to get executable directory").join("config")
}

pub fn read_config() -> Result<ConfigData, Box<dyn std::error::Error>> {
    let config_dir = get_config_dir();
    let config_file = config_dir.join("configuration.json");

    let json_data = fs::read_to_string(&config_file).expect("Failed to read config file, Plase make sure to run `sequent config` first");
    let config = serde_json::from_str(&json_data).expect("Failed to parse config file");
    Ok(config)
}