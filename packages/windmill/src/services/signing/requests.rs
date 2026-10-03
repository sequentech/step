// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! A signing request's life besides its signatures: what its panel shows,
//! cancelling it, handing the laptop to the next member, a certificate file
//! that didn't open, expiry, the export of an event's requests, and the
//! executions tasks report back.
//!
//! Every step takes the event's signing lock before the request's row lock
//! (see [`lock_signing_event`]).

use super::approve::{log_step_refusal, DocumentSigner, RefusedStep};
use super::executors::SigningExecutorRegistry;
use super::log::{stage, Actor, LogStep, SystemOutcome};
use super::pdf::DocumentRevisionView;
use super::signers::{list_signers, GroupChange};
use super::{
    action_title, allowed_by, allowed_by_permission, log_scope, Allowance, SigningCaller,
    SigningError, SigningResult,
};
use crate::postgres::signing::*;
use crate::postgres::signing_certificates::get_signing_request_in_tenant;
use crate::services::documents::{get_document_url, upload_and_return_document};
use crate::tasks::signing_log_outbox::kick_signing_log_outbox;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deadpool_postgres::{Client, Transaction};
use electoral_log::messages::newtypes::SigningStatementKind;
use openssl::nid::Nid;
use openssl::x509::X509;
use sequent_core::services::keycloak::get_tenant_realm;
use sequent_core::signing::{
    sha256_hex, CancelReason, CertificateOpenFailure, ExecutionMode, SigningAction,
    SigningRequestStatus, SigningRule,
};
use sequent_core::types::permissions::Permissions;
use sequent_core::util::temp_path::write_into_named_temp_file;
use serde::Serialize;
use serde_json::{json, Map, Value};
use strum::IntoEnumIterator;
use tracing::{instrument, warn};
use uuid::Uuid;

/// How many requests one expiry or sweeper pass handles.
const BATCH: i64 = 500;
/// The longest certificate file name that is logged.
const MAX_FILE_NAME: usize = 255;

// Log steps of a request

/// The details every step of a request logs.
pub fn request_details(request: &SigningRequestRow) -> Map<String, Value> {
    let mut details = Map::new();
    details.insert("election_id".into(), json!(request.election_id));
    details.insert("area_id".into(), json!(request.area_id));
    details.insert("action".into(), json!(request.action.to_string()));
    details.insert("request_id".into(), json!(request.id));
    details.insert("code".into(), json!(request.code));
    details
}

/// Stages a step of `request` with the request's details plus `extra`.
pub async fn stage_request_step(
    hasura_transaction: &Transaction<'_>,
    request: &SigningRequestRow,
    kind: SigningStatementKind,
    user: Actor,
    system: SystemOutcome,
    description: String,
    extra: Value,
) -> Result<Uuid> {
    let mut details = request_details(request);
    if let Value::Object(extra) = extra {
        details.extend(extra);
    }
    stage(
        hasura_transaction,
        &LogStep {
            kind,
            user,
            system,
            scope: log_scope(
                request.tenant_id,
                request.election_event_id,
                request.election_id,
                request.area_id,
            ),
            description,
            details: Value::Object(details),
        },
    )
    .await
}

/// The person who started a request, who is logged for what happens to it
/// on its own (expiry, a task's result).
pub fn requester(request: &SigningRequestRow) -> Actor {
    Actor {
        user_id: request.requested_by.clone(),
        username: request.requested_by_username.clone(),
    }
}

fn cancel_reason_text(reason: CancelReason) -> &'static str {
    match reason {
        CancelReason::ByRequester => "cancelled by the person who started it",
        CancelReason::ByOperator => "cancelled by an operator",
        CancelReason::RuleChanged => "the signing rule changed",
        CancelReason::PayloadChanged => "what it signs changed",
        CancelReason::Superseded => "a newer request replaced it",
        CancelReason::CertificateRevoked => "the certificate of a signature it counted was revoked",
    }
}

/// What allowed a cancel for `reason`, when a person's own step did; a
/// request replaced by a new one is cancelled by whoever started that one.
fn cancel_allowance(reason: CancelReason) -> Option<Allowance> {
    match reason {
        CancelReason::ByRequester => Some(Allowance::Requester),
        CancelReason::ByOperator => {
            Some(Allowance::Permission(Permissions::SIGNING_REQUESTS_CANCEL))
        }
        CancelReason::RuleChanged => Some(Allowance::Permission(Permissions::SIGNING_RULES_WRITE)),
        CancelReason::CertificateRevoked => Some(Allowance::Permission(
            Permissions::SIGNING_CERTIFICATES_REVOKE,
        )),
        CancelReason::PayloadChanged | CancelReason::Superseded => None,
    }
}

/// The signatures of a cancelled request, which no longer count: each
/// approval with its signer and certificate.
pub fn voided_approvals(approvals: &[SigningApprovalRow]) -> Value {
    Value::from(
        approvals
            .iter()
            .map(|approval| {
                json!({
                    "approval_id": approval.id,
                    "user_id": approval.user_id,
                    "username": approval.username,
                    "fingerprint": approval.fingerprint_sha256,
                })
            })
            .collect::<Vec<_>>(),
    )
}

