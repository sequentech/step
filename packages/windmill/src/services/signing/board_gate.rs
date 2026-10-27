// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Single-use authorisation of board operations which return data to their
//! requester. The same authenticated request is repeated after the board signs;
//! a completed quorum is consumed in the transaction that starts the operation.

use super::guard::{effective_rule, guard, GuardOutcome, GuardRequest, RequestScope};
use super::log::SystemOutcome;
use super::requests::{cancel_request, relock, stage_request_step};
use super::{action_title, SigningCaller, SigningError, SigningResult};
use crate::postgres::signing::{
    find_unexecuted_signing_request, list_signing_approvals, lock_signing_event,
    update_signing_request_status, SigningRequestTransition,
};
use chrono::Utc;
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::SigningStatementKind;
use sequent_core::signing::{canonical_json, CancelReason, SigningAction, SigningRequestStatus};
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use std::collections::HashSet;
use uuid::Uuid;

/// A quorum must be initiated by a board account, never by an administrator
/// carrying both sets of capabilities. Board membership is a managed capability
/// rather than a hard-coded tenant role name.
pub fn require_board(caller: &SigningCaller) -> SigningResult<()> {
    if !caller.has(Permissions::SIGNING_REQUESTS_CANCEL)
        || caller.has(Permissions::SIGNING_RULES_WRITE)
        || caller.has(Permissions::ROLE_WRITE)
        || caller.has(Permissions::USER_WRITE)
        || caller.has(Permissions::USER_CREATE)
        || caller.has(Permissions::ROLE_CREATE)
        || caller.has(Permissions::TENANT_WRITE)
        || caller.has(Permissions::SIGNING_ISSUERS_WRITE)
        || caller.has(Permissions::SIGNING_CHECKS_WRITE)
        || caller.has(Permissions::SIGNING_CERTIFICATES_REGISTER)
        || caller.has(Permissions::SIGNING_CERTIFICATES_REVOKE)
    {
        return Err(SigningError::Forbidden(
            "This operation requires an election board account with no administrator capabilities."
                .into(),
        ));
    }
    Ok(())
}

#[derive(Debug)]
pub enum BoardGateOutcome {
    Authorized(Uuid),
    Waiting(super::guard::SigningRequestSummary),
}

/// The caller commits a waiting request, presents its code, and repeats the
/// same operation after the board signs. An authorised result names the
/// consumed request for a durable worker binding.
pub async fn gate(
    transaction: &Transaction<'_>,
    caller: &SigningCaller,
    tenant_id: Uuid,
    election_event_id: Uuid,
    action: SigningAction,
    operation: &str,
    subject: Value,
) -> SigningResult<BoardGateOutcome> {
    require_board(caller)?;
    if !action.is_board_gate() {
        return Err(SigningError::bad_input(
            "This is not a board-operation gate.",
        ));
    }
    lock_signing_event(transaction, tenant_id, election_event_id).await?;
    let rule = effective_rule(transaction, tenant_id, election_event_id, action).await?;
    rule.validate()
        .map_err(|error| SigningError::bad_input(error.to_string()))?;
    let scope = RequestScope {
        tenant_id,
        election_event_id,
        election_id: None,
        area_id: None,
        trustee_id: None,
        subject_key: Some(format!("{}:{operation}", caller.user_id)),
    };
    let request = GuardRequest {
        action,
        scope,
        subject,
        document: None,
        config_revision: None,
    };
    if let Some(found) = find_unexecuted_signing_request(
        transaction,
        tenant_id,
        election_event_id,
        action,
        &request.scope.scope_key(),
    )
    .await?
    {
        let completed = relock(transaction, &found).await?;
        let same = completed.status == SigningRequestStatus::Completed
            && completed.requested_by == caller.user_id
            && completed.rule_revision == rule.revision
            && canonical_json(&completed.subject).map_err(anyhow::Error::from)?
                == canonical_json(&request.subject).map_err(anyhow::Error::from)?
            && completed
                .expires_at
                .is_none_or(|expiry| expiry > Utc::now());
        if same {
            let approvals =
                list_signing_approvals(transaction, tenant_id, election_event_id, completed.id)
                    .await?;
            let people: HashSet<_> = approvals.iter().map(|a| &a.user_id).collect();
            let valid_certificates:i64=transaction.query_one("SELECT count(*) FROM sequent_backend.signing_approval a JOIN sequent_backend.staff_certificate c ON c.id=a.certificate_id AND c.tenant_id=a.tenant_id AND c.election_event_id=a.election_event_id WHERE a.request_id=$1 AND c.status='active' AND c.not_before<=clock_timestamp() AND c.not_after>clock_timestamp()", &[&completed.id]).await?.try_get(0)?;
            if valid_certificates < i64::from(rule.required()) {
                return Err(SigningError::Conflict("A required signing certificate expired or was revoked; request new board authorisations.".into()));
            }

            if people.len() < usize::from(rule.required()) {
                return Err(SigningError::Conflict(
                    "The completed request no longer has its required distinct authorisations."
                        .into(),
                ));
            }
            let executed = update_signing_request_status(
                transaction,
                tenant_id,
                election_event_id,
                completed.id,
                &SigningRequestTransition::Execute { result: None },
            )
            .await?
            .ok_or_else(|| SigningError::Conflict("The authorisation was used already.".into()))?;
            stage_request_step(
                transaction,
                &executed,
                SigningStatementKind::SigningActionExecuted,
                caller.actor(),
                SystemOutcome::Info,
                format!(
                    "Authorised {} with request {}",
                    action_title(action),
                    executed.code
                ),
                json!({"status": executed.status.to_string(), "operation": operation}),
            )
            .await?;
            return Ok(BoardGateOutcome::Authorized(executed.id));
        }
        cancel_request(
            transaction,
            &completed,
            CancelReason::PayloadChanged,
            caller.actor(),
            None,
        )
        .await?;
    }
    match guard(transaction, caller, &request).await? {
        GuardOutcome::SigningRequired(summary) => Ok(BoardGateOutcome::Waiting(summary)),
        GuardOutcome::Proceed => Err(SigningError::Conflict(
            "A required election-board authorisation cannot be bypassed.".into(),
        )),
    }
}
