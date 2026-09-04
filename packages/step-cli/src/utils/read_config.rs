// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde_json;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use crate::types::config::ConfigData;
use crate::utils::keycloak::refresh_keycloak_token;

pub use sequent_core::util::external_config::load_external_config;
pub use sequent_core::util::external_config::EXTERNAL_CONFIG_FILE_NAME;

pub const CREATE_CONFIG_FILE_NAME: &str = "configuration.json";

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

pub fn write_config(config_data: &ConfigData) -> Result<PathBuf, Box<dyn Error>> {
    let config_dir = get_config_dir()?;
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }
    let config_file = config_dir.join(CREATE_CONFIG_FILE_NAME);
    let json_data = serde_json::to_string_pretty(config_data)?;
    fs::write(&config_file, json_data)?;
    Ok(config_file)
}

/// Refreshes the session's JWT via the stored refresh_token and persists the
/// new tokens to disk, so any later read_config() call (e.g. from a polling
/// loop) picks up the refreshed access token instead of the one the session
/// started with.
pub fn refresh_and_save_token() -> Result<ConfigData, Box<dyn Error>> {
    let config_data = read_config()?;
    let auth_details = refresh_keycloak_token(
        &config_data.keycloak_url,
        &config_data.refresh_token,
        &config_data.client_id,
        &config_data.client_secret,
        &config_data.tenant_id,
    )?;
    let config_data = ConfigData {
        auth_token: auth_details.access_token,
        refresh_token: auth_details.refresh_token,
        ..config_data
    };
    write_config(&config_data)?;
    Ok(config_data)
}
