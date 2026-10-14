// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Voter counts by channel, per event (`messaging_voter_count`), for the
//! event's Messaging tab and Send Notification. Counting a realm takes time,
//! so it runs here, never in a request:
//!
//! - on demand: the Recount button and every save of the event's messaging
//!   configuration ask for a recount (`request_recount`), then send
//!   [`recount_messaging_voters`];
//! - from beat: [`recount_messaging_voters_due`] asks for the events with a
//!   messaging configuration counted longest ago, a few per pass, each sent
//!   [`RECOUNT_STAGGER_SECONDS`] after the previous one.

use crate::postgres::messaging_insights::{events_due_for_recount, request_recount};
use crate::services::celery_app::get_celery_app;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::messaging::insights::{
    recount_event_voters, recount_timeout, RECOUNT_PENDING_FOR,
};
use crate::types::error::Result;
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use chrono::{Duration, Utc};
use tracing::{info, instrument};
use uuid::Uuid;

/// Seconds between two beat passes.
pub const RECOUNT_PASS_INTERVAL_SECONDS: u64 = 3600;
/// An event is recounted from beat when its last count is older than this.
pub const RECOUNT_EVERY: Duration = Duration::hours(6);
/// Events a beat pass asks to recount, at most.
pub const RECOUNTS_PER_PASS: i64 = 12;
/// Seconds between two recounts a beat pass sends, so the Keycloak
/// database counts one event at a time.
pub const RECOUNT_STAGGER_SECONDS: u32 = 120;

/// Recounts one event's voters by channel.
#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 900, max_retries = 0)]
pub async fn recount_messaging_voters(tenant_id: String, election_event_id: String) -> Result<()> {
    let tenant_id = Uuid::parse_str(&tenant_id).context("Invalid tenant id")?;
    let election_event_id = Uuid::parse_str(&election_event_id).context("Invalid event id")?;
    let mut hasura = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|error| anyhow!("Error getting hasura client: {error}"))?;
    let mut keycloak = get_keycloak_pool()
        .await
        .get()
        .await
        .map_err(|error| anyhow!("Error getting keycloak client: {error}"))?;
    let outcome = recount_event_voters(
        &mut hasura,
        &mut keycloak,
        &tenant_id,
        &election_event_id,
        recount_timeout(),
    )
    .await?;
    info!(%outcome, "Recounted the event's voters by channel");
    Ok(())
}

/// Beat's pass: asks for the recounts due, staggered.
#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn recount_messaging_voters_due() -> Result<()> {
    let mut hasura = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|error| anyhow!("Error getting hasura client: {error}"))?;
    let now = Utc::now();
    let tx = hasura.transaction().await?;
    let due = events_due_for_recount(
        &tx,
        now - RECOUNT_EVERY,
        now - RECOUNT_PENDING_FOR,
        RECOUNTS_PER_PASS,
    )
    .await?;
    let mut asked = vec![];
    for (tenant_id, election_event_id) in due {
        if request_recount(
            &tx,
            &tenant_id,
            &election_event_id,
            now,
            RECOUNT_PENDING_FOR,
        )
        .await?
        {
            asked.push((tenant_id, election_event_id));
        }
    }
    tx.commit().await?;
    let celery_app = get_celery_app().await;
    for (position, (tenant_id, election_event_id)) in asked.iter().enumerate() {
        let countdown = RECOUNT_STAGGER_SECONDS.saturating_mul(position as u32);
        celery_app
            .send_task(
                recount_messaging_voters::new(tenant_id.to_string(), election_event_id.to_string())
                    .with_countdown(countdown),
            )
            .await?;
    }
    info!(events = asked.len(), "Asked for the voter recounts due");
    Ok(())
}
