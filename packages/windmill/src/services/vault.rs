// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use tracing::instrument;

#[derive(Serialize, Deserialize)]
struct VaultSecret {
    data: String,
}

#[derive(Serialize, Deserialize)]
struct VaultRead {
    auth: Option<String>,
    data: VaultSecret,
    lease_duration: i64,
    lease_id: String,
    renewable: bool,
}

#[instrument(skip(value))]
pub async fn save_secret(key: String, value: String) -> Result<()> {
    let server_url = env::var("VAULT_SERVER_URL").expect(&format!("VAULT_SERVER_URL must be set"));
    let token = env::var("VAULT_TOKEN").expect(&format!("VAULT_TOKEN must be set"));
    let client = reqwest::Client::new();
    let pm_endpoint = format!("{}/v1/secrets/{}", &server_url, &key);
    let json_value = json!({"data": value});
    client
        .post(pm_endpoint)
        .bearer_auth(token)
        .json(&json_value)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

#[instrument]
pub async fn read_secret(key: String) -> Result<String> {
    let server_url = env::var("VAULT_SERVER_URL").expect(&format!("VAULT_SERVER_URL must be set"));
    let token = env::var("VAULT_TOKEN").expect(&format!("VAULT_TOKEN must be set"));
    let client = reqwest::Client::new();
    let pm_endpoint = format!("{}/v1/secret/{}", &server_url, &key);
    let response = client
        .get(pm_endpoint)
        .bearer_auth(token)
        .send()
        .await?
        .error_for_status()?;
    let read: VaultRead = response.json().await?;
    Ok(read.data.data)
}
