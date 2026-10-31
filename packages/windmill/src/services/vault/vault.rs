// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::secret::{
    delete_secret_by_key, get_secret_by_id, get_secret_by_key, get_secrets_by_key_prefix,
    insert_secret, update_secret_value, Secret,
};
use crate::services::admin_public_key::NewAdminPublicKey;
use crate::services::database::get_hasura_pool;
use crate::services::vault::{
    aws_secret_manager::AwsSecretManager, env_var_master_secret::EnvVarMasterSecret,
    hashicorp_vault::HashiCorpVault,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use cryptography::utils::serialization::{Deserializable, Serializable};
use cryptography::utils::symm::{
    decrypt, encrypt, gen_key, sk_from_bytes, EncryptionData, SymmetricKey,
};
use deadpool_postgres::Transaction;
use std::str::FromStr;
use strum_macros::{Display, EnumString};
use tokio;
use tokio::sync::OnceCell;
use tracing::{info, instrument};
use voting_crypto::signing::{SigningKey, VerificationKey};

const MASTER_SECRET_KEY_NAME: &str = "master_secret";

const LOWER_AWS_SECRETS_MANAGER: &str = "awssecretsmanager";

#[derive(EnumString, Display, Debug)]
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
            let bytes = hex::decode(secret).context("Failed to decode master secret")?;
            sk_from_bytes(&bytes).map_err(|err| anyhow!("Invalid master secret: {err}"))
        }
        Ok(None) => {
            let new_key =
                gen_key().map_err(|err| anyhow!("Failed to generate master secret: {err}"))?;
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
    MASTER_SECRET
        .get_or_try_init(initialize_master_secret)
        .await
        .cloned()
}

#[async_trait]
pub trait Vault: Send {
    async fn save_secret(&self, key: String, value: String) -> Result<()>;
    async fn read_secret(&self, key: String) -> Result<Option<String>>;
    fn vault_type(&self) -> VaultManagerType;
}

#[instrument(err)]
pub fn get_vault() -> Result<Box<dyn Vault + Send>> {
    let mut vault_name = std::env::var("SECRETS_BACKEND")
        .unwrap_or(VaultManagerType::EnvVarMasterSecret.to_string());

    if LOWER_AWS_SECRETS_MANAGER.to_string() == vault_name.to_lowercase() {
        vault_name = VaultManagerType::AwsSecretManager.to_string();
    }

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
    save_secret_and_return(hasura_transaction, tenant_id, election_event_id, key, value)
        .await
        .map(|_| ())
}

#[instrument(skip(hasura_transaction, value), err)]
pub async fn save_secret_and_return(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: Option<&str>,
    key: &str,
    value: &str,
) -> Result<Secret> {
    if get_secret_by_key(hasura_transaction, tenant_id, election_event_id, key)
        .await?
        .is_some()
    {
        return Err(anyhow!("Unexpected: key already exists"));
    }
    let master_secret = get_master_secret().await?;
    let encrypted_data = encrypt(master_secret, value.as_bytes())
        .map_err(|err| anyhow!("Error encrypting secret: {err}"))?;
    let encrypted_bytes = encrypted_data.ser();

    insert_secret(
        hasura_transaction,
        tenant_id,
        election_event_id,
        key,
        &encrypted_bytes,
    )
    .await
    .context("Error saving secret")
}

/// Encrypts short-lived data, such as a held Messenger code, with the
/// master key.
pub async fn encrypt_with_master_secret(plaintext: &[u8]) -> Result<Vec<u8>> {
    let master_secret = get_master_secret().await?;
    Ok(encrypt(master_secret, plaintext)
        .context("Error encrypting data")?
        .ser())
}

pub async fn decrypt_with_master_secret(ciphertext: &[u8]) -> Result<Vec<u8>> {
    let encrypted_data = EncryptionData::deser(ciphertext).context("Error deserializing data")?;
    let master_secret = get_master_secret().await?;
    decrypt(&master_secret, &encrypted_data).context("Error decrypting data")
}

/// Stores `value` under `key`, replacing the previous value if there is one.
/// Used for credentials that rotate.
#[instrument(skip(hasura_transaction, value), err)]
pub async fn replace_secret(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    key: &str,
    value: &str,
) -> Result<()> {
    let master_secret = get_master_secret().await?;
    let encrypted_bytes = encrypt(master_secret, value.as_bytes())
        .context("Error encrypting secret")?
        .ser();
    if !update_secret_value(hasura_transaction, tenant_id, key, &encrypted_bytes).await? {
        insert_secret(hasura_transaction, tenant_id, None, key, &encrypted_bytes)
            .await
            .context("Error saving secret")?;
    }
    Ok(())
}

#[instrument(skip(hasura_transaction), err)]
pub async fn delete_secret(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    key: &str,
) -> Result<bool> {
    delete_secret_by_key(hasura_transaction, tenant_id, key).await
}