/// Cancels a waiting request and stages its SigningRequestCancelled step,
/// which lists the signatures that no longer count and, for a person's own
/// cancel, what allowed it. The caller holds the event lock and the
/// request's row lock.
pub async fn cancel_request(
    hasura_transaction: &Transaction<'_>,
    request: &SigningRequestRow,
    reason: CancelReason,
    by: Actor,
    note: Option<&str>,
) -> Result<SigningRequestRow> {
    let cancelled = update_signing_request_status(
        hasura_transaction,
        request.tenant_id,
        request.election_event_id,
        request.id,
        &SigningRequestTransition::Cancel {
            reason,
            by: Some(by.user_id.clone()),
        },
    )
    .await?
    .ok_or_else(|| anyhow!("signing request {} is not waiting", request.id))?;
    let approvals = list_signing_approvals(
        hasura_transaction,
        cancelled.tenant_id,
        cancelled.election_event_id,
        cancelled.id,
    )
    .await?;
    let mut details = json!({
        "reason": reason.to_string(),
        "note": note,
        "voided_approvals": voided_approvals(&approvals),
    });
    if let Some(allowance) = cancel_allowance(reason) {
        details["allowed_by"] = allowed_by(&[allowance]);
    }
    stage_request_step(
        hasura_transaction,
        &cancelled,
        SigningStatementKind::SigningRequestCancelled,
        by,
        SystemOutcome::Info,
        format!(
            "Cancelled signing request {}: {}",
            cancelled.code,
            cancel_reason_text(reason)
        ),
        details,
    )
    .await?;
    Ok(cancelled)
}

/// Expires a waiting request whose time is up and stages its
/// SigningRequestExpired step for the requester; `None` when the database
/// finds it still waiting within its time. The caller holds the event lock
/// and the request's row lock.
pub async fn expire_request(
    hasura_transaction: &Transaction<'_>,
    request: &SigningRequestRow,
) -> Result<Option<SigningRequestRow>> {
    let Some(expired) = update_signing_request_status(
        hasura_transaction,
        request.tenant_id,
        request.election_event_id,
        request.id,
        &SigningRequestTransition::Expire,
    )
    .await?
    else {
        return Ok(None);
    };
    let count = count_signing_approvals(
        hasura_transaction,
        expired.tenant_id,
        expired.election_event_id,
        expired.id,
    )
    .await?;
    stage_request_step(
        hasura_transaction,
        &expired,
        SigningStatementKind::SigningRequestExpired,
        requester(&expired),
        SystemOutcome::Info,
        format!(
            "Signing request {} expired with {} of {} signatures",
            expired.code, count, expired.required
        ),
        json!({ "count": count, "required": expired.required, "expires_at": expired.expires_at }),
    )
    .await?;
    Ok(Some(expired))
}

/// Whether a waiting request's time is up at `now`.
/// Whether a request's time is up, by the database's clock: read after the
/// locks, so a wait for them doesn't count against the request.
pub async fn is_overdue(
    hasura_transaction: &Transaction<'_>,
    request: &SigningRequestRow,
) -> Result<bool> {
    signing_request_is_overdue(hasura_transaction, request).await
}

/// Reads a request of the caller's tenant without locking it: enough to
/// decide who may act on it, since its action, Post, requester and trustee
/// never change.
pub async fn read_request(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    request_id: Uuid,
) -> SigningResult<SigningRequestRow> {
    get_signing_request_in_tenant(hasura_transaction, tenant_id, request_id)
        .await?
        .ok_or_else(not_found)
}

/// Takes the request's event signing lock, then its row lock, and reads it
/// again.
pub async fn relock(
    hasura_transaction: &Transaction<'_>,
    request: &SigningRequestRow,
) -> SigningResult<SigningRequestRow> {
    lock_signing_event(
        hasura_transaction,
        request.tenant_id,
        request.election_event_id,
    )
    .await?;
    lock_signing_request(
        hasura_transaction,
        request.tenant_id,
        request.election_event_id,
        request.id,
    )
    .await?
    .ok_or_else(not_found)
}

/// [`read_request`] then [`relock`].
pub async fn lock_request(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    request_id: Uuid,
) -> SigningResult<SigningRequestRow> {
    let found = read_request(hasura_transaction, tenant_id, request_id).await?;
    relock(hasura_transaction, &found).await
}

fn not_found() -> SigningError {
    SigningError::NotFound("There is no such signing request.".into())
}

fn is_requester(caller: &SigningCaller, request: &SigningRequestRow) -> bool {
    caller.user_id == request.requested_by
}

/// Whether the caller holds the request's sign permission and signs for its
/// Post.
pub fn can_sign(caller: &SigningCaller, request: &SigningRequestRow) -> bool {
    caller.has(request.action.sign_permission())
        && caller.signs_for(request.permission_label.as_deref())
}

/// Whether the caller is the trustee of a trustee's request: their token's
/// trustee, looked up in the tenant, is the request's by id.
pub async fn is_the_trustee(
    hasura_transaction: &Transaction<'_>,
    caller: &SigningCaller,
    request: &SigningRequestRow,
) -> Result<bool> {
    let (Some(trustee_id), Some(name)) = (request.trustee_id, caller.trustee.as_deref()) else {
        return Ok(false);
    };
    Ok(
        find_signing_trustee_id(hasura_transaction, request.tenant_id, name).await?
            == Some(trustee_id),
    )
}

