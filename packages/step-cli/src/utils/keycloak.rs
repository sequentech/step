// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use deadpool_postgres::{Config as PgConfig, Pool, Runtime};
use std::env;
use std::error::Error;
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

pub async fn get_keyckloak_pool() -> Result<Pool, Box<dyn std::error::Error>> {
    let kc_cfg = PgConfig {
        host: Some(env::var("KC_DB_URL_HOST")?),
        port: Some(env::var("KC_DB_URL_PORT")?.parse::<u16>()?),
        user: Some(env::var("KC_DB_USERNAME")?),
        password: Some(env::var("KC_DB_PASSWORD")?),
        dbname: Some(env::var("KC_DB")?),
        ..Default::default()
    };
    Ok(kc_cfg.create_pool(Some(Runtime::Tokio1), NoTls)?)
}
