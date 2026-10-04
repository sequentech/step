// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Scheduled lifecycle windows (VOTE-LIFECYCLE): a readiness test, final
//! testing or test voting window opens or closes for one election at the
//! Post's local time. The scheduler only records the window's state in
//! `election.status.lifecycle_windows` and logs it; what a window enables
//! belongs to the readiness and test voting features.

use crate::postgres::election::{get_election_by_id, update_election_voting_status};
use crate::postgres::scheduled_event::*;
use crate::services::pg_lock::PgLock;
use crate::services::providers::transactions_provider::provide_hasura_transaction;
use crate::services::signing::actions::voting::SCHEDULER;
use crate::services::signing::log::{stage, Actor, LogScope, LogStep, SystemOutcome};
use crate::tasks::scheduled_events::fires_later;
use crate::types::error::Result;
use anyhow::{anyhow, Context, Result as AnyhowResult};
use celery::error::TaskError;
use chrono::{Duration, Utc};
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::SigningStatementKind;
use sequent_core::ballot::{ElectionStatus, LifecycleWindow, LifecycleWindowState};
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::services::date::ISO8601;
use sequent_core::types::scheduled_event::*;
use serde_json::json;
use std::collections::BTreeMap;
use tracing::{info, instrument, warn};
use uuid::Uuid;

/// The window a processor switches, and to what.
pub fn lifecycle_window(
    processor: &EventProcessors,
) -> Option<(LifecycleWindow, LifecycleWindowState)> {
    use LifecycleWindow::*;
    use LifecycleWindowState::*;
    match processor {
        EventProcessors::START_READINESS_TEST => Some((READINESS_TEST, OPEN)),
        EventProcessors::END_READINESS_TEST => Some((READINESS_TEST, CLOSED)),
        EventProcessors::START_FINAL_TESTING => Some((FINAL_TESTING, OPEN)),
        EventProcessors::END_FINAL_TESTING => Some((FINAL_TESTING, CLOSED)),
        EventProcessors::START_TEST_VOTING => Some((TEST_VOTING, OPEN)),
        EventProcessors::END_TEST_VOTING => Some((TEST_VOTING, CLOSED)),
        _ => None,
    }
}

/// The electoral log's sentence for a window change.
pub fn window_description(window: LifecycleWindow, state: LifecycleWindowState) -> String {
    let name = match window {
        LifecycleWindow::READINESS_TEST => "election readiness test",
        LifecycleWindow::FINAL_TESTING => "final testing and lockdown",
        LifecycleWindow::TEST_VOTING => "test voting",
    };
    let verb = match state {
        LifecycleWindowState::OPEN => "Opened",
        LifecycleWindowState::CLOSED => "Closed",
    };
    format!("{verb} the {name} window on schedule")
}

/// Sets the window in `status`; answers the state it had before.
pub fn set_window(
    status: &mut ElectionStatus,
    window: LifecycleWindow,
    state: LifecycleWindowState,
) -> Option<LifecycleWindowState> {
    status
        .lifecycle_windows
        .get_or_insert_with(BTreeMap::new)
        .insert(window, state)
}

#[instrument(skip(hasura_transaction), err)]
pub async fn manage_election_lifecycle_window_wrapped(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    scheduled_event_id: &str,
    election_id: Option<&str>,
) -> AnyhowResult<()> {
    let Some(scheduled_event) = find_scheduled_event_by_id(
        hasura_transaction,
        Some(tenant_id.to_owned()),
        Some(election_event_id.to_owned()),
        scheduled_event_id,
    )
    .await
    .with_context(|| "Error obtaining scheduled event by id")?
    else {
        return Err(anyhow!(
            "Can't find scheduled event with id: {scheduled_event_id}"
        ));
    };
    if fires_later(&scheduled_event, Utc::now()) {
        info!("Scheduled event {scheduled_event_id} was moved to a later time; it runs then");
        return Ok(());
    }
    let window = scheduled_event
        .event_processor
        .as_ref()
        .and_then(lifecycle_window);
    let (Some((window, state)), Some(election_id)) = (window, election_id) else {
        // Saving refuses these; one written another way is reported and
        // marked done so it doesn't run again.
        warn!(
            "Scheduled event {scheduled_event_id} ({:?}) isn't a lifecycle window of an election",
            scheduled_event.event_processor
        );
        stop_scheduled_event(hasura_transaction, tenant_id, scheduled_event_id).await?;
        return Ok(());
    };
    // The election's status is read and written below.
    lock_elections(
        hasura_transaction,
        tenant_id,
        election_event_id,
        Some(election_id),
    )
    .await?;
    let Some(election) = get_election_by_id(
        hasura_transaction,
        tenant_id,
        election_event_id,
        election_id,
    )
    .await
    .with_context(|| "Error obtaining election by id")?
    else {
        return Err(anyhow!("Election not found"));
    };
    let mut status: ElectionStatus = match election.status.clone() {
        Some(value) => deserialize_value(value)?,
        None => ElectionStatus::default(),
    };
    let previous = set_window(&mut status, window, state);
    if previous != Some(state) {
        update_election_voting_status(
            hasura_transaction,
            tenant_id,
            election_event_id,
            election_id,
            serde_json::to_value(&status)?,
        )
        .await?;
        let cron = scheduled_event.cron_config.clone().unwrap_or_default();
        stage(
            hasura_transaction,
            &LogStep {
                kind: SigningStatementKind::LifecycleWindowChanged,
                user: Actor {
                    user_id: SCHEDULER.to_owned(),
                    username: SCHEDULER.to_owned(),
                },
                system: SystemOutcome::Info,
                scope: LogScope {
                    tenant_id: Uuid::parse_str(tenant_id)?,
                    election_event_id: Uuid::parse_str(election_event_id)?,
                    election_id: Some(Uuid::parse_str(election_id)?),
                    area_id: None,
                },
                description: window_description(window, state),
                details: json!({
                    "window": window,
                    "state": state,
                    "previous": previous,
                    "election_id": election_id,
                    "scheduled_event_id": scheduled_event_id,
                    "event_processor": scheduled_event.event_processor,
                    "scheduled_date": cron.scheduled_date,
                    "local": cron.local,
                    "timezone": cron.timezone,
                }),
            },
        )
        .await?;
    } else {
        info!("The {window} window of election {election_id} is already {state}");
    }
    stop_scheduled_event(hasura_transaction, tenant_id, scheduled_event_id)
        .await
        .with_context(|| "Error stopping scheduled event")?;
    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 10, max_retries = 0, expires = 30)]
