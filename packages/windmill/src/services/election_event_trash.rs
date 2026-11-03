// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Election event Trash: deleting an election event moves it to Trash, where
//! it can be restored until its retention ends and the beat's
//! `scheduled_events` task purges it with the permanent deletion.
use crate::postgres::election_event_trash::{
    claim_expired_election_events, get_trash_state, list_trashed_election_events,
    lock_election_event, restore_election_event as restore_election_event_row,
    trash_election_event, TrashState, TrashedElectionEvent, DEFAULT_TRASH_PAGE_SIZE,
    MAX_TRASH_PAGE_SIZE,
};
use crate::postgres::scheduled_event::{
    find_scheduled_event_by_election_event_id, stop_scheduled_event,
};
use crate::services::database::get_hasura_pool;
use crate::tasks::delete_election_event::purge_trashed_election_event_t;
use crate::tasks::scheduled_events::get_datetime;
use anyhow::{anyhow, Context, Result};
use celery::Celery;
use chrono::{DateTime, Local, Utc};
use deadpool_postgres::Transaction;
use sequent_core::ballot::{ElectionEventStatus, ElectionStatus, VotingStatusChannel};
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use thiserror::Error;
use tracing::{error, info, instrument};

pub const TRASH_RETENTION_DAYS_ENV: &str = "WINDMILL_ELECTION_EVENT_TRASH_RETENTION_DAYS";
pub const DEFAULT_TRASH_RETENTION_DAYS: i32 = 7;
pub const MAX_TRASH_RETENTION_DAYS: i32 = 3650;
/// Expired election events claimed for permanent deletion per beat pass.
pub const PURGE_BATCH_SIZE: i64 = 10;
/// A claimed purge that hasn't finished after this long is claimed again.
pub const PURGE_RETRY_AFTER_SECONDS: f64 = 3600.0;
/// Who the task executions of automatic permanent deletions are run by.
pub const TRASH_RETENTION_EXECUTOR: &str = "Trash retention";

const VOTING_STATUS_CHANNELS: [VotingStatusChannel; 4] = [
    VotingStatusChannel::ONLINE,
    VotingStatusChannel::KIOSK,
    VotingStatusChannel::EARLY_VOTING,
    VotingStatusChannel::TELEPHONE,
];

/// Why an election event can't be moved to Trash or restored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TrashError {
    #[error("The election event was not found")]
    NotFound,
    #[error("Voting is open or paused in the election event; close it before deleting the event")]
    VotingInProgress,
    #[error("The election event is not in Trash")]
    NotInTrash,
    #[error("The election event is in Trash; restore it first")]
    InTrash,
    #[error("The election event's permanent deletion has started; it can't be restored")]
    PurgeStarted,
}

/// Parses the Trash retention in days. Unset or empty is the default; any
/// other value outside 1..=MAX_TRASH_RETENTION_DAYS is a configuration error,
/// so that a mistyped value never shortens the retention.
pub fn parse_trash_retention_days(value: Option<&str>) -> Result<i32> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(DEFAULT_TRASH_RETENTION_DAYS);
    };
    value
        .parse::<i32>()
        .ok()
        .filter(|days| (1..=MAX_TRASH_RETENTION_DAYS).contains(days))
        .ok_or_else(|| {
            anyhow!(
                "Invalid {TRASH_RETENTION_DAYS_ENV} {value:?}: expected whole days from 1 to {MAX_TRASH_RETENTION_DAYS}"
            )
        })
}

/// The configured Trash retention in days.
pub fn trash_retention_days() -> Result<i32> {
    match std::env::var(TRASH_RETENTION_DAYS_ENV) {
        Ok(value) => parse_trash_retention_days(Some(&value)),
        Err(std::env::VarError::NotPresent) => parse_trash_retention_days(None),
        Err(error) => Err(anyhow!("Invalid {TRASH_RETENTION_DAYS_ENV}: {error}")),
    }
}

