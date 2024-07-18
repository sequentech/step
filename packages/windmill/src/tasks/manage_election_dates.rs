use crate::hasura::election_event::{get_election_event, get_election_event_helper};
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election::{get_election_by_id, update_election_voting_status};
use crate::postgres::election_event::{get_election_event_by_id, update_election_event_status};
use crate::postgres::scheduled_event::*;
use crate::services::database::get_hasura_pool;
use crate::services::election_event_status::get_election_event_status;
use crate::types::error::{Error, Result};
use crate::types::scheduled_event::{CronConfig, EventProcessors};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use sequent_core::ballot::{ElectionEventStatus, ElectionStatus, VotingStatus};
use serde::{Deserialize, Serialize};
use tracing::{event, Level};
use tracing::{info, instrument};

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn manage_election_date(
    tenant_id: Option<String>,
    election_event_id: Option<String>,
    scheduled_event_id: String,
    election_id: String,
) -> Result<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| anyhow!("Error getting hasura client {}", e))?;
    let hasura_transaction = hasura_db_client.transaction().await?;
    let scheduled_manage_date_opt = find_scheduled_event_by_id(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        &scheduled_event_id,
    )
    .await?;
    let Some(scheduled_manage_date) = scheduled_manage_date_opt else {
        event!(
            Level::WARN,
            "Can't find scheduled event with id: {scheduled_event_id}"
        );
        return Ok(());
    };

    let Some(tenant_id) = scheduled_manage_date.tenant_id.clone() else {
        event!(Level::WARN, "Missing tenant_id");
        return Ok(());
    };

    let Some(election_event_id) = scheduled_manage_date.election_event_id.clone() else {
        event!(Level::WARN, "Missing election_event_id");
        return Ok(());
    };

    let election_event =
        get_election_event_by_id(&hasura_transaction, &tenant_id, &election_event_id).await?;

    let Some(_election) = get_election_by_id(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &election_id,
    )
    .await?
    else {
        event!(Level::WARN, "Election not found");
        return Ok(());
    };

    let mut election_status: ElectionStatus = Default::default();

    let Some(event_processor) = scheduled_manage_date.event_processor.clone() else {
        event!(Level::WARN, "Missing event processor");
        return Ok(());
    };

    election_status.voting_status = if EventProcessors::START_ELECTION == event_processor {
        VotingStatus::OPEN
    } else {
        VotingStatus::CLOSED
    };

    // update the database
    update_election_voting_status(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &election_id,
        serde_json::to_value(election_status)?,
    )
    .await?;
    let mut elsection_event_status: ElectionEventStatus =
        get_election_event_status(election_event.status).unwrap_or(Default::default());

    if (event_processor == EventProcessors::START_ELECTION
        && elsection_event_status.voting_status == VotingStatus::NOT_STARTED)
    {
        elsection_event_status.voting_status = VotingStatus::OPEN;
        update_election_event_status(
            &hasura_transaction,
            &tenant_id,
            &election_event_id,
            serde_json::to_value(elsection_event_status)?,
        )
        .await?;
    }

    stop_scheduled_event(&hasura_transaction, &tenant_id, &scheduled_manage_date.id).await?;

    let _commit = hasura_transaction
        .commit()
        .await
        .map_err(|e| anyhow!("Commit failed manae_election_dates: {}", e));

    Ok(())
}
