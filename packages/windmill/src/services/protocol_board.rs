// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The crypto core's boards, as windmill handles it: loading the
//! `Configuration` its row stores, and writing back what the
//! `protocol_board` crate has decided.

use anyhow::{anyhow, Context as _, Result};
use deadpool_postgres::Transaction;
use protocol_board::{
    encode_manager_key, transition, BoardHandle, BoardManager, BoardName, CeremonyState, DkgBoard,
    PlatformEvent, SignedConfiguration, Timestamps,
};
use sequent_core::services::date::{get_now_utc_unix_ms, ISO8601};
use tracing::instrument;
use uuid::Uuid;

use crate::postgres::keys_ceremony::{
    get_keys_ceremony_by_id_for_update, update_keys_ceremony_status,
};
use crate::postgres::protocol_board::get_dkg_board_by_keys_ceremony;
use crate::services::vault;

pub fn board_handle(board: &BoardName) -> Result<BoardHandle> {
    let base_url = std::env::var("WBRAID_B4_URL")
        .ok()
        .filter(|url| !url.trim().is_empty())
        // An unset address would otherwise look like an outage.
        .ok_or_else(|| anyhow!("WBRAID_B4_URL must be set to the board service URL"))?;
    Ok(BoardHandle::new(base_url.trim(), board.clone()))
}

/// The current time, in the two forms a ceremony records it.
pub fn ceremony_now() -> Timestamps {
    Timestamps {
        iso8601: ISO8601::to_string(&ISO8601::now()),
        unix_ms: get_now_utc_unix_ms().to_string(),
    }
}

#[instrument(err, skip(hasura_transaction))]
pub async fn load_dkg_board(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    keys_ceremony_id: &str,
) -> Result<DkgBoard> {
    let row = get_dkg_board_by_keys_ceremony(
        hasura_transaction,
        tenant_id,
        election_event_id,
        keys_ceremony_id,
    )
    .await?
    .ok_or_else(|| anyhow!("keys ceremony {keys_ceremony_id} has no DKG board recorded"))?;

    let keys_ceremony_id = Uuid::parse_str(&row.keys_ceremony_id)
        .with_context(|| format!("board {} names an unreadable ceremony id", row.name))?;
    let configuration = SignedConfiguration::parse(&row.manager_message)
        .with_context(|| format!("board {} stores an unreadable configuration", row.name))?;

    Ok(DkgBoard::from_signed_configuration(
        &keys_ceremony_id,
        configuration,
    ))
}

/// Apply one event to a ceremony and commit what it decided.
///
/// The ceremony is loaded again under a row lock: windmill's beat and a
/// trustee's report both write it, and which event prevails is a rule of the
/// transition, not of write order, so every transition must run on the latest
/// committed state.
///
/// A no-op transition commits nothing, this lets a task run on every
/// beat without touching the database when the board has not
/// moved.
#[instrument(err, skip(hasura_transaction, event))]
pub async fn apply_event(
    hasura_transaction: Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    keys_ceremony_id: &str,
    event: PlatformEvent,
) -> Result<()> {
    let keys_ceremony = get_keys_ceremony_by_id_for_update(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        keys_ceremony_id,
    )
    .await?;
    let unreadable = || format!("keys ceremony {keys_ceremony_id} has an unreadable status");
    let state = CeremonyState::new(
        keys_ceremony.execution_status().with_context(unreadable)?,
        keys_ceremony.status().with_context(unreadable)?,
    );

    let next = transition(&state, keys_ceremony.policy(), event, &ceremony_now());
    if next == state {
        return Ok(());
    }

    update_keys_ceremony_status(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        keys_ceremony_id,
        &serde_json::to_value(&next.status)?,
        &next.execution.to_string(),
    )
    .await?;

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error committing transaction")?;
    Ok(())
}

pub fn manager_key_vault_path(board: &BoardName) -> String {
    format!("protocol-boards/{board}/manager")
}

/// Keep the board's protocol manager key in the vault.
///
/// The caller should commit `hasura_transaction` together with the board row
/// that stores the Configuration this key signed.
#[instrument(err, skip(hasura_transaction, manager))]
pub async fn save_manager_key(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    board: &BoardName,
    manager: &BoardManager,
) -> Result<()> {
    tracing::info!("storing the protocol manager key of board {board}");
    vault::save_secret(
        hasura_transaction,
        tenant_id,
        Some(election_event_id),
        &manager_key_vault_path(board),
        &encode_manager_key(manager)?,
    )
    .await
}