pub async fn manage_election_lifecycle_window(
    tenant_id: String,
    election_event_id: String,
    scheduled_event_id: String,
    election_id: Option<String>,
) -> Result<()> {
    let lock: PgLock = PgLock::acquire(
        format!(
            "execute_manage_election_lifecycle_window-{}-{}-{}",
            tenant_id, election_event_id, scheduled_event_id
        ),
        Uuid::new_v4().to_string(),
        ISO8601::now() + Duration::seconds(120),
    )
    .await
    .with_context(|| "Error acquiring pglock")?;

    let res = provide_hasura_transaction(|hasura_transaction| {
        let tenant_id = tenant_id.clone();
        let election_event_id = election_event_id.clone();
        let scheduled_event_id = scheduled_event_id.clone();
        let election_id = election_id.clone();
        Box::pin(async move {
            manage_election_lifecycle_window_wrapped(
                hasura_transaction,
                &tenant_id,
                &election_event_id,
                &scheduled_event_id,
                election_id.as_deref(),
            )
            .await
        })
    })
    .await;

    info!("result: {:?}", res);

    lock.release()
        .await
        .with_context(|| "Error releasing pglock")?;

    Ok(res?)
}

#[cfg(test)]
mod lifecycle_window_tests {
    use super::*;

    #[test]
    fn each_window_processor_opens_or_closes_its_window() {
        use LifecycleWindow::*;
        use LifecycleWindowState::*;
        for (processor, expected) in [
            (
                EventProcessors::START_READINESS_TEST,
                (READINESS_TEST, OPEN),
            ),
            (
                EventProcessors::END_READINESS_TEST,
                (READINESS_TEST, CLOSED),
            ),
            (EventProcessors::START_FINAL_TESTING, (FINAL_TESTING, OPEN)),
            (EventProcessors::END_FINAL_TESTING, (FINAL_TESTING, CLOSED)),
            (EventProcessors::START_TEST_VOTING, (TEST_VOTING, OPEN)),
            (EventProcessors::END_TEST_VOTING, (TEST_VOTING, CLOSED)),
        ] {
            assert_eq!(lifecycle_window(&processor), Some(expected));
        }
        assert_eq!(
            lifecycle_window(&EventProcessors::START_VOTING_PERIOD),
            None
        );
        assert_eq!(
            window_description(TEST_VOTING, OPEN),
            "Opened the test voting window on schedule"
        );
    }

    #[test]
    fn the_window_state_is_kept_in_the_status_and_survives_other_status_writes() {
        let mut status = ElectionStatus::default();
        assert_eq!(
            set_window(
                &mut status,
                LifecycleWindow::FINAL_TESTING,
                LifecycleWindowState::OPEN
            ),
            None
        );
        let value = serde_json::to_value(&status).unwrap();
        assert_eq!(value["lifecycle_windows"], json!({"FINAL_TESTING": "OPEN"}));
        // Another writer reads and writes the whole status.
        let mut reread: ElectionStatus = serde_json::from_value(value).unwrap();
        reread.allow_tally = sequent_core::ballot::AllowTallyStatus::ALLOWED;
        assert_eq!(
            serde_json::to_value(&reread).unwrap()["lifecycle_windows"],
            json!({"FINAL_TESTING": "OPEN"})
        );
        assert_eq!(
            set_window(
                &mut reread,
                LifecycleWindow::FINAL_TESTING,
                LifecycleWindowState::CLOSED
            ),
            Some(LifecycleWindowState::OPEN)
        );
        // Statuses written before the field existed read as no windows, and
        // are written back unchanged.
        let legacy: ElectionStatus =
            serde_json::from_value(json!({"voting_status": "OPEN"})).unwrap();
        assert_eq!(legacy.lifecycle_windows, None);
        assert!(serde_json::to_value(&legacy)
            .unwrap()
            .get("lifecycle_windows")
            .is_none());
    }
}
