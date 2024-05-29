// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::vault::{
    aws_secret_manager::AwsSecretManager, hashicorp_vault::HashiCorpVault,
};
use anyhow::Result;
use async_trait::async_trait;
use std::str::FromStr;
use strum_macros::EnumString;
use tracing::{info, instrument};

#[derive(EnumString)]
enum VaultManagerType {
    HashiCorpVault,
    AwsSecretManager,
}

#[async_trait]
pub trait Vault: Send {
    async fn save_secret(&self, key: String, value: String) -> Result<()>;
    async fn read_secret(&self, key: String) -> Result<Option<String>>;
}

fn get_vault() -> Result<Box<dyn Vault + Send>> {
    let vault_name = std::env::var("SECRETS_BACKEND").unwrap_or("HashiCorpVault".to_string());

    info!("Vault: vault_name={vault_name}");

    let vault = VaultManagerType::from_str(&vault_name)?;

    Ok(match vault {
        VaultManagerType::HashiCorpVault => Box::new(HashiCorpVault {}),
        VaultManagerType::AwsSecretManager => Box::new(AwsSecretManager {}),
    })
}

#[instrument(skip(value), err)]
pub async fn save_secret(key: String, value: String) -> Result<()> {
    let vault = get_vault()?;

    vault.save_secret(key, value).await
}

#[instrument(err)]
pub async fn read_secret(key: String) -> Result<Option<String>> {
    let vault = get_vault()?;

    vault.read_secret(key).await
}