/// [`can_sign`], and for a trustee's request, being that trustee.
pub async fn may_sign(
    hasura_transaction: &Transaction<'_>,
    caller: &SigningCaller,
    request: &SigningRequestRow,
) -> Result<bool> {
    if !can_sign(caller, request) {
        return Ok(false);
    }
    if request.action.is_trustee() {
        return is_the_trustee(hasura_transaction, caller, request).await;
    }
    Ok(true)
}

/// Seconds within which a person's repeated step of one kind on a request
/// is not logged again: a refused signature or step, a handover, a
/// certificate file that didn't open. The step is still answered (and an
/// open failure still counted); only its log entry is left out.
pub const LOG_THROTTLE_SECONDS: i64 = 10;

/// Whether the caller logged a step of `kind` on the request in the last
/// [`LOG_THROTTLE_SECONDS`].
pub async fn throttled(
    hasura_transaction: &Transaction<'_>,
    request: &SigningRequestRow,
    kind: SigningStatementKind,
    caller: &SigningCaller,
) -> Result<bool> {
    Ok(count_recent_signing_steps(
        hasura_transaction,
        request.tenant_id,
        request.election_event_id,
        request.id,
        kind,
        &caller.user_id,
        LOG_THROTTLE_SECONDS,
    )
    .await?
        > 0)
}

/// Whether a refused step's transaction is committed, to keep its logged
/// refusal: a forbidden cancel, handover or open-failure report is logged
/// (as a forbidden signature is). Any other error rolls the step back.
pub fn refusal_is_logged(error: &SigningError) -> bool {
    matches!(error, SigningError::Forbidden(_))
}

/// Logs that the caller may not take `step` on the request, after taking
/// its locks, and answers `message` as forbidden.
async fn forbid(
    hasura_transaction: &Transaction<'_>,
    caller: &SigningCaller,
    found: &SigningRequestRow,
    step: RefusedStep,
    message: &str,
) -> SigningResult<SigningError> {
    let request = relock(hasura_transaction, found).await?;
    log_step_refusal(
        hasura_transaction,
        &request,
        caller,
        step,
        "forbidden",
        message,
        None,
    )
    .await?;
    Ok(SigningError::Forbidden(message.into()))
}

fn require_waiting(request: &SigningRequestRow) -> SigningResult<()> {
    if request.status != SigningRequestStatus::Waiting {
        return Err(SigningError::Closed {
            status: request.status,
            message: format!(
                "Signing request {} is {}, not waiting.",
                request.code, request.status
            ),
        });
    }
    Ok(())
}

// The panel

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SigningRequestView {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub action: SigningAction,
    pub election_id: Option<Uuid>,
    pub area_id: Option<Uuid>,
    pub trustee_id: Option<Uuid>,
    pub subject: Value,
    pub canonical_payload: String,
    pub payload_sha256: String,
    pub document_id: Option<Uuid>,
    pub document_sha256: Option<String>,
    pub code: String,
    pub config_revision: Option<String>,
    pub rule_revision: i64,
    pub required: i32,
    pub status: SigningRequestStatus,
    pub cancel_reason: Option<CancelReason>,
    pub cancelled_by: Option<String>,
    pub requested_by: String,
    pub requested_by_username: String,
    pub requested_by_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub executed_at: Option<DateTime<Utc>>,
    pub execution_result: Option<Value>,
    pub open_failures: i32,
}

impl From<&SigningRequestRow> for SigningRequestView {
    fn from(row: &SigningRequestRow) -> Self {
        SigningRequestView {
            id: row.id,
            tenant_id: row.tenant_id,
            election_event_id: row.election_event_id,
            action: row.action,
            election_id: row.election_id,
            area_id: row.area_id,
            trustee_id: row.trustee_id,
            subject: row.subject.clone(),
            canonical_payload: row.canonical_payload.clone(),
            payload_sha256: row.payload_sha256.clone(),
            document_id: row.document_id,
            document_sha256: row.document_sha256.clone(),
            code: row.code.clone(),
            config_revision: row.config_revision.clone(),
            rule_revision: row.rule_revision,
            required: row.required,
            status: row.status,
            cancel_reason: row.cancel_reason,
            cancelled_by: row.cancelled_by.clone(),
            requested_by: row.requested_by.clone(),
            requested_by_username: row.requested_by_username.clone(),
            requested_by_name: row.requested_by_name.clone(),
            created_at: row.created_at,
            expires_at: row.expires_at,
            completed_at: row.completed_at,
            executed_at: row.executed_at,
            execution_result: row.execution_result.clone(),
            open_failures: row.open_failures,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SignerStatus {
    Signed,
    NotSigned,
}

/// A person who can sign, or did sign, with their status on the request.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SignerView {
    pub user_id: String,
    pub username: String,
    /// First and last name; the username without one.
    pub display_name: String,
    pub title: Option<String>,
    /// The viewer ("(you)").
    pub is_you: bool,
    pub status: SignerStatus,
    pub signed_at: Option<DateTime<Utc>>,
    pub certificate_subject: Option<String>,
    /// The CN of the certificate they signed with.
    pub certificate_cn: Option<String>,
}

/// A labelled value of what a request signs, for actions without a
/// document: one per field of its subject.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SigningDetail {
    pub key: String,
    pub value: String,
}

/// Everything the signing panel shows.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SigningPanel {
    pub request: SigningRequestView,
    /// The rule as it was when the request started.
    pub rule: SigningRule,
    pub count: i64,
    pub required: i32,
    pub signers: Vec<SignerView>,
    /// The request's own document, whose SHA-256 the payload names: a PDF
    /// request's base revision, a transmission's EML. Short-lived.
    pub document_url: Option<String>,
    pub document_name: Option<String>,
    pub document_pages: Option<i32>,
    /// A PDF request's document as it stands (view only).
    pub document_revision: Option<DocumentRevisionView>,
    pub details: Vec<SigningDetail>,
    pub election_name: Option<String>,
    pub area_name: Option<String>,
}

