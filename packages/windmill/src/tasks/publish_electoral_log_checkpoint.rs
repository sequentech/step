// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::electoral_log_audit::publish_event_checkpoint;
use crate::types::error::Result;
use celery::error::TaskError;
use electoral_log::messages::newtypes::ElectoralLogCheckpointReason;
use tracing::instrument;

/// Publish a signed checkpoint of an election event's electoral log.
///
/// Queued when voting closes, so the closure never waits for the publication.
#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 2)]
pub async fn publish_electoral_log_checkpoint(
    tenant_id: String,
    election_event_id: String,
    reason: ElectoralLogCheckpointReason,
) -> Result<()> {
    publish_event_checkpoint(&tenant_id, &election_event_id, reason).await?;
    Ok(())
}
