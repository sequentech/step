// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::secret::{get_secret_by_key, insert_secret};
use crate::services::electoral_log::ElectoralLog;
use crate::services::vault::{
    aws_secret_manager::AwsSecretManager, env_var_master_secret::EnvVarMasterSecret,
    hashicorp_vault::HashiCorpVault,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use std::str::FromStr;
use strand::serialization::{StrandDeserialize, StrandSerialize};
use strand::signature::{StrandSignaturePk, StrandSignatureSk};
use strand::symm::{decrypt, encrypt, gen_key, EncryptionData, SymmetricKey};
use strum_macros::EnumString;
use tokio;
use tokio::sync::OnceCell;
use tracing::{info, instrument};

const MASTER_SECRET_KEY_NAME: &str = "master_secret";

#[derive(EnumString)]
pub enum VaultManagerType {
    HashiCorpVault,
    AwsSecretManager,
    EnvVarMasterSecret,
}

static MASTER_SECRET: OnceCell<SymmetricKey> = OnceCell::const_new();

#[instrument]
pub async fn check_master_secret() -> Result<()> {
    let vault = get_vault()?;

    vault
        .read_secret(MASTER_SECRET_KEY_NAME.to_string())
        .await?;

    Ok(())
}

#[instrument]
async fn initialize_master_secret() -> Result<SymmetricKey> {
    let vault = get_vault().with_context(|| "Failed to initialize vault")?;

    match vault.read_secret(MASTER_SECRET_KEY_NAME.to_string()).await {
        Ok(Some(secret)) => {
            let bytes = hex::decode(secret).expect("Failed to decode master secret");
            Ok(SymmetricKey::from_slice(&bytes).to_owned())
        }
        Ok(None) => {
            let new_key = gen_key();
            let hex_key = hex::encode(new_key.as_slice());
            vault
                .save_secret(MASTER_SECRET_KEY_NAME.to_string(), hex_key.clone())
                .await
                .with_context(|| "Failed to save master secret")?;
            Ok(new_key)
        }
        Err(e) => Err(e).with_context(|| "Failed to access vault for master secret"),
    }
}
#[instrument]
pub async fn get_master_secret() -> Result<SymmetricKey> {
    if let Some(secret) = MASTER_SECRET.get() {
        return Ok(secret.clone());
    }
    initialize_master_secret().await
}

#[async_trait]
pub trait Vault: Send {
    async fn save_secret(&self, key: String, value: String) -> Result<()>;
    async fn read_secret(&self, key: String) -> Result<Option<String>>;
    fn vault_type(&self) -> VaultManagerType;
}

#[instrument(err)]
pub fn get_vault() -> Result<Box<dyn Vault + Send>> {
    let vault_name = std::env::var("SECRETS_BACKEND").unwrap_or("EnvVarMasterSecret".to_string());

    info!("Vault: vault_name={vault_name}");

    let vault = VaultManagerType::from_str(&vault_name)?;

    Ok(match vault {
        VaultManagerType::HashiCorpVault => Box::new(HashiCorpVault {}),
        VaultManagerType::AwsSecretManager => Box::new(AwsSecretManager {}),
        VaultManagerType::EnvVarMasterSecret => Box::new(EnvVarMasterSecret {}),
    })
}

#[instrument(skip(hasura_transaction, value), err)]
pub async fn save_secret(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: Option<&str>,
    key: &str,
    value: &str,
) -> Result<()> {
    if get_secret_by_key(hasura_transaction, tenant_id, election_event_id, key)
        .await?
        .is_some()
    {
        return Err(anyhow!("Unexpected: key already exists"));
    }
    let master_secret = get_master_secret().await?;
    let encrypted_data =
        encrypt(master_secret, value.as_bytes()).context("Error encrypting secret")?;
    let encrypted_bytes = encrypted_data
        .strand_serialize()
        .context("Error serializing encrypted data")?;

    insert_secret(
        hasura_transaction,
        tenant_id,
        election_event_id,
        key,
        &encrypted_bytes,
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
    let Some(secret) =
        get_secret_by_key(hasura_transaction, tenant_id, election_event_id, key).await?
    else {
        return Ok(None);
    };

    let encrypted_data = EncryptionData::strand_deserialize(&secret.value)
        .context("Error deserializing encrypted data")?;
    let master_secret = get_master_secret().await?;
    let decrypted_bytes =
        decrypt(&master_secret, &encrypted_data).context("Error decrypting secret")?;
    let decrypted_str =
        String::from_utf8(decrypted_bytes).context("Error converting decrypted bytes to string")?;
    Ok(Some(decrypted_str))
}

#[instrument(err)]
pub async fn get_admin_user_signing_key(
    hasura_transaction: &Transaction<'_>,
    elog_database: &str,
    tenant_id: &str,
    user_id: &str,
    username: Option<String>,
    elections_ids: Option<String>,
    user_area_id: Option<String>,
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
        ElectoralLog::post_admin_pk(
            hasura_transaction,
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

fn voter_vault_lookup_key(tenant_id: &str, event_id: &str, user_id: &str) -> String {
    format!("voter_signing_key-{}-{}-{}", tenant_id, event_id, user_id)
}

fn admin_vault_lookup_key(tenant_id: &str, user_id: &str) -> String {
    format!("admin_signing_key-{}-{}", tenant_id, user_id)
}
