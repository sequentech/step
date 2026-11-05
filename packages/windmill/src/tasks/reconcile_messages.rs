// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::database::get_hasura_pool;
use crate::services::messaging::audit::post_attempt_outcomes_or_warn;
use crate::services::messaging::dispatch::Dispatcher;
use crate::services::messaging::reconcile::sweep_messages;
use crate::tasks::send_template::RetryByOrigin;
use crate::types::error::Result;
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use tracing::instrument;

/// The messaging sweep: expires Messenger links, settles attempts a stopped
/// worker left queued, sends the same-channel retries that are due, asks
/// providers about unknown outcomes, and writes the electoral log entries
/// a stopped sender left out. Never sends an unknown outcome again. Runs on
/// the beat queue, so a bulk send doesn't hold it up; each pass is bounded
/// (`SweepLimits`).
#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(expires = 120, max_retries = 0)]
pub async fn reconcile_messages() -> Result<()> {
    let mut client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| format!("Error getting hasura db pool: {err}"))?;
    let dispatcher = Dispatcher::global()
        .await
        .map_err(|err| format!("Error creating the message dispatcher: {err}"))?;
    let renderer = RetryByOrigin::production().await;
    let (_, recorded) = sweep_messages(&mut client, dispatcher, &renderer)
        .await
        .map_err(|err| format!("Error in the messaging sweep: {err:#}"))?;
    let tx = client
        .transaction()
        .await
        .map_err(|err| format!("Error starting a transaction: {err}"))?;
    post_attempt_outcomes_or_warn(&tx, &recorded, None, None).await;
    tx.commit()
        .await
        .map_err(|err| format!("Error committing: {err}"))?;
    Ok(())
}