/// The subject's fields in key order, lists joined with commas.
pub fn subject_details(subject: &Value) -> Vec<SigningDetail> {
    let Value::Object(fields) = subject else {
        return vec![];
    };
    let mut keys: Vec<&String> = fields.keys().collect();
    keys.sort();
    keys.into_iter()
        .map(|key| {
            let value = match &fields[key] {
                Value::String(text) => text.clone(),
                Value::Null => String::new(),
                Value::Array(items) => items
                    .iter()
                    .map(|item| match item {
                        Value::String(text) => text.clone(),
                        other => other.to_string(),
                    })
                    .collect::<Vec<_>>()
                    .join(", "),
                other => other.to_string(),
            };
            SigningDetail {
                key: key.clone(),
                value,
            }
        })
        .collect()
}

/// The subject and common name of a PEM certificate.
pub fn certificate_names(pem: &str) -> (Option<String>, Option<String>) {
    let Ok(certificate) = X509::from_pem(pem.as_bytes()) else {
        return (None, None);
    };
    let name = certificate.subject_name();
    let text = |entry: &openssl::x509::X509NameEntryRef| {
        entry.data().as_utf8().map(|value| value.to_string()).ok()
    };
    let subject = name
        .entries()
        .filter_map(|entry| {
            let key = entry.object().nid().short_name().ok()?;
            Some(format!("{key}={}", text(entry)?))
        })
        .collect::<Vec<_>>()
        .join(", ");
    let common_name = name.entries_by_nid(Nid::COMMONNAME).next().and_then(text);
    ((!subject.is_empty()).then_some(subject), common_name)
}

/// The panel of a request, for its requester, a person who can sign it, or
/// a reader of the event's requests who reaches its Post.
#[instrument(skip(hasura_transaction, keycloak_transaction, documents), err)]
pub async fn get_panel(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    documents: &dyn DocumentSigner,
    caller: &SigningCaller,
    tenant_id: Uuid,
    request_id: Uuid,
) -> SigningResult<SigningPanel> {
    let request = read_request(hasura_transaction, tenant_id, request_id).await?;
    let reads = caller.has(Permissions::SIGNING_REQUESTS_READ)
        && caller.reaches(request.permission_label.as_deref());
    let signs = may_sign(hasura_transaction, caller, &request).await?;
    if !(is_requester(caller, &request) || signs || reads) {
        return Err(SigningError::Forbidden(
            "You can't see this signing request.".into(),
        ));
    }
    let rule: SigningRule = serde_json::from_value(request.rule_snapshot.clone())
        .context("Error reading the request's rule")?;
    let approvals = list_signing_approvals(
        hasura_transaction,
        tenant_id,
        request.election_event_id,
        request.id,
    )
    .await?;
    let trustee_name = match request.trustee_id {
        Some(trustee_id) => {
            get_signing_trustee_name(hasura_transaction, tenant_id, trustee_id).await?
        }
        None => None,
    };
    let signers = list_signers(
        keycloak_transaction,
        &get_tenant_realm(&tenant_id.to_string()),
        request.action,
        &GroupChange::default(),
    )
    .await?;
    let mut views: Vec<SignerView> = signers
        .into_iter()
        .filter(|signer| signer.eligible_for(request.permission_label.as_deref()))
        .filter(
            |signer| match (request.action.is_trustee(), &trustee_name) {
                (true, Some(trustee)) => signer.username == *trustee,
                _ => true,
            },
        )
        .map(|signer| SignerView {
            is_you: signer.user_id == caller.user_id,
            display_name: signer.name().unwrap_or_else(|| signer.username.clone()),
            user_id: signer.user_id,
            username: signer.username,
            title: signer.title,
            status: SignerStatus::NotSigned,
            signed_at: None,
            certificate_subject: None,
            certificate_cn: None,
        })
        .collect();
    for approval in &approvals {
        let (subject, common_name) = certificate_names(&approval.certificate_pem);
        let index = match views
            .iter()
            .position(|view| view.user_id == approval.user_id)
        {
            Some(index) => index,
            None => {
                views.push(SignerView {
                    user_id: approval.user_id.clone(),
                    username: approval.username.clone(),
                    display_name: approval
                        .display_name
                        .clone()
                        .unwrap_or_else(|| approval.username.clone()),
                    title: None,
                    is_you: approval.user_id == caller.user_id,
                    status: SignerStatus::NotSigned,
                    signed_at: None,
                    certificate_subject: None,
                    certificate_cn: None,
                });
                views.len() - 1
            }
        };
        views[index].status = SignerStatus::Signed;
        views[index].signed_at = Some(approval.signed_at);
        views[index].certificate_subject = subject;
        views[index].certificate_cn = common_name;
    }
    let election_name = match request.election_id {
        Some(election_id) => get_signing_post(
            hasura_transaction,
            tenant_id,
            request.election_event_id,
            election_id,
        )
        .await?
        .map(|post| post.name),
        None => None,
    };
    let area_name = match request.area_id {
        Some(area_id) => {
            get_signing_area_name(
                hasura_transaction,
                tenant_id,
                request.election_event_id,
                area_id,
            )
            .await?
        }
        None => None,
    };
    let document = documents
        .panel_document(hasura_transaction, &request)
        .await?;
    Ok(SigningPanel {
        request: SigningRequestView::from(&request),
        rule,
        count: approvals.len() as i64,
        required: request.required,
        signers: views,
        document_url: document.url,
        document_name: document.name,
        document_pages: None,
        document_revision: document.revision,
        details: subject_details(&request.subject),
        election_name,
        area_name,
    })
}

