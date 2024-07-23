// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde_json;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use crate::types::config::ConfigData;

pub fn get_config_dir() -> Result<PathBuf, Box<dyn Error>> {
    let exe_path = env::current_exe().map_err(|_| "Failed to get current executable path")?;
    let parent_dir = exe_path
        .parent()
        .ok_or("Failed to get executable directory")?;
    Ok(parent_dir.join("config"))
}

pub fn read_config() -> Result<ConfigData, Box<dyn Error>> {
    let config_dir = get_config_dir()?;
    let config_file = config_dir.join("configuration.json");

    let json_data = fs::read_to_string(&config_file).map_err(|_| {
        "Failed to read config file, Please make sure to run `sequent config` first"
    })?;
    let config = serde_json::from_str(&json_data).map_err(|_| "Failed to parse config file")?;
    Ok(config)
}
