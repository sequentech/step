use super::{aws_secret_manager, hashicorp_vault};
use anyhow::Result;
use std::str::FromStr;
use strum_macros::EnumString;
use tracing::{info, instrument};

#[derive(EnumString)]
enum VaultManager {
    HashiCorpVault,
    AWSSecretManager,
}

async fn get_config() -> Result<VaultManager> {
    let vault_name =
        std::env::var("VAULT_NAME".to_string()).unwrap_or("HashiCorpVault".to_string());

    info!("Vault: vault_name={vault_name}");

    let vault = VaultManager::from_str(&vault_name)?;

    Ok(vault)
}

#[instrument(skip(value), err)]
pub async fn save_secret(key: String, value: String) -> Result<()> {
    let vault = get_config().await?;

    match vault {
        VaultManager::HashiCorpVault => hashicorp_vault::save_secret(key, value).await,
        VaultManager::AWSSecretManager => aws_secret_manager::save_secret(key, value).await,
    }
}

#[instrument(err)]
pub async fn read_secret(key: String) -> Result<Option<String>> {
    let vault = get_config().await?;

    match vault {
        VaultManager::HashiCorpVault => hashicorp_vault::read_secret(key).await,
        VaultManager::AWSSecretManager => aws_secret_manager::read_secret(key).await,
    }
}
