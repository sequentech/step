// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The monitoring snapshot job: every beat sends a pass for each event on
//! configured dashboards; a pass that finds its event's lock held is
//! skipped, the next beat retries. An event on any other dashboard gets no
//! pass, so beat prunes the runs it still keeps itself.
//!
//! Beat ticks once per snapshot interval
//! ([`sequent_core::monitoring::cadence`]). Every message the job sends
//! expires after one interval: by then the next tick has asked again, so an
//! older request would only count twice what one pass counts.

use crate::postgres::lock::LOCK_HELD;
use crate::postgres::monitoring_config::EventRef;
use crate::services::celery_app::get_celery_app;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::monitoring::cadence;
use crate::services::monitoring::login_counter::prune_login_counter_receipts;
use crate::services::monitoring::snapshot::{
    prune_snapshots, refresh_event_snapshot, PassOptions, EXPORT_WINDOW,
};
use crate::services::pg_lock::PgLock;
use crate::types::error::Result;
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use celery::task::Signature;
use chrono::{Duration, Utc};
use deadpool_postgres::Client;
use sequent_core::monitoring::cadence::{
    Cadence, MAX_SNAPSHOT_INTERVAL_SECONDS, MIN_SNAPSHOT_INTERVAL_SECONDS,
};
use sequent_core::services::date::ISO8601;
use tracing::{info, instrument, warn};
use uuid::Uuid;

/// How long a delivery's receipt is kept: longer than any redelivery.
const RECEIPTS_KEPT_FOR: Duration = Duration::days(7);

fn full_pass_every() -> Duration {
    let seconds = cadence::configured().voter_full_pass.seconds;
    Duration::seconds(i64::try_from(seconds).unwrap_or(i64::MAX))
}

/// Seconds as a message's expiry takes them.
fn expiry(interval_seconds: u64) -> u32 {
    u32::try_from(interval_seconds).unwrap_or(u32::MAX)
}

/// The fan-out beat sends every tick of `cadence`: it carries the interval,
/// so the passes it sends expire with it.
pub fn scheduled_fan_out(cadence: &Cadence) -> Signature<refresh_monitoring_snapshots> {
    let interval = cadence.snapshot_interval.seconds;
    refresh_monitoring_snapshots::new(Some(interval)).with_expires_in(expiry(interval))
}

/// The interval a fan-out works with: the one beat sent, within bounds, or
/// this worker's own when the message has none.
fn fan_out_interval(sent: Option<u64>) -> u64 {
    match sent {
        Some(seconds) => {
            seconds.clamp(MIN_SNAPSHOT_INTERVAL_SECONDS, MAX_SNAPSHOT_INTERVAL_SECONDS)
        }
        None => cadence::configured().snapshot_interval.seconds,
    }
}

/// One event's pass, expiring after one interval.
fn event_pass(
    tenant_id: Uuid,
    election_event_id: Uuid,
    interval_seconds: u64,
) -> Signature<refresh_monitoring_event_snapshot> {
    refresh_monitoring_event_snapshot::new(tenant_id.to_string(), election_event_id.to_string())
        .with_expires_in(expiry(interval_seconds))
}

// No `expires` here or on the pass: a task-level expiry and a message's own
// `with_expires_in` together send no expiry at all, so each message sets it.
#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn refresh_monitoring_snapshots(interval_seconds: Option<u64>) -> Result<()> {
    let interval = fan_out_interval(interval_seconds);
    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|error| anyhow!("Error getting hasura client: {error}"))?;
    let events: Vec<(Uuid, Uuid)> = client
        .query(
            "SELECT tenant_id, election_event_id FROM sequent_backend.monitoring_event
             WHERE dashboard_mode = 'CONFIGURED'",
            &[],
        )
        .await
        .context("Failed to list the configured events")?
        .iter()
        .map(|row| (row.get(0), row.get(1)))
        .collect();
    {
        let transaction = client.transaction().await?;
        prune_login_counter_receipts(&transaction, RECEIPTS_KEPT_FOR).await?;
        transaction.commit().await?;
    }
    prune_events_without_passes(&mut client).await;
    let celery_app = get_celery_app().await;
    for (tenant_id, election_event_id) in events {
        celery_app
            .send_task(event_pass(tenant_id, election_event_id, interval))
            .await?;
    }
    Ok(())
}