// Cancel, handover, open failures

/// The longest cancel note that is logged.
const MAX_NOTE: usize = 500;

/// Cancels a waiting request: its requester, or someone who may cancel the
/// event's requests and reaches its Post.
#[instrument(skip(hasura_transaction), err)]
pub async fn cancel(
    hasura_transaction: &Transaction<'_>,
    caller: &SigningCaller,
    tenant_id: Uuid,
    request_id: Uuid,
    note: Option<&str>,
) -> SigningResult<SigningRequestRow> {
    let found = read_request(hasura_transaction, tenant_id, request_id).await?;
    let reason = if is_requester(caller, &found) {
        CancelReason::ByRequester
    } else if caller.has(Permissions::SIGNING_REQUESTS_CANCEL)
        && caller.reaches(found.permission_label.as_deref())
    {
        CancelReason::ByOperator
    } else {
        return Err(forbid(
            hasura_transaction,
            caller,
            &found,
            RefusedStep::Cancel,
            "You can't cancel this signing request.",
        )
        .await?);
    };
    let request = relock(hasura_transaction, &found).await?;
    require_waiting(&request)?;
    let note: Option<String> = note.map(|note| note.trim().chars().take(MAX_NOTE).collect());
    Ok(cancel_request(
        hasura_transaction,
        &request,
        reason,
        caller.actor(),
        note.as_deref(),
    )
    .await?)
}

/// Logs that the signer on this laptop hands over to the next member, at
/// most once every [`LOG_THROTTLE_SECONDS`] per person and request.
#[instrument(skip(hasura_transaction), err)]
pub async fn handover(
    hasura_transaction: &Transaction<'_>,
    caller: &SigningCaller,
    tenant_id: Uuid,
    request_id: Uuid,
) -> SigningResult<SigningRequestRow> {
    let found = read_request(hasura_transaction, tenant_id, request_id).await?;
    if !(is_requester(caller, &found) || may_sign(hasura_transaction, caller, &found).await?) {
        return Err(forbid(
            hasura_transaction,
            caller,
            &found,
            RefusedStep::Handover,
            "You can't hand over this signing request.",
        )
        .await?);
    }
    let request = relock(hasura_transaction, &found).await?;
    require_waiting(&request)?;
    if !throttled(
        hasura_transaction,
        &request,
        SigningStatementKind::SigningHandover,
        caller,
    )
    .await?
    {
        stage_request_step(
            hasura_transaction,
            &request,
            SigningStatementKind::SigningHandover,
            caller.actor(),
            SystemOutcome::Info,
            format!(
                "Handed signing request {} over to the next member",
                request.code
            ),
            json!({}),
        )
        .await?;
    }
    Ok(request)
}

/// Only the file's own name, cut to a bounded length: a path or a long
/// name is not logged.
fn file_name_only(file_name: &str) -> String {
    let base = file_name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default()
        .trim();
    base.chars().take(MAX_FILE_NAME).collect()
}

/// Counts a certificate file that didn't open in the signer's browser and
/// logs its name and why, never the file or its password; at most once
/// every [`LOG_THROTTLE_SECONDS`] per person and request (the count keeps
/// every one).
#[instrument(skip(hasura_transaction), err)]
pub async fn report_open_failure(
    hasura_transaction: &Transaction<'_>,
    caller: &SigningCaller,
    tenant_id: Uuid,
    request_id: Uuid,
    file_name: &str,
    reason: CertificateOpenFailure,
) -> SigningResult<i32> {
    let found = read_request(hasura_transaction, tenant_id, request_id).await?;
    if !may_sign(hasura_transaction, caller, &found).await? {
        return Err(forbid(
            hasura_transaction,
            caller,
            &found,
            RefusedStep::OpenFailure,
            "You can't sign this signing request.",
        )
        .await?);
    }
    let request = relock(hasura_transaction, &found).await?;
    require_waiting(&request)?;
    let file_name = file_name_only(file_name);
    let failures = increment_signing_request_open_failures(
        hasura_transaction,
        tenant_id,
        request.election_event_id,
        request.id,
    )
    .await?;
    if !throttled(
        hasura_transaction,
        &request,
        SigningStatementKind::SigningCertificateOpenFailed,
        caller,
    )
    .await?
    {
        stage_request_step(
            hasura_transaction,
            &request,
            SigningStatementKind::SigningCertificateOpenFailed,
            caller.actor(),
            SystemOutcome::Error,
            format!(
                "A certificate file didn't open for signing request {}: {}",
                request.code, reason
            ),
            json!({ "file_name": file_name, "reason": reason.to_string(), "open_failures": failures }),
        )
        .await?;
    }
    Ok(failures)
}

