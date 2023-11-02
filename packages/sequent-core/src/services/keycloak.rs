// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::connection;
use anyhow::{anyhow, Result};
use reqwest;
use serde::{Deserialize, Serialize};
use serde_urlencoded;
use std::env;
use tracing::instrument;

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

// Client Credentials OpenID Authentication flow.
// This enables servers to authenticate, without using a browser.
#[instrument]
pub async fn get_client_credentials() -> Result<connection::AuthHeaders> {
    let keycloak_endpoint = env::var("KEYCLOAK_ENDPOINT")
        .expect(&format!("KEYCLOAK_ENDPOINT must be set"));
    let client_id = env::var("KEYCLOAK_CLIENT_ID")
        .expect(&format!("KEYCLOAK_CLIENT_ID must be set"));
    let client_secret = env::var("KEYCLOAK_CLIENT_SECRET")
        .expect(&format!("KEYCLOAK_CLIENT_SECRET must be set"));
    let body_string = serde_urlencoded::to_string::<[(String, String); 4]>([
        ("client_id".into(), client_id),
        ("scope".into(), "openid".into()),
        ("client_secret".into(), client_secret),
        ("grant_type".into(), "client_credentials".into()),
    ])
    .unwrap();

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
    Ok(connection::AuthHeaders {
        key: "authorization".into(),
        value: format!("Bearer {}", credentials.access_token),
    })
}
