// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use deadpool_postgres::{Config as PgConfig, Pool, Runtime};
use serde_json;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use tokio_postgres::NoTls;

use crate::types::keycloak::KeycloakTokenResponse;

pub fn generate_keycloak_token(
    keycloak_url: &str,
    username: &str,
    password: &str,
    client_id: &str,
    client_secret: &str,
    tenant_id: &str,
) -> Result<KeycloakTokenResponse, Box<dyn Error>> {
    let params = [
        ("grant_type", "password"),
        ("scope", "openid"),
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("username", username),
        ("password", password),
        ("acr_values", "gold")
    ];
    println!("params: {:?}", params);
    let realm = format!("tenant-{}", tenant_id);
    let url = format!(
        "{}/realms/{}/protocol/openid-connect/token",
        keycloak_url, realm
    );

    let client = reqwest::blocking::Client::new();
    let response = client.post(&url).form(&params).send()?;

    if response.status().is_success() {
        let token_response: KeycloakTokenResponse = response.json()?;
        print!("{}\n", token_response.access_token);
        Ok(token_response)
    } else {
        let status = response.status();
        let error_message = response.text()?;
        let error = format!("HTTP Status: {}\nError Message: {}", status, error_message);
        Err(Box::from(error))
    }
}

pub fn refresh_keycloak_token(
    keycloak_url: &str,
    refresh_token: &str,
    client_id: &str,
    client_secret: &str,
    tenant_id: &str,
) -> Result<KeycloakTokenResponse, Box<dyn Error>> {
    let params = [
        ("grant_type", "refresh_token"),
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("refresh_token", refresh_token),
    ];

    let realm = format!("tenant-{}", tenant_id);
    let url = format!(
        "{}/realms/{}/protocol/openid-connect/token",
        keycloak_url, realm
    );

    let client = reqwest::blocking::Client::new();
    let response = client.post(&url).form(&params).send()?;

    if response.status().is_success() {
        let token_response: KeycloakTokenResponse = response.json()?;
        Ok(token_response)
    } else {
        let status = response.status();
        let error_message = response.text()?;
        let error = format!("HTTP Status: {}\nError Message: {}", status, error_message);
        Err(Box::from(error))
    }
}

pub fn get_auth_token_dir() -> PathBuf {
    let exe_path = env::current_exe().expect("Failed to get current executable path");
    exe_path
        .parent()
        .expect("Failed to get executable directory")
        .join("keycloak")
}

pub fn read_token() -> Result<KeycloakTokenResponse, Box<dyn std::error::Error>> {
    let auth_dir = get_auth_token_dir();
    let auth_file = auth_dir.join("authToken.json");

    let json_data = fs::read_to_string(&auth_file)
        .expect("Failed to read auth file, Plase make sure to run `sequent generate-auth` first");
    let auth_data = serde_json::from_str(&json_data).expect("Failed to parse auth file");
    Ok(auth_data)
}

pub async fn get_keyckloak_pool() -> Result<Pool, Box<dyn std::error::Error>> {
    let mut kc_cfg = PgConfig::default();
    kc_cfg.host = Some(env::var("KC_DB_URL_HOST")?);
    kc_cfg.port = Some(env::var("KC_DB_URL_PORT")?.parse::<u16>()?);
    kc_cfg.user = Some(env::var("KC_DB_USERNAME")?);
    kc_cfg.password = Some(env::var("KC_DB_PASSWORD")?);
    kc_cfg.dbname = Some(env::var("KC_DB")?);
    Ok(kc_cfg.create_pool(Some(Runtime::Tokio1), NoTls)?)
}