// Expiry and the sweeper

/// Expires every request whose time is up by the database's clock, each in
/// its own transaction; how many it expired.
#[instrument(skip(client), err)]
pub async fn expire_overdue_requests(client: &mut Client) -> Result<usize> {
    let overdue = {
        let transaction = client.transaction().await?;
        let overdue = list_overdue_signing_requests(&transaction, BATCH).await?;
        transaction.commit().await?;
        overdue
    };
    let mut expired = 0;
    for candidate in overdue {
        let transaction = client.transaction().await?;
        lock_signing_event(
            &transaction,
            candidate.tenant_id,
            candidate.election_event_id,
        )
        .await?;
        let Some(request) = lock_signing_request(
            &transaction,
            candidate.tenant_id,
            candidate.election_event_id,
            candidate.id,
        )
        .await?
        else {
            continue;
        };
        if request.status == SigningRequestStatus::Waiting
            && expire_request(&transaction, &request).await?.is_some()
        {
            expired += 1;
        }
        transaction.commit().await?;
    }
    if expired > 0 {
        kick_signing_log_outbox();
    }
    Ok(expired)
}

/// How long a completed request waits for its task before the sweeper
/// sends the task again.
pub const REDISPATCH_AFTER_SECONDS: i64 = 120;
/// How long a task's claim of an execution holds before another copy may
/// claim it.
pub const EXECUTION_LEASE_SECONDS: i64 = 900;
/// How many times a task may claim an execution; then the request fails.
pub const MAX_EXECUTION_ATTEMPTS: i32 = 3;

/// The result of an execution that didn't run: a stable code and a message
/// for people; the error itself goes to the service's log only.
pub fn execution_failure(code: &str) -> Value {
    json!({ "error": { "code": code, "message": "The action could not run." } })
}

/// Fails a completed request that didn't run and logs it, in the caller's
/// transaction, which holds its locks.
async fn fail_execution(
    hasura_transaction: &Transaction<'_>,
    request: &SigningRequestRow,
    code: &str,
) -> Result<Option<SigningRequestRow>> {
    let Some(failed) = update_signing_request_status(
        hasura_transaction,
        request.tenant_id,
        request.election_event_id,
        request.id,
        &SigningRequestTransition::Fail {
            result: Some(execution_failure(code)),
        },
    )
    .await?
    else {
        return Ok(None);
    };
    stage_executed(hasura_transaction, &failed, SystemOutcome::Error).await?;
    Ok(Some(failed))
}

/// Sends again the task of every deferred request completed
/// [`REDISPATCH_AFTER_SECONDS`] ago whose execution is unclaimed, whose
/// claim is older than [`EXECUTION_LEASE_SECONDS`], or whose task failed. A
/// request whose task claimed it [`MAX_EXECUTION_ATTEMPTS`] times fails
/// instead. How many tasks it sent.
#[instrument(skip(client, executors), err)]
pub async fn redispatch_unexecuted_requests(
    client: &mut Client,
    executors: &SigningExecutorRegistry,
) -> Result<usize> {
    let deferred: Vec<SigningAction> = SigningAction::iter()
        .filter(|action| action.mode() == ExecutionMode::Deferred)
        .collect();
    let transaction = client.transaction().await?;
    let candidates = list_unexecuted_signing_requests(
        &transaction,
        &deferred,
        REDISPATCH_AFTER_SECONDS,
        EXECUTION_LEASE_SECONDS,
        BATCH,
    )
    .await?;
    transaction.commit().await?;
    let mut tasks = vec![];
    let mut failed = 0;
    for candidate in candidates {
        let request = &candidate.request;
        let transaction = client.transaction().await?;
        lock_signing_event(&transaction, request.tenant_id, request.election_event_id).await?;
        let Some(locked) = lock_signing_request(
            &transaction,
            request.tenant_id,
            request.election_event_id,
            request.id,
        )
        .await?
        else {
            continue;
        };
        if locked.status != SigningRequestStatus::Completed || locked.executed_at.is_some() {
            continue;
        }
        if locked.execution_attempts >= MAX_EXECUTION_ATTEMPTS {
            if fail_execution(&transaction, &locked, "execution-attempts-exhausted")
                .await?
                .is_some()
            {
                failed += 1;
            }
            transaction.commit().await?;
            continue;
        }
        if candidate.task_failed {
            release_signing_execution(
                &transaction,
                locked.tenant_id,
                locked.election_event_id,
                locked.id,
            )
            .await?;
        }
        transaction.commit().await?;
        match executors.get(locked.action) {
            Some(executor) => match executor.redispatch(&locked).await {
                Ok(Some(task)) => tasks.push((locked.id, task)),
                Ok(None) => {}
                Err(error) => warn!(request_id = %locked.id, "redispatch failed: {error:?}"),
            },
            None => warn!(request_id = %locked.id, "no executor for a completed signing request"),
        }
    }
    if failed > 0 {
        kick_signing_log_outbox();
    }
    let mut sent = 0;
    for (request_id, task) in tasks {
        match task.run().await {
            Ok(()) => sent += 1,
            Err(error) => warn!(%request_id, "resending failed: {error:?}"),
        }
    }
    Ok(sent)
}

