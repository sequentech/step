// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::database::get_hasura_pool;
use crate::services::electoral_log::ElectoralLog;
use crate::services::vault::{
    aws_secret_manager::AwsSecretManager, hashicorp_vault::HashiCorpVault,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use std::str::FromStr;
use strand::signature::{StrandSignaturePk, StrandSignatureSk};
use strum_macros::EnumString;
use tracing::{info, instrument};
use uuid::Uuid;

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
pub async fn save_secret(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: Option<&str>,
    key: &str,
    value: &str,
) -> Result<()> {
    let tenant_uuid = Uuid::parse_str(tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let election_event_uuid = match election_event_id {
        Some(id) => Some(
            Uuid::parse_str(id)
                .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?,
        ),
        None => None,
    };

    // Check if the secret already exists
    if read_secret(&hasura_transaction, tenant_id, election_event_id, key)
        .await?
        .is_some()
    {
        return Err(anyhow!("Unexpected: key already exists"));
    }

    hasura_transaction
        .execute(
            "INSERT INTO sequent_backend.secret (tenant_id, key, value, election_event_id) 
             VALUES ($1, $2, $3, $4)",
            &[&tenant_uuid, &key, &value, &election_event_uuid],
        )
        .await
        .context("Error saving secret")?;

    Ok(())
}

#[instrument(skip(hasura_transaction), err)]
pub async fn read_secret(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: Option<&str>,
    key: &str,
) -> Result<Option<String>> {
    let tenant_uuid = Uuid::parse_str(tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;

    let election_event_uuid = match election_event_id {
        Some(id) => Some(
            Uuid::parse_str(id)
                .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?,
        ),
        None => None,
    };

    let result: Option<String> = hasura_transaction
        .query_opt(
            "SELECT value FROM sequent_backend.secret 
             WHERE tenant_id = $1 
               AND key = $2 
               AND (election_event_id = $3 OR $3 IS NULL) 
             LIMIT 1",
            &[&tenant_uuid, &key, &election_event_uuid],
        )
        .await
        .context("Error reading secret")?
        .map(|row| row.get(0));

    // TODO Decrypt the value before returning

    Ok(result)
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
    hasura_transaction: &Transaction<'_>,
    elog_database: &str,
    tenant_id: &str,
    user_id: &str,
) -> Result<StrandSignatureSk> {
    let lookup_key = admin_vault_lookup_key(&tenant_id, &user_id);
    let sk_der_b64 = read_secret(hasura_transaction, tenant_id, None, &lookup_key).await?;

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
        save_secret(hasura_transaction, tenant_id, None, &lookup_key, &sk_string).await?;
        ElectoralLog::post_admin_pk(hasura_transaction, elog_database, tenant_id, user_id, &pk)
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
