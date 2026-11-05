// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::celery_app::get_celery_app;
use crate::services::vitrine::statements::{
    log_event_statements, pending_events, EventMilestone, RUN_EXPIRES_SECS,
};
use crate::types::error::Result;
use celery::error::TaskError;
use tracing::instrument;

/// Queue a run for every transparent event with statements to log in its
/// `events` log or receipts to keep. Scheduled by Windmill beat.
#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0, expires = 10)]
pub async fn schedule_vitrine_statements() -> Result<()> {
    let celery_app = get_celery_app().await;
    for (tenant_id, election_event_id) in pending_events().await? {
        celery_app
            .send_task(
                log_vitrine_statements::new(tenant_id, election_event_id, None)
                    .with_expires_in(RUN_EXPIRES_SECS),
            )
            .await
            .map_err(|error| {
                anyhow::anyhow!("Error queueing a Vitrine statements run: {error:?}")
            })?;
    }
    Ok(())
}

/// Log a transparent event's due statements, and the milestone if one is
/// given, in its `events` log, and keep their receipt chains.
#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0, time_limit = 120)]
pub async fn log_vitrine_statements(
    tenant_id: String,
    election_event_id: String,
    milestone: Option<EventMilestone>,
) -> Result<()> {
    log_event_statements(&tenant_id, &election_event_id, milestone.as_ref()).await?;
    Ok(())
}
