// SPDX-FileCopyrightText: 2023 Rafael Fernández López <rafael.fernandez@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election_event::{
    get_election_event_by_id, update_election_event_presentation,
};
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
use sequent_core::ballot::{ElectionEventPresentation, InitReport, LockedDown, VotingStatus};
use sequent_core::serialization::deserialize_with_path::{self, deserialize_value};
use sequent_core::services::date::ISO8601;
use sequent_core::types::scheduled_event::*;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use tracing::{error, event, info, Level};
use uuid::Uuid;

#[instrument(err)]
async fn manage_election_event_lockdown_wrapped(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    scheduled_event_id: String,
) -> AnyhowResult<()> {
    let scheduled_event = find_scheduled_event_by_id(
        hasura_transaction,
        Some(tenant_id.clone()),
        Some(election_event_id.clone()),
        &scheduled_event_id,
    )
    .await
    .with_context(|| "Error obtaining scheduled event by id")?;

    let Some(scheduled_event) = scheduled_event else {
        return Err(anyhow!(
            "Can't find scheduled event with id: {}",
            scheduled_event_id
        ));
    };

    let locked_down =
        scheduled_event.event_processor == Some(EventProcessors::START_LOCKDOWN_PERIOD);

    let election_event =
        get_election_event_by_id(hasura_transaction, &tenant_id, &election_event_id).await?;

    if let Some(election_event_presentation) = election_event.presentation {
        let election_event_presentation: ElectionEventPresentation = ElectionEventPresentation {
            locked_down: if locked_down {
                Some(LockedDown::LOCKED_DOWN)
            } else {
                Some(LockedDown::NOT_LOCKED_DOWN)
            },
            ..deserialize_with_path::deserialize_value(election_event_presentation)?
        };
        update_election_event_presentation(
            hasura_transaction,
            &tenant_id,
            &election_event_id,
            serde_json::to_value(election_event_presentation)?,
        )
        .await?;
    }

    stop_scheduled_event(&hasura_transaction, &tenant_id, &scheduled_event.id)
        .await
        .with_context(|| "Error stopping scheduled event")?;

    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 10, max_retries = 0, expires = 30)]
pub async fn manage_election_event_lockdown(
    tenant_id: String,
    election_event_id: String,
    scheduled_event_id: String,
) -> Result<()> {
    let res = provide_hasura_transaction(|hasura_transaction| {
        let tenant_id = tenant_id.clone();
        let election_event_id = election_event_id.clone();
        let scheduled_event_id = scheduled_event_id.clone();
        Box::pin(async move {
            // Your async code here
            manage_election_event_lockdown_wrapped(
                hasura_transaction,
                tenant_id,
                election_event_id,
                scheduled_event_id,
            )
            .await
        })
    })
    .await;

    info!("result: {:?}", res);

    Ok(res?)
}
