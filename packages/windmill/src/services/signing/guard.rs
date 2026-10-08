// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The guard the routes of protected actions call before they act.
//!
//! With the action's rule `NotRequired` the route runs as it always did
//! ([`GuardOutcome::Proceed`]). With `Required` the route answers the
//! request instead ([`GuardOutcome::SigningRequired`]): the one already
//! waiting for the same scope when it signs the same thing, otherwise a new
//! one, which cancels the one waiting. The action runs when the last
//! signature arrives (`Deferred`), or, for a trustee's `Gate` step, when
//! the trustee's route consumes the completed request ([`consume_gate`]).

use super::log::SystemOutcome;
use super::requests::{
    cancel_request, expire_request, is_overdue, is_the_trustee, read_request, relock,
    stage_request_step,
};
use super::{action_title, InvalidReason, SigningCaller, SigningError, SigningResult};
use crate::postgres::signing::*;
use crate::postgres::signing_document_revision::signing_document_access_restricted;
use anyhow::Context;
use chrono::{DateTime, Duration, SubsecRound, Utc};
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::SigningStatementKind;
use sequent_core::signing::{
    canonical_json, payload_sha256, CancelReason, DocumentKind, ExecutionMode, SigningAction,
    SigningPayload, SigningPayloadFields, SigningRequestStatus, SigningRule, SigningScope,
};
use serde::Serialize;
use serde_json::{json, Value};
use tracing::instrument;
use uuid::Uuid;

/// What a request is bound to. Post = `election_id`, country = `area_id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestScope {
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub election_id: Option<Uuid>,
    pub area_id: Option<Uuid>,
    pub trustee_id: Option<Uuid>,
    /// What one request stands for within the scope, when an action can
    /// wait for several at once (an application, a tally session).
    pub subject_key: Option<String>,
}

impl RequestScope {
    /// `election_id|area_id|trustee_id|subject_key`: a new request for the
    /// same key replaces the one waiting.
    pub fn scope_key(&self) -> String {
        let part = |value: Option<String>| value.unwrap_or_default();
        [
            part(self.election_id.map(|id| id.to_string())),
            part(self.area_id.map(|id| id.to_string())),
            part(self.trustee_id.map(|id| id.to_string())),
            part(self.subject_key.clone()),
        ]
        .join("|")
    }

    /// Refuses a scope that doesn't fit the action's.
    fn check(&self, action: SigningAction) -> SigningResult<()> {
        let fits = match action.scope() {
            SigningScope::Post | SigningScope::PostAndCountry => self.election_id.is_some(),
            SigningScope::Event => self.election_id.is_none() && self.area_id.is_none(),
            SigningScope::Trustee => self.trustee_id.is_some(),
        };
        if fits {
            Ok(())
        } else {
            Err(SigningError::bad_input(format!(
                "A {action} request is scoped to {}.",
                action.scope()
            )))
        }
    }
}

/// The document a request signs, for actions with one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SigningDocument {
    pub document_id: Option<Uuid>,
    /// Lowercase hex SHA-256 of the document.
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GuardRequest {
    pub action: SigningAction,
    pub scope: RequestScope,
    /// The action's subject (the `*Subject` types of `sequent_core::signing`).
    pub subject: Value,
    pub document: Option<SigningDocument>,
    /// The event's configuration version, when the action has one.
    pub config_revision: Option<String>,
}

/// What a guarded route answers instead of running the action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SigningRequestSummary {
    pub id: Uuid,
    pub code: String,
    pub required: i32,
    pub expires_at: Option<DateTime<Utc>>,
}

