// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::config::ConfigData;
use crate::utils::keycloak::refresh_keycloak_token;
use crate::utils::read_config::{get_config_dir, read_config, CREATE_CONFIG_FILE_NAME};
use clap::Args;
use std::fs;
use std::path::Path;

#[derive(Args, Debug)]
#[command(about = "Refresh auth jwt", long_about = None)]
pub struct Refresh;

impl Refresh {
    pub fn run(&self) {
        match refresh_token() {
            Ok(_) => {}
            Err(err) => {
                eprintln!("Error! Failed to refresh token: {}", err)
            }
        }
    }
}

fn refresh_token() -> Result<(), Box<dyn std::error::Error>> {
    let config_data = read_config()?;
    let auth_details = refresh_keycloak_token(
        &config_data.keycloak_url,
        &config_data.refresh_token,
        &config_data.client_id,
        &config_data.client_secret,
        &config_data.tenant_id,
    )?;

    let config_data = ConfigData {
        endpoint_url: config_data.endpoint_url.clone(),
        tenant_id: config_data.tenant_id.clone(),
        keycloak_url: config_data.keycloak_url.clone(),
        auth_token: auth_details.access_token.clone(),
        refresh_token: auth_details.refresh_token.clone(),
        client_id: config_data.client_id.clone(),
        client_secret: config_data.client_secret.clone(),
        username: config_data.username.clone(),
    };

    let config_dir = get_config_dir()?;
    let config_file = config_dir.join(CREATE_CONFIG_FILE_NAME);

    if !Path::new(&config_dir).exists() {
        fs::create_dir_all(&config_dir)?;
    }

    let json_data = serde_json::to_string_pretty(&config_data)?;

    fs::write(&config_file, json_data)?;

    println!(
        "Success! Configuration refreshed successfully at {:?}",
        config_file
    );
    Ok(())
}