/// Whether voting is open or paused on any channel of the election event or
/// of its elections.
pub fn voting_in_progress(
    event_status: Option<Value>,
    election_statuses: Vec<Option<Value>>,
) -> Result<bool> {
    if let Some(status) = event_status {
        let status: ElectionEventStatus = deserialize_value(status)
            .map_err(|err| anyhow!("Error parsing election event status: {err:?}"))?;
        if VOTING_STATUS_CHANNELS
            .iter()
            .any(|channel| status.status_by_channel(*channel).is_open())
        {
            return Ok(true);
        }
    }
    for status in election_statuses.into_iter().flatten() {
        let status: ElectionStatus = deserialize_value(status)
            .map_err(|err| anyhow!("Error parsing election status: {err:?}"))?;
        if VOTING_STATUS_CHANNELS
            .iter()
            .any(|channel| status.status_by_channel(*channel).is_open())
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The result of moving an election event to Trash.
#[derive(Debug, Clone, PartialEq)]
pub enum TrashOutcome {
    Trashed {
        trashed_at: DateTime<Utc>,
        purge_after: DateTime<Utc>,
    },
    AlreadyInTrash,
}

/// Moves the election event to Trash for `retention_days`, keeping all its
/// data. Refused while voting is open or paused.
#[instrument(skip(hasura_transaction), err)]
pub async fn move_election_event_to_trash(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    retention_days: i32,
) -> Result<TrashOutcome> {
    let Some(locked) =
        lock_election_event(hasura_transaction, tenant_id, election_event_id).await?
    else {
        return Err(TrashError::NotFound.into());
    };
    if locked.in_trash {
        return Ok(TrashOutcome::AlreadyInTrash);
    }
    if voting_in_progress(locked.status, locked.election_statuses)? {
        return Err(TrashError::VotingInProgress.into());
    }
    let (trashed_at, purge_after) = trash_election_event(
        hasura_transaction,
        tenant_id,
        election_event_id,
        retention_days,
    )
    .await?
    .ok_or_else(|| anyhow!("The election event changed while moving it to Trash"))?;
    Ok(TrashOutcome::Trashed {
        trashed_at,
        purge_after,
    })
}

/// Restores the election event from Trash with the archive state it had,
/// unless its permanent deletion has started. Its voting status is not
/// changed, and the schedules whose time passed while it was in Trash are
/// stopped rather than run late. Returns whether it is archived.
#[instrument(skip(hasura_transaction), err)]
pub async fn restore_election_event(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<bool> {
    let Some(is_archived) =
        restore_election_event_row(hasura_transaction, tenant_id, election_event_id).await?
    else {
        return Err(
            match get_trash_state(hasura_transaction, tenant_id, election_event_id).await? {
                TrashState::Missing => TrashError::NotFound,
                TrashState::Active => TrashError::NotInTrash,
                TrashState::InTrash | TrashState::Purging => TrashError::PurgeStarted,
            }
            .into(),
        );
    };

    let now = Local::now();
    for scheduled_event in
        find_scheduled_event_by_election_event_id(hasura_transaction, tenant_id, election_event_id)
            .await?
    {
        let overdue = get_datetime(&scheduled_event).is_some_and(|date| date <= now);
        if overdue && scheduled_event.stopped_at.is_none() {
            stop_scheduled_event(hasura_transaction, tenant_id, &scheduled_event.id).await?;
        }
    }
    Ok(is_archived)
}

/// A tenant's Trash, as the Admin Portal shows it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElectionEventTrash {
    pub items: Vec<TrashedElectionEvent>,
    pub total: i64,
    /// `None` when the retention is misconfigured: no event can be moved to
    /// Trash until it is fixed.
    pub retention_days: Option<i32>,
}

/// A page of the tenant's Trash: `limit` events (default
/// DEFAULT_TRASH_PAGE_SIZE, at most MAX_TRASH_PAGE_SIZE) after the first
/// `offset`, newest first.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_election_event_trash(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<ElectionEventTrash> {
    let limit = limit
        .unwrap_or(DEFAULT_TRASH_PAGE_SIZE)
        .clamp(1, MAX_TRASH_PAGE_SIZE);
    let offset = offset.unwrap_or(0).max(0);
    let (items, total) =
        list_trashed_election_events(hasura_transaction, tenant_id, limit, offset).await?;
    let retention_days = trash_retention_days().map_err(|err| error!("{err:#}")).ok();
    Ok(ElectionEventTrash {
        items,
        total,
        retention_days,
    })
}

/// Claims the election events whose Trash time is over and sends their
/// permanent deletion. The claim is committed first, so from then on they
/// can't be restored; a purge that fails or is lost is claimed again after
/// PURGE_RETRY_AFTER_SECONDS.
#[instrument(skip(celery_app), err)]
pub async fn dispatch_expired_election_events(celery_app: Arc<Celery>) -> Result<()> {
    let mut hasura_db_client = get_hasura_pool()
        .await
        .get()
        .await
        .context("Error getting hasura client")?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .context("Error starting the Trash claim transaction")?;
    let claimed = claim_expired_election_events(
        &hasura_transaction,
        PURGE_BATCH_SIZE,
        PURGE_RETRY_AFTER_SECONDS,
    )
    .await?;
    hasura_transaction
        .commit()
        .await
        .context("Error committing the Trash claim")?;

    for (tenant_id, election_event_id) in claimed {
        info!(
            tenant_id,
            election_event_id, "Purging election event whose Trash time is over"
        );
        if let Err(err) = celery_app
            .send_task(purge_trashed_election_event_t::new(
                tenant_id.clone(),
                election_event_id.clone(),
            ))
            .await
        {
            error!(
                tenant_id,
                election_event_id,
                "Error sending the election event purge, it will be retried: {err}"
            );
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "election_event_trash_database_tests.rs"]
mod database_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn retention_defaults_to_seven_days_when_unset_or_empty() {
        assert_eq!(parse_trash_retention_days(None).unwrap(), 7);
        assert_eq!(parse_trash_retention_days(Some("")).unwrap(), 7);
        assert_eq!(parse_trash_retention_days(Some("  ")).unwrap(), 7);
    }

    #[test]
    fn retention_accepts_whole_days_in_range() {
        assert_eq!(parse_trash_retention_days(Some("1")).unwrap(), 1);
        assert_eq!(parse_trash_retention_days(Some(" 30 ")).unwrap(), 30);
        assert_eq!(parse_trash_retention_days(Some("3650")).unwrap(), 3650);
    }

    #[test]
    fn invalid_retention_is_an_error_not_a_shorter_retention() {
        for value in ["0", "-1", "7d", "1.5", "3651", "99999999999", "seven"] {
            let error = parse_trash_retention_days(Some(value)).unwrap_err();
            assert!(
                error.to_string().contains(TRASH_RETENTION_DAYS_ENV),
                "{value}: {error}"
            );
        }
    }

    #[test]
    fn voting_in_progress_on_any_channel_of_event_or_elections() {
        let closed = json!({"voting_status": "CLOSED"});
        assert!(!voting_in_progress(None, vec![]).unwrap());
        assert!(
            !voting_in_progress(Some(closed.clone()), vec![None, Some(closed.clone())]).unwrap()
        );
        assert!(
            !voting_in_progress(Some(json!({"voting_status": "NOT_STARTED"})), vec![]).unwrap()
        );
        for status in ["OPEN", "PAUSED"] {
            assert!(voting_in_progress(Some(json!({"voting_status": status})), vec![]).unwrap());
            assert!(voting_in_progress(
                Some(closed.clone()),
                vec![Some(json!({"kiosk_voting_status": status}))]
            )
            .unwrap());
            assert!(voting_in_progress(
                None,
                vec![Some(json!({"telephone_voting_status": status}))]
            )
            .unwrap());
        }
    }

    #[test]
    fn unreadable_status_refuses_rather_than_trashes() {
        assert!(voting_in_progress(Some(json!({"voting_status": "WEIRD"})), vec![]).is_err());
        assert!(voting_in_progress(None, vec![Some(json!(42))]).is_err());
    }
}
