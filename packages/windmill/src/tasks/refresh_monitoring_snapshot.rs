// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The monitoring snapshot job: every beat sends a pass for each event on
//! configured dashboards; a pass that finds its event's lock held is
//! skipped, the next beat retries.

use crate::postgres::monitoring_config::EventRef;
use crate::services::celery_app::get_celery_app;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::monitoring::login_counter::prune_login_counter_receipts;
use crate::services::monitoring::snapshot::{
    prune_snapshots, refresh_event_snapshot, PassOptions, EXPORT_WINDOW,
};
use crate::services::pg_lock::PgLock;
use crate::types::error::Result;
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use chrono::{Duration, Utc};
use sequent_core::services::date::ISO8601;
use tracing::{info, instrument, warn};
use uuid::Uuid;

/// How long a delivery's receipt is kept: longer than any redelivery.
const RECEIPTS_KEPT_FOR: Duration = Duration::days(7);

fn full_pass_every() -> Duration {
    std::env::var("MONITORING_VOTER_FULL_PASS_SECONDS")
        .ok()
        .and_then(|seconds| seconds.parse().ok())
        .map(Duration::seconds)
        .unwrap_or(Duration::seconds(300))
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(expires = 30)]
pub async fn refresh_monitoring_snapshots() -> Result<()> {
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
    let celery_app = get_celery_app().await;
    for (tenant_id, election_event_id) in events {
        celery_app
            .send_task(
                refresh_monitoring_event_snapshot::new(
                    tenant_id.to_string(),
                    election_event_id.to_string(),
                )
                .with_expires_in(30),
            )
            .await?;
    }
    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 600, max_retries = 0, expires = 30)]
pub async fn refresh_monitoring_event_snapshot(
    tenant_id: String,
    election_event_id: String,
) -> Result<()> {
    let event = EventRef {
        tenant_id: Uuid::parse_str(&tenant_id).context("Invalid tenant id")?,
        election_event_id: Uuid::parse_str(&election_event_id).context("Invalid event id")?,
    };
    let lock = match PgLock::acquire(
        format!("monitoring_snapshot-{tenant_id}-{election_event_id}"),
        Uuid::new_v4().to_string(),
        ISO8601::now() + Duration::seconds(600),
    )
    .await
    {
        Ok(lock) => lock,
        Err(error) => {
            info!("Another pass of the event is running: {error}");
            return Ok(());
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
