// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Audit board names, clients and application signing identities.
//! Cryptographic signing is supplied by wbraid's native signature scheme.

use crate::services::electoral_log_signers;
use crate::services::vault;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use electoral_log::adapters::events::EventDatabases;
use electoral_log::adapters::postgres::PostgresStore;
use electoral_log::signer::{software, SharedSigner};
use electoral_log::BoardClient;
use sequent_core::ballot::ElectoralLogSignerPolicy;
use std::sync::Arc;
use tokio::sync::OnceCell;
use tracing::instrument;
use voting_crypto::signing::{SigningKey, VerificationKey};

pub fn get_protocol_manager_secret_path(board_name: &str) -> String {
    format!("boards/{board_name}/protocol-manager")
}

#[instrument(skip(hasura_transaction), err)]
pub async fn create_protocol_manager_keys(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    board_name: &str,
) -> Result<()> {
    // A board's key is made once: an existing key (software or external
    // record) is never replaced, since it verifies the board's whole log.
    if board_has_key(hasura_transaction, tenant_id, election_event_id, board_name).await? {
        return Err(anyhow!(
            "this board already has a key: it is never replaced (refused)"
        ));
    }
    // A board of an event whose log an external signer signs gets its key
    // there; its public key and version are recorded, and no software key is
    // made or kept for it.
    if electoral_log_signers::event_policy_for_share(
        hasura_transaction,
        tenant_id,
        election_event_id,
    )
    .await?
        == ElectoralLogSignerPolicy::EXTERNAL
    {
        let key = electoral_log_signers::installed()?.create_board_signer(
            tenant_id,
            election_event_id,
            board_name,
        )?;
        key.verification_key()
            .context("the external signer returned an unreadable public key")?;
        key.check_origin(tenant_id, election_event_id, board_name)
            .context("the external signer returned a key made for another board")?;
        vault::save_secret(
            hasura_transaction,
            tenant_id,
            Some(election_event_id),
            &electoral_log_signers::board_key_path(board_name),
            &serde_json::to_string(&key)?,
        )
        .await?;
        return Ok(());
    }
    // create protocol manager keys
    let protocol_manager = SigningKey::generate()?;
    // save protocol manager keys in vault
    #[derive(serde::Serialize)]
    struct SigningIdentity {
        signing_key: String,
    }
    let protocol_config = toml::to_string(&SigningIdentity {
        signing_key: protocol_manager.to_b64_string()?,
    })?;
    let protocol_key = get_protocol_manager_secret_path(board_name);
    vault::save_secret(
        hasura_transaction,
        tenant_id,
        Some(election_event_id),
        &protocol_key,
        &protocol_config,
    )
    .await?;
    Ok(())
}