/// Claims a dispatched execution for the task `task_execution_id`, before
/// the task's side effect: `true` once; `false` for another copy of the
/// task, while the claim is held, or once the request ran. Commit the claim
/// before acting.
#[instrument(skip(hasura_transaction), err)]
pub async fn claim_dispatched(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    request_id: Uuid,
    task_execution_id: Uuid,
) -> Result<bool> {
    lock_signing_event(hasura_transaction, tenant_id, election_event_id).await?;
    if lock_signing_request(hasura_transaction, tenant_id, election_event_id, request_id)
        .await?
        .is_none()
    {
        return Ok(false);
    }
    Ok(claim_signing_execution(
        hasura_transaction,
        tenant_id,
        election_event_id,
        request_id,
        task_execution_id,
        EXECUTION_LEASE_SECONDS,
    )
    .await?
    .is_some())
}

/// What a dispatched execution reports back. A task calls
/// [`finish_dispatched`] in its own transaction.
#[derive(Debug, Clone, PartialEq)]
pub enum DispatchedResult {
    Executed(Option<Value>),
    /// A stable code; the error itself goes to the task's log.
    Failed {
        code: String,
    },
}

/// Records the result of a dispatched execution, once, from the task that
/// claimed it: `false` when the request is not completed, already
/// reported, unclaimed, or claimed for another task. The step is logged for
/// the requester.
#[instrument(skip(hasura_transaction), err)]
pub async fn finish_dispatched(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    request_id: Uuid,
    task_execution_id: Uuid,
    result: DispatchedResult,
) -> Result<bool> {
    lock_signing_event(hasura_transaction, tenant_id, election_event_id).await?;
    let Some(request) =
        lock_signing_request(hasura_transaction, tenant_id, election_event_id, request_id).await?
    else {
        return Ok(false);
    };
    if request.status != SigningRequestStatus::Completed
        || request.executed_at.is_some()
        || request.task_execution_id != Some(task_execution_id)
        || request.execution_started_at.is_none()
    {
        return Ok(false);
    }
    match result {
        DispatchedResult::Executed(result) => {
            let updated = update_signing_request_status(
                hasura_transaction,
                tenant_id,
                election_event_id,
                request_id,
                &SigningRequestTransition::Execute { result },
            )
            .await?
            .ok_or_else(|| anyhow!("signing request {request_id} is not completed"))?;
            stage_executed(hasura_transaction, &updated, SystemOutcome::Info).await?;
        }
        DispatchedResult::Failed { code } => {
            fail_execution(hasura_transaction, &request, &code).await?;
        }
    }
    Ok(true)
}

/// Who runs a request's action, as its SigningActionExecuted entry names
/// them (the Logs table): its last signer, by `signed_at`. An action runs
/// only once its request has every signature, so there is one; the
/// requester stands in only for a request without any.
pub async fn last_signer(
    hasura_transaction: &Transaction<'_>,
    request: &SigningRequestRow,
) -> Result<Actor> {
    let approvals = list_signing_approvals(
        hasura_transaction,
        request.tenant_id,
        request.election_event_id,
        request.id,
    )
    .await?;
    Ok(approvals
        .iter()
        .max_by_key(|approval| (approval.signed_at, approval.created_at))
        .map(|approval| Actor {
            user_id: approval.user_id.clone(),
            username: approval.username.clone(),
        })
        .unwrap_or_else(|| requester(request)))
}

/// Stages SigningActionExecuted for a request that ran (Info) or failed
/// (Error), for its last signer ([`last_signer`]), whether it ran with
/// that signature or later in a task.
pub async fn stage_executed(
    hasura_transaction: &Transaction<'_>,
    request: &SigningRequestRow,
    system: SystemOutcome,
) -> Result<Uuid> {
    let description = match system {
        SystemOutcome::Info => format!(
            "Ran {} for signing request {}",
            action_title(request.action),
            request.code
        ),
        SystemOutcome::Error => format!(
            "Could not run {} for signing request {}",
            action_title(request.action),
            request.code
        ),
    };
    let user = last_signer(hasura_transaction, request).await?;
    stage_request_step(
        hasura_transaction,
        request,
        SigningStatementKind::SigningActionExecuted,
        user,
        system,
        description,
        json!({ "status": request.status.to_string(), "result": request.execution_result }),
    )
    .await
}

// Export

/// The CSV export of an event's requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SigningExport {
    pub file_name: String,
    pub content: String,
    pub sha256: String,
    pub rows: usize,
}

const EXPORT_HEADER: [&str; 18] = [
    "request_id",
    "action",
    "election_id",
    "area_id",
    "status",
    "cancel_reason",
    "code",
    "signatures",
    "required",
    "requested_by",
    "created_at",
    "expires_at",
    "completed_at",
    "executed_at",
    "last_signature_at",
    "signers",
    "payload_sha256",
    "document_sha256",
];

/// A cell a spreadsheet would read as a formula starts with a quote.
fn cell(value: String) -> String {
    match value.chars().next() {
        Some('=' | '+' | '-' | '@' | '\t' | '\r') => format!("'{value}"),
        _ => value,
    }
}

fn text<T: ToString>(value: &Option<T>) -> String {
    value.as_ref().map(ToString::to_string).unwrap_or_default()
}

