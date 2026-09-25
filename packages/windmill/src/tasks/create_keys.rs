// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Put a keys ceremony's prepared `Configuration` on its board.
//!
//! The Configuration was signed and stored when the ceremony was created, so
//! this task chooses nothing, it just posts the stored message. Safe to retry.

use crate::postgres::keys_ceremony::get_keys_ceremony_by_id;
use crate::services::database::get_hasura_pool;
use crate::services::protocol_board::board_handle;
use crate::services::protocol_board::{apply_event, load_dkg_board};
use crate::types::error::{Error, Result};
use anyhow::{Context, Result as AnyhowResult};
use celery::error::TaskError;
use deadpool_postgres::{Client as DbClient, Transaction};
use protocol_board::{CeremonyState, PlatformEvent};
use sequent_core::types::ceremonies::KeysCeremonyExecutionStatus;
use tracing::{info, instrument, warn};

pub async fn create_keys_impl(
    tenant_id: String,
    election_event_id: String,
    keys_ceremony_id: String,
) -> AnyhowResult<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool().await.get().await?;
    let hasura_transaction: Transaction = hasura_db_client.transaction().await?;

    let keys_ceremony = get_keys_ceremony_by_id(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &keys_ceremony_id,
    )
    .await
    .with_context(|| "error finding keys ceremony")?;

    let state = CeremonyState::new(keys_ceremony.execution_status()?, keys_ceremony.status()?);
    if state.execution != KeysCeremonyExecutionStatus::STARTED {
        info!("Unexpected key ceremony status: {}", state.execution);
        return Ok(());
    }

    let dkg = load_dkg_board(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &keys_ceremony_id,
    )
    .await?;

    if let Err(err) = board_handle(&dkg.name)?
        .publish_configuration(&dkg.configuration)
        .await
    {
        // Nothing is written on an outage: the next beat tries again.
        warn!("board {} is not reachable: {err:#}", dkg.name);
        return Ok(());
    }
    info!("posted the configuration of board {}", dkg.name);

    apply_event(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &keys_ceremony_id,
        &state,
        keys_ceremony.policy(),
        PlatformEvent::Published { board: dkg.name },
    )
    .await
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
