// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Statements of the election event Trash. An event in Trash keeps all its
//! data and is archived until `purge_after`; `purge_started_at` is the
//! committed claim after which it can no longer be restored.
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::instrument;
use uuid::Uuid;

/// Trash pages hold this many events unless asked for fewer.
pub const DEFAULT_TRASH_PAGE_SIZE: i64 = 50;
pub const MAX_TRASH_PAGE_SIZE: i64 = 100;

/// Moving to Trash and restoring hold the event FOR UPDATE, which waits for
/// and then blocks the FOR KEY SHARE that election status writers take
/// (guard_trashed_election_event_status) and the FOR UPDATE that scheduled
/// changes take (lock_scheduling_event).
pub const LOCK_ELECTION_EVENT_SQL: &str = r#"
    SELECT status, trashed_at IS NOT NULL AS in_trash
    FROM sequent_backend.election_event
    WHERE tenant_id = $1 AND id = $2
    FOR UPDATE
"#;

/// Read once the event is locked, so it includes every status written by a
/// writer that held the event until it committed.
pub const ELECTION_STATUSES_SQL: &str = r#"
    SELECT status
    FROM sequent_backend.election
    WHERE tenant_id = $1 AND election_event_id = $2
"#;

pub const TRASH_ELECTION_EVENT_SQL: &str = r#"
    UPDATE sequent_backend.election_event
    SET trashed_at = now(),
        purge_after = now() + make_interval(days => $3),
        archived_before_trash = is_archived,
        is_archived = true
    WHERE tenant_id = $1 AND id = $2 AND trashed_at IS NULL
    RETURNING trashed_at, purge_after
"#;

pub const RESTORE_ELECTION_EVENT_SQL: &str = r#"
    UPDATE sequent_backend.election_event
    SET is_archived = archived_before_trash,
        trashed_at = NULL,
        purge_after = NULL,
        archived_before_trash = NULL
    WHERE tenant_id = $1 AND id = $2
      AND trashed_at IS NOT NULL
      AND purge_started_at IS NULL
    RETURNING is_archived
"#;

pub const TRASH_STATE_SQL: &str = r#"
    SELECT trashed_at IS NOT NULL AS in_trash, purge_started_at IS NOT NULL AS purging
    FROM sequent_backend.election_event
    WHERE tenant_id = $1 AND id = $2
"#;

pub const LIST_TRASHED_ELECTION_EVENTS_SQL: &str = r#"
    SELECT id, presentation, trashed_at, purge_after, purge_started_at,
           archived_before_trash
    FROM sequent_backend.election_event
    WHERE tenant_id = $1 AND trashed_at IS NOT NULL
    ORDER BY trashed_at DESC, id
    LIMIT $2 OFFSET $3
"#;

pub const COUNT_TRASHED_ELECTION_EVENTS_SQL: &str = r#"
    SELECT count(*) AS total
    FROM sequent_backend.election_event
    WHERE tenant_id = $1 AND trashed_at IS NOT NULL
"#;

/// Claims up to `$1` events whose Trash time is over, and again those whose
/// claim is older than `$2` seconds and still unlocked (a purge that failed
/// or never ran). A purge in progress holds its row and is skipped.
pub const CLAIM_EXPIRED_ELECTION_EVENTS_SQL: &str = r#"
    UPDATE sequent_backend.election_event AS event
    SET purge_started_at = now()
    FROM (
        SELECT id
        FROM sequent_backend.election_event
        WHERE trashed_at IS NOT NULL
          AND purge_after <= now()
          AND (purge_started_at IS NULL
               OR purge_started_at <= now() - make_interval(secs => $2))
        ORDER BY purge_after
        LIMIT $1
        FOR NO KEY UPDATE SKIP LOCKED
    ) AS due
    WHERE event.id = due.id
    RETURNING event.tenant_id, event.id
"#;

pub const LOCK_CLAIMED_ELECTION_EVENT_SQL: &str = r#"
    SELECT id
    FROM sequent_backend.election_event
    WHERE tenant_id = $1 AND id = $2
      AND trashed_at IS NOT NULL
      AND purge_started_at IS NOT NULL
    FOR UPDATE
"#;

/// An election event's row as the Trash shows it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrashedElectionEvent {
    pub id: String,
    pub presentation: Option<Value>,
    pub trashed_at: DateTime<Utc>,
    pub purge_after: DateTime<Utc>,
    pub purge_started_at: Option<DateTime<Utc>>,
    pub archived_before_trash: bool,
}

/// The state of an election event, read under its row lock before moving it
/// to Trash.
pub struct LockedElectionEvent {
    pub status: Option<Value>,
    pub in_trash: bool,
    pub election_statuses: Vec<Option<Value>>,
}

/// Where an election event is in its Trash lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrashState {
    Missing,
    Active,
    InTrash,
    Purging,
}

fn ids(tenant_id: &str, election_event_id: &str) -> Result<(Uuid, Uuid)> {
    Ok((parse_uuid_v4(tenant_id)?, parse_uuid_v4(election_event_id)?))
}