fn time(value: &Option<DateTime<Utc>>) -> String {
    value.map(|time| time.to_rfc3339()).unwrap_or_default()
}

/// Which requests an export holds, besides the Posts the reader reaches.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExportFilter {
    pub status: Option<SigningRequestStatus>,
    pub action: Option<SigningAction>,
}

/// An export kept as a document of the event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredExport {
    pub document_id: String,
    /// A short-lived link to download the file with, so that a reader who
    /// may export but not read documents can still download it. `None`
    /// when the store has no such link.
    pub url: Option<String>,
}

/// Keeps an export as a document of the event.
#[async_trait]
pub trait SigningExportStore: Send + Sync {
    async fn store(
        &self,
        hasura_transaction: &Transaction<'_>,
        tenant_id: Uuid,
        election_event_id: Uuid,
        file_name: &str,
        content: &[u8],
    ) -> Result<StoredExport>;
}

/// A private document in the object storage, like the other exports, with
/// a presigned download link, as the request panel links its document.
#[derive(Debug, Clone, Copy, Default)]
pub struct DocumentExportStore;

#[async_trait]
impl SigningExportStore for DocumentExportStore {
    async fn store(
        &self,
        hasura_transaction: &Transaction<'_>,
        tenant_id: Uuid,
        election_event_id: Uuid,
        file_name: &str,
        content: &[u8],
    ) -> Result<StoredExport> {
        let (_temp_path, path, size) =
            write_into_named_temp_file(&content.to_vec(), "signing-requests-", ".csv")?;
        let document = upload_and_return_document(
            hasura_transaction,
            &path,
            size,
            "text/csv",
            &tenant_id.to_string(),
            Some(election_event_id.to_string()),
            file_name,
            None,
            false,
        )
        .await?;
        let url = get_document_url(
            hasura_transaction,
            &tenant_id.to_string(),
            Some(&election_event_id.to_string()),
            &document.id,
        )
        .await?;
        Ok(StoredExport {
            document_id: document.id,
            url,
        })
    }
}

/// Exports the requests of the Posts the caller reaches as CSV and logs the
/// file's SHA-256. The caller holds `signing-requests-export`.
#[instrument(skip(hasura_transaction), err)]
pub async fn export_requests(
    hasura_transaction: &Transaction<'_>,
    caller: &SigningCaller,
    tenant_id: Uuid,
    election_event_id: Uuid,
    filter: &ExportFilter,
    now: DateTime<Utc>,
) -> SigningResult<SigningExport> {
    lock_signing_event(hasura_transaction, tenant_id, election_event_id).await?;
    let requests: Vec<SigningRequestRow> =
        list_signing_requests(hasura_transaction, tenant_id, election_event_id)
            .await?
            .into_iter()
            .filter(|request| caller.reaches(request.permission_label.as_deref()))
            .filter(|request| filter.status.is_none_or(|status| request.status == status))
            .filter(|request| filter.action.is_none_or(|action| request.action == action))
            .collect();
    let approvals =
        list_event_signing_approvals(hasura_transaction, tenant_id, election_event_id).await?;
    let mut writer = csv::Writer::from_writer(vec![]);
    writer
        .write_record(EXPORT_HEADER)
        .context("Error writing the export")?;
    for request in &requests {
        let signed: Vec<&SigningApprovalRow> = approvals
            .iter()
            .filter(|approval| approval.request_id == request.id)
            .collect();
        let signers: Vec<&str> = signed
            .iter()
            .map(|approval| approval.username.as_str())
            .collect();
        writer
            .write_record(
                [
                    request.id.to_string(),
                    request.action.to_string(),
                    text(&request.election_id),
                    text(&request.area_id),
                    request.status.to_string(),
                    text(&request.cancel_reason),
                    request.code.clone(),
                    signed.len().to_string(),
                    request.required.to_string(),
                    request.requested_by_username.clone(),
                    request.created_at.to_rfc3339(),
                    time(&request.expires_at),
                    time(&request.completed_at),
                    time(&request.executed_at),
                    time(&signed.last().map(|approval| approval.signed_at)),
                    signers.join(";"),
                    request.payload_sha256.clone(),
                    text(&request.document_sha256),
                ]
                .map(cell),
            )
            .context("Error writing the export")?;
    }
    let bytes = writer
        .into_inner()
        .map_err(|error| anyhow!("Error writing the export: {error:?}"))?;
    let content = String::from_utf8(bytes).context("Error writing the export")?;
    let sha256 = sha256_hex(content.as_bytes());
    let file_name = format!(
        "signing-requests-{election_event_id}-{}.csv",
        now.format("%Y%m%dT%H%M%SZ")
    );
    stage(
        hasura_transaction,
        &LogStep {
            kind: SigningStatementKind::SigningRequestsExported,
            user: caller.actor(),
            system: SystemOutcome::Info,
            scope: log_scope(tenant_id, election_event_id, None, None),
            description: format!("Exported {} signing requests", requests.len()),
            details: json!({
                "file_name": file_name,
                "sha256": sha256,
                "rows": requests.len(),
                "labels": caller.labels,
                "status": filter.status,
                "action": filter.action,
                "allowed_by": allowed_by_permission(Permissions::SIGNING_REQUESTS_EXPORT),
            }),
        },
    )
    .await?;
    Ok(SigningExport {
        file_name,
        content,
        sha256,
        rows: requests.len(),
    })
}
