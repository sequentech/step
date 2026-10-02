// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::database::get_hasura_pool;
use crate::services::messaging::dispatch::Dispatcher;
use crate::services::messaging::reconcile::reconcile_messages as reconcile;
use crate::types::error::Result;
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use tracing::instrument;

/// Resolves message attempts whose outcome is unknown and expires
/// Messenger links. Never sends a message again.
#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(expires = 60, max_retries = 0)]
pub async fn reconcile_messages() -> Result<()> {
    let mut client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| format!("Error getting hasura db pool: {err}"))?;
    let dispatcher = Dispatcher::global()
        .await
        .map_err(|err| format!("Error creating the message dispatcher: {err}"))?;
    reconcile(&mut client, dispatcher)
        .await
        .map_err(|err| format!("Error reconciling messages: {err:#}"))?;
    Ok(())
}