/// What a board's key is: a software key from the vault, or an external key
/// as recorded when it was made. Checks that the event's setting hasn't
/// changed since.
pub enum BoardIdentity {
    Software(SigningKey),
    External(electoral_log_signers::BoardKey),
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_board_identity(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: Option<&str>,
    board_name: &str,
) -> Result<BoardIdentity> {
    let software_key = vault::read_secret(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &get_protocol_manager_secret_path(board_name),
    )
    .await?;
    // Tenant logs (no event) are always the platform's own key.
    let Some(event) = election_event_id else {
        let contents = software_key.ok_or_else(|| anyhow!("audit board signing key not found"))?;
        return Ok(BoardIdentity::Software(parse_signing_identity(&contents)?));
    };
    let external: Option<electoral_log_signers::BoardKey> = vault::read_secret(
        hasura_transaction,
        tenant_id,
        Some(event),
        &electoral_log_signers::board_key_path(board_name),
    )
    .await?
    .map(|text| serde_json::from_str(&text))
    .transpose()
    .context("the board's external key record is unreadable")?;
    let policy = electoral_log_signers::event_policy(hasura_transaction, tenant_id, event).await?;
    electoral_log_signers::check_unchanged(policy, external.as_ref(), software_key.is_some())?;
    match external {
        Some(key) => {
            key.check_origin(tenant_id, event, board_name)?;
            Ok(BoardIdentity::External(key))
        }
        None => {
            let contents =
                software_key.ok_or_else(|| anyhow!("audit board signing key not found"))?;
            Ok(BoardIdentity::Software(parse_signing_identity(&contents)?))
        }
    }
}

/// The board's software key. Refused for a board whose key an external
/// signer holds, whatever the vault holds: signing goes through
/// [`get_board_signer`], verification through [`get_board_public_key`].
#[instrument(skip(hasura_transaction), err)]
pub async fn get_board_signing_key(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: Option<&str>,
    board_name: &str,
) -> Result<SigningKey> {
    match get_board_identity(hasura_transaction, tenant_id, election_event_id, board_name).await? {
        BoardIdentity::Software(key) => Ok(key),
        BoardIdentity::External(_) => Err(anyhow!(
            "this board's key is held by an external signer, not in software (refused)"
        )),
    }
}

/// The board's signer: its software key, or the installed external signer
/// for exactly the recorded key (refused where none is installed, or where
/// the signer reports another public key).
#[instrument(skip(hasura_transaction), err)]
pub async fn get_board_signer(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: Option<&str>,
    board_name: &str,
) -> Result<SharedSigner> {
    match get_board_identity(hasura_transaction, tenant_id, election_event_id, board_name).await? {
        BoardIdentity::Software(key) => Ok(software(key)),
        BoardIdentity::External(key) => {
            let event =
                election_event_id.ok_or_else(|| anyhow!("an external board needs its event"))?;
            let signer = electoral_log_signers::installed()?
                .board_signer(tenant_id, event, board_name, &key)?;
            if signer.public_key()? != key.verification_key()? {
                return Err(anyhow!(
                    "the external signer's public key is not the board's recorded key (refused)"
                ));
            }
            Ok(signer)
        }
    }
}

/// The board's public key, for verification: from the software key, or from
/// the recorded external key (no signer needed).
#[instrument(skip(hasura_transaction), err)]
pub async fn get_board_public_key(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: Option<&str>,
    board_name: &str,
) -> Result<VerificationKey> {
    match get_board_identity(hasura_transaction, tenant_id, election_event_id, board_name).await? {
        BoardIdentity::Software(key) => VerificationKey::from_sk(&key),
        BoardIdentity::External(key) => key.verification_key(),
    }
}

/// How an external board's key record travels in an event export, in the
/// column that otherwise holds the software key.
pub const EXTERNAL_KEY_EXPORT_PREFIX: &str = "external-signer:";

/// The board's key for an event export: the software key, or the external
/// key's record (public key and version; the private key never leaves the
/// signer).
pub async fn export_board_key(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    board_name: &str,
) -> Result<String> {
    if let Some(record) = vault::read_secret(
        hasura_transaction,
        tenant_id,
        Some(election_event_id),
        &electoral_log_signers::board_key_path(board_name),
    )
    .await?
    {
        return Ok(format!("{EXTERNAL_KEY_EXPORT_PREFIX}{record}"));
    }
    vault::read_secret(
        hasura_transaction,
        tenant_id,
        Some(election_event_id),
        &get_protocol_manager_secret_path(board_name),
    )
    .await?
    .ok_or_else(|| anyhow!("audit signing identity is missing"))
}

/// Restores a board's key from an event export. A software key goes to the
/// vault, for a software event only; an external key's record is restored
/// only for an external event whose installed signer holds exactly that key
/// (version and public key).
pub async fn import_board_key(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    board_name: &str,
    value: &str,
) -> Result<()> {
    let policy =
        electoral_log_signers::event_policy(hasura_transaction, tenant_id, election_event_id)
            .await?;
    match value.strip_prefix(EXTERNAL_KEY_EXPORT_PREFIX) {
        Some(record) => {
            if policy != ElectoralLogSignerPolicy::EXTERNAL {
                return Err(anyhow!(
                    "the export's board key is held by an external signer, but this event's setting is software (refused)"
                ));
            }
            let key: electoral_log_signers::BoardKey = serde_json::from_str(record)
                .context("the exported external key record is unreadable")?;
            // A key serves the board it was made for: an import under other
            // ids (a copy of the event) can't reuse it.
            key.check_origin(tenant_id, election_event_id, board_name)
                .context("the export's external board key can't serve a copy of the event")?;
            let signer = electoral_log_signers::installed()?.board_signer(
                tenant_id,
                election_event_id,
                board_name,
                &key,
            )?;
            if signer.public_key()? != key.verification_key()? {
                return Err(anyhow!(
                    "the installed external signer doesn't hold the exported board key (refused)"
                ));
            }
            vault::save_secret(
                hasura_transaction,
                tenant_id,
                Some(election_event_id),
                &electoral_log_signers::board_key_path(board_name),
                record,
            )
            .await
        }
        None => {
            if policy == ElectoralLogSignerPolicy::EXTERNAL {
                return Err(anyhow!(
                    "this event's log is signed by an external signer: an export's software board key can't be imported (refused)"
                ));
            }
            vault::save_secret(
                hasura_transaction,
                tenant_id,
                Some(election_event_id),
                &get_protocol_manager_secret_path(board_name),
                value,
            )
            .await
        }
    }
}

/// Whether a board has a key (software or external record).
pub async fn board_has_key(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    board_name: &str,
) -> Result<bool> {
    for path in [
        get_protocol_manager_secret_path(board_name),
        electoral_log_signers::board_key_path(board_name),
    ] {
        if vault::read_secret(
            hasura_transaction,
            tenant_id,
            Some(election_event_id),
            &path,
        )
        .await?
        .is_some()
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Like [`get_board_signer`], but `None` for a software event's board that
/// has no key at all, where the caller may skip it.
pub async fn get_board_signer_if_any(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    board_name: &str,
) -> Result<Option<SharedSigner>> {
    let software = vault::read_secret(
        hasura_transaction,
        tenant_id,
        Some(election_event_id),
        &get_protocol_manager_secret_path(board_name),
    )
    .await?;
    let external = vault::read_secret(
        hasura_transaction,
        tenant_id,
        Some(election_event_id),
        &electoral_log_signers::board_key_path(board_name),
    )
    .await?;
    if software.is_none() && external.is_none() {
        // A keyless board of an external event is a failed creation, never
        // a board to skip.
        if electoral_log_signers::event_policy(hasura_transaction, tenant_id, election_event_id)
            .await?
            == ElectoralLogSignerPolicy::EXTERNAL
        {
            return Err(anyhow!(
                "this external board has no key: its creation failed and must be repeated (refused)"
            ));
        }
        return Ok(None);
    }
    get_board_signer(
        hasura_transaction,
        tenant_id,
        Some(election_event_id),
        board_name,
    )
    .await
    .map(Some)
}

/// Decode the native audit signing identity stored in the vault.
pub fn parse_signing_identity(contents: &str) -> Result<SigningKey> {
    #[derive(serde::Deserialize)]
    struct SigningIdentity {
        signing_key: String,
    }
    let identity: SigningIdentity =
        toml::from_str(contents).context("Invalid audit signing identity")?;
    SigningKey::from_b64_string(&identity.signing_key)
}

static ELECTORAL_LOG_DATABASES: OnceCell<EventDatabases> = OnceCell::const_new();
static ELECTORAL_LOG_CLIENT: OnceCell<BoardClient> = OnceCell::const_new();

/// Keep the process-wide pools' connection tasks alive across database tests.
/// A separate Tokio test runtime can shut down while another test uses its pool.
#[cfg(test)]
pub(crate) fn test_runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("electoral log test runtime")
    })
}

