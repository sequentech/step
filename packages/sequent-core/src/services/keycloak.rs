// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::connection;
use anyhow::{anyhow, Result};
use keycloak::{
    types::*,
    {KeycloakAdmin, KeycloakAdminToken},
};
use reqwest;
use serde::{Deserialize, Serialize};
use serde_urlencoded;
use std::env;
use tracing::{event, Level, instrument};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: i64,
    pub refresh_expires_in: i64,
    pub token_type: String,
    pub id_token: String,
    #[serde(rename = "not-before-policy")]
    pub not_before_policy: i64,
    pub scope: String,
}

struct KeycloakLoginConfig {
    url: String,
    client_id: String,
    client_secret: String,
}

fn get_keycloak_login_config() -> KeycloakLoginConfig {
    let url =
        env::var("KEYCLOAK_URL").expect(&format!("KEYCLOAK_URL must be set"));
    let client_id = env::var("KEYCLOAK_CLIENT_ID")
        .expect(&format!("KEYCLOAK_CLIENT_ID must be set"));
    let client_secret = env::var("KEYCLOAK_CLIENT_SECRET")
        .expect(&format!("KEYCLOAK_CLIENT_SECRET must be set"));
    KeycloakLoginConfig {
        url,
        client_id,
        client_secret,
    }
}

// Client Credentials OpenID Authentication flow.
// This enables servers to authenticate, without using a browser.
#[instrument]
pub async fn get_client_credentials() -> Result<connection::AuthHeaders> {
    let login_config = get_keycloak_login_config();
    let body_string = serde_urlencoded::to_string::<[(String, String); 4]>([
        ("client_id".into(), login_config.client_id.clone()),
        ("scope".into(), "openid".into()),
        ("client_secret".into(), login_config.client_secret.clone()),
        ("grant_type".into(), "client_credentials".into()),
    ])
    .unwrap();

    let keycloak_endpoint = format!(
        "{}/realms/electoral-process/protocol/openid-connect/token",
        login_config.url
    );

    let client = reqwest::Client::new();
    let res = client
        .post(keycloak_endpoint)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body_string)
        .send()
        .await?;
    let text = res.text().await?;

    let credentials: TokenResponse = serde_json::from_str(&text)
        .map_err(|err| anyhow!(format!("{:?}, Response: {}", err, text)))?;
    event!(Level::INFO, "Successfully acquired credentials");
    Ok(connection::AuthHeaders {
        key: "authorization".into(),
        value: format!("Bearer {}", credentials.access_token),
    })
}

#[instrument]
pub async fn get_keycloak_client() -> Result<KeycloakAdmin> {
    let login_config = get_keycloak_login_config();
    let client = reqwest::Client::new();
    let admin_token = KeycloakAdminToken::acquire(
        &login_config.url,
        &login_config.client_id,
        &login_config.client_secret,
        &client,
    )
    .await?;
    event!(Level::INFO, "Successfully acquired credentials");
    Ok(KeycloakAdmin::new(&login_config.url, admin_token, client))
}
