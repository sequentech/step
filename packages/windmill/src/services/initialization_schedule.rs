// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Scheduled openings and initialization (VOTE-LIFECYCLE design §9, §5c).
//!
//! - A Post's scheduled opening waits while the Post isn't initialized at
//!   the event's scope: the row stays active and opens the Post at the first
//!   run after it is. Once its voting period has closed it doesn't open it:
//!   the row stops.
//! - An event-wide scheduled opening opens the Posts that can, and keeps
//!   waiting for the others (only those: Posts it opened, then paused, are
//!   not reopened).
//! - Each wait and each refusal is logged once per Post and reason: one
//!   `SigningActionExecuted` step by the scheduler, with its explanation.

use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::election_initialization::set_scheduled_event_annotation;
use crate::services::election_event_status::{
    update_scheduled_event_voting_status_for, TransitionRefusal,
};
use crate::services::initialization_scope::post_display_name;
use crate::services::scheduled_outcome::{transition_of, EventState, ScheduledRow};
use crate::services::signing::log::{stage, Actor, LogScope, LogStep, SystemOutcome};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::SigningStatementKind;
use sequent_core::ballot::{VotingStatus, VotingStatusChannel};
use sequent_core::types::hasura::core::Election;
use sequent_core::types::scheduled_event::ScheduledEvent;
use serde_json::{json, Map, Value};
use std::collections::HashSet;
use tracing::{info, instrument, warn};
use uuid::Uuid;

/// The scheduler's name in the logs.
pub const SCHEDULER_ACTOR: &str = "scheduled-event";
/// Annotation: the reason each Post waits, by election id, as last logged.
pub const WAIT_ANNOTATION: &str = "initialization_wait";
/// Annotation: the Posts an event-wide opening still waits for.
pub const PENDING_ANNOTATION: &str = "initialization_pending";

/// What a Post's scheduled opening does now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduledPostOpening {
    /// Open the Post.
    Open,
    /// Don't open it yet; keep the row active.
    Wait,
    /// Don't open it: its voting period has closed. Stop the row.
    AfterClose,
}

/// The instant the Post's voting closes: its own scheduled close, else the
/// event-wide one.
pub fn effective_close(
    closes: &[(Option<String>, String)],
    election_id: &str,
) -> Option<DateTime<Utc>> {
    let instant = |value: &String| {
        DateTime::parse_from_rfc3339(value)
            .map(|instant| instant.with_timezone(&Utc))
            .ok()
    };
    let own = closes
        .iter()
        .filter(|(post, _)| post.as_deref() == Some(election_id))
        .filter_map(|(_, date)| instant(date))
        .min();
    own.or_else(|| {
        closes
            .iter()
            .filter(|(post, _)| post.is_none())
            .filter_map(|(_, date)| instant(date))
            .min()
    })
}

