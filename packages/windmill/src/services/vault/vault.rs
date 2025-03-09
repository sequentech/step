// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::electoral_log::ElectoralLog;
use crate::services::vault::{
    aws_secret_manager::AwsSecretManager, hashicorp_vault::HashiCorpVault,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use std::str::FromStr;
use strand::signature::{StrandSignaturePk, StrandSignatureSk};
use strum_macros::EnumString;
use tracing::{info, instrument};

#[derive(EnumString)]
pub enum VaultManagerType {
    HashiCorpVault,
    AwsSecretManager,
}

#[async_trait]
pub trait Vault: Send {
    async fn save_secret(&self, key: String, value: String) -> Result<()>;
    async fn read_secret(&self, key: String) -> Result<Option<String>>;
    fn vault_type(&self) -> VaultManagerType;
}

pub fn get_vault() -> Result<Box<dyn Vault + Send>> {
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

    if let Some(value) = vault
        .read_secret(key.clone())
        .await
        .context("Error reading keys")?
    {
        return Err(anyhow!("Unexpected: key already exists"));
    }

    vault.save_secret(key, value).await
}

#[instrument(err)]
pub async fn read_secret(key: String) -> Result<Option<String>> {
    let vault = get_vault()?;

    vault.read_secret(key).await
}

/// Returns the private signing key for the given admin user.
///
/// The private key is obtained from the vault.
/// If no such key exists, it is generated and a log post
/// is published with the corresponding public key
/// (with StatementBody::AdminPublicKey).
///
/// There is a possibility that the private key is saved
/// but the notification fails. This is logged in
/// electorallog::post_admin_pk
#[instrument(err)]
pub async fn get_admin_user_signing_key(
    elog_database: &str,
    tenant_id: &str,
    user_id: &str,
    username: Option<String>,
    elections_ids: Option<String>,
    user_area_id: Option<String>,
) -> Result<StrandSignatureSk> {
    let lookup_key = admin_vault_lookup_key(&tenant_id, &user_id);
    let sk_der_b64 = read_secret(lookup_key.clone()).await?;

    let sk = if let Some(sk_der_b64) = sk_der_b64 {
        StrandSignatureSk::from_der_b64_string(&sk_der_b64)?
    } else {
        info!(
            "Vault: generating private signing key for admin user {}",
            lookup_key.clone()
        );
        let sk = StrandSignatureSk::gen()?;
        let sk_string = sk.to_der_b64_string()?;
        let pk = StrandSignaturePk::from_sk(&sk)?;
        let pk = pk.to_der_b64_string()?;

        // We save the secret right before notifying the public key
        // to minimize the chances that the second call fails while
        // while the first one succeeds. If this happens the
        // secret will exist but the pk notification will not.
        save_secret(lookup_key.clone(), sk_string).await?;
        ElectoralLog::post_admin_pk(
            elog_database,
            tenant_id,
            user_id,
            username,
            &pk,
            elections_ids,
            user_area_id,
        )
        .await?;

        sk
    };

    Ok(sk)
}

/// Returns the vault lookup key for a voters private signing key
fn voter_vault_lookup_key(tenant_id: &str, event_id: &str, user_id: &str) -> String {
    format!("voter_signing_key-{}-{}-{}", tenant_id, event_id, user_id)
}

/// Returns the vault lookup key for an admin user's private signing key
fn admin_vault_lookup_key(tenant_id: &str, user_id: &str) -> String {
    format!("admin_signing_key-{}-{}", tenant_id, user_id)
}
