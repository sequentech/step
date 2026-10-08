// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Signed closes are authoritative (VOTE-LIFECYCLE §5a): on every scheduler
//! tick, each election event with a signed lifecycle snapshot gets this task,
//! which closes the Posts whose signed close time has passed, whatever
//! happened to the live scheduled row since. The scheduler's own tick is a
//! read-only transaction, so the closes run here, in their own transaction.

use crate::services::pg_lock::PgLock;
use crate::services::providers::transactions_provider::provide_hasura_transaction;
use crate::services::signing::actions::voting::{enforce_signed_closes, StatusCloser};
use crate::types::error::Result;
use anyhow::{Context, Result as AnyhowResult};
use celery::error::TaskError;
use chrono::Duration;
use deadpool_postgres::Transaction;
use sequent_core::services::date::ISO8601;
use std::future::Future;
use tracing::{error, info, instrument};
use uuid::Uuid;

/// The election events that have a signed lifecycle snapshot (the only ones a
/// signed close can apply to).
pub async fn events_with_signed_snapshots(
    hasura_transaction: &Transaction<'_>,
) -> AnyhowResult<Vec<(String, String)>> {
    let rows = hasura_transaction
        .query(
            "SELECT DISTINCT tenant_id::text, election_event_id::text
               FROM sequent_backend.lifecycle_snapshot
              WHERE approval_request_id IS NOT NULL
              ORDER BY tenant_id::text, election_event_id::text",
            &[],
        )
        .await
        .context("Error listing the events with signed lifecycle snapshots")?;
    Ok(rows.iter().map(|row| (row.get(0), row.get(1))).collect())
}

/// Dispatches checks independently of live schedule rows. A broker failure
/// for one event does not prevent the other signed deadlines being checked.
pub async fn dispatch_signed_close_checks<F, Fut>(
    hasura_transaction: &Transaction<'_>,
    mut send: F,
) -> AnyhowResult<usize>
where
    F: FnMut(String, String) -> Fut,
    Fut: Future<Output = AnyhowResult<()>>,
{
    let events = events_with_signed_snapshots(hasura_transaction).await?;
    let mut failed = 0;
    for (tenant_id, election_event_id) in events {
        if let Err(err) = send(tenant_id, election_event_id.clone()).await {
            failed += 1;
            error!("Couldn't send the signed-close check for event {election_event_id}: {err:#}");
        }
    }
    Ok(failed)
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 30, max_retries = 0, expires = 30)]
pub async fn enforce_signed_closes_task(
    tenant_id: String,
    election_event_id: String,
) -> Result<()> {
    let lock: PgLock = PgLock::acquire(
        format!("enforce_signed_closes-{tenant_id}-{election_event_id}"),
        Uuid::new_v4().to_string(),
        ISO8601::now() + Duration::seconds(120),
    )
    .await
    .with_context(|| "Error acquiring pglock")?;

    let res = provide_hasura_transaction(|hasura_transaction| {
        let tenant_id = tenant_id.clone();
        let election_event_id = election_event_id.clone();
        Box::pin(async move {
            let closed = enforce_signed_closes(
                hasura_transaction,
                &tenant_id,
                &election_event_id,
                &StatusCloser,
            )
            .await?;
            if !closed.is_empty() {
                info!(
                    "Closed {} Posts at their signed close time in event {election_event_id}",
                    closed.len()
                );
            }
            Ok(())
        })
    })
    .await;

    lock.release()
        .await
        .with_context(|| "Error releasing pglock")?;
    res?;
    crate::tasks::signing_log_outbox::kick_signing_log_outbox();
    crate::tasks::seal_ballot_boxes::kick_ballot_box_sealer();
    Ok(())
}
