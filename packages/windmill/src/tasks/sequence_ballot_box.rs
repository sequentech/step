// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::ballot_box::{events_waiting_for_sequencer, sequence_event};
use crate::services::celery_app::get_celery_app;
use crate::types::error::Result;
use celery::error::TaskError;
use tracing::{info, instrument};

/// Seconds after which a queued sequencer run is dropped, since a newer one follows.
const SEQUENCE_TASK_EXPIRES_SECS: u32 = 60;

/// Queue a sequencer run for every event whose ballot box holds ballots waiting to
/// be appended to its board. Scheduled by Windmill beat.
#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0, expires = 10)]
pub async fn schedule_ballot_box_sequencers() -> Result<()> {
    let celery_app = get_celery_app().await;
    for (tenant_id, election_event_id) in events_waiting_for_sequencer().await? {
        celery_app
            .send_task(
                sequence_ballot_box::new(tenant_id, election_event_id)
                    .with_expires_in(SEQUENCE_TASK_EXPIRES_SECS),
            )
            .await
            .map_err(|error| anyhow::anyhow!("Error queueing a sequencer run: {error:?}"))?;
    }
    Ok(())
}

/// Append the records of an event's accepted ballots to its board.
#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0, time_limit = 120)]
pub async fn sequence_ballot_box(tenant_id: String, election_event_id: String) -> Result<()> {
    match sequence_event(&tenant_id, &election_event_id).await? {
        Some(appended) => info!("Appended {appended} ballots of event {election_event_id}"),
        None => info!("Another sequencer is working on event {election_event_id}"),
    }
    Ok(())
}