/// Locks the election event, and so its elections, against status changes
/// until the transaction ends, and returns their state. `None` when the event
/// doesn't exist in the tenant.
#[instrument(skip(hasura_transaction), err)]
pub async fn lock_election_event(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Option<LockedElectionEvent>> {
    let (tenant, event) = ids(tenant_id, election_event_id)?;
    let Some(row) = hasura_transaction
        .query_opt(LOCK_ELECTION_EVENT_SQL, &[&tenant, &event])
        .await?
    else {
        return Ok(None);
    };
    let election_statuses = hasura_transaction
        .query(ELECTION_STATUSES_SQL, &[&tenant, &event])
        .await?
        .into_iter()
        .map(|row| row.try_get("status"))
        .collect::<std::result::Result<_, _>>()?;
    Ok(Some(LockedElectionEvent {
        status: row.try_get("status")?,
        in_trash: row.try_get("in_trash")?,
        election_statuses,
    }))
}

/// Moves the election event to Trash for `retention_days`. Returns its Trash
/// dates, or `None` when it was already in Trash.
#[instrument(skip(hasura_transaction), err)]
pub async fn trash_election_event(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    retention_days: i32,
) -> Result<Option<(DateTime<Utc>, DateTime<Utc>)>> {
    let (tenant, event) = ids(tenant_id, election_event_id)?;
    let row = hasura_transaction
        .query_opt(
            TRASH_ELECTION_EVENT_SQL,
            &[&tenant, &event, &retention_days],
        )
        .await?;
    row.map(|row| Ok((row.try_get("trashed_at")?, row.try_get("purge_after")?)))
        .transpose()
}

/// Takes the election event out of Trash with the archive state it had.
/// Returns that state, or `None` when it is not in Trash or its purge
/// started.
#[instrument(skip(hasura_transaction), err)]
pub async fn restore_election_event(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Option<bool>> {
    let (tenant, event) = ids(tenant_id, election_event_id)?;
    hasura_transaction
        .query_opt(LOCK_ELECTION_EVENT_SQL, &[&tenant, &event])
        .await?;
    let row = hasura_transaction
        .query_opt(RESTORE_ELECTION_EVENT_SQL, &[&tenant, &event])
        .await?;
    row.map(|row| Ok(row.try_get("is_archived")?)).transpose()
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_trash_state(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<TrashState> {
    let (tenant, event) = ids(tenant_id, election_event_id)?;
    let Some(row) = hasura_transaction
        .query_opt(TRASH_STATE_SQL, &[&tenant, &event])
        .await?
    else {
        return Ok(TrashState::Missing);
    };
    Ok(
        match (row.try_get::<_, bool>("in_trash")?, row.try_get("purging")?) {
            (false, _) => TrashState::Active,
            (true, false) => TrashState::InTrash,
            (true, true) => TrashState::Purging,
        },
    )
}

/// A page of the tenant's election events in Trash, newest first, and how
/// many there are in all.
#[instrument(skip(hasura_transaction), err)]
pub async fn list_trashed_election_events(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    limit: i64,
    offset: i64,
) -> Result<(Vec<TrashedElectionEvent>, i64)> {
    let tenant = parse_uuid_v4(tenant_id)?;
    let rows = hasura_transaction
        .query(
            LIST_TRASHED_ELECTION_EVENTS_SQL,
            &[&tenant, &limit, &offset],
        )
        .await?;
    let total = hasura_transaction
        .query_one(COUNT_TRASHED_ELECTION_EVENTS_SQL, &[&tenant])
        .await?
        .try_get("total")?;
    let events = rows
        .into_iter()
        .map(|row| {
            Ok(TrashedElectionEvent {
                id: row.try_get::<_, Uuid>("id")?.to_string(),
                presentation: row.try_get("presentation")?,
                trashed_at: row.try_get("trashed_at")?,
                purge_after: row.try_get("purge_after")?,
                purge_started_at: row.try_get("purge_started_at")?,
                archived_before_trash: row.try_get("archived_before_trash")?,
            })
        })
        .collect::<Result<_>>()?;
    Ok((events, total))
}

/// Claims at most `limit` expired election events for permanent deletion and
/// returns their `(tenant_id, election_event_id)`.
#[instrument(skip(hasura_transaction), err)]
pub async fn claim_expired_election_events(
    hasura_transaction: &Transaction<'_>,
    limit: i64,
    retry_after_seconds: f64,
) -> Result<Vec<(String, String)>> {
    hasura_transaction
        .query(
            CLAIM_EXPIRED_ELECTION_EVENTS_SQL,
            &[&limit, &retry_after_seconds],
        )
        .await?
        .into_iter()
        .map(|row| {
            Ok((
                row.try_get::<_, Uuid>("tenant_id")?.to_string(),
                row.try_get::<_, Uuid>("id")?.to_string(),
            ))
        })
        .collect()
}

/// Locks a claimed election event for its permanent deletion. `false` when
/// there is nothing left to delete.
#[instrument(skip(hasura_transaction), err)]
pub async fn lock_claimed_election_event(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<bool> {
    let (tenant, event) = ids(tenant_id, election_event_id)?;
    Ok(hasura_transaction
        .query_opt(LOCK_CLAIMED_ELECTION_EVENT_SQL, &[&tenant, &event])
        .await
        .map_err(|err| anyhow!("Error locking the election event to purge: {err}"))?
        .is_some())
}
