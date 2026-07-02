// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::scheduled_event::*;
use crate::services::database::get_hasura_pool;
use crate::services::election_event_status::update_event_voting_status;
use crate::services::pg_lock::PgLock;
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Result as AnyhowResult};
use celery::error::TaskError;
use chrono::Duration;
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use sequent_core::ballot::{ElectionStatus, InitReport, VotingStatus, VotingStatusChannel};
use sequent_core::services::date::ISO8601;
use sequent_core::types::scheduled_event::*;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use tracing::{event, info, Level};
use uuid::Uuid;

#[instrument(err)]
pub async fn manage_election_event_date_wrapped(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    scheduled_event_id: String,
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
            "Can't find scheduled event with id: {scheduled_event_id}"
        ));
    };

    let Some(event_processor) = scheduled_manage_date.event_processor.clone() else {
        return Err(anyhow!("Missing event processor"));
    };

    let voting_status = match event_processor {
        EventProcessors::START_VOTING_PERIOD => VotingStatus::OPEN,
        EventProcessors::END_VOTING_PERIOD => VotingStatus::CLOSED,
        _ => {
            info!("Invalid scheduled event type: {:?}", event_processor);
            stop_scheduled_event(&hasura_transaction, &tenant_id, &scheduled_manage_date.id)
                .await?;
            return Ok(());
        }
    };
    update_event_voting_status(
        &hasura_transaction,
        &tenant_id,
        None,
        None,
        &election_event_id,
        &voting_status,
        &Some(vec![VotingStatusChannel::ONLINE]),
    )
    .await?;

    stop_scheduled_event(&hasura_transaction, &tenant_id, &scheduled_manage_date.id).await?;

    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 10, max_retries = 0, expires = 30)]
pub async fn manage_election_event_date(
    tenant_id: String,
    election_event_id: String,
    scheduled_event_id: String,
) -> Result<()> {
    let lock: PgLock = PgLock::acquire(
        format!(
            "execute_manage_election_event_date-{}-{}-{}",
            tenant_id, election_event_id, scheduled_event_id
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
    let res = manage_election_event_date_wrapped(
        &hasura_transaction,
        tenant_id.clone(),
        election_event_id.clone(),
        scheduled_event_id.clone(),
    )
    .await;

    match res {
        Ok(data) => {
            let commit = hasura_transaction
                .commit()
                .await
                .map_err(|e| anyhow!("Commit failed manage_event_election_dates: {}", e));
            lock.release().await?;
            commit?;
        }
        Err(err) => {
            let rollback = hasura_transaction.rollback().await;
            lock.release().await?;
            rollback?;
            return Err(anyhow!("{}", err).into());
        }
    }

    Ok(())
}