/// Events not on configured dashboards that keep a run besides the one
/// shown. Passes prune an event on configured dashboards; one switched
/// away gets none, so these are pruned by the fan-out.
pub async fn events_to_prune(client: &Client) -> Result<Vec<EventRef>> {
    Ok(client
        .query(
            "SELECT e.tenant_id, e.election_event_id FROM sequent_backend.monitoring_event e
             WHERE e.dashboard_mode <> 'CONFIGURED'
               AND EXISTS (
                   SELECT 1 FROM sequent_backend.monitoring_snapshot_run r
                   LEFT JOIN sequent_backend.monitoring_snapshot_state s
                     ON s.tenant_id = r.tenant_id AND s.election_event_id = r.election_event_id
                   WHERE r.tenant_id = e.tenant_id AND r.election_event_id = e.election_event_id
                     AND r.revision IS DISTINCT FROM s.live_snapshot_revision
               )",
            &[],
        )
        .await
        .context("Failed to list the events to prune")?
        .iter()
        .map(|row| EventRef {
            tenant_id: row.get(0),
            election_event_id: row.get(1),
        })
        .collect())
}

fn snapshot_lock_key(event: EventRef) -> String {
    format!(
        "monitoring_snapshot-{}-{}",
        event.tenant_id, event.election_event_id
    )
}

/// Prunes each of [`events_to_prune`] under its snapshot lock, keeping
/// the run shown and those an export may still name. An event whose lock
/// is held, or whose pruning fails, is left for the next beat.
async fn prune_events_without_passes(client: &mut Client) {
    let events = match events_to_prune(client).await {
        Ok(events) => events,
        Err(error) => {
            warn!("Listing the monitoring events to prune failed: {error:#}");
            return;
        }
    };
    for event in events {
        let lock = match PgLock::acquire(
            snapshot_lock_key(event),
            Uuid::new_v4().to_string(),
            ISO8601::now() + Duration::seconds(600),
        )
        .await
        {
            Ok(lock) => lock,
            Err(error) => {
                if error.to_string() != LOCK_HELD {
                    warn!(
                        ?event,
                        "Taking the snapshot lock to prune failed: {error:#}"
                    );
                }
                continue;
            }
        };
        match prune_snapshots(client, event, EXPORT_WINDOW).await {
            Ok(pruned) => info!(
                ?event,
                ?pruned,
                "Pruned the snapshots of an event without passes"
            ),
            Err(error) => warn!(?event, "Pruning monitoring snapshots failed: {error:#}"),
        }
        if let Err(error) = lock.release().await {
            warn!(?event, "Releasing the snapshot lock failed: {error:#}");
        }
    }
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 600, max_retries = 0)]
pub async fn refresh_monitoring_event_snapshot(
    tenant_id: String,
    election_event_id: String,
) -> Result<()> {
    let event = EventRef {
        tenant_id: Uuid::parse_str(&tenant_id).context("Invalid tenant id")?,
        election_event_id: Uuid::parse_str(&election_event_id).context("Invalid event id")?,
    };
    let lock = match PgLock::acquire(
        snapshot_lock_key(event),
        Uuid::new_v4().to_string(),
        ISO8601::now() + Duration::seconds(600),
    )
    .await
    {
        Ok(lock) => lock,
        Err(error) if error.to_string() == LOCK_HELD => {
            info!("Another pass of the event is running");
            return Ok(());
        }
        Err(error) => {
            return Err(error
                .context("Failed to take the event's snapshot lock")
                .into())
        }
    };
    let result = pass(event).await;
    lock.release().await?;
    result
}

async fn pass(event: EventRef) -> Result<()> {
    let voter_group = std::env::var("KEYCLOAK_VOTER_GROUP_NAME")
        .map_err(|error| anyhow!("Missing KEYCLOAK_VOTER_GROUP_NAME: {error}"))?;
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
    let outcome = refresh_event_snapshot(
        &mut hasura,
        &mut keycloak,
        event,
        PassOptions {
            voter_group: &voter_group,
            full_pass_every: full_pass_every(),
            now: Utc::now(),
        },
    )
    .await?;
    info!(?outcome, "Monitoring snapshot pass");
    if let Err(error) = prune_snapshots(&mut hasura, event, EXPORT_WINDOW).await {
        warn!("Pruning monitoring snapshots failed, the next pass retries: {error:#}");
    }
    Ok(())
}

#[cfg(test)]
#[path = "refresh_monitoring_snapshot_tests.rs"]
mod refresh_monitoring_snapshot_tests;
