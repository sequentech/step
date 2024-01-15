use crate::services::vault::{
    aws_secret_manager::AwsSecretManager, hashicorp_vault::HashiCorpVault,
};

use anyhow::Result;
use std::str::FromStr;
use strum_macros::EnumString;
use tracing::{info, instrument};

#[derive(EnumString)]
enum VaultManagerType {
    HashiCorpVault,
    AwsSecretManager,
}

pub trait Vault {
    async fn save_secret(key: String, value: String) -> Result<()>;
    async fn read_secret(key: String) -> Result<Option<String>>;
}

fn get_vault() -> Box<dyn Vault> {
    let vault_name = std::env::var("VAULT_MANAGER").unwrap_or("HashiCorpVault".to_string());

    info!("Vault: vault_name={vault_name}");

    let vault = VaultManagerType::from_str(&vault_name)?;

    match vault {
        VaultManagerType::HashiCorpVault => HashiCorpVault::new(),
        VaultManagerType::AwsSecretManager => AwsSecretManager::new(),
    }
}

#[instrument(skip(value), err)]
pub async fn save_secret(key: String, value: String) -> Result<()> {
    let vault = get_vault();

    vault.save_secret(key, value).await
}

#[instrument(err)]
pub async fn read_secret(key: String) -> Result<Option<String>> {
    let vault = get_vault();

    vault.read_secret(key).await
}