/// The process-wide pools of the election events' electoral-log databases.
pub async fn get_event_databases() -> Result<EventDatabases> {
    let databases = ELECTORAL_LOG_DATABASES
        .get_or_try_init(|| async { EventDatabases::from_env() })
        .await?;
    Ok(databases.clone())
}

/// The store of an election event's electoral-log database.
pub async fn get_event_store(election_event_id: &str) -> Result<PostgresStore> {
    get_event_databases().await?.store(election_event_id).await
}

/// Boards of every election event, each in its event's database.
pub async fn get_board_client() -> Result<BoardClient> {
    let client = ELECTORAL_LOG_CLIENT
        .get_or_try_init(|| async {
            Ok::<_, anyhow::Error>(BoardClient::new(Arc::new(get_event_databases().await?)))
        })
        .await?;
    Ok(client.clone())
}

pub fn get_event_board(tenant_id: &str, election_event_id: &str, slug: &str) -> String {
    let tenant: String = tenant_id
        .to_string()
        .chars()
        .filter(|&c| c != '-')
        .take(17)
        .collect();
    format!("{}tenant{}event{}", slug, tenant, election_event_id)
        .chars()
        .filter(|&c| c != '-')
        .collect()
}

/// The board of a tenant's log, which records what happens outside election
/// events: named like an event's board without the event, with the whole tenant
/// ID, which routes it to the tenant's log database.
pub fn get_tenant_board(tenant_id: &str, slug: &str) -> String {
    format!("{slug}tenant{tenant_id}")
        .chars()
        .filter(|&c| c != '-')
        .collect()
}

