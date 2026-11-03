// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! `sequent_backend.audit_log_outbox`: the audit entries of committed
//! changes, waiting for the worker that signs and posts them to their log.

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::AuditStatementKind;
use electoral_log::messages::statement::{StatementEventType, StatementLogType};
use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio_postgres::Row;
use tracing::instrument;
use uuid::Uuid;

/// The log an entry belongs to: its tenant's, or one election event's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AuditLogTarget {
    pub tenant_id: Uuid,
    /// `None`: the tenant log.
    pub election_event_id: Option<Uuid>,
}

/// An entry to queue.
#[derive(Debug, Clone, PartialEq)]
pub struct NewAuditLogEntry {
    pub target: AuditLogTarget,
    pub statement_kind: AuditStatementKind,
    pub event_type: StatementEventType,
    pub log_type: StatementLogType,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub election_id: Option<Uuid>,
    pub area_id: Option<Uuid>,
    pub description: String,
    pub details: Value,
}

/// A queued entry.
#[derive(Debug, Clone, PartialEq)]
pub struct AuditLogOutboxRow {
    pub id: Uuid,
    pub seq: i64,
    pub target: AuditLogTarget,
    pub statement_kind: AuditStatementKind,
    pub event_type: StatementEventType,
    pub log_type: StatementLogType,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub election_id: Option<Uuid>,
    pub area_id: Option<Uuid>,
    pub description: String,
    pub details: Value,
    pub occurred_at: DateTime<Utc>,
    pub attempts: i32,
    pub last_attempt_at: Option<DateTime<Utc>>,
}

/// Reads a text column written with the `Display` of an electoral-log enum,
/// whose serde form is the same variant name.
fn log_enum<T: DeserializeOwned>(row: &Row, column: &str) -> Result<T> {
    let text: String = row.try_get(column)?;
    serde_json::from_value(Value::String(text.clone()))
        .map_err(|err| anyhow!("{column}: unknown value {text:?}: {err}"))
}

impl TryFrom<Row> for AuditLogOutboxRow {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self> {
        Ok(AuditLogOutboxRow {
            id: row.try_get("id")?,
            seq: row.try_get("seq")?,
            target: AuditLogTarget {
                tenant_id: row.try_get("tenant_id")?,
                election_event_id: row.try_get("election_event_id")?,
            },
            statement_kind: log_enum(&row, "statement_kind")?,
            event_type: log_enum(&row, "event_type")?,
            log_type: log_enum(&row, "log_type")?,
            user_id: row.try_get("user_id")?,
            username: row.try_get("username")?,
            election_id: row.try_get("election_id")?,
            area_id: row.try_get("area_id")?,
            description: row.try_get("description")?,
            details: row.try_get("details")?,
            occurred_at: row.try_get("occurred_at")?,
            attempts: row.try_get("attempts")?,
            last_attempt_at: row.try_get("last_attempt_at")?,
        })
    }
}

/// Queues an entry in the caller's transaction, so it exists exactly when the
/// change it records commits. Returns the entry's id.
#[instrument(skip_all, fields(kind = %entry.statement_kind), err)]
pub async fn insert_audit_log_outbox(
    hasura_transaction: &Transaction<'_>,
    entry: &NewAuditLogEntry,
) -> Result<Uuid> {
    Ok(hasura_transaction
        .query_one(
            "INSERT INTO sequent_backend.audit_log_outbox
                 (tenant_id, election_event_id, statement_kind, event_type, log_type,
                  user_id, username, election_id, area_id, description, details)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
             RETURNING id",
            &[
                &entry.target.tenant_id,
                &entry.target.election_event_id,
                &entry.statement_kind.to_string(),
                &entry.event_type.to_string(),
                &entry.log_type.to_string(),
                &entry.user_id,
                &entry.username,
                &entry.election_id,
                &entry.area_id,
                &entry.description,
                &entry.details,
            ],
        )
        .await
        .context("Error queueing the audit log entry")?
        .try_get(0)?)
}

