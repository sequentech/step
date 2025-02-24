// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::maintenance::vacuum_analyze_direct;
use crate::postgres::scheduled_event::*;
use crate::services::pg_lock::PgLock;
use crate::services::providers::transactions_provider::provide_hasura_transaction;
use crate::types::error::Result;
use anyhow::{anyhow, Context, Result as AnyhowResult};
use celery::error::TaskError;
use chrono::Duration;
use deadpool_postgres::Transaction;
use tracing::info;
use tracing::instrument;

use sequent_core::services::date::ISO8601;
use uuid::Uuid;

#[instrument(err)]
async fn database_maintenance_wrapped(
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

    // Execute database maintenance
    info!("Performing scheduled database mainteinance.");
    vacuum_analyze_direct().await?;

    stop_scheduled_event(&hasura_transaction, &tenant_id, &scheduled_event.id)
        .await
        .with_context(|| "Error stopping scheduled event")?;

    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 10, max_retries = 0, expires = 3600)]
pub async fn database_maintenance(
    tenant_id: String,
    election_event_id: String,
    scheduled_event_id: String,
) -> Result<()> {
    let lock: PgLock = PgLock::acquire(
        format!(
            "execute_database_maintenance-{}-{}-{}",
            tenant_id, election_event_id, scheduled_event_id
        ),
        Uuid::new_v4().to_string(),
        ISO8601::now() + Duration::seconds(3600),
    )
    .await
    .with_context(|| "Error acquiring pglock")?;

    let res = provide_hasura_transaction(|hasura_transaction| {
        let tenant_id = tenant_id.clone();
        let election_event_id = election_event_id.clone();
        let scheduled_event_id = scheduled_event_id.clone();
        Box::pin(async move {
            // Your async code here
            database_maintenance_wrapped(
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

    lock.release()
        .await
        .with_context(|| "Error releasing pglock")?;

    Ok(res?)
}