fn annotation_map(scheduled_event: &ScheduledEvent, key: &str) -> Map<String, Value> {
    scheduled_event
        .annotations
        .as_ref()
        .and_then(|annotations| annotations.get(key))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

/// The sentence a wait or a refusal is logged with.
pub fn wait_description(post: &str, reason: &WaitReason) -> String {
    match reason {
        WaitReason::Initialization(refusal) => format!(
            "Scheduled opening of Post {post} waits for its initialization: {} The scheduler opens it at its first run after that.",
            refusal.message(post, &VotingStatus::OPEN, &VotingStatus::NOT_STARTED)
        ),
        WaitReason::AfterClose(close) => format!(
            "Scheduled opening of Post {post} not run: its voting period closed at {}.",
            close.to_rfc3339()
        ),
    }
}

/// Why a scheduled opening doesn't open a Post now.
#[derive(Debug, Clone, PartialEq)]
pub enum WaitReason {
    Initialization(TransitionRefusal),
    AfterClose(DateTime<Utc>),
}

impl WaitReason {
    pub fn code(&self) -> String {
        match self {
            WaitReason::Initialization(refusal) => refusal.code().to_string(),
            WaitReason::AfterClose(_) => "after-close".to_string(),
        }
    }
}

/// Logs that `scheduled_event` doesn't open `election` now, unless the same
/// reason was logged last time.
async fn log_wait_once(
    hasura_transaction: &Transaction<'_>,
    scheduled_event: &ScheduledEvent,
    waits: &mut Map<String, Value>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    election_id: &str,
    post: &str,
    reason: &WaitReason,
) -> Result<()> {
    let code = reason.code();
    if waits.get(election_id).and_then(Value::as_str) == Some(code.as_str()) {
        return Ok(());
    }
    let description = wait_description(post, reason);
    warn!("{description}");
    let mut details = json!({
        "action": "open-voting",
        "election_id": election_id,
        "election_name": post,
        "scheduled_event_id": scheduled_event.id,
        "outcome": match reason {
            WaitReason::Initialization(_) => "waiting-for-initialization",
            WaitReason::AfterClose(_) => "refused-after-close",
        },
        "reason": code,
    });
    if let WaitReason::AfterClose(close) = reason {
        details["closes_at"] = json!(close.to_rfc3339());
    }
    stage(
        hasura_transaction,
        &LogStep {
            kind: SigningStatementKind::SigningActionExecuted,
            user: Actor {
                user_id: SCHEDULER_ACTOR.to_string(),
                username: SCHEDULER_ACTOR.to_string(),
            },
            system: SystemOutcome::Error,
            scope: LogScope {
                tenant_id,
                election_event_id,
                election_id: Uuid::parse_str(election_id).ok(),
                area_id: None,
            },
            description,
            details,
        },
    )
    .await?;
    waits.insert(election_id.to_string(), Value::String(code));
    Ok(())
}

/// What `scheduled_event`, a scheduled opening of `election`, does now
/// (see the module docs). Logs a wait or a refusal once per reason.
#[instrument(skip(hasura_transaction, election, scheduled_event), err)]
pub async fn scheduled_post_opening(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election: &Election,
    scheduled_event: &ScheduledEvent,
) -> Result<ScheduledPostOpening> {
    let tenant = Uuid::parse_str(tenant_id)?;
    let event = Uuid::parse_str(election_event_id)?;
    let scheduled_event_id = Uuid::parse_str(&scheduled_event.id)?;
    let election_event =
        get_election_event_by_id(hasura_transaction, tenant_id, election_event_id).await?;
    let post = post_display_name(election, &election_event.get_default_language());
    let state = EventState::read(hasura_transaction, tenant, event).await?;
    let cron_config = scheduled_event
        .cron_config
        .as_ref()
        .map(serde_json::to_value)
        .transpose()?;
    let row = transition_of(
        &scheduled_event.id,
        "START_VOTING_PERIOD",
        cron_config.as_ref(),
        scheduled_event.event_payload.as_ref(),
    )
    .context("Scheduled opening isn't a voting transition")?;
    let reason = state.opening_wait_reason(
        &ScheduledRow {
            transition: row,
            annotations: scheduled_event
                .annotations
                .clone()
                .unwrap_or_else(|| json!({})),
            written_now: false,
        },
        Uuid::parse_str(&election.id)?,
    );
    let Some(reason) = reason else {
        return Ok(ScheduledPostOpening::Open);
    };
    let mut waits = annotation_map(scheduled_event, WAIT_ANNOTATION);
    log_wait_once(
        hasura_transaction,
        scheduled_event,
        &mut waits,
        tenant,
        event,
        &election.id,
        &post,
        &reason,
    )
    .await?;
    set_scheduled_event_annotation(
        hasura_transaction,
        scheduled_event_id,
        WAIT_ANNOTATION,
        &Value::Object(waits),
    )
    .await?;
    Ok(match reason {
        WaitReason::AfterClose(_) => ScheduledPostOpening::AfterClose,
        WaitReason::Initialization(_) => ScheduledPostOpening::Wait,
    })
}

/// Runs `scheduled_event`, an event-wide scheduled opening: opens every
/// Post that can open (on its first run) or that it still waits for (on
/// later runs), and logs once why each other Post waits. Returns whether it
/// is done (no Post waits), so the caller stops the row.
#[instrument(skip(hasura_transaction, scheduled_event), err)]
pub async fn open_event_on_schedule(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    scheduled_event: &ScheduledEvent,
    channels: &Option<Vec<VotingStatusChannel>>,
    // Posts with a scheduled row of their own: that row decides for them.
    own_rows: &HashSet<String>,
) -> Result<bool> {
    let tenant = Uuid::parse_str(tenant_id)?;
    let event = Uuid::parse_str(election_event_id)?;
    let scheduled_event_id = Uuid::parse_str(&scheduled_event.id)?;
    let pending: Option<HashSet<String>> = scheduled_event
        .annotations
        .as_ref()
        .and_then(|annotations| annotations.get(PENDING_ANNOTATION))
        .and_then(|value| serde_json::from_value(value.clone()).ok());
    let state = EventState::read(hasura_transaction, tenant, event).await?;
    let cron_config = scheduled_event
        .cron_config
        .as_ref()
        .map(serde_json::to_value)
        .transpose()?;
    let transition = transition_of(
        &scheduled_event.id,
        "START_VOTING_PERIOD",
        cron_config.as_ref(),
        scheduled_event.event_payload.as_ref(),
    )
    .context("Scheduled opening isn't a voting transition")?;
    let row = ScheduledRow {
        transition,
        annotations: scheduled_event
            .annotations
            .clone()
            .unwrap_or_else(|| json!({})),
        written_now: false,
    };
    let mut excluded = own_rows.clone();
    let mut waits = annotation_map(scheduled_event, WAIT_ANNOTATION);
    for post in state.posts_of(&row) {
        let id = post.to_string();
        if excluded.contains(&id)
            || pending
                .as_ref()
                .is_some_and(|pending| !pending.contains(&id))
        {
            continue;
        }
        if let Some(reason @ WaitReason::AfterClose(_)) = state.opening_wait_reason(&row, post) {
            log_wait_once(
                hasura_transaction,
                scheduled_event,
                &mut waits,
                tenant,
                event,
                &id,
                &state.post_name(Some(post)),
                &reason,
            )
            .await?;
            excluded.insert(id);
        }
    }
    let skipped = update_scheduled_event_voting_status_for(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &VotingStatus::OPEN,
        channels,
        &excluded,
        pending.as_ref(),
    )
    .await?;
    for (election_id, (post, refusal)) in &skipped {
        log_wait_once(
            hasura_transaction,
            scheduled_event,
            &mut waits,
            tenant,
            event,
            election_id,
            post,
            &WaitReason::Initialization(refusal.clone()),
        )
        .await?;
    }
    set_scheduled_event_annotation(
        hasura_transaction,
        scheduled_event_id,
        WAIT_ANNOTATION,
        &Value::Object(waits),
    )
    .await?;
    let waiting: Vec<&String> = skipped.keys().collect();
    set_scheduled_event_annotation(
        hasura_transaction,
        scheduled_event_id,
        PENDING_ANNOTATION,
        &json!(waiting),
    )
    .await
    .context("Error recording the Posts the opening waits for")?;
    if skipped.is_empty() {
        info!("Event-wide scheduled opening {} done", scheduled_event.id);
    }
    Ok(skipped.is_empty())
}

async fn post_name(
    hasura_transaction: &Transaction<'_>,
    election_event: &sequent_core::types::hasura::core::ElectionEvent,
    election_id: &str,
) -> Result<String> {
    Ok(crate::postgres::election::get_election_by_id(
        hasura_transaction,
        &election_event.tenant_id,
        &election_event.id,
        election_id,
    )
    .await?
    .map(|election| post_display_name(&election, &election_event.get_default_language()))
    .unwrap_or_else(|| election_id.to_string()))
}

#[cfg(test)]
#[path = "initialization_schedule_tests.rs"]
mod tests;