impl From<&SigningRequestRow> for SigningRequestSummary {
    fn from(request: &SigningRequestRow) -> Self {
        SigningRequestSummary {
            id: request.id,
            code: request.code.clone(),
            required: request.required,
            expires_at: request.expires_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardOutcome {
    /// No signatures needed: the route runs the action.
    Proceed,
    /// The route answers `{signing_request}` and the action waits.
    SigningRequired(SigningRequestSummary),
}

fn canonical(value: &Value, what: &str) -> SigningResult<String> {
    canonical_json(value).map_err(|error| SigningError::bad_input(format!("{what}: {error}")))
}

/// The subject field that names the document's SHA-256, for actions with
/// a document.
fn document_field(action: SigningAction) -> Option<&'static str> {
    match action.document() {
        DocumentKind::Pdf => Some("document_sha256"),
        DocumentKind::Eml => Some("eml_sha256"),
        DocumentKind::NoDocument => None,
    }
}

/// An action with a document signs it: its subject names the document's
/// SHA-256, and that is the document the request keeps.
fn check_document(request: &GuardRequest, document_sha256: Option<&str>) -> SigningResult<()> {
    let Some(field) = document_field(request.action) else {
        return Ok(());
    };
    let named = request.subject.get(field).and_then(Value::as_str);
    if document_sha256.is_none() || named != document_sha256 {
        return Err(SigningError::bad_input(format!(
            "A {} request signs its document: the subject's {field} must be the document's SHA-256.",
            request.action
        )));
    }
    Ok(())
}

/// The rule an event applies to `action`.
pub async fn effective_rule(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    action: SigningAction,
) -> anyhow::Result<SigningRule> {
    Ok(
        get_signing_rule(hasura_transaction, tenant_id, election_event_id, action)
            .await?
            .map(|row| row.rule)
            .unwrap_or_else(|| SigningRule::default_for(action)),
    )
}

/// [`guard_at`] now.
pub async fn guard(
    hasura_transaction: &Transaction<'_>,
    caller: &SigningCaller,
    request: &GuardRequest,
) -> SigningResult<GuardOutcome> {
    guard_at(hasura_transaction, caller, request, Utc::now()).await
}

/// Decides whether a protected action runs now or waits for signatures;
/// see the module documentation. The caller has checked that `caller` may
/// start the action (its permission and Post).
///
/// Call it before taking any other lock in the route's transaction: when
/// the action needs signatures it takes the event's signing lock, which
/// every signing step takes first.
#[instrument(skip(hasura_transaction, request), fields(action = %request.action), err)]
pub async fn guard_at(
    hasura_transaction: &Transaction<'_>,
    caller: &SigningCaller,
    request: &GuardRequest,
    now: DateTime<Utc>,
) -> SigningResult<GuardOutcome> {
    let scope = &request.scope;
    scope.check(request.action)?;
    // Most actions need no signatures: read the rule without the lock.
    let rule = effective_rule(
        hasura_transaction,
        scope.tenant_id,
        scope.election_event_id,
        request.action,
    )
    .await?;
    if !rule.is_required() {
        return Ok(GuardOutcome::Proceed);
    }
    lock_signing_event(hasura_transaction, scope.tenant_id, scope.election_event_id).await?;
    let rule = effective_rule(
        hasura_transaction,
        scope.tenant_id,
        scope.election_event_id,
        request.action,
    )
    .await?;
    if !rule.is_required() {
        return Ok(GuardOutcome::Proceed);
    }
    if let Some(area_id) = scope.area_id {
        if !signing_area_exists(
            hasura_transaction,
            scope.tenant_id,
            scope.election_event_id,
            area_id,
        )
        .await?
        {
            return Err(SigningError::NotFound("There is no such country.".into()));
        }
    }
    if let Some(trustee_id) = scope.trustee_id {
        if get_signing_trustee_name(hasura_transaction, scope.tenant_id, trustee_id)
            .await?
            .is_none()
        {
            return Err(SigningError::NotFound("There is no such trustee.".into()));
        }
    }
    let permission_label = match scope.election_id {
        Some(election_id) => {
            get_signing_post(
                hasura_transaction,
                scope.tenant_id,
                scope.election_event_id,
                election_id,
            )
            .await?
            .ok_or_else(|| SigningError::NotFound("There is no such Post.".into()))?
            .permission_label
        }
        None => None,
    };
    let subject = canonical(&request.subject, "subject")?;
    let document_sha256 = request
        .document
        .as_ref()
        .map(|document| document.sha256.clone());
    check_document(request, document_sha256.as_deref())?;
    if let Some(document_id) = request
        .document
        .as_ref()
        .and_then(|document| document.document_id)
    {
        if signing_document_access_restricted(
            hasura_transaction,
            scope.tenant_id,
            scope.election_event_id,
            document_id,
        )
        .await?
        {
            return Err(SigningError::invalid(
                InvalidReason::Document,
                "This document is password-protected or holds voter secrets: it can't be signed.",
            ));
        }
    }
    let scope_key = scope.scope_key();

    // Signed already and about to run: that request stands. A trustee's
    // gate signed for something else would never be used, so a new one
    // replaces it and the trustee is never stuck.
    if let Some(unexecuted) = find_unexecuted_signing_request(
        hasura_transaction,
        scope.tenant_id,
        scope.election_event_id,
        request.action,
        &scope_key,
    )
    .await?
    {
        let same_payload = canonical(&unexecuted.subject, "subject")? == subject
            && unexecuted.document_sha256 == document_sha256;
        if request.action.mode() != ExecutionMode::Gate || same_payload {
            return Ok(GuardOutcome::SigningRequired(SigningRequestSummary::from(
                &unexecuted,
            )));
        }
        cancel_request(
            hasura_transaction,
            &unexecuted,
            CancelReason::Superseded,
            caller.actor(),
            None,
        )
        .await?;
    }

    if let Some(waiting) = lock_waiting_signing_request(
        hasura_transaction,
        scope.tenant_id,
        scope.election_event_id,
        request.action,
        &scope_key,
    )
    .await?
    {
        let expired = is_overdue(hasura_transaction, &waiting).await?
            && expire_request(hasura_transaction, &waiting)
                .await?
                .is_some();
        if !expired {
            let same_payload = canonical(&waiting.subject, "subject")? == subject
                && waiting.document_sha256 == document_sha256;
            if same_payload
                && waiting.config_revision == request.config_revision
                && waiting.rule_revision == rule.revision
            {
                return Ok(GuardOutcome::SigningRequired(SigningRequestSummary::from(
                    &waiting,
                )));
            }
            let reason = if same_payload {
                CancelReason::Superseded
            } else {
                CancelReason::PayloadChanged
            };
            cancel_request(hasura_transaction, &waiting, reason, caller.actor(), None).await?;
        }
    }

    let id = Uuid::new_v4();
    let created_at = now.trunc_subsecs(0);
    let expires_at = rule
        .expires_minutes
        .map(|minutes| created_at + Duration::minutes(i64::from(minutes)));
    let payload = SigningPayload::new(SigningPayloadFields {
        tenant_id: scope.tenant_id.to_string(),
        election_event_id: scope.election_event_id.to_string(),
        request_id: id,
        action: request.action,
        election_id: scope.election_id.map(|id| id.to_string()),
        area_id: scope.area_id.map(|id| id.to_string()),
        subject: request.subject.clone(),
        config_revision: request.config_revision.clone(),
        rule_revision: rule.revision,
        requested_by: caller.user_id.clone(),
        created_at,
        expires_at,
    })
    .map_err(|error| SigningError::bad_input(format!("subject: {error}")))?;
    let canonical_payload = payload
        .canonical()
        .map_err(|error| SigningError::bad_input(format!("payload: {error}")))?;
    let inserted = insert_signing_request(
        hasura_transaction,
        &NewSigningRequest {
            id,
            tenant_id: scope.tenant_id,
            election_event_id: scope.election_event_id,
            action: request.action,
            election_id: scope.election_id,
            area_id: scope.area_id,
            trustee_id: scope.trustee_id,
            scope_key,
            subject: request.subject.clone(),
            payload_sha256: payload_sha256(&canonical_payload),
            canonical_payload,
            document_id: request
                .document
                .as_ref()
                .and_then(|document| document.document_id),
            document_sha256,
            code: payload.code().to_owned(),
            config_revision: request.config_revision.clone(),
            rule_revision: rule.revision,
            rule_snapshot: serde_json::to_value(&rule).context("Error keeping the rule")?,
            required: i32::from(rule.required()),
            requested_by: caller.user_id.clone(),
            requested_by_username: caller.username.clone(),
            requested_by_name: Some(caller.display_name.clone()),
            expires_at,
            permission_label,
            created_at,
        },
    )
    .await?;
    let Some(created) = inserted else {
        // Another request started for the scope first: answer that one.
        let waiting = lock_waiting_signing_request(
            hasura_transaction,
            scope.tenant_id,
            scope.election_event_id,
            request.action,
            &scope.scope_key(),
        )
        .await?
        .ok_or_else(|| SigningError::Conflict("The request could not be started.".into()))?;
        return Ok(GuardOutcome::SigningRequired(SigningRequestSummary::from(
            &waiting,
        )));
    };
    stage_request_step(
        hasura_transaction,
        &created,
        SigningStatementKind::SigningRequestCreated,
        caller.actor(),
        SystemOutcome::Info,
        format!(
            "Started signing request {} to {}: {} signatures needed",
            created.code,
            action_title(created.action).to_lowercase(),
            created.required
        ),
        json!({
            "required": created.required,
            "expires_at": created.expires_at,
            "payload_sha256": created.payload_sha256,
            "document_sha256": created.document_sha256,
            "rule_revision": created.rule_revision,
        }),
    )
    .await?;
    Ok(GuardOutcome::SigningRequired(SigningRequestSummary::from(
        &created,
    )))
}

/// Consumes the completed request of a trustee's `Gate` step: of
/// `action` in the event, signed by the calling trustee, signing
/// `expected_subject`, not consumed yet. Marks it executed and logs it, in
/// the caller's transaction. Nobody else learns its status or subject.
#[instrument(skip(hasura_transaction, expected_subject), err)]
pub async fn consume_gate(
    hasura_transaction: &Transaction<'_>,
    caller: &SigningCaller,
    tenant_id: Uuid,
    election_event_id: Uuid,
    action: SigningAction,
    request_id: Uuid,
    expected_subject: &Value,
) -> SigningResult<SigningRequestRow> {
    let found = read_request(hasura_transaction, tenant_id, request_id).await?;
    if found.election_event_id != election_event_id || found.action != action {
        return Err(SigningError::NotFound(
            "There is no such signing request.".into(),
        ));
    }
    if action.mode() != ExecutionMode::Gate {
        return Err(SigningError::bad_input(format!(
            "A {action} request runs when it is signed."
        )));
    }
    if !is_the_trustee(hasura_transaction, caller, &found).await? {
        return Err(SigningError::Forbidden(
            "Only the trustee who signed this request can use it.".into(),
        ));
    }
    let request = relock(hasura_transaction, &found).await?;
    let approvals = list_signing_approvals(
        hasura_transaction,
        tenant_id,
        request.election_event_id,
        request.id,
    )
    .await?;
    if !approvals
        .iter()
        .any(|approval| approval.user_id == caller.user_id)
    {
        return Err(SigningError::Forbidden(
            "Only the trustee who signed this request can use it.".into(),
        ));
    }
    if request.status != SigningRequestStatus::Completed {
        return Err(SigningError::Closed {
            status: request.status,
            message: format!(
                "Signing request {} is {}, not completed.",
                request.code, request.status
            ),
        });
    }
    if canonical(&request.subject, "subject")? != canonical(expected_subject, "subject")? {
        return Err(SigningError::Conflict(format!(
            "Signing request {} signs something else.",
            request.code
        )));
    }
    let executed = update_signing_request_status(
        hasura_transaction,
        tenant_id,
        request.election_event_id,
        request.id,
        &SigningRequestTransition::Execute { result: None },
    )
    .await?
    .ok_or_else(|| SigningError::Conflict("The request was used already.".into()))?;
    stage_request_step(
        hasura_transaction,
        &executed,
        SigningStatementKind::SigningActionExecuted,
        caller.actor(),
        SystemOutcome::Info,
        format!(
            "Ran {} for signing request {}",
            action_title(executed.action),
            executed.code
        ),
        json!({ "status": executed.status.to_string() }),
    )
    .await?;
    Ok(executed)
}
