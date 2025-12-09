// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election::get_election_by_id;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::scheduled_event::*;
use crate::services::database::get_hasura_pool;
use crate::services::pg_lock::PgLock;
use crate::services::providers::transactions_provider::provide_hasura_transaction;
use crate::services::voting_status::{self};
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context, Result as AnyhowResult};
use async_trait::async_trait;
use celery::error::TaskError;
use chrono::Duration;
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use sequent_core::ballot::{ElectionStatus, VotingStatus, VotingStatusChannel};
use sequent_core::services::date::ISO8601;
use sequent_core::types::scheduled_event::*;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use tracing::{error, event, info, Level};
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
    .await
    .with_context(|| "Error obtaining scheduled event by id")?;

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
    .await
    .with_context(|| "Error obtaining election by id")?
    else {
        return Err(anyhow!("Election not found"));
    };

    let Some(event_processor) = scheduled_manage_date.event_processor.clone() else {
        return Err(anyhow!("Missing event processor"));
    };

    let status = match event_processor {
        EventProcessors::START_VOTING_PERIOD => VotingStatus::OPEN,
        EventProcessors::END_VOTING_PERIOD => VotingStatus::CLOSED,
        _ => {
            info!("Invalid scheduled event type: {:?}", event_processor);
            stop_scheduled_event(&hasura_transaction, &tenant_id, &scheduled_manage_date.id)
                .await?;
            return Ok(());
        }
    };

    let voting_channels: Vec<VotingStatusChannel> = match event_processor {
        EventProcessors::START_VOTING_PERIOD => {
            vec![VotingStatusChannel::ONLINE, VotingStatusChannel::KIOSK]
        }
        EventProcessors::END_VOTING_PERIOD => vec![VotingStatusChannel::ONLINE],
        _ => {
            info!("Invalid scheduled event type: {:?}", event_processor);
            stop_scheduled_event(&hasura_transaction, &tenant_id, &scheduled_manage_date.id)
                .await?;
            return Ok(());
        }
    };

    let result = voting_status::update_election_status(
        tenant_id.clone(),
        None,
        None,
        hasura_transaction,
        &election_event_id,
        &election_id,
        &status,
        &Some(voting_channels),
    )
    .await;
    info!("result: {result:?}");

    stop_scheduled_event(&hasura_transaction, &tenant_id, &scheduled_manage_date.id)
        .await
        .map_err(|err| anyhow!("Error stopping scheduled event: {err:?}"))?;

    result?;

    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 10, max_retries = 0, expires = 30)]
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
    .await
    .with_context(|| "Error acquiring pglock")?;

    let res = provide_hasura_transaction(|hasura_transaction| {
        let tenant_id = tenant_id.clone();
        let election_event_id = election_event_id.clone();
        let scheduled_event_id = scheduled_event_id.clone();
        let election_id = election_id.clone();
        Box::pin(async move {
            // Your async code here
            manage_election_date_wrapper(
                hasura_transaction,
                tenant_id,
                election_event_id,
                scheduled_event_id,
                election_id,
            )
            .await
        })
    })
    .await;

    info!("result: {:?}", res);

    lock.release()
        .await
        .with_context(|| "Error releasing pglock")?;

    Ok(res?)
}