/// Takes a log's worker lock until the transaction ends, without waiting:
/// `false` while another worker posts the log.
#[instrument(skip(hasura_transaction), err)]
pub async fn try_lock_audit_log_worker(
    hasura_transaction: &Transaction<'_>,
    target: AuditLogTarget,
) -> Result<bool> {
    let event = target
        .election_event_id
        .map(|id| id.to_string())
        .unwrap_or_default();
    Ok(hasura_transaction
        .query_one(
            "SELECT pg_try_advisory_xact_lock(hashtextextended($1, 0))",
            &[&format!("audit-log-worker:{}:{event}", target.tenant_id)],
        )
        .await
        .context("Error trying the audit log worker lock")?
        .try_get(0)?)
}

/// The logs with entries still to post.
#[instrument(skip(hasura_transaction), err)]
pub async fn list_audit_log_outbox_targets(
    hasura_transaction: &Transaction<'_>,
) -> Result<Vec<AuditLogTarget>> {
    hasura_transaction
        .query(
            "SELECT DISTINCT tenant_id, election_event_id
             FROM sequent_backend.audit_log_outbox
             WHERE posted_at IS NULL
             ORDER BY tenant_id, election_event_id",
            &[],
        )
        .await
        .context("Error listing the audit log outboxes")?
        .into_iter()
        .map(|row| {
            Ok(AuditLogTarget {
                tenant_id: row.try_get(0)?,
                election_event_id: row.try_get(1)?,
            })
        })
        .collect()
}

/// Up to `limit` unposted entries of a log in the order they were queued,
/// locked until the transaction ends. A row this version can't read comes
/// back as its id and the error, so the worker can mark it failed and stop
/// there.
#[instrument(skip(hasura_transaction), err)]
pub async fn fetch_unposted_audit_log_outbox(
    hasura_transaction: &Transaction<'_>,
    target: AuditLogTarget,
    limit: i64,
) -> Result<Vec<(Uuid, Result<AuditLogOutboxRow>)>> {
    hasura_transaction
        .query(
            "SELECT * FROM sequent_backend.audit_log_outbox
             WHERE tenant_id = $1 AND election_event_id IS NOT DISTINCT FROM $2
                 AND posted_at IS NULL
             ORDER BY seq
             LIMIT $3
             FOR UPDATE",
            &[&target.tenant_id, &target.election_event_id, &limit],
        )
        .await
        .context("Error fetching unposted audit log entries")?
        .into_iter()
        .map(|row| Ok((row.try_get("id")?, AuditLogOutboxRow::try_from(row))))
        .collect()
}

#[instrument(skip(hasura_transaction), err)]
pub async fn mark_audit_log_outbox_posted(
    hasura_transaction: &Transaction<'_>,
    ids: &[Uuid],
) -> Result<()> {
    hasura_transaction
        .execute(
            "UPDATE sequent_backend.audit_log_outbox
             SET posted_at = clock_timestamp(), last_error = NULL
             WHERE id = ANY($1) AND posted_at IS NULL",
            &[&ids],
        )
        .await
        .context("Error marking the audit log entries posted")?;
    Ok(())
}

/// Counts a failed attempt; returns the attempts so far (0 if the entry was
/// posted meanwhile).
#[instrument(skip(hasura_transaction), err)]
pub async fn mark_audit_log_outbox_failed(
    hasura_transaction: &Transaction<'_>,
    id: Uuid,
    error: &str,
) -> Result<i32> {
    Ok(hasura_transaction
        .query_opt(
            "UPDATE sequent_backend.audit_log_outbox
             SET attempts = attempts + 1, last_error = $2, last_attempt_at = clock_timestamp()
             WHERE id = $1 AND posted_at IS NULL
             RETURNING attempts",
            &[&id, &error],
        )
        .await
        .context("Error marking the audit log entry failed")?
        .map(|row| row.try_get(0))
        .transpose()?
        .unwrap_or(0))
}

/// How long the oldest unposted entry has waited, in seconds, and how many
/// entries wait: what monitoring alerts on.
#[instrument(skip(hasura_transaction), err)]
pub async fn audit_log_outbox_backlog(hasura_transaction: &Transaction<'_>) -> Result<(i64, f64)> {
    let row = hasura_transaction
        .query_one(
            "SELECT count(*),
                    COALESCE(EXTRACT(EPOCH FROM clock_timestamp() - min(created_at)), 0)::float8
             FROM sequent_backend.audit_log_outbox
             WHERE posted_at IS NULL",
            &[],
        )
        .await
        .context("Error measuring the audit log outbox")?;
    Ok((row.try_get(0)?, row.try_get(1)?))
}
