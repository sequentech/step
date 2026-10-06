// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election_event::{
    get_election_event_by_id, update_election_event_presentation,
};
use crate::postgres::scheduled_event::*;
use crate::postgres::trusted_write;
use crate::services::database::get_hasura_pool;
use crate::services::pg_lock::PgLock;
use crate::services::providers::transactions_provider::provide_hasura_transaction;
use crate::services::signing::actions::voting::SCHEDULER;
use crate::services::signing::log::{stage, Actor, LogScope, LogStep, SystemOutcome};
use crate::services::voting_status::{self};
use crate::tasks::signing_log_outbox::kick_signing_log_outbox;
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context, Result as AnyhowResult};
use async_trait::async_trait;
use celery::error::TaskError;
use chrono::Duration;
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::SigningStatementKind;
use sequent_core::ballot::{ElectionEventPresentation, InitReport, LockedDown, VotingStatus};
use sequent_core::serialization::deserialize_with_path::{self, deserialize_value};
use sequent_core::services::date::ISO8601;
use sequent_core::types::scheduled_event::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::instrument;
use tracing::{error, event, info, Level};
use uuid::Uuid;

/// The lockdown entry's description.
pub fn lockdown_description(locked_down: bool, previous: bool) -> String {
    match (locked_down, previous) {
        (true, false) => "Locked down the election event on schedule.",
        (true, true) => "Locked down the election event on schedule (it was already locked down).",
        (false, true) => "Lifted the lockdown of the election event on schedule.",
        (false, false) => {
            "Lifted the lockdown of the election event on schedule (it wasn't locked down)."
        }
    }
    .to_string()
}

#[instrument(err)]
pub async fn manage_election_event_lockdown_wrapped(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    scheduled_event_id: String,
) -> AnyhowResult<()> {
    // Re-read only after the editor's transaction has released the schedule.
    lock_scheduled_event(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &scheduled_event_id,
    )
    .await?;
    if get_election_event_by_id(hasura_transaction, &tenant_id, &election_event_id)
        .await?
        .is_archived
    {
        info!("Skipping scheduled transition {scheduled_event_id}: the event is archived");
        return Ok(());
    }

    let scheduled_event = find_scheduled_event_by_id(
        hasura_transaction,
        Some(tenant_id.clone()),
        Some(election_event_id.clone()),
        &scheduled_event_id,
    )
    .await
    .with_context(|| "Error obtaining scheduled event by id")?;

    let Some(scheduled_event) = scheduled_event else {
        return Err(anyhow!(
            "Can't find scheduled event with id: {}",
            scheduled_event_id
        ));
    };
    // Queued before the row moved to a later time: it runs then.
    if crate::tasks::scheduled_events::fires_later(&scheduled_event, chrono::Utc::now()) {
        info!("Scheduled event {scheduled_event_id} was moved to a later time; it runs then");
        return Ok(());
    }

    let locked_down =
        scheduled_event.event_processor == Some(EventProcessors::START_LOCKDOWN_PERIOD);

    let election_event =
        get_election_event_by_id(hasura_transaction, &tenant_id, &election_event_id).await?;

    // An event without a presentation is not locked down.
    let presentation: ElectionEventPresentation = match election_event.presentation {
        Some(presentation) => deserialize_with_path::deserialize_value(presentation)?,
        None => ElectionEventPresentation::default(),
    };
    let previous = presentation.locked_down == Some(LockedDown::LOCKED_DOWN);
    let presentation = ElectionEventPresentation {
        locked_down: Some(if locked_down {
            LockedDown::LOCKED_DOWN
        } else {
            LockedDown::NOT_LOCKED_DOWN
        }),
        ..presentation
    };
    // The lockdown changes only here (START/END_LOCKDOWN_PERIOD).
    trusted_write(hasura_transaction).await?;
    update_election_event_presentation(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        serde_json::to_value(presentation)?,
    )
    .await?;
    stage(
        hasura_transaction,
        &LogStep {
            kind: SigningStatementKind::LockdownChanged,
            user: Actor {
                user_id: SCHEDULER.to_owned(),
                username: SCHEDULER.to_owned(),
            },
            system: SystemOutcome::Info,
            scope: LogScope {
                tenant_id: Uuid::parse_str(&tenant_id)?,
                election_event_id: Uuid::parse_str(&election_event_id)?,
                election_id: None,
                area_id: None,
            },
            description: lockdown_description(locked_down, previous),
            details: json!({
                "locked_down": locked_down,
                "previous": previous,
                "source": "scheduled",
                "scheduled_event_id": scheduled_event.id,
            }),
        },
    )
    .await?;

    stop_scheduled_event(&hasura_transaction, &tenant_id, &scheduled_event.id)
        .await
        .with_context(|| "Error stopping scheduled event")?;

    // Locking down or lifting it is a write that the scheduled outcomes are
    // recomputed after (VOTE-LIFECYCLE §5c); it changes none today.
    crate::services::scheduled_outcome::recompute_predictions(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &crate::services::signing::log::Actor {
            user_id: crate::services::signing::actions::voting::SCHEDULER.to_owned(),
            username: crate::services::signing::actions::voting::SCHEDULER.to_owned(),
        },
    )
    .await?;

    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 10, max_retries = 0, expires = 30)]
pub async fn manage_election_event_lockdown(
    tenant_id: String,
    election_event_id: String,
    scheduled_event_id: String,
) -> Result<()> {
    let res = provide_hasura_transaction(|hasura_transaction| {
        let tenant_id = tenant_id.clone();
        let election_event_id = election_event_id.clone();
        let scheduled_event_id = scheduled_event_id.clone();
        Box::pin(async move {
            // Your async code here
            manage_election_event_lockdown_wrapped(
                hasura_transaction,
                tenant_id,
                election_event_id,
                scheduled_event_id,
            )
            .await
        })
    })
    .await;

    info!("result: {:?}", res);
    if res.is_ok() {
        kick_signing_log_outbox();
    }

    Ok(res?)
}
