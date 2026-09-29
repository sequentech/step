// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Read a keys ceremony's board and move the ceremony to what it says.
//!
//! Called on every beat while the ceremony is running. It decides nothing
//! itself: it reads the board, hands the reading to the ceremony's own rules,
//! and saves the result. A board that cannot be reached leaves everything
//! alone; one that can never serve this ceremony ends it.

use crate::postgres::keys_ceremony::get_keys_ceremony_by_id;
use crate::services::database::get_hasura_pool;
use crate::services::protocol_board::board_handle;
use crate::services::protocol_board::{apply_event, load_dkg_board};
use crate::types::error::Result;
use anyhow::{Context, Result as AnyhowResult};
use celery::error::TaskError;
use deadpool_postgres::{Client as DbClient, Transaction};
use protocol_board::PlatformEvent;
use sequent_core::types::ceremonies::KeysCeremonyExecutionStatus;
use tracing::{info, instrument, warn};

pub async fn set_public_key_impl(
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

    let execution_status = keys_ceremony.execution_status()?;
    if execution_status != KeysCeremonyExecutionStatus::IN_PROGRESS {
        info!("Unexpected status {execution_status}");
        return Ok(());
    }
    if keys_ceremony.status()?.public_key.is_some() {
        info!("Public key already set");
        return Ok(());
    }

    let dkg = load_dkg_board(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &keys_ceremony_id,
    )
    .await?;

    let view = match board_handle(&dkg.name)?
        .fetch_dkg_status(dkg.configuration.hash())
        .await
    {
        Ok(view) => view,
        Err(err) => {
            // Nothing is written on an outage: the next beat tries again.
            warn!("board {} is not reachable: {err:#}", dkg.name);
            return Ok(());
        }
    };

    apply_event(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &keys_ceremony_id,
        PlatformEvent::ReadBoard(view),
    )
    .await?;
    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn set_public_key(
    tenant_id: String,
    election_event_id: String,
    keys_ceremony_id: String,
) -> Result<()> {
    set_public_key_impl(tenant_id, election_event_id, keys_ceremony_id).await?;

    Ok(())
}
