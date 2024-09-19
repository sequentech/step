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
use anyhow::{anyhow, Result as AnyhowResult};
use celery::error::TaskError;
use chrono::Duration;
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use sequent_core::ballot::{ElectionStatus, VotingStatus};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use tracing::{event, Level};
use uuid::Uuid;

#[instrument(err)]
async fn manage_election_date_wrapper(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    scheduled_event_id: String,
    election_id: String,
) -> AnyhowResult<()> {
    let scheduled_manage_date_opt = find_scheduled_event_by_id(
        hasura_transaction,
        Some(tenant_id.clone()),
        Some(election_event_id.clone()),
        &scheduled_event_id,
    )
    .await?;
    let Some(scheduled_manage_date) = scheduled_manage_date_opt else {
        return Err(anyhow!(
            "Can't find scheduled event with id: {}",
            scheduled_event_id
        ));
    };

    let Some(_election) = get_election_by_id(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &election_id,
    )
    .await?
    else {
        return Err(anyhow!("Election not found"));
    };

    let mut election_status: ElectionStatus = Default::default();

    let Some(event_processor) = scheduled_manage_date.event_processor.clone() else {
        return Err(anyhow!("Missing event processor"));
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
            return Err(anyhow!("Invalid scheduled event type: {event_processor:?}"));
        }
    };

    voting_status::update_election_status(
        tenant_id.clone(),
        hasura_transaction,
        &election_event_id,
        &election_id,
        &election_status.voting_status,
    )
    .await?;

    stop_scheduled_event(&hasura_transaction, &tenant_id, &scheduled_manage_date.id).await?;

    Ok(())
}

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

    let res = manage_election_date_wrapper(
        &hasura_transaction,
        tenant_id.clone(),
        election_event_id.clone(),
        scheduled_event_id.clone(),
        election_id.clone(),
    )
    .await;

    match res {
        Ok(data) => {
            let commit = hasura_transaction
                .commit()
                .await
                .map_err(|e| anyhow!("Commit failed manage_election_dates: {}", e));
            lock.release().await?;
            commit?;
        }
        Err(err) => {
            lock.release().await?;
            return Err(anyhow!("{}", err).into());
        }
    }

    Ok(())
}
