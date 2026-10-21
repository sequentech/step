// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Audit board names, clients and application signing identities.
//! Cryptographic signing is supplied by wbraid's native signature scheme.

use crate::services::vault;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use electoral_log::adapters::events::EventDatabases;
use electoral_log::adapters::postgres::PostgresStore;
use electoral_log::BoardClient;
use std::sync::Arc;
use tokio::sync::OnceCell;
use tracing::instrument;
use voting_crypto::signing::SigningKey;

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

#[instrument(skip(hasura_transaction), err)]
pub async fn get_board_signing_key(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: Option<&str>,
    board_name: &str,
) -> Result<SigningKey> {
    let path = get_protocol_manager_secret_path(board_name);
    let contents = vault::read_secret(hasura_transaction, tenant_id, election_event_id, &path)
        .await?
        .ok_or_else(|| anyhow!("audit board signing key not found"))?;
    parse_signing_identity(&contents)
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
