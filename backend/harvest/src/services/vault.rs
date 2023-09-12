// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use reqwest;
use rocket::serde::Deserialize;
use serde_json::json;
use std::env;

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct VaultStatus {
    r#type: String,
    initialized: bool,
    sealed: bool,
    t: usize,
    n: usize,
    progress: usize,
    nonce: String,
    version: String,
    build_date: String,
    migration: bool,
    recovery_seal: bool,
    storage_type: String,
}

async fn is_sealed() -> Result<bool> {
    let server_url = env::var("VAULT_SERVER_URL")
        .expect(&format!("VAULT_SERVER_URL must be set"));

    let client = reqwest::Client::new();
    let res = client
        .get(format!("{}/sys/seal-status", server_url))
        .send()
        .await?;
    res.error_for_status_ref()?;
    let status = res.json::<VaultStatus>().await?;
    Ok(status.sealed)
}

async fn unseal() -> Result<()> {
    let server_url = env::var("VAULT_SERVER_URL")
        .expect(&format!("VAULT_SERVER_URL must be set"));
    let token =
        env::var("VAULT_TOKEN").expect(&format!("VAULT_TOKEN must be set"));
    let unseal_key = env::var("VAULT_UNSEAL_KEY")
        .expect(&format!("VAULT_TOKEN must be set"));
    let client = reqwest::Client::new();
    let pm_endpoint = format!("{}/sys/unseal", &server_url);
    let json_value = json!({"key": unseal_key});
    client
        .post(pm_endpoint)
        .bearer_auth(token)
        .json(&json_value)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

async fn assert_unsealed() -> Result<()> {
    if is_sealed().await? {
        unseal().await?;
    }
    Ok(())
}

pub async fn save_secret(key: String, value: String) -> Result<()> {
    // unseal vault if required
    assert_unsealed().await?;
    let server_url = env::var("VAULT_SERVER_URL")
        .expect(&format!("VAULT_SERVER_URL must be set"));
    let token =
        env::var("VAULT_TOKEN").expect(&format!("VAULT_TOKEN must be set"));
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