async fn decrypt_stored_secret(secret: &Secret) -> Result<String> {
    let encrypted_data = voting_crypto::envelope::read_encrypted_value(&secret.value)
        .context("Error deserializing encrypted data")?;
    let master_secret = get_master_secret().await?;
    let decrypted_bytes = decrypt(&master_secret, &encrypted_data)
        .map_err(|err| anyhow!("Error decrypting secret: {err}"))?;
    String::from_utf8(decrypted_bytes).context("Error converting decrypted bytes to string")
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

    decrypt_stored_secret(&secret).await.map(Some)
}

#[instrument(skip(hasura_transaction), err)]
pub async fn read_secret_by_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: Option<&str>,
    secret_id: &str,
) -> Result<Option<(Secret, String)>> {
    let Some(secret) =
        get_secret_by_id(hasura_transaction, tenant_id, election_event_id, secret_id).await?
    else {
        return Ok(None);
    };

    let value = decrypt_stored_secret(&secret).await?;
    Ok(Some((secret, value)))
}

/// The admin user's signing key, made and saved in `hasura_transaction` the
/// first time. Its public key is posted to the tenant's log once that
/// transaction commits, by a task of this process: see
/// [`get_admin_user_signing_key_to_announce`].
#[instrument(err)]
pub async fn get_admin_user_signing_key(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    user_id: &str,
    username: Option<String>,
    elections_ids: Option<String>,
    user_area_id: Option<String>,
) -> Result<SigningKey> {
    let (sk, new_key) = get_admin_user_signing_key_to_announce(
        hasura_transaction,
        tenant_id,
        user_id,
        username,
        elections_ids,
        user_area_id,
    )
    .await?;
    if let Some(new_key) = new_key {
        let pool = get_hasura_pool().await;
        tokio::spawn(async move { new_key.post_after_commit(&pool).await });
    }
    Ok(sk)
}

/// The admin user's signing key and, when it was made now and saved in
/// `hasura_transaction`, its public key to post to the tenant's log after
/// that transaction commits. Posted before, a transaction that then rolled
/// back would leave the log announcing a key nobody has.
#[instrument(err)]
pub async fn get_admin_user_signing_key_to_announce(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    user_id: &str,
    username: Option<String>,
    elections_ids: Option<String>,
    user_area_id: Option<String>,
) -> Result<(SigningKey, Option<NewAdminPublicKey>)> {
    let lookup_key = admin_vault_lookup_key(&tenant_id, &user_id);
    if let Some(sk_der_b64) = read_secret(hasura_transaction, tenant_id, None, &lookup_key).await? {
        return Ok((SigningKey::from_b64_string(&sk_der_b64)?, None));
    }
    info!(
        "Vault: generating private signing key for admin user {}",
        lookup_key.clone()
    );
    let sk = SigningKey::generate()?;
    save_secret(
        hasura_transaction,
        tenant_id,
        None,
        &lookup_key,
        &sk.to_b64_string()?,
    )
    .await?;
    let transaction_id: i64 = hasura_transaction
        .query_one("SELECT txid_current()", &[])
        .await
        .context("Error reading the transaction that saves the admin key")?
        .try_get(0)?;
    let new_key = NewAdminPublicKey {
        tenant_id: tenant_id.to_string(),
        user_id: user_id.to_string(),
        username,
        public_key: VerificationKey::from_sk(&sk)?.to_b64_string()?,
        elections_ids,
        user_area_id,
        transaction_id,
    };
    Ok((sk, Some(new_key)))
}

/// The admin user's signing key, if they have one.
#[instrument(skip(hasura_transaction), err)]
pub async fn read_admin_user_signing_key(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    user_id: &str,
) -> Result<Option<SigningKey>> {
    let lookup_key = admin_vault_lookup_key(tenant_id, user_id);
    read_secret(hasura_transaction, tenant_id, None, &lookup_key)
        .await?
        .map(|sk_der_b64| SigningKey::from_b64_string(&sk_der_b64))
        .transpose()
}

/// The admin signing keys of a tenant, by the user they belong to.
#[instrument(skip(hasura_transaction), err)]
pub async fn list_admin_user_signing_keys(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
) -> Result<Vec<(String, SigningKey)>> {
    let prefix = admin_vault_lookup_key(tenant_id, "");
    let mut keys = vec![];
    for secret in get_secrets_by_key_prefix(hasura_transaction, tenant_id, &prefix).await? {
        let user_id = secret.key[prefix.len()..].to_string();
        let key = SigningKey::from_b64_string(&decrypt_stored_secret(&secret).await?)?;
        keys.push((user_id, key));
    }
    Ok(keys)
}

/// Whether the admin user has a signing key, without reading it.
#[instrument(skip(hasura_transaction), err)]
pub async fn admin_user_signing_key_exists(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    user_id: &str,
) -> Result<bool> {
    let lookup_key = admin_vault_lookup_key(tenant_id, user_id);
    Ok(
        get_secret_by_key(hasura_transaction, tenant_id, None, &lookup_key)
            .await?
            .is_some(),
    )
}

fn voter_vault_lookup_key(tenant_id: &str, event_id: &str, user_id: &str) -> String {
    format!("voter_signing_key-{}-{}-{}", tenant_id, event_id, user_id)
}

fn admin_vault_lookup_key(tenant_id: &str, user_id: &str) -> String {
    format!("admin_signing_key-{}-{}", tenant_id, user_id)
}
