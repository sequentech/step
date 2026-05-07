// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! HashiCorp Vault backend.
//!
//! This implementation reads/writes raw string secrets to the Vault HTTP API
//! using `VAULT_SERVER_URL` and `VAULT_TOKEN`.

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
/// Secret payload as stored under the configured Vault path.
#[allow(missing_docs_in_private_items)]
struct VaultSecret {
    data: Option<String>,
    value: Option<String>,
}

#[derive(Serialize, Deserialize)]
/// Response wrapper returned by the Vault HTTP API on secret reads.
struct VaultRead {
    /// Authentication metadata returned by Vault.
    auth: Option<String>,
    /// Secret data envelope.
    data: VaultSecret,
    /// Lease duration in seconds.
    lease_duration: i64,
    /// Lease identifier.
    lease_id: String,
    /// Whether the lease can be renewed.
    renewable: bool,
}

#[derive(Debug)]
/// HashiCorp Vault secret backend.
pub struct HashiCorpVault;

#[async_trait]
impl Vault for HashiCorpVault {
    #[instrument(skip(value), err)]
    /// Stores a secret value at `secrets/<key>` via the Vault HTTP API.
    ///
    /// # Errors
    ///
    /// Returns an error if required environment variables are missing, if the
    /// request fails, or if Vault returns a non-success status.
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
    /// Reads a secret value from `secrets/<key>` via the Vault HTTP API.
    ///
    /// # Errors
    ///
    /// Returns an error if required environment variables are missing, if the
    /// request fails, or if the response cannot be parsed.
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
        let value = read.data.data.or(read.data.value);
        Ok(value)
    }

    #[instrument]
    /// Identifies this backend as HashiCorp Vault.
    fn vault_type(&self) -> VaultManagerType {
        VaultManagerType::HashiCorpVault
    }
}
