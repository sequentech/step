// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Rows of the signing tables: rules, certificate checks, requests, their
//! approvals, staff certificates and the electoral log outbox. Every query is
//! scoped to its tenant and election event. Enums are stored as their
//! kebab-case text, which the tables' CHECK constraints list.

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::SigningStatementKind;
use electoral_log::messages::statement::{StatementEventType, StatementLogType};
use sequent_core::signing::{
    CancelReason, CertificatePostBinding, CertificateRegistration, CrlUnavailablePolicy,
    RequesterSigning, RevocationCheck, RevocationStatus, SignatureAlgorithm, SigningAction,
    SigningChecks, SigningRequestStatus, SigningRequirement, SigningRule,
    StaffCertificateRegistration, StaffCertificateStatus,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::fmt::Display;
use std::str::FromStr;
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

/// Reads the text column `column` as the enum it stores.
fn text_enum<T>(row: &Row, column: &str) -> Result<T>
where
    T: FromStr,
    T::Err: Display,
{
    let text: String = row.try_get(column)?;
    T::from_str(&text).map_err(|err| anyhow!("{column}: unknown value {text:?}: {err}"))
}

fn optional_text_enum<T>(row: &Row, column: &str) -> Result<Option<T>>
where
    T: FromStr,
    T::Err: Display,
{
    row.try_get::<_, Option<String>>(column)?
        .map(|text| {
            T::from_str(&text).map_err(|err| anyhow!("{column}: unknown value {text:?}: {err}"))
        })
        .transpose()
}

/// The outbox's `entry`: the USER entry of a step is posted before its
/// SYSTEM entry.
fn entry_of(event_type: &StatementEventType) -> i16 {
    match event_type {
        StatementEventType::USER => 0,
        StatementEventType::SYSTEM => 1,
    }
}

/// Reads a text column written with the `Display` of an electoral-log
/// enum, whose serde form is the same variant name.
fn log_enum<T: DeserializeOwned>(row: &Row, column: &str) -> Result<T> {
    let text: String = row.try_get(column)?;
    serde_json::from_value(Value::String(text.clone()))
        .map_err(|err| anyhow!("{column}: unknown value {text:?}: {err}"))
}

/// Takes the election event's signing lock until the transaction ends.
///
/// Call it first in every transaction that writes signing rows, before any
/// row lock (a request, a certificate): the signing steps of an event then
/// commit one after another, so their log entries are queued in the order
/// they commit and the outbox worker, which takes the same lock, never sees
/// a gap it would later have to fill. Taking it again in the same
/// transaction is harmless.
#[instrument(skip(hasura_transaction), err)]
pub async fn lock_signing_event(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
) -> Result<()> {
    hasura_transaction
        .execute(
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
            &[&format!("signing-event:{tenant_id}:{election_event_id}")],
        )
        .await
        .context("Error taking the signing lock of the election event")?;
    Ok(())
}

/// Takes the election event's outbox worker lock until the transaction
/// ends, without waiting: `false` while another worker posts the event.
/// It is not the signing lock, so signing steps never wait for a worker.
#[instrument(skip(hasura_transaction), err)]
pub async fn try_lock_signing_log_worker(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
) -> Result<bool> {
    Ok(hasura_transaction
        .query_one(
            "SELECT pg_try_advisory_xact_lock(hashtextextended($1, 0))",
            &[&format!("signing-worker:{tenant_id}:{election_event_id}")],
        )
        .await
        .context("Error trying the signing log worker lock of the election event")?
        .try_get(0)?)
}

// Rules

/// A saved rule. Without a row an action keeps [`SigningRule::default_for`].
#[derive(Debug, Clone, PartialEq)]
pub struct SigningRuleRow {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub rule: SigningRule,
    pub updated_by: String,
    /// The display name of `updated_by` when they saved.
    pub updated_by_name: Option<String>,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<Row> for SigningRuleRow {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self> {
        let signatures: i32 = row.try_get("signatures")?;
        let expires_minutes: Option<i32> = row.try_get("expires_minutes")?;
        Ok(SigningRuleRow {
            id: row.try_get("id")?,
            tenant_id: row.try_get("tenant_id")?,
            election_event_id: row.try_get("election_event_id")?,
            rule: SigningRule {
                action: text_enum(&row, "action")?,
                requirement: text_enum(&row, "requirement")?,
                signatures: u16::try_from(signatures).context("signatures")?,
                requester_signing: text_enum(&row, "requester_signing")?,
                expires_minutes: expires_minutes
                    .map(u32::try_from)
                    .transpose()
                    .context("expires_minutes")?,
                revision: row.try_get("revision")?,
            },
            updated_by: row.try_get("updated_by")?,
            updated_by_name: row.try_get("updated_by_name")?,
            updated_at: row.try_get("updated_at")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_signing_rule(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    action: SigningAction,
) -> Result<Option<SigningRuleRow>> {
    hasura_transaction
        .query_opt(
            "SELECT * FROM sequent_backend.signing_rule
             WHERE tenant_id = $1 AND election_event_id = $2 AND action = $3",
            &[&tenant_id, &election_event_id, &action.to_string()],
        )
        .await
        .context("Error reading the signing rule")?
        .map(SigningRuleRow::try_from)
        .transpose()
}

/// Saves `rule` if the saved revision is still `expected_revision` (0 when
/// the action has no row yet), moving the revision on by one. `None` when
/// another save came first. `rule.revision` is ignored.
#[instrument(skip(hasura_transaction), err)]
pub async fn upsert_signing_rule(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    rule: &SigningRule,
    expected_revision: i64,
    updated_by: &str,
    updated_by_name: Option<&str>,
) -> Result<Option<SigningRuleRow>> {
    let signatures = i32::from(rule.signatures);
    let expires_minutes = rule
        .expires_minutes
        .map(i32::try_from)
        .transpose()
        .context("expires_minutes")?;
    let action = rule.action.to_string();
    let requirement = rule.requirement.to_string();
    let requester_signing = rule.requester_signing.to_string();
    // The first save inserts; a later one updates the revision it read.
    let sql = if expected_revision == 0 {
        "INSERT INTO sequent_backend.signing_rule
             (tenant_id, election_event_id, action, requirement, signatures,
              requester_signing, expires_minutes, updated_by, revision, updated_by_name)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::bigint + 1, $10)
         ON CONFLICT (tenant_id, election_event_id, action) DO NOTHING
         RETURNING *"
    } else {
        "UPDATE sequent_backend.signing_rule SET
             requirement = $4, signatures = $5, requester_signing = $6,
             expires_minutes = $7, updated_by = $8, updated_by_name = $10,
             revision = revision + 1, updated_at = clock_timestamp()
         WHERE tenant_id = $1 AND election_event_id = $2 AND action = $3
             AND revision = $9
         RETURNING *"
    };
    hasura_transaction
        .query_opt(
            sql,
            &[
                &tenant_id,
                &election_event_id,
                &action,
                &requirement,
                &signatures,
                &requester_signing,
                &expires_minutes,
                &updated_by,
                &expected_revision,
                &updated_by_name,
            ],
        )
        .await
        .context("Error saving the signing rule")?
        .map(SigningRuleRow::try_from)
        .transpose()
}

// Certificate checks

/// Saved checks. Without a row an event keeps [`SigningChecks::default`].
#[derive(Debug, Clone, PartialEq)]
pub struct SigningChecksRow {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub checks: SigningChecks,
    pub updated_by: String,
    /// The display name of `updated_by` when they saved.
    pub updated_by_name: Option<String>,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<Row> for SigningChecksRow {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self> {
        Ok(SigningChecksRow {
            id: row.try_get("id")?,
            tenant_id: row.try_get("tenant_id")?,
            election_event_id: row.try_get("election_event_id")?,
            checks: SigningChecks {
                revocation_check: text_enum::<RevocationCheck>(&row, "revocation_check")?,
                crl_unavailable: text_enum::<CrlUnavailablePolicy>(&row, "crl_unavailable")?,
                registration: text_enum::<CertificateRegistration>(&row, "registration")?,
                post_binding: text_enum::<CertificatePostBinding>(&row, "post_binding")?,
                revision: row.try_get("revision")?,
            },
            updated_by: row.try_get("updated_by")?,
            updated_by_name: row.try_get("updated_by_name")?,
            updated_at: row.try_get("updated_at")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_signing_checks(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
) -> Result<Option<SigningChecksRow>> {
    hasura_transaction
        .query_opt(
            "SELECT * FROM sequent_backend.signing_checks
             WHERE tenant_id = $1 AND election_event_id = $2",
            &[&tenant_id, &election_event_id],
        )
        .await
        .context("Error reading the signing checks")?
        .map(SigningChecksRow::try_from)
        .transpose()
}

/// Saves `checks` like [`upsert_signing_rule`] saves a rule.
#[instrument(skip(hasura_transaction), err)]
pub async fn upsert_signing_checks(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    checks: &SigningChecks,
    expected_revision: i64,
    updated_by: &str,
    updated_by_name: Option<&str>,
) -> Result<Option<SigningChecksRow>> {
    let sql = if expected_revision == 0 {
        "INSERT INTO sequent_backend.signing_checks
             (tenant_id, election_event_id, revocation_check, crl_unavailable,
              registration, post_binding, updated_by, revision, updated_by_name)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8::bigint + 1, $9)
         ON CONFLICT (tenant_id, election_event_id) DO NOTHING
         RETURNING *"
    } else {
        "UPDATE sequent_backend.signing_checks SET
             revocation_check = $3, crl_unavailable = $4, registration = $5,
             post_binding = $6, updated_by = $7, updated_by_name = $9,
             revision = revision + 1, updated_at = clock_timestamp()
         WHERE tenant_id = $1 AND election_event_id = $2 AND revision = $8
         RETURNING *"
    };
    hasura_transaction
        .query_opt(
            sql,
            &[
                &tenant_id,
                &election_event_id,
                &checks.revocation_check.to_string(),
                &checks.crl_unavailable.to_string(),
                &checks.registration.to_string(),
                &checks.post_binding.to_string(),
                &updated_by,
                &expected_revision,
                &updated_by_name,
            ],
        )
        .await
        .context("Error saving the signing checks")?
        .map(SigningChecksRow::try_from)
        .transpose()
}

// Requests

#[derive(Debug, Clone, PartialEq)]
pub struct SigningRequestRow {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub action: SigningAction,
    pub election_id: Option<Uuid>,
    pub area_id: Option<Uuid>,
    pub trustee_id: Option<Uuid>,
    pub scope_key: String,
    pub subject: Value,
    pub canonical_payload: String,
    pub payload_sha256: String,
    pub document_id: Option<Uuid>,
    pub document_sha256: Option<String>,
    pub code: String,
    pub config_revision: Option<String>,
    pub rule_revision: i64,
    pub rule_snapshot: Value,
    pub required: i32,
    pub status: SigningRequestStatus,
    pub cancel_reason: Option<CancelReason>,
    pub cancelled_by: Option<String>,
    pub requested_by: String,
    pub requested_by_username: String,
    pub requested_by_name: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub executed_at: Option<DateTime<Utc>>,
    pub execution_result: Option<Value>,
    pub task_execution_id: Option<Uuid>,
    /// When the task running a dispatched execution claimed it.
    pub execution_started_at: Option<DateTime<Utc>>,
    /// How many times a task claimed the execution.
    pub execution_attempts: i32,
    pub open_failures: i32,
    pub permission_label: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<Row> for SigningRequestRow {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self> {
        Ok(SigningRequestRow {
            id: row.try_get("id")?,
            tenant_id: row.try_get("tenant_id")?,
            election_event_id: row.try_get("election_event_id")?,
            action: text_enum(&row, "action")?,
            election_id: row.try_get("election_id")?,
            area_id: row.try_get("area_id")?,
            trustee_id: row.try_get("trustee_id")?,
            scope_key: row.try_get("scope_key")?,
            subject: row.try_get("subject")?,
            canonical_payload: row.try_get("canonical_payload")?,
            payload_sha256: row.try_get("payload_sha256")?,
            document_id: row.try_get("document_id")?,
            document_sha256: row.try_get("document_sha256")?,
            code: row.try_get("code")?,
            config_revision: row.try_get("config_revision")?,
            rule_revision: row.try_get("rule_revision")?,
            rule_snapshot: row.try_get("rule_snapshot")?,
            required: row.try_get("required")?,
            status: text_enum(&row, "status")?,
            cancel_reason: optional_text_enum(&row, "cancel_reason")?,
            cancelled_by: row.try_get("cancelled_by")?,
            requested_by: row.try_get("requested_by")?,
            requested_by_username: row.try_get("requested_by_username")?,
            requested_by_name: row.try_get("requested_by_name")?,
            expires_at: row.try_get("expires_at")?,
            completed_at: row.try_get("completed_at")?,
            executed_at: row.try_get("executed_at")?,
            execution_result: row.try_get("execution_result")?,
            task_execution_id: row.try_get("task_execution_id")?,
            execution_started_at: row.try_get("execution_started_at")?,
            execution_attempts: row.try_get("execution_attempts")?,
            open_failures: row.try_get("open_failures")?,
            permission_label: row.try_get("permission_label")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

/// A request to insert, `Waiting`. The id and creation time are the
/// caller's, because the canonical payload already carries them.
#[derive(Debug, Clone)]
pub struct NewSigningRequest {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub action: SigningAction,
    pub election_id: Option<Uuid>,
    pub area_id: Option<Uuid>,
    pub trustee_id: Option<Uuid>,
    pub scope_key: String,
    pub subject: Value,
    pub canonical_payload: String,
    pub payload_sha256: String,
    pub document_id: Option<Uuid>,
    pub document_sha256: Option<String>,
    pub code: String,
    pub config_revision: Option<String>,
    pub rule_revision: i64,
    pub rule_snapshot: Value,
    pub required: i32,
    pub requested_by: String,
    pub requested_by_username: String,
    pub requested_by_name: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub permission_label: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Inserts a waiting request. `None` while another request waits for the
/// same action and scope key (`signing_request_one_waiting`); the
/// transaction stays usable.
#[instrument(skip_all, fields(id = %request.id), err)]
pub async fn insert_signing_request(
    hasura_transaction: &Transaction<'_>,
    request: &NewSigningRequest,
) -> Result<Option<SigningRequestRow>> {
    hasura_transaction
        .query_opt(
            "INSERT INTO sequent_backend.signing_request
                 (id, tenant_id, election_event_id, action, election_id, area_id,
                  trustee_id, scope_key, subject, canonical_payload, payload_sha256,
                  document_id, document_sha256, code, config_revision, rule_revision,
                  rule_snapshot, required, status, requested_by, requested_by_username,
                  expires_at, permission_label, created_at, requested_by_name)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15,
                     $16, $17, $18, $19, $20, $21, $22, $23, $24, $25)
             ON CONFLICT (tenant_id, election_event_id, action, scope_key)
                 WHERE status = 'waiting'
                 DO NOTHING
             RETURNING *",
            &[
                &request.id,
                &request.tenant_id,
                &request.election_event_id,
                &request.action.to_string(),
                &request.election_id,
                &request.area_id,
                &request.trustee_id,
                &request.scope_key,
                &request.subject,
                &request.canonical_payload,
                &request.payload_sha256,
                &request.document_id,
                &request.document_sha256,
                &request.code,
                &request.config_revision,
                &request.rule_revision,
                &request.rule_snapshot,
                &request.required,
                &SigningRequestStatus::Waiting.to_string(),
                &request.requested_by,
                &request.requested_by_username,
                &request.expires_at,
                &request.permission_label,
                &request.created_at,
                &request.requested_by_name,
            ],
        )
        .await
        .context("Error inserting the signing request")?
        .map(SigningRequestRow::try_from)
        .transpose()
}

const SIGNING_REQUEST_BY_ID: &str = "SELECT * FROM sequent_backend.signing_request
     WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3";

#[instrument(skip(hasura_transaction), err)]
pub async fn get_signing_request(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    id: Uuid,
) -> Result<Option<SigningRequestRow>> {
    hasura_transaction
        .query_opt(
            SIGNING_REQUEST_BY_ID,
            &[&tenant_id, &election_event_id, &id],
        )
        .await
        .context("Error reading the signing request")?
        .map(SigningRequestRow::try_from)
        .transpose()
}

/// Reads the request and locks its row until the transaction ends: the
/// approvals of one request run one after another.
#[instrument(skip(hasura_transaction), err)]
pub async fn lock_signing_request(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    id: Uuid,
) -> Result<Option<SigningRequestRow>> {
    hasura_transaction
        .query_opt(
            &format!("{SIGNING_REQUEST_BY_ID} FOR UPDATE"),
            &[&tenant_id, &election_event_id, &id],
        )
        .await
        .context("Error locking the signing request")?
        .map(SigningRequestRow::try_from)
        .transpose()
}

/// A change of a request's status. Each moves on from one status only:
/// `Complete`, `Cancel` and `Expire` from `Waiting`, `Execute` and `Fail`
/// from `Completed`.
#[derive(Debug, Clone, PartialEq)]
pub enum SigningRequestTransition {
    Complete,
    Execute {
        result: Option<Value>,
    },
    Fail {
        result: Option<Value>,
    },
    Cancel {
        reason: CancelReason,
        by: Option<String>,
    },
    /// Cancels a completed `Gate` request that was never used: a trustee's
    /// request for something else than what they now sign.
    CancelCompleted {
        reason: CancelReason,
        by: Option<String>,
    },
    Expire,
}

impl SigningRequestTransition {
    fn from_status(&self) -> SigningRequestStatus {
        match self {
            SigningRequestTransition::Complete
            | SigningRequestTransition::Cancel { .. }
            | SigningRequestTransition::Expire => SigningRequestStatus::Waiting,
            SigningRequestTransition::Execute { .. }
            | SigningRequestTransition::Fail { .. }
            | SigningRequestTransition::CancelCompleted { .. } => SigningRequestStatus::Completed,
        }
    }

    fn to_status(&self) -> SigningRequestStatus {
        match self {
            SigningRequestTransition::Complete => SigningRequestStatus::Completed,
            SigningRequestTransition::Execute { .. } => SigningRequestStatus::Executed,
            SigningRequestTransition::Fail { .. } => SigningRequestStatus::Failed,
            SigningRequestTransition::Cancel { .. }
            | SigningRequestTransition::CancelCompleted { .. } => SigningRequestStatus::Cancelled,
            SigningRequestTransition::Expire => SigningRequestStatus::Expired,
        }
    }
}

/// Applies `transition` to the request. `None` when the request is not in
/// the status the transition moves on from, or, for `Expire`, when it has no
/// time limit or its time has not come.
#[instrument(skip(hasura_transaction), err)]
pub async fn update_signing_request_status(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    id: Uuid,
    transition: &SigningRequestTransition,
) -> Result<Option<SigningRequestRow>> {
    let (cancel_reason, cancelled_by) = match transition {
        SigningRequestTransition::Cancel { reason, by }
        | SigningRequestTransition::CancelCompleted { reason, by } => {
            (Some(reason.to_string()), by.clone())
        }
        _ => (None, None),
    };
    let execution_result = match transition {
        SigningRequestTransition::Execute { result }
        | SigningRequestTransition::Fail { result } => result.clone(),
        _ => None,
    };
    let completes = matches!(transition, SigningRequestTransition::Complete);
    let executes = matches!(transition, SigningRequestTransition::Execute { .. });
    let expires = matches!(transition, SigningRequestTransition::Expire);
    hasura_transaction
        .query_opt(
            "UPDATE sequent_backend.signing_request SET
                 status = $5,
                 cancel_reason = COALESCE($6, cancel_reason),
                 cancelled_by = COALESCE($7, cancelled_by),
                 execution_result = COALESCE($8, execution_result),
                 completed_at = CASE WHEN $9 THEN clock_timestamp() ELSE completed_at END,
                 executed_at = CASE WHEN $10 THEN clock_timestamp() ELSE executed_at END
             WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3 AND status = $4
                 AND (NOT $11 OR expires_at <= clock_timestamp())
             RETURNING *",
            &[
                &tenant_id,
                &election_event_id,
                &id,
                &transition.from_status().to_string(),
                &transition.to_status().to_string(),
                &cancel_reason,
                &cancelled_by,
                &execution_result,
                &completes,
                &executes,
                &expires,
            ],
        )
        .await
        .context("Error updating the signing request status")?
        .map(SigningRequestRow::try_from)
        .transpose()
}

/// The requests of `action` still waiting in the event, oldest first.
#[instrument(skip(hasura_transaction), err)]
pub async fn list_waiting_signing_requests(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    action: SigningAction,
) -> Result<Vec<SigningRequestRow>> {
    hasura_transaction
        .query(
            "SELECT * FROM sequent_backend.signing_request
             WHERE tenant_id = $1 AND election_event_id = $2 AND action = $3 AND status = $4
             ORDER BY created_at, id",
            &[
                &tenant_id,
                &election_event_id,
                &action.to_string(),
                &SigningRequestStatus::Waiting.to_string(),
            ],
        )
        .await
        .context("Error listing waiting signing requests")?
        .into_iter()
        .map(SigningRequestRow::try_from)
        .collect()
}

// Approvals

#[derive(Debug, Clone, PartialEq)]
pub struct SigningApprovalRow {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub request_id: Uuid,
    pub user_id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub auth_time: Option<DateTime<Utc>>,
    pub certificate_id: Uuid,
    pub certificate_pem: String,
    pub chain_pem: String,
    pub fingerprint_sha256: String,
    pub spki_sha256: String,
    pub holder_sha256: String,
    pub algorithm: SignatureAlgorithm,
    pub payload_signature: Vec<u8>,
    pub document_signature: Option<Vec<u8>>,
    pub pdf_cms: Option<Vec<u8>>,
    pub revocation_status: RevocationStatus,
    pub signed_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<Row> for SigningApprovalRow {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self> {
        Ok(SigningApprovalRow {
            id: row.try_get("id")?,
            tenant_id: row.try_get("tenant_id")?,
            election_event_id: row.try_get("election_event_id")?,
            request_id: row.try_get("request_id")?,
            user_id: row.try_get("user_id")?,
            username: row.try_get("username")?,
            display_name: row.try_get("display_name")?,
            auth_time: row.try_get("auth_time")?,
            certificate_id: row.try_get("certificate_id")?,
            certificate_pem: row.try_get("certificate_pem")?,
            chain_pem: row.try_get("chain_pem")?,
            fingerprint_sha256: row.try_get("fingerprint_sha256")?,
            spki_sha256: row.try_get("spki_sha256")?,
            holder_sha256: row.try_get("holder_sha256")?,
            algorithm: text_enum(&row, "algorithm")?,
            payload_signature: row.try_get("payload_signature")?,
            document_signature: row.try_get("document_signature")?,
            pdf_cms: row.try_get("pdf_cms")?,
            revocation_status: text_enum(&row, "revocation_status")?,
            signed_at: row.try_get("signed_at")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct NewSigningApproval {
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub request_id: Uuid,
    pub user_id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub auth_time: Option<DateTime<Utc>>,
    pub certificate_id: Uuid,
    pub certificate_pem: String,
    pub chain_pem: String,
    pub fingerprint_sha256: String,
    pub spki_sha256: String,
    pub holder_sha256: String,
    pub algorithm: SignatureAlgorithm,
    pub payload_signature: Vec<u8>,
    pub document_signature: Option<Vec<u8>>,
    pub pdf_cms: Option<Vec<u8>>,
    pub revocation_status: RevocationStatus,
}

/// Why an approval was not recorded: the request already has an approval
/// of the same account, certificate, key or holder. One person fills one
/// slot (the `signing_approval_one_per_*` constraints).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigningApprovalConflict {
    DuplicateUser,
    DuplicateCertificate,
    DuplicateKey,
    DuplicateHolder,
}

/// Records one signature, or says which identity already signed the
/// request. A conflict leaves the transaction usable, so the refusal can be
/// logged in it.
#[instrument(skip_all, fields(request_id = %approval.request_id), err)]
pub async fn insert_signing_approval(
    hasura_transaction: &Transaction<'_>,
    approval: &NewSigningApproval,
) -> Result<std::result::Result<SigningApprovalRow, SigningApprovalConflict>> {
    let inserted = hasura_transaction
        .query_opt(
            "INSERT INTO sequent_backend.signing_approval
                 (tenant_id, election_event_id, request_id, user_id, username, auth_time,
                  certificate_id, certificate_pem, chain_pem, fingerprint_sha256,
                  spki_sha256, holder_sha256, algorithm, payload_signature,
                  document_signature, pdf_cms, revocation_status, display_name)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15,
                     $16, $17, $18)
             ON CONFLICT DO NOTHING
             RETURNING *",
            &[
                &approval.tenant_id,
                &approval.election_event_id,
                &approval.request_id,
                &approval.user_id,
                &approval.username,
                &approval.auth_time,
                &approval.certificate_id,
                &approval.certificate_pem,
                &approval.chain_pem,
                &approval.fingerprint_sha256,
                &approval.spki_sha256,
                &approval.holder_sha256,
                &approval.algorithm.to_string(),
                &approval.payload_signature,
                &approval.document_signature,
                &approval.pdf_cms,
                &approval.revocation_status.to_string(),
                &approval.display_name,
            ],
        )
        .await
        .context("Error inserting the signing approval")?;
    if let Some(row) = inserted {
        return Ok(Ok(SigningApprovalRow::try_from(row)?));
    }
    let signed = hasura_transaction
        .query_one(
            "SELECT
                 coalesce(bool_or(user_id = $4), false),
                 coalesce(bool_or(fingerprint_sha256 = $5), false),
                 coalesce(bool_or(spki_sha256 = $6), false),
                 coalesce(bool_or(holder_sha256 = $7), false)
             FROM sequent_backend.signing_approval
             WHERE tenant_id = $1 AND election_event_id = $2 AND request_id = $3",
            &[
                &approval.tenant_id,
                &approval.election_event_id,
                &approval.request_id,
                &approval.user_id,
                &approval.fingerprint_sha256,
                &approval.spki_sha256,
                &approval.holder_sha256,
            ],
        )
        .await
        .context("Error reading the conflicting signing approval")?;
    let identities = [
        SigningApprovalConflict::DuplicateUser,
        SigningApprovalConflict::DuplicateCertificate,
        SigningApprovalConflict::DuplicateKey,
        SigningApprovalConflict::DuplicateHolder,
    ];
    for (column, conflict) in identities.into_iter().enumerate() {
        if signed.try_get::<_, bool>(column)? {
            return Ok(Err(conflict));
        }
    }
    Err(anyhow!(
        "the signing approval conflicts with no approval of its request"
    ))
}

/// The approvals of a request in the order they were signed.
#[instrument(skip(hasura_transaction), err)]
pub async fn list_signing_approvals(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    request_id: Uuid,
) -> Result<Vec<SigningApprovalRow>> {
    hasura_transaction
        .query(
            "SELECT * FROM sequent_backend.signing_approval
             WHERE tenant_id = $1 AND election_event_id = $2 AND request_id = $3
             ORDER BY signed_at, id",
            &[&tenant_id, &election_event_id, &request_id],
        )
        .await
        .context("Error listing signing approvals")?
        .into_iter()
        .map(SigningApprovalRow::try_from)
        .collect()
}

#[instrument(skip(hasura_transaction), err)]
pub async fn count_signing_approvals(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    request_id: Uuid,
) -> Result<i64> {
    Ok(hasura_transaction
        .query_one(
            "SELECT count(*) FROM sequent_backend.signing_approval
             WHERE tenant_id = $1 AND election_event_id = $2 AND request_id = $3",
            &[&tenant_id, &election_event_id, &request_id],
        )
        .await
        .context("Error counting signing approvals")?
        .try_get(0)?)
}

// Staff certificates

#[derive(Debug, Clone, PartialEq)]
pub struct StaffCertificateRow {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub user_id: String,
    pub username: String,
    pub user_display_name: Option<String>,
    pub election_id: Option<Uuid>,
    pub fingerprint_sha256: String,
    pub spki_sha256: String,
    pub holder_sha256: String,
    pub serial: String,
    pub subject: String,
    pub issuer: String,
    pub not_before: DateTime<Utc>,
    pub not_after: DateTime<Utc>,
    pub pem: String,
    pub status: StaffCertificateStatus,
    pub registration: StaffCertificateRegistration,
    pub linked_to: Option<String>,
    pub registered_by: String,
    pub registered_by_name: Option<String>,
    pub registered_at: DateTime<Utc>,
    pub revoked_by: Option<String>,
    pub revoked_by_name: Option<String>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revoke_reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<Row> for StaffCertificateRow {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self> {
        Ok(StaffCertificateRow {
            id: row.try_get("id")?,
            tenant_id: row.try_get("tenant_id")?,
            election_event_id: row.try_get("election_event_id")?,
            user_id: row.try_get("user_id")?,
            username: row.try_get("username")?,
            user_display_name: row.try_get("user_display_name")?,
            election_id: row.try_get("election_id")?,
            fingerprint_sha256: row.try_get("fingerprint_sha256")?,
            spki_sha256: row.try_get("spki_sha256")?,
            holder_sha256: row.try_get("holder_sha256")?,
            serial: row.try_get("serial")?,
            subject: row.try_get("subject")?,
            issuer: row.try_get("issuer")?,
            not_before: row.try_get("not_before")?,
            not_after: row.try_get("not_after")?,
            pem: row.try_get("pem")?,
            status: text_enum(&row, "status")?,
            registration: text_enum(&row, "registration")?,
            linked_to: row.try_get("linked_to")?,
            registered_by: row.try_get("registered_by")?,
            registered_by_name: row.try_get("registered_by_name")?,
            registered_at: row.try_get("registered_at")?,
            revoked_by: row.try_get("revoked_by")?,
            revoked_by_name: row.try_get("revoked_by_name")?,
            revoked_at: row.try_get("revoked_at")?,
            revoke_reason: row.try_get("revoke_reason")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

/// A registration to insert, `Active`.
#[derive(Debug, Clone)]
pub struct NewStaffCertificate {
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub user_id: String,
    pub username: String,
    pub user_display_name: Option<String>,
    pub election_id: Option<Uuid>,
    pub fingerprint_sha256: String,
    pub spki_sha256: String,
    pub holder_sha256: String,
    pub serial: String,
    pub subject: String,
    pub issuer: String,
    pub not_before: DateTime<Utc>,
    pub not_after: DateTime<Utc>,
    pub pem: String,
    pub registration: StaffCertificateRegistration,
    /// The account the certificate is already registered to, when a
    /// Security Officer links it to the holder's other account.
    pub linked_to: Option<String>,
    pub registered_by: String,
    pub registered_by_name: Option<String>,
}

#[instrument(skip_all, fields(user_id = %certificate.user_id), err)]
pub async fn insert_staff_certificate(
    hasura_transaction: &Transaction<'_>,
    certificate: &NewStaffCertificate,
) -> Result<StaffCertificateRow> {
    let row = hasura_transaction
        .query_one(
            "INSERT INTO sequent_backend.staff_certificate
                 (tenant_id, election_event_id, user_id, username, election_id,
                  fingerprint_sha256, spki_sha256, holder_sha256, serial, subject, issuer,
                  not_before, not_after, pem, status, registration, linked_to, registered_by,
                  user_display_name, registered_by_name)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16,
                     $17, $18, $19, $20)
             RETURNING *",
            &[
                &certificate.tenant_id,
                &certificate.election_event_id,
                &certificate.user_id,
                &certificate.username,
                &certificate.election_id,
                &certificate.fingerprint_sha256,
                &certificate.spki_sha256,
                &certificate.holder_sha256,
                &certificate.serial,
                &certificate.subject,
                &certificate.issuer,
                &certificate.not_before,
                &certificate.not_after,
                &certificate.pem,
                &StaffCertificateStatus::Active.to_string(),
                &certificate.registration.to_string(),
                &certificate.linked_to,
                &certificate.registered_by,
                &certificate.user_display_name,
                &certificate.registered_by_name,
            ],
        )
        .await
        .context("Error inserting the staff certificate")?;
    StaffCertificateRow::try_from(row)
}

const STAFF_CERTIFICATE_BY_ID: &str = "SELECT * FROM sequent_backend.staff_certificate
     WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3";

/// Reads a registration and share-locks it until the transaction ends: an
/// approval that signs with it blocks a concurrent revocation, which takes
/// [`lock_staff_certificate_for_update`].
#[instrument(skip(hasura_transaction), err)]
pub async fn lock_staff_certificate_for_share(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    id: Uuid,
) -> Result<Option<StaffCertificateRow>> {
    hasura_transaction
        .query_opt(
            &format!("{STAFF_CERTIFICATE_BY_ID} FOR SHARE"),
            &[&tenant_id, &election_event_id, &id],
        )
        .await
        .context("Error share-locking the staff certificate")?
        .map(StaffCertificateRow::try_from)
        .transpose()
}

/// Reads a registration and locks it for a change (a revocation) until the
/// transaction ends; it waits for the approvals signing with it.
#[instrument(skip(hasura_transaction), err)]
pub async fn lock_staff_certificate_for_update(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    id: Uuid,
) -> Result<Option<StaffCertificateRow>> {
    hasura_transaction
        .query_opt(
            &format!("{STAFF_CERTIFICATE_BY_ID} FOR UPDATE"),
            &[&tenant_id, &election_event_id, &id],
        )
        .await
        .context("Error locking the staff certificate")?
        .map(StaffCertificateRow::try_from)
        .transpose()
}

/// The active registrations of a certificate in the event: one, plus the
/// links a Security Officer made to the holder's other accounts.
#[instrument(skip(hasura_transaction), err)]
pub async fn find_active_staff_certificates_by_fingerprint(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    fingerprint_sha256: &str,
) -> Result<Vec<StaffCertificateRow>> {
    hasura_transaction
        .query(
            "SELECT * FROM sequent_backend.staff_certificate
             WHERE tenant_id = $1 AND election_event_id = $2 AND fingerprint_sha256 = $3
                 AND status = $4
             ORDER BY registered_at, id",
            &[
                &tenant_id,
                &election_event_id,
                &fingerprint_sha256,
                &StaffCertificateStatus::Active.to_string(),
            ],
        )
        .await
        .context("Error finding staff certificates by fingerprint")?
        .into_iter()
        .map(StaffCertificateRow::try_from)
        .collect()
}

/// What identifies a certificate's holder across the tenant.
#[derive(Debug, Clone, Copy)]
enum TenantWideIdentity {
    Certificate,
    Key,
    Holder,
}

impl TenantWideIdentity {
    fn column(self) -> &'static str {
        match self {
            TenantWideIdentity::Certificate => "fingerprint_sha256",
            TenantWideIdentity::Key => "spki_sha256",
            TenantWideIdentity::Holder => "holder_sha256",
        }
    }
}

async fn find_staff_certificates_in_tenant(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    identity: TenantWideIdentity,
    sha256: &str,
    status: StaffCertificateStatus,
) -> Result<Vec<StaffCertificateRow>> {
    let column = identity.column();
    hasura_transaction
        .query(
            &format!(
                "SELECT * FROM sequent_backend.staff_certificate
                 WHERE tenant_id = $1 AND {column} = $2 AND status = $3
                 ORDER BY registered_at, id"
            ),
            &[&tenant_id, &sha256, &status.to_string()],
        )
        .await
        .with_context(|| format!("Error finding {status} staff certificates by {column}"))?
        .into_iter()
        .map(StaffCertificateRow::try_from)
        .collect()
}

/// The active registrations of a key (SPKI) anywhere in the tenant.
#[instrument(skip(hasura_transaction), err)]
pub async fn find_active_staff_certificates_by_spki(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    spki_sha256: &str,
) -> Result<Vec<StaffCertificateRow>> {
    find_staff_certificates_in_tenant(
        hasura_transaction,
        tenant_id,
        TenantWideIdentity::Key,
        spki_sha256,
        StaffCertificateStatus::Active,
    )
    .await
}

/// The active registrations of a holder (subject name) anywhere in the
/// tenant.
#[instrument(skip(hasura_transaction), err)]
pub async fn find_active_staff_certificates_by_holder(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    holder_sha256: &str,
) -> Result<Vec<StaffCertificateRow>> {
    find_staff_certificates_in_tenant(
        hasura_transaction,
        tenant_id,
        TenantWideIdentity::Holder,
        holder_sha256,
        StaffCertificateStatus::Active,
    )
    .await
}

/// The revoked registrations of a certificate anywhere in the tenant. A
/// revocation sticks: first use can't register the certificate again, only
/// a Security Officer can.
#[instrument(skip(hasura_transaction), err)]
pub async fn find_revoked_staff_certificates_by_fingerprint(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    fingerprint_sha256: &str,
) -> Result<Vec<StaffCertificateRow>> {
    find_staff_certificates_in_tenant(
        hasura_transaction,
        tenant_id,
        TenantWideIdentity::Certificate,
        fingerprint_sha256,
        StaffCertificateStatus::Revoked,
    )
    .await
}

/// The revoked registrations of a key anywhere in the tenant, so that a
/// certificate reissued for a revoked key doesn't register on first use.
/// The holder is not blocked: a person may get a new key after a revocation.
#[instrument(skip(hasura_transaction), err)]
pub async fn find_revoked_staff_certificates_by_spki(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    spki_sha256: &str,
) -> Result<Vec<StaffCertificateRow>> {
    find_staff_certificates_in_tenant(
        hasura_transaction,
        tenant_id,
        TenantWideIdentity::Key,
        spki_sha256,
        StaffCertificateStatus::Revoked,
    )
    .await
}

// Electoral log outbox

#[derive(Debug, Clone)]
pub struct SigningLogOutboxRow {
    pub id: i64,
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub step_id: Uuid,
    pub statement_kind: SigningStatementKind,
    pub event_type: StatementEventType,
    pub log_type: StatementLogType,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub election_id: Option<Uuid>,
    pub area_id: Option<Uuid>,
    pub body: Value,
    pub occurred_at: DateTime<Utc>,
    pub posted_at: Option<DateTime<Utc>>,
    pub attempts: i32,
    pub last_error: Option<String>,
}

impl TryFrom<Row> for SigningLogOutboxRow {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self> {
        Ok(SigningLogOutboxRow {
            id: row.try_get("id")?,
            tenant_id: row.try_get("tenant_id")?,
            election_event_id: row.try_get("election_event_id")?,
            step_id: row.try_get("step_id")?,
            statement_kind: log_enum(&row, "statement_kind")?,
            event_type: log_enum(&row, "event_type")?,
            log_type: log_enum(&row, "log_type")?,
            user_id: row.try_get("user_id")?,
            username: row.try_get("username")?,
            election_id: row.try_get("election_id")?,
            area_id: row.try_get("area_id")?,
            body: row.try_get("body")?,
            occurred_at: row.try_get("occurred_at")?,
            posted_at: row.try_get("posted_at")?,
            attempts: row.try_get("attempts")?,
            last_error: row.try_get("last_error")?,
        })
    }
}

/// One entry of a step. USER entries name their user; SYSTEM entries don't.
#[derive(Debug, Clone)]
pub struct NewSigningLogOutboxEntry {
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub step_id: Uuid,
    pub statement_kind: SigningStatementKind,
    pub event_type: StatementEventType,
    pub log_type: StatementLogType,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub election_id: Option<Uuid>,
    pub area_id: Option<Uuid>,
    pub body: Value,
    /// When the step happened; both entries of a step share it.
    pub occurred_at: DateTime<Utc>,
}

/// Queues an entry; returns its id, the order it is posted in. It takes the
/// event's signing lock (see [`lock_signing_event`]) first, so entries are
/// queued in commit order even if the caller forgot it.
#[instrument(skip_all, fields(step_id = %entry.step_id), err)]
pub async fn insert_signing_log_outbox(
    hasura_transaction: &Transaction<'_>,
    entry: &NewSigningLogOutboxEntry,
) -> Result<i64> {
    lock_signing_event(hasura_transaction, entry.tenant_id, entry.election_event_id).await?;
    Ok(hasura_transaction
        .query_one(
            "INSERT INTO sequent_backend.signing_log_outbox
                 (tenant_id, election_event_id, step_id, entry, statement_kind, event_type,
                  log_type, user_id, username, election_id, area_id, body, occurred_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
             RETURNING id",
            &[
                &entry.tenant_id,
                &entry.election_event_id,
                &entry.step_id,
                &entry_of(&entry.event_type),
                &entry.statement_kind.to_string(),
                &entry.event_type.to_string(),
                &entry.log_type.to_string(),
                &entry.user_id,
                &entry.username,
                &entry.election_id,
                &entry.area_id,
                &entry.body,
                &entry.occurred_at,
            ],
        )
        .await
        .context("Error queueing the signing log entry")?
        .try_get(0)?)
}

/// Up to `limit` unposted entries of the event in the order they were
/// queued, locked until the transaction ends. The worker takes
/// [`lock_signing_event`] first, so it waits for the steps being written and
/// sees them in commit order; it skips nothing, since order matters more.
#[instrument(skip(hasura_transaction), err)]
pub async fn fetch_unposted_signing_log_outbox(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    limit: i64,
) -> Result<Vec<SigningLogOutboxRow>> {
    fetch_unposted_signing_log_outbox_rows(
        hasura_transaction,
        tenant_id,
        election_event_id,
        i64::MAX,
        limit,
    )
    .await?
    .into_iter()
    .map(|(_, row)| row)
    .collect()
}

/// [`fetch_unposted_signing_log_outbox`] up to the id `up_to_id`, each row
/// read on its own: a row this version can't read comes back as its id and
/// the error, so the worker can mark it failed and stop there.
#[instrument(skip(hasura_transaction), err)]
pub async fn fetch_unposted_signing_log_outbox_rows(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    up_to_id: i64,
    limit: i64,
) -> Result<Vec<(i64, Result<SigningLogOutboxRow>)>> {
    hasura_transaction
        .query(
            "SELECT * FROM sequent_backend.signing_log_outbox
             WHERE tenant_id = $1 AND election_event_id = $2 AND posted_at IS NULL
                 AND id <= $3
             ORDER BY id
             LIMIT $4
             FOR UPDATE",
            &[&tenant_id, &election_event_id, &up_to_id, &limit],
        )
        .await
        .context("Error fetching unposted signing log entries")?
        .into_iter()
        .map(|row| Ok((row.try_get("id")?, SigningLogOutboxRow::try_from(row))))
        .collect()
}

#[instrument(skip(hasura_transaction), err)]
pub async fn mark_signing_log_outbox_posted(
    hasura_transaction: &Transaction<'_>,
    id: i64,
) -> Result<()> {
    hasura_transaction
        .execute(
            "UPDATE sequent_backend.signing_log_outbox
             SET posted_at = clock_timestamp(), last_error = NULL
             WHERE id = $1 AND posted_at IS NULL",
            &[&id],
        )
        .await
        .context("Error marking the signing log entry posted")?;
    Ok(())
}

/// Counts a failed attempt; returns the attempts so far (0 if the entry
/// was posted meanwhile).
#[instrument(skip(hasura_transaction), err)]
pub async fn mark_signing_log_outbox_failed(
    hasura_transaction: &Transaction<'_>,
    id: i64,
    error: &str,
) -> Result<i32> {
    Ok(hasura_transaction
        .query_opt(
            "UPDATE sequent_backend.signing_log_outbox
             SET attempts = attempts + 1, last_error = $2, last_attempt_at = clock_timestamp()
             WHERE id = $1 AND posted_at IS NULL
             RETURNING attempts",
            &[&id, &error],
        )
        .await
        .context("Error marking the signing log entry failed")?
        .map(|row| row.try_get(0))
        .transpose()?
        .unwrap_or(0))
}

/// The (tenant, election event) pairs with entries still to post.
#[instrument(skip(hasura_transaction), err)]
pub async fn list_signing_log_outbox_events(
    hasura_transaction: &Transaction<'_>,
) -> Result<Vec<(Uuid, Uuid)>> {
    hasura_transaction
        .query(
            "SELECT DISTINCT tenant_id, election_event_id
             FROM sequent_backend.signing_log_outbox
             WHERE posted_at IS NULL",
            &[],
        )
        .await
        .context("Error listing the events with unposted signing log entries")?
        .into_iter()
        .map(|row| Ok((row.try_get(0)?, row.try_get(1)?)))
        .collect()
}

/// The event's unposted entries: the first one's attempts, the last id,
/// and the database's clock.
#[derive(Debug, Clone, PartialEq)]
pub struct SigningLogOutboxSpan {
    pub first_id: i64,
    pub last_id: i64,
    pub attempts: i32,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub now: DateTime<Utc>,
}

#[instrument(skip(hasura_transaction), err)]
pub async fn unposted_signing_log_outbox_span(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
) -> Result<Option<SigningLogOutboxSpan>> {
    hasura_transaction
        .query_opt(
            "SELECT id, attempts, last_attempt_at, clock_timestamp() AS now,
                 (SELECT max(id) FROM sequent_backend.signing_log_outbox
                  WHERE tenant_id = $1 AND election_event_id = $2 AND posted_at IS NULL)
                     AS last_id
             FROM sequent_backend.signing_log_outbox
             WHERE tenant_id = $1 AND election_event_id = $2 AND posted_at IS NULL
             ORDER BY id
             LIMIT 1",
            &[&tenant_id, &election_event_id],
        )
        .await
        .context("Error reading the unposted signing log entries")?
        .map(|row| {
            Ok(SigningLogOutboxSpan {
                first_id: row.try_get("id")?,
                last_id: row.try_get("last_id")?,
                attempts: row.try_get("attempts")?,
                last_attempt_at: row.try_get("last_attempt_at")?,
                now: row.try_get("now")?,
            })
        })
        .transpose()
}

/// A person whose entries wait to be posted, with the Post and country of
/// the first of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SigningLogOutboxUser {
    pub user_id: String,
    pub username: Option<String>,
    pub election_id: Option<Uuid>,
    pub area_id: Option<Uuid>,
}

/// The people of the event's unposted USER entries up to `up_to_id`. It
/// locks nothing.
#[instrument(skip(hasura_transaction), err)]
pub async fn unposted_signing_log_outbox_users(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    up_to_id: i64,
) -> Result<Vec<SigningLogOutboxUser>> {
    hasura_transaction
        .query(
            "SELECT DISTINCT ON (user_id) user_id, username, election_id, area_id
             FROM sequent_backend.signing_log_outbox
             WHERE tenant_id = $1 AND election_event_id = $2 AND posted_at IS NULL
                 AND id <= $3 AND user_id IS NOT NULL
             ORDER BY user_id, id",
            &[&tenant_id, &election_event_id, &up_to_id],
        )
        .await
        .context("Error reading the people of the unposted signing log entries")?
        .into_iter()
        .map(|row| {
            Ok(SigningLogOutboxUser {
                user_id: row.try_get("user_id")?,
                username: row.try_get("username")?,
                election_id: row.try_get("election_id")?,
                area_id: row.try_get("area_id")?,
            })
        })
        .collect()
}

// Rules (lists)

/// The saved rules of an event, in action order of their text.
#[instrument(skip(hasura_transaction), err)]
pub async fn list_signing_rules(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
) -> Result<Vec<SigningRuleRow>> {
    hasura_transaction
        .query(
            "SELECT * FROM sequent_backend.signing_rule
             WHERE tenant_id = $1 AND election_event_id = $2
             ORDER BY action",
            &[&tenant_id, &election_event_id],
        )
        .await
        .context("Error listing the signing rules")?
        .into_iter()
        .map(SigningRuleRow::try_from)
        .collect()
}

/// The events of the tenant that have a saved rule for `action`.
#[instrument(skip(hasura_transaction), err)]
pub async fn list_events_with_signing_rule(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    action: SigningAction,
) -> Result<Vec<Uuid>> {
    Ok(hasura_transaction
        .query(
            "SELECT election_event_id FROM sequent_backend.signing_rule
             WHERE tenant_id = $1 AND action = $2
             ORDER BY election_event_id",
            &[&tenant_id, &action.to_string()],
        )
        .await
        .context("Error listing the events with a signing rule")?
        .into_iter()
        .map(|row| row.get(0))
        .collect())
}

// Requests (lookups)

/// The request waiting for an action and scope key, locked until the
/// transaction ends.
#[instrument(skip(hasura_transaction), err)]
pub async fn lock_waiting_signing_request(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    action: SigningAction,
    scope_key: &str,
) -> Result<Option<SigningRequestRow>> {
    hasura_transaction
        .query_opt(
            "SELECT * FROM sequent_backend.signing_request
             WHERE tenant_id = $1 AND election_event_id = $2 AND action = $3
                 AND scope_key = $4 AND status = $5
             FOR UPDATE",
            &[
                &tenant_id,
                &election_event_id,
                &action.to_string(),
                &scope_key,
                &SigningRequestStatus::Waiting.to_string(),
            ],
        )
        .await
        .context("Error reading the waiting signing request")?
        .map(SigningRequestRow::try_from)
        .transpose()
}

/// Counts one more certificate file that didn't open; the new count.
#[instrument(skip(hasura_transaction), err)]
pub async fn increment_signing_request_open_failures(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    id: Uuid,
) -> Result<i32> {
    Ok(hasura_transaction
        .query_one(
            "UPDATE sequent_backend.signing_request SET open_failures = open_failures + 1
             WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3
             RETURNING open_failures",
            &[&tenant_id, &election_event_id, &id],
        )
        .await
        .context("Error counting the certificate open failure")?
        .try_get(0)?)
}

/// Records the task that runs a dispatched execution.
#[instrument(skip(hasura_transaction), err)]
pub async fn set_signing_request_task_execution(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    id: Uuid,
    task_execution_id: Uuid,
) -> Result<()> {
    hasura_transaction
        .execute(
            "UPDATE sequent_backend.signing_request SET task_execution_id = $4
             WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3",
            &[&tenant_id, &election_event_id, &id, &task_execution_id],
        )
        .await
        .context("Error recording the signing request's task")?;
    Ok(())
}

/// Up to `limit` requests of every tenant still waiting though their time
/// is up by the database's clock, oldest first.
#[instrument(skip(hasura_transaction), err)]
pub async fn list_overdue_signing_requests(
    hasura_transaction: &Transaction<'_>,
    limit: i64,
) -> Result<Vec<SigningRequestRow>> {
    hasura_transaction
        .query(
            "SELECT * FROM sequent_backend.signing_request
             WHERE status = $1 AND expires_at IS NOT NULL AND expires_at <= clock_timestamp()
             ORDER BY expires_at, id
             LIMIT $2",
            &[&SigningRequestStatus::Waiting.to_string(), &limit],
        )
        .await
        .context("Error listing overdue signing requests")?
        .into_iter()
        .map(SigningRequestRow::try_from)
        .collect()
}

/// Whether a request's time is up by the database's clock.
#[instrument(skip(hasura_transaction), err)]
pub async fn signing_request_is_overdue(
    hasura_transaction: &Transaction<'_>,
    request: &SigningRequestRow,
) -> Result<bool> {
    let Some(expires_at) = request.expires_at else {
        return Ok(false);
    };
    Ok(hasura_transaction
        .query_one(
            "SELECT $1::timestamptz <= clock_timestamp()",
            &[&expires_at],
        )
        .await
        .context("Error reading the database clock")?
        .try_get(0)?)
}

/// A completed request a task should run again: never claimed, its claim
/// older than `lease_seconds`, or its task failed.
#[derive(Debug, Clone)]
pub struct UnexecutedSigningRequest {
    pub request: SigningRequestRow,
    pub task_failed: bool,
}

/// Up to `limit` requests of `actions` in every tenant, completed at least
/// `completed_seconds` ago by the database's clock and not executed, whose
/// execution is unclaimed, stale or failed; oldest first.
#[instrument(skip(hasura_transaction), err)]
pub async fn list_unexecuted_signing_requests(
    hasura_transaction: &Transaction<'_>,
    actions: &[SigningAction],
    completed_seconds: i64,
    lease_seconds: i64,
    limit: i64,
) -> Result<Vec<UnexecutedSigningRequest>> {
    let actions: Vec<String> = actions.iter().map(ToString::to_string).collect();
    hasura_transaction
        .query(
            "SELECT r.*, (t.execution_status IS NOT DISTINCT FROM 'FAILED') AS task_failed
             FROM sequent_backend.signing_request r
             LEFT JOIN sequent_backend.tasks_execution t ON t.id = r.task_execution_id
             WHERE r.status = $1 AND r.executed_at IS NULL AND r.action = ANY($2)
                 AND r.completed_at <= clock_timestamp() - make_interval(secs => $3::bigint)
                 AND (r.execution_started_at IS NULL
                      OR r.execution_started_at
                          <= clock_timestamp() - make_interval(secs => $4::bigint)
                      OR t.execution_status = 'FAILED')
             ORDER BY r.completed_at, r.id
             LIMIT $5",
            &[
                &SigningRequestStatus::Completed.to_string(),
                &actions,
                &completed_seconds,
                &lease_seconds,
                &limit,
            ],
        )
        .await
        .context("Error listing unexecuted signing requests")?
        .into_iter()
        .map(|row| {
            let task_failed: bool = row.try_get("task_failed")?;
            Ok(UnexecutedSigningRequest {
                request: SigningRequestRow::try_from(row)?,
                task_failed,
            })
        })
        .collect()
}

/// Claims a completed request's execution for the task `task_execution_id`
/// it was dispatched to: once, unless the claim is older than
/// `lease_seconds` or `released` says the earlier task failed. `None` when
/// another copy holds it, or it ran, or it belongs to another task.
#[instrument(skip(hasura_transaction), err)]
pub async fn claim_signing_execution(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    id: Uuid,
    task_execution_id: Uuid,
    lease_seconds: i64,
) -> Result<Option<SigningRequestRow>> {
    hasura_transaction
        .query_opt(
            "UPDATE sequent_backend.signing_request SET
                 execution_started_at = clock_timestamp(),
                 execution_attempts = execution_attempts + 1
             WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3
                 AND status = $4 AND executed_at IS NULL AND task_execution_id = $5
                 AND (execution_started_at IS NULL
                      OR execution_started_at
                          <= clock_timestamp() - make_interval(secs => $6::bigint))
             RETURNING *",
            &[
                &tenant_id,
                &election_event_id,
                &id,
                &SigningRequestStatus::Completed.to_string(),
                &task_execution_id,
                &lease_seconds,
            ],
        )
        .await
        .context("Error claiming the signing execution")?
        .map(SigningRequestRow::try_from)
        .transpose()
}

/// Drops the claim of an execution whose task failed, so a new copy can
/// claim it.
#[instrument(skip(hasura_transaction), err)]
pub async fn release_signing_execution(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    id: Uuid,
) -> Result<()> {
    hasura_transaction
        .execute(
            "UPDATE sequent_backend.signing_request SET execution_started_at = NULL
             WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3 AND executed_at IS NULL",
            &[&tenant_id, &election_event_id, &id],
        )
        .await
        .context("Error releasing the signing execution")?;
    Ok(())
}

/// The completed request waiting for the same action and scope key to run,
/// if any.
#[instrument(skip(hasura_transaction), err)]
pub async fn find_unexecuted_signing_request(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    action: SigningAction,
    scope_key: &str,
) -> Result<Option<SigningRequestRow>> {
    hasura_transaction
        .query_opt(
            "SELECT * FROM sequent_backend.signing_request
             WHERE tenant_id = $1 AND election_event_id = $2 AND action = $3
                 AND scope_key = $4 AND status = $5 AND executed_at IS NULL
             ORDER BY completed_at DESC LIMIT 1",
            &[
                &tenant_id,
                &election_event_id,
                &action.to_string(),
                &scope_key,
                &SigningRequestStatus::Completed.to_string(),
            ],
        )
        .await
        .context("Error reading the unexecuted signing request")?
        .map(SigningRequestRow::try_from)
        .transpose()
}

/// How many log steps of `kind` the user staged for a request in the last
/// `seconds`, by the database's clock.
#[instrument(skip(hasura_transaction), err)]
pub async fn count_recent_signing_steps(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    request_id: Uuid,
    kind: SigningStatementKind,
    user_id: &str,
    seconds: i64,
) -> Result<i64> {
    Ok(hasura_transaction
        .query_one(
            "SELECT count(*) FROM sequent_backend.signing_log_outbox
             WHERE tenant_id = $1 AND election_event_id = $2 AND entry = 0
                 AND statement_kind = $3 AND user_id = $4
                 AND body->'details'->>'request_id' = $5
                 AND occurred_at > clock_timestamp() - make_interval(secs => $6::bigint)",
            &[
                &tenant_id,
                &election_event_id,
                &kind.to_string(),
                &user_id,
                &request_id.to_string(),
                &seconds,
            ],
        )
        .await
        .context("Error counting recent signing steps")?
        .try_get(0)?)
}

/// A trustee of the tenant by name: its id.
#[instrument(skip(hasura_transaction), err)]
pub async fn find_signing_trustee_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    name: &str,
) -> Result<Option<Uuid>> {
    Ok(hasura_transaction
        .query_opt(
            "SELECT id FROM sequent_backend.trustee WHERE tenant_id = $1 AND name = $2
             ORDER BY id LIMIT 1",
            &[&tenant_id, &name],
        )
        .await
        .context("Error reading the trustee")?
        .map(|row| row.get(0)))
}

/// Whether the event has the area.
#[instrument(skip(hasura_transaction), err)]
pub async fn signing_area_exists(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    area_id: Uuid,
) -> Result<bool> {
    Ok(hasura_transaction
        .query_opt(
            "SELECT 1 FROM sequent_backend.area
             WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3",
            &[&tenant_id, &election_event_id, &area_id],
        )
        .await
        .context("Error reading the country")?
        .is_some())
}

/// The requests of an event, oldest first.
#[instrument(skip(hasura_transaction), err)]
pub async fn list_signing_requests(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
) -> Result<Vec<SigningRequestRow>> {
    hasura_transaction
        .query(
            "SELECT * FROM sequent_backend.signing_request
             WHERE tenant_id = $1 AND election_event_id = $2
             ORDER BY created_at, id",
            &[&tenant_id, &election_event_id],
        )
        .await
        .context("Error listing signing requests")?
        .into_iter()
        .map(SigningRequestRow::try_from)
        .collect()
}

/// The approvals of every request of an event in the order they were
/// signed.
#[instrument(skip(hasura_transaction), err)]
pub async fn list_event_signing_approvals(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
) -> Result<Vec<SigningApprovalRow>> {
    hasura_transaction
        .query(
            "SELECT * FROM sequent_backend.signing_approval
             WHERE tenant_id = $1 AND election_event_id = $2
             ORDER BY signed_at, id",
            &[&tenant_id, &election_event_id],
        )
        .await
        .context("Error listing the event's signing approvals")?
        .into_iter()
        .map(SigningApprovalRow::try_from)
        .collect()
}

// What requests are about

/// A Post: an election of the event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SigningPost {
    pub id: Uuid,
    pub name: String,
    pub permission_label: Option<String>,
}

/// The Post's name as the portal shows it.
fn signing_post(row: &Row) -> Result<SigningPost> {
    let id: Uuid = row.try_get("id")?;
    Ok(SigningPost {
        id,
        name: crate::services::monitoring::snapshot::post_name(
            row.try_get("presentation")?,
            row.try_get("external_id")?,
            id,
        ),
        permission_label: row.try_get("permission_label")?,
    })
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_signing_post(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    election_id: Uuid,
) -> Result<Option<SigningPost>> {
    hasura_transaction
        .query_opt(
            "SELECT id, presentation, external_id, permission_label
             FROM sequent_backend.election
             WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3",
            &[&tenant_id, &election_event_id, &election_id],
        )
        .await
        .context("Error reading the Post")?
        .as_ref()
        .map(signing_post)
        .transpose()
}

/// The Posts of an event, by name.
#[instrument(skip(hasura_transaction), err)]
pub async fn list_signing_posts(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
) -> Result<Vec<SigningPost>> {
    let mut posts = hasura_transaction
        .query(
            "SELECT id, presentation, external_id, permission_label
             FROM sequent_backend.election
             WHERE tenant_id = $1 AND election_event_id = $2",
            &[&tenant_id, &election_event_id],
        )
        .await
        .context("Error listing the Posts")?
        .iter()
        .map(signing_post)
        .collect::<Result<Vec<_>>>()?;
    posts.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
    Ok(posts)
}

/// The name of a country (area) of the event.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_signing_area_name(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    area_id: Uuid,
) -> Result<Option<String>> {
    Ok(hasura_transaction
        .query_opt(
            "SELECT name FROM sequent_backend.area
             WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3",
            &[&tenant_id, &election_event_id, &area_id],
        )
        .await
        .context("Error reading the country")?
        .and_then(|row| row.get::<_, Option<String>>(0)))
}

/// The name of a trustee of the tenant, which is the trustee's username.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_signing_trustee_name(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    trustee_id: Uuid,
) -> Result<Option<String>> {
    Ok(hasura_transaction
        .query_opt(
            "SELECT name FROM sequent_backend.trustee WHERE tenant_id = $1 AND id = $2",
            &[&tenant_id, &trustee_id],
        )
        .await
        .context("Error reading the trustee")?
        .and_then(|row| row.get::<_, Option<String>>(0)))
}
