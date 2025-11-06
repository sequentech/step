// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election::{get_election_by_id, update_election_voting_status};
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
use sequent_core::ballot::{ElectionPresentation, ElectionStatus, InitReport};
use sequent_core::serialization::deserialize_with_path::{self, deserialize_value};
use sequent_core::services::date::ISO8601;
use sequent_core::types::scheduled_event::*;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use tracing::{error, event, info, Level};
use uuid::Uuid;

#[instrument(err)]
async fn manage_election_init_report_wrapped(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    scheduled_event_id: String,
    election_id: String,
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

    let Some(mut election) = get_election_by_id(
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

    let Some(event_payload) = scheduled_event.event_payload.clone() else {
        event!(Level::WARN, "Missing election_event_id");
        return Ok(());
    };
    let event_payload: ManageAllowInitPayload = deserialize_value(event_payload)?;

    // Handle election.status: deserialize if Some, else initialize with default values
    let election_status: ElectionStatus = if let Some(status_value) = election.status.take() {
        deserialize_value::<ElectionStatus>(status_value)?
    } else {
        ElectionStatus::default()
    };

    // Update init_report based on event_payload
    let updated_status = ElectionStatus {
        init_report: if event_payload.allow_init == Some(true) {
            InitReport::ALLOWED
        } else {
            InitReport::DISALLOWED
        },
        ..election_status
    };

    update_election_voting_status(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &election_id,
        serde_json::to_value(updated_status)?,
    )
    .await?;

    stop_scheduled_event(&hasura_transaction, &tenant_id, &scheduled_event.id)
        .await
        .with_context(|| "Error stopping scheduled event")?;

    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 10, max_retries = 0, expires = 30)]
pub async fn manage_election_init_report(
    tenant_id: String,
    election_event_id: String,
    scheduled_event_id: String,
    election_id: String,
) -> Result<()> {
    let res = provide_hasura_transaction(|hasura_transaction| {
        let tenant_id = tenant_id.clone();
        let election_event_id = election_event_id.clone();
        let scheduled_event_id = scheduled_event_id.clone();
        let election_id = election_id.clone();
        Box::pin(async move {
            // Your async code here
            manage_election_init_report_wrapped(
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

    Ok(res?)
}
