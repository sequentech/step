// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use reqwest;
use anyhow::Result;
use std::env;
use tracing::instrument;
use serde::{Serialize, Deserialize};
use serde_json::json;

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
pub async fn client_credentials_login() -> Result<TokenResponse> {
    let keycloak_endpoint =
        env::var("KEYCLOAK_ENDPOINT").expect(&format!("KEYCLOAK_ENDPOINT must be set"));
    let client_id =
        env::var("KEYCLOAK_CLIENT_ID").expect(&format!("KEYCLOAK_CLIENT_ID must be set"));
    let client_secret =
        env::var("KEYCLOAK_CLIENT_SECRET").expect(&format!("KEYCLOAK_CLIENT_SECRET must be set"));
    let request_body = json!({
        "client_id": client_id,
        "scope": "openid",
        "client_secret": client_secret,
        "grant_type": "client_credentials"
    });

    let client = reqwest::Client::new();
    let res = client
        .post(keycloak_endpoint)
        //.header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: TokenResponse =res.json().await?;
    Ok(response_body)
}
