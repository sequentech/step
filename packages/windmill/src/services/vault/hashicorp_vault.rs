// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::Vault;
use anyhow::{Context, Result};
use async_trait::async_trait;
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

#[derive(Debug)]
pub struct HashiCorpVault;

#[async_trait]
impl Vault for HashiCorpVault {
    #[instrument(skip(value), err)]
    async fn save_secret(&self, key: String, value: String) -> Result<()> {
        let server_url = env::var("VAULT_SERVER_URL").context("VAULT_SERVER_URL must be set")?;
        let token = env::var("VAULT_TOKEN").context("VAULT_TOKEN must be set")?;
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

    #[instrument(err)]
    async fn read_secret(&self, key: String) -> Result<Option<String>> {
        let server_url = env::var("VAULT_SERVER_URL").context("VAULT_SERVER_URL must be set")?;
        let token = env::var("VAULT_TOKEN").context("VAULT_TOKEN must be set")?;
        let client = reqwest::Client::new();
        let pm_endpoint = format!("{}/v1/secrets/{}", &server_url, &key);
        let response = client.get(pm_endpoint).bearer_auth(token).send().await?;
        let unwrapped = if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        } else {
            response
        }
        .error_for_status()?;
        let read: VaultRead = unwrapped.json().await?;
        Ok(Some(read.data.data))
    }
}
