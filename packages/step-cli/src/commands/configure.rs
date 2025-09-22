// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::config::ConfigData;
use crate::utils::keycloak::generate_keycloak_token;
use crate::utils::read_config::{get_config_dir, CREATE_CONFIG_FILE_NAME};
use clap::Args;
use std::fs;
use std::path::Path;

#[derive(Args, Debug)]
#[command(about = "Create a config file", long_about = None)]
pub struct Config {
    /// Tenant ID
    #[arg(long)]
    tenant_id: String,

    /// Endpoint URL
    #[arg(long)]
    endpoint_url: String,

    /// Keycloak endpoint URL
    #[arg(long)]
    keycloak_url: String,

    /// Keycloak user name
    #[arg(long)]
    keycloak_user: String,

    /// Keycloak password
    #[arg(long)]
    keycloak_password: String,

    /// Keycloak Client ID
    #[arg(long)]
    keycloak_client_id: String,

    /// Keycloak Client secret
    #[arg(long)]
    keycloak_client_secret: String,
}

impl Config {
    pub fn run(&self) {
        match create_config(
            &self.endpoint_url,
            &self.keycloak_url,
            &self.keycloak_user,
            &self.keycloak_password,
            &self.keycloak_client_id,
            &self.keycloak_client_secret,
            &self.tenant_id,
        ) {
            Ok(_) => {}
            Err(err) => {
                eprintln!("Error! Failed to create configuration file: {}", err)
            }
        }
    }
}

pub fn create_config(
    endpoint_url: &str,
    keycloak_url: &str,
    username: &str,
    password: &str,
    client_id: &str,
    client_secret: &str,
    tenant_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let auth_details = generate_keycloak_token(
        &keycloak_url,
        &username,
        &password,
        &client_id,
        &client_secret,
        &tenant_id,
    )?;
    let config_data = ConfigData {
        endpoint_url: endpoint_url.to_string(),
        tenant_id: tenant_id.to_string(),
        keycloak_url: keycloak_url.to_string(),
        auth_token: auth_details.access_token.clone(),
        refresh_token: auth_details.refresh_token.clone(),
        client_id: client_id.to_string(),
        client_secret: client_secret.to_string(),
        username: username.to_string(),
    };

    let config_dir = get_config_dir()?;
    let config_file = config_dir.join(CREATE_CONFIG_FILE_NAME);

    if !Path::new(&config_dir).exists() {
        fs::create_dir_all(&config_dir)?;
    }

    let json_data = serde_json::to_string_pretty(&config_data)?;

    fs::write(&config_file, json_data)?;

    println!(
        "Success! Configuration saved successfully at {:?}",
        config_file
    );
    Ok(())
}