/// The board of the tenant's log in this environment.
pub fn tenant_board(tenant_id: &str) -> Result<String> {
    let slug = std::env::var("ENV_SLUG").context("missing env var ENV_SLUG")?;
    Ok(get_tenant_board(tenant_id, &slug))
}

/// A tenant's log: its board and the system key that signs its entries.
pub struct TenantLog {
    pub board: String,
    pub system_key: SigningKey,
}

/// The tenant's log, created with its database, board and system key the
/// first time. The key is saved in `hasura_transaction`: a caller that rolls
/// back makes the next call generate it again, for a log that holds no entry
/// signed with the lost one.
#[instrument(skip(hasura_transaction), err)]
pub async fn ensure_tenant_log(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
) -> Result<TenantLog> {
    let board = tenant_board(tenant_id)?;
    let path = get_protocol_manager_secret_path(&board);
    // Callers creating the same log take turns, so one key is saved.
    hasura_transaction
        .execute(
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
            &[&format!("tenant-log:{tenant_id}")],
        )
        .await
        .context("Error taking the tenant log lock")?;
    if let Some(contents) = vault::read_secret(hasura_transaction, tenant_id, None, &path).await? {
        return Ok(TenantLog {
            board,
            system_key: parse_signing_identity(&contents)?,
        });
    }
    get_event_databases()
        .await?
        .create_tenant(tenant_id)
        .await?;
    get_board_client().await?.create_board(&board).await?;
    let system_key = SigningKey::generate()?;
    #[derive(serde::Serialize)]
    struct SigningIdentity {
        signing_key: String,
    }
    let identity = toml::to_string(&SigningIdentity {
        signing_key: system_key.to_b64_string()?,
    })?;
    vault::save_secret(hasura_transaction, tenant_id, None, &path, &identity).await?;
    Ok(TenantLog { board, system_key })
}

pub fn get_election_board(tenant_id: &str, election_id: &str, slug: &str) -> String {
    let tenant: String = tenant_id
        .to_string()
        .chars()
        .filter(|&c| c != '-')
        .take(17)
        .collect();
    format!("{}tenant{}election{}", slug, tenant, election_id)
        .chars()
        .filter(|&c| c != '-')
        .collect()
}
