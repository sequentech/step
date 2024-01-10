use anyhow::{anyhow, Context, Result};
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use tracing::{info, instrument};

enum VaultManager {
    HashiCorpVault,
    AWSSecretManager,
}

async fn get_config() -> Result<()> {
    let vault_name =
        std::env::var("VAULT_NAME").map_err(|err| anyhow!("VAULT_NAME env var missing"))?;

    info!("Vault: vault_name={vault_name}");

    Ok(())
}

#[instrument(skip(value), err)]
pub async fn save_secret(key: String, value: String) -> Result<()> {
    Ok(())
}

#[instrument(err)]
pub async fn read_secret(key: String) -> Result<Option<String>> {
    Ok(Some("my secret".to_owned()))
}
