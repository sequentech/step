// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use clap::Args;
use std::fs;
use std::path::Path;

use crate::{
    types::keycloak::KeycloakTokenResponse,
    utils::{
        keycloak::{generate_keycloak_token, get_auth_token_dir},
        read_config::read_config,
    },
};

#[derive(Args, Debug)]
#[command(about = "Create a keycloak auth token", long_about = None)]
pub struct GenerateToken {
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
}

impl GenerateToken {
    pub fn run(&self) {
        match create_auth_file(
            &self.keycloak_url,
            &self.keycloak_user,
            &self.keycloak_password,
            &self.keycloak_client_id,
        ) {
            Ok(path) => {
                println!("Successfully generated auth token at: {}", path);
            }
            Err(err) => {
                eprintln!("Failed to create auth token: {}", err)
            }
        }
    }
}

fn create_auth_file(
    keycloak_url: &str,
    username: &str,
    password: &str,
    client_id: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let config = read_config()?; // use teant_id from the config

    let auth_dir = get_auth_token_dir();
    let auth_file = auth_dir.join("authToken.json");

    if !Path::new(&auth_dir).exists() {
        fs::create_dir_all(&auth_dir).expect("Failed to create config directory");
    }
    let token_data = generate_keycloak_token(
        keycloak_url,
        username,
        password,
        client_id,
        &config.tenant_id,
    )?;

    let auth_data = KeycloakTokenResponse {
        access_token: token_data.access_token.clone(),
        refresh_token: token_data.access_token.clone(),
    };

    let json_data =
        serde_json::to_string_pretty(&auth_data).expect("Failed to serialize config data");

    fs::write(&auth_file, json_data).expect("Failed to write config file");

    let file_path = format!("{:?}", auth_file);
    Ok(file_path)
}
