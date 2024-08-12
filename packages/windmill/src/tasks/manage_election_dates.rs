use crate::hasura::election_event::{get_election_event, get_election_event_helper};
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election::{get_election_by_id, update_election_voting_status};
use crate::postgres::election_event::{get_election_event_by_id, update_election_event_status};
use crate::postgres::scheduled_event::*;
use crate::services::database::get_hasura_pool;
use crate::services::date::ISO8601;
use crate::services::election_event_board::get_election_event_board;
use crate::services::election_event_status::get_election_event_status;
use crate::services::electoral_log::ElectoralLog;
use crate::services::pg_lock::PgLock;
use crate::services::voting_status::update_board_on_status_change;
use crate::types::error::{Error, Result};
use crate::types::scheduled_event::{CronConfig, EventProcessors};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use chrono::Duration;
use deadpool_postgres::Client as DbClient;
use sequent_core::ballot::{ElectionEventStatus, ElectionStatus, VotingStatus};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use tracing::{event, Level};
use uuid::Uuid;

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn manage_election_date(
    tenant_id: String,
    election_event_id: String,
    scheduled_event_id: String,
    election_id: String,
) -> Result<()> {
    let lock: PgLock = PgLock::acquire(
        format!(
            "execute_manage_election_date-{}-{}-{}-{}",
            tenant_id, election_event_id, scheduled_event_id, election_id
        ),
        Uuid::new_v4().to_string(),
        ISO8601::now() + Duration::seconds(120),
    )
    .await?;
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| anyhow!("Error getting hasura client {}", e))?;
    let hasura_transaction = hasura_db_client.transaction().await?;
    let scheduled_manage_date_opt = find_scheduled_event_by_id(
        &hasura_transaction,
        Some(tenant_id.clone()),
        Some(election_event_id.clone()),
        &scheduled_event_id,
    )
    .await?;
    let Some(scheduled_manage_date) = scheduled_manage_date_opt else {
        event!(
            Level::WARN,
            "Can't find scheduled event with id: {scheduled_event_id}"
        );
        lock.release().await?;
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
        lock.release().await?;
        return Ok(());
    };

    let mut election_status: ElectionStatus = Default::default();

    let Some(event_processor) = scheduled_manage_date.event_processor.clone() else {
        event!(Level::WARN, "Missing event processor");
        lock.release().await?;
        return Ok(());
    };

    election_status.voting_status = if EventProcessors::START_ELECTION == event_processor {
        VotingStatus::OPEN
    } else {
        VotingStatus::CLOSED
    };

    election_status.voting_status = match event_processor {
        EventProcessors::START_ELECTION => VotingStatus::OPEN,
        EventProcessors::END_ELECTION => VotingStatus::CLOSED,
        _ => {
            lock.release().await?;
            return Err(Error::Anyhow(anyhow!(
                "Invalid scheduled event type: {event_processor:?}"
            )));
        }
    };

    // update the database
    update_election_voting_status(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &election_id,
        serde_json::to_value(&election_status)?,
    )
    .await?;
    let mut election_event_status: ElectionEventStatus =
        get_election_event_status(election_event.status).unwrap_or(Default::default());

    if (event_processor == EventProcessors::START_ELECTION
        && election_event_status.voting_status == VotingStatus::NOT_STARTED)
    {
        election_event_status.voting_status = VotingStatus::OPEN;
        update_election_event_status(
            &hasura_transaction,
            &tenant_id,
            &election_event_id,
            serde_json::to_value(election_event_status)?,
        )
        .await?;

        update_board_on_status_change(
            election_event_id.clone(),
            election_event.bulletin_board_reference.clone(),
            election_status.voting_status.clone(),
            None,
            Some(vec![election_id.clone()]),
        )
        .await?;
    }

    update_board_on_status_change(
        election_event_id.clone(),
        election_event.bulletin_board_reference.clone(),
        election_status.voting_status.clone(),
        Some(election_id.clone()),
        None,
    )
    .await?;

    stop_scheduled_event(&hasura_transaction, &tenant_id, &scheduled_manage_date.id).await?;

    let _commit = hasura_transaction
        .commit()
        .await
        .map_err(|e| anyhow!("Commit failed manae_election_dates: {}", e));
    lock.release().await?;

    Ok(())
}
