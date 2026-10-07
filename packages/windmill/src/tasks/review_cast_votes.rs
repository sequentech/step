// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::ballot_box::event_tenants;
use crate::services::protocol_manager::get_electoral_log_store;
use crate::services::{celery_app::get_celery_app, database::PgConfig};
use crate::tasks::process_cast_vote::process_cast_vote;
use crate::types::error::Result;
use anyhow::anyhow;
use celery::error::TaskError;
use electoral_log::adapters::ballot_box_status::BallotToReview;
use tracing::{info, instrument, warn};

/// Recovery work older than one beat interval is redundant: the next scan will
/// enqueue the cast vote again if it is still pending.
const RECOVERY_TASK_EXPIRES_SECS: u32 = 90;

/// Votes younger than this are skipped by the review beat: their
/// process_cast_vote task published directly by harvest is normally still in
/// flight, so re-enqueueing them would only produce redundant PgLock skips.
const PENDING_ENQUEUE_GRACE_SECS: f64 = 90.0;

/// Enqueues `process_cast_vote` for every Datafix vote whose outcome is still
/// pending.
#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0, expires = 90)]
pub async fn review_cast_votes() -> Result<()> {
    let celery_app = get_celery_app().await;
    let batch_size: i64 = PgConfig::from_env()?.default_sql_batch_size.into();

    info!("review_cast_votes: Checking pending cast votes");
    let store = get_electoral_log_store().await?;
    let mut after: Option<BallotToReview> = None;
    loop {
        let ballots = store
            .ballots_to_review(after.as_ref(), PENDING_ENQUEUE_GRACE_SECS, batch_size)
            .await?;
        let Some(last) = ballots.last().cloned() else {
            break;
        };
        info!("review_cast_votes: Processing {} cast votes", ballots.len());
        let mut events: Vec<String> = ballots
            .iter()
            .map(|ballot| ballot.election_event_id.clone())
            .collect();
        events.dedup();
        let tenants = event_tenants(&events).await?;
        // For this Celery has to be properly configured with acks_late=true and a realistic value for prefetch_count, which establishes the number of tasks executed in parallel.
        for ballot in &ballots {
            let Some(tenant_id) = tenants.get(&ballot.election_event_id) else {
                warn!(
                    "Pending cast vote {} belongs to an unknown election event {}",
                    ballot.id, ballot.election_event_id
                );
                continue;
            };
            celery_app
                .send_task(
                    process_cast_vote::new(
                        tenant_id.clone(),
                        ballot.election_event_id.clone(),
                        ballot.id.clone(),
                    )
                    .with_expires_in(RECOVERY_TASK_EXPIRES_SECS),
                )
                .await
                .map_err(|e| anyhow!("Error sending cast_vote_actions task: {e:?}"))?;
        }
        after = Some(last);
    }
    Ok(())
}
