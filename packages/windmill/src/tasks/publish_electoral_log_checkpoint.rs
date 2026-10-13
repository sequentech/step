// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::electoral_log_audit::{
    publish_event_checkpoint, publish_periodic_checkpoints,
};
use crate::types::error::Result;
use celery::error::TaskError;
use electoral_log::messages::newtypes::ElectoralLogCheckpointReason;
use tracing::{info, instrument};

/// Publish a signed checkpoint of an election event's electoral log.
///
/// Queued when voting opens or closes, so the change never waits for the publication.
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

/// Publish a checkpoint of the log of every election event with open voting that grew
/// since its last checkpoint. Windmill beat schedules it every
/// `ELECTORAL_LOG_CHECKPOINT_INTERVAL_SECS` seconds.
#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0, expires = 60)]
pub async fn publish_periodic_electoral_log_checkpoints() -> Result<()> {
    let outcome = publish_periodic_checkpoints().await?;
    info!(
        "Periodic electoral-log checkpoints: {} published, {} up to date, {} failed",
        outcome.published, outcome.up_to_date, outcome.failed
    );
    if outcome.failed > 0 {
        return Err(anyhow::anyhow!(
            "{} periodic electoral-log checkpoints could not be published",
            outcome.failed
        )
        .into());
    }
    Ok(())
}
