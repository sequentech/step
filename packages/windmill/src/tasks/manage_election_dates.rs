// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election::get_election_by_id;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::scheduled_event::*;
use crate::services::database::get_hasura_pool;
use crate::services::date::ISO8601;
use crate::services::pg_lock::PgLock;
use crate::services::voting_status::{self};
use crate::types::error::{Error, Result};
use crate::types::scheduled_event::EventProcessors;
use anyhow::anyhow;
use celery::error::TaskError;
use chrono::Duration;
use deadpool_postgres::Client as DbClient;
use sequent_core::ballot::{ElectionStatus, VotingStatus};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use tracing::{event, Level};
use uuid::Uuid;

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 30, max_retries = 0)]
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

    let result = voting_status::update_election_status(
        tenant_id.clone(),
        &hasura_transaction,
        &election_event_id,
        &election_id,
        &election_status.voting_status,
    )
    .await;

    match result.err() {
        Some(error) => {
            lock.release().await?;
            return Err(Error::Anyhow(error));
        }
        None => (),
    }

    let result =
        stop_scheduled_event(&hasura_transaction, &tenant_id, &scheduled_manage_date.id).await;

    match result.err() {
        Some(error) => {
            lock.release().await?;
            return Err(Error::Anyhow(error));
        }
        None => (),
    }

    let commit = hasura_transaction
        .commit()
        .await
        .map_err(|e| anyhow!("Commit failed manae_election_dates: {}", e));
    lock.release().await?;

    match commit.err() {
        Some(error) => {
            return Err(Error::Anyhow(error));
        }
        None => (),
    }

    Ok(())
}
