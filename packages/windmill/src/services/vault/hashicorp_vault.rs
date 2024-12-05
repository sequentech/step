// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::{Vault, VaultManagerType};
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest;
use sequent_core::serialization::deserialize_with_path::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use tracing::{info, instrument};

#[derive(Serialize, Deserialize)]
struct VaultSecret {
    data: Option<String>,
    value: Option<String>,
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
        let json_value = serde_json::to_value(VaultSecret {
            data: Some(value),
            value: None,
        })?;
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
        info!("info: {:?}", unwrapped);
        let text = unwrapped.text().await?;
        info!("text: {}", text);
        let read: VaultRead = deserialize_str(&text)?;
        let value = if let Some(v) = read.data.data {
            Some(v)
        } else if let Some(v) = read.data.value {
            Some(v)
        } else {
            None
        };
        Ok(value)
    }

    #[instrument]
    fn vault_type(&self) -> VaultManagerType {
        VaultManagerType::HashiCorpVault
    }
}
