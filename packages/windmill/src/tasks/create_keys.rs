// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::keys_ceremony::{get_keys_ceremony_by_id, update_keys_ceremony_status};
use crate::postgres::trustee::get_trustees_by_id;
use crate::services::ceremonies::keys_ceremony::get_keys_ceremony_board;
use crate::services::database::get_hasura_pool;
use crate::services::protocol_manager::check_configuration_exists;
use crate::services::{ceremonies, public_keys};
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context, Result as AnyhowResult};
use celery::error::TaskError;
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::ceremonies::KeysCeremonyExecutionStatus;
use sequent_core::types::hasura::core::KeysCeremony;
use serde::{Deserialize, Serialize};
use std::default::Default;
use tracing::{info, instrument};

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct CreateKeysBody {
    pub trustee_pks: Vec<String>,
    pub threshold: usize,
}

pub async fn create_keys_impl(
    tenant_id: String,
    election_event_id: String,
    keys_ceremony_id: String,
) -> AnyhowResult<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool().await.get().await?;

    let hasura_transaction = hasura_db_client.transaction().await?;

    let keys_ceremony = get_keys_ceremony_by_id(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &keys_ceremony_id,
    )
    .await
    .with_context(|| "error finding keys ceremony")?;

    let trustees = get_trustees_by_id(
        &hasura_transaction,
        &tenant_id,
        &keys_ceremony.trustee_ids,
        Some(&election_event_id),
        Some(&keys_ceremony_id),
    )
    .await?;
    info!("trustees: {:?}", trustees);

    // Explicit guard: every trustee in this ceremony must have a key
    // registered and scoped to (election_event_id, keys_ceremony_id) before
    // we build a Configuration. Configuration::new panics via
    // assert!(c.is_valid()) if the trustee list is too short — silently
    // dropping a missing trustee here (the old behavior) risked crashing the
    // beat worker. This should be unreachable once process_board's gate is
    // in place (it only dispatches this task once the gate is satisfied),
    // but stays as a defensive check against beat-task races or a
    // manually-triggered task.
    if trustees.len() != keys_ceremony.trustee_ids.len()
        || trustees.iter().any(|trustee| trustee.public_key.is_none())
    {
        info!(
            "Not all trustees have a registered key yet for ceremony {}; skipping",
            keys_ceremony_id
        );
        return Ok(());
    }

    let trustee_pks: Vec<String> = trustees
        .clone()
        .into_iter()
        .filter_map(|trustee| trustee.public_key)
        .collect();
    info!("trustee_pks: {:?}", trustee_pks);

    let (board_name, _) = get_keys_ceremony_board(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &keys_ceremony,
    )
    .await?;

    let execution_status = keys_ceremony.execution_status()?;
    let status = keys_ceremony.status()?;

    // check config is not already created
    if execution_status != KeysCeremonyExecutionStatus::AWAITING_TRUSTEE_KEYS
        || status.public_key.is_some()
    {
        info!("Unexpected status: {}", execution_status);
        return Ok(());
    }

    let configuration_exists = check_configuration_exists(board_name.as_str()).await?;

    if !configuration_exists {
        // create config/keys for board
        public_keys::create_keys(
            &hasura_transaction,
            &tenant_id,
            &election_event_id,
            board_name.as_str(),
            trustee_pks,
            keys_ceremony.threshold as usize,
        )
        .await?;
    }

    // Transition AWAITING_TRUSTEE_KEYS -> IN_PROGRESS. The per-ceremony key
    // rows in `trustee_ceremony_key` (one per (trustee, event, ceremony)) are
    // never overwritten by other ceremonies, so they already serve as the
    // frozen record of the keys that went into this Configuration — tally-time
    // code reads them back via the scoped trustee query. No separate snapshot
    // into status is needed.
    update_keys_ceremony_status(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &keys_ceremony.id,
        &serde_json::to_value(status)?,
        &execution_status
            .try_transition(KeysCeremonyExecutionStatus::IN_PROGRESS)?
            .to_string(),
    )
    .await?;

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")?;

    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn create_keys(
    tenant_id: String,
    election_event_id: String,
    keys_ceremony_id: String,
) -> Result<()> {
    create_keys_impl(tenant_id, election_event_id, keys_ceremony_id)
        .await
        .map_err(|err| Error::from(err.context("Task failed")))
}
