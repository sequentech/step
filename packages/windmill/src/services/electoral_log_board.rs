// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Audit board names, clients and application signing identities.
//! Cryptographic signing is supplied by wbraid's native signature scheme.

use crate::services::vault;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use electoral_log::BoardClient;
use std::env;
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
    #[derive(serde::Deserialize)]
    struct SigningIdentity {
        signing_key: String,
    }
    let identity: SigningIdentity =
        toml::from_str(&contents).context("Invalid audit signing identity")?;
    SigningKey::from_b64_string(&identity.signing_key)
}

#[instrument(err)]
pub async fn get_board_client() -> Result<BoardClient> {
    let url =
        env::var("WBRAID_AUDIT_DATABASE_URL").context("WBRAID_AUDIT_DATABASE_URL must be set")?;
    BoardClient::new(&url).await
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
