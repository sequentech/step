// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde_json;
use serde_json::Value;
use std::env;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use crate::types::config::{ConfigData, ExternalConfigData};

pub const CREATE_CONFIG_FILE_NAME: &str = "configuration.json";
pub const EXTERNAL_CONFIG_FILE_NAME: &str = "external_config.json";

pub fn get_config_dir() -> Result<PathBuf, Box<dyn Error>> {
    let exe_path = env::current_exe().map_err(|_| "Failed to get current executable path")?;
    let parent_dir = exe_path
        .parent()
        .ok_or("Failed to get executable directory")?;
    Ok(parent_dir.join("config"))
}

pub fn read_config() -> Result<ConfigData, Box<dyn Error>> {
    let config_dir = get_config_dir()?;
    let config_file = config_dir.join(CREATE_CONFIG_FILE_NAME);

    let json_data = fs::read_to_string(&config_file).map_err(|_| {
        "Failed to read config file, Please make sure to run `sequent config` first"
    })?;
    let config = serde_json::from_str(&json_data).map_err(|_| "Failed to parse config file")?;
    Ok(config)
}

pub fn load_external_config(working_dir: &str) -> Result<ExternalConfigData, Box<dyn Error>> {
    let config_path = PathBuf::from(working_dir).join(EXTERNAL_CONFIG_FILE_NAME);
    let file = File::open(config_path)?;
    let reader = BufReader::new(file);
    let config = serde_json::from_reader(reader)?;
    Ok(config)
}
