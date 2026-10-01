// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Approving a voter manually (`/change-application-status`, the approve
//! decision).
//!
//! The request signs the application, its status, the registry record it is
//! matched to and the decision, and with them what the panel shows: when
//! the application was submitted, why it needs a person and who the registry
//! record is. It is scoped to the Post the application's area votes in
//! (refused as `ambiguous-post` when the area votes in several and the
//! application's label doesn't say which), one request per application:
//! matching another registry record replaces the waiting one, and rejecting
//! the application cancels it (PayloadChanged).

use super::{event_ids, gate, refuse, subject_of, EffectProgress};
use crate::postgres::signing::{
    list_waiting_signing_requests, lock_signing_event, lock_waiting_signing_request,
    SigningApprovalRow, SigningRequestRow,
};
use crate::postgres::signing_actions::{
    annotate_application, get_application_for_signing, posts_of_area, registry_user_names,
    ApplicationForSigning,
};
use crate::services::application::{confirm_application, get_group_names};
use crate::services::database::get_keycloak_pool;
use crate::services::signing::guard::{GuardOutcome, GuardRequest, RequestScope};
use crate::services::signing::requests::cancel_request;
use crate::services::signing::{InvalidReason, SigningCaller, SigningError, SigningResult};
use crate::services::users::check_is_user_verified;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use sequent_core::services::keycloak::{get_event_realm, get_tenant_realm};
use sequent_core::signing::{ApplicationDecision, CancelReason, SigningAction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

/// What an approve-voter request signs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoterApprovalSubject {
    pub application_id: String,
    /// The registry record (the voter's account) the application is
    /// matched to.
    pub applicant_registry_id: String,
    pub decision: ApplicationDecision,
    /// The application's status when the request started.
    pub status: String,
    /// The day the application was submitted (YYYY-MM-DD, UTC).
    pub submitted_at: String,
    /// Why no automatic match was made, as the verification recorded it.
    pub reason: String,
    /// The registry record: "Last, First (username)".
    pub registry_record: String,
}

/// How a registry record reads.
pub fn registry_line(
    last_name: Option<&str>,
    first_name: Option<&str>,
    username: Option<&str>,
    fallback: &str,
) -> String {
    let name = [last_name, first_name]
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    match (name.is_empty(), username.filter(|u| !u.is_empty())) {
        (false, Some(username)) => format!("{name} ({username})"),
        (false, None) => name,
        (true, Some(username)) => username.to_owned(),
        (true, None) => fallback.to_owned(),
    }
}

/// The Post among `posts` (with whether each carries the application's
/// label) the application votes in.
pub fn choose_post(posts: &[(Uuid, bool)]) -> SigningResult<Uuid> {
    match posts {
        [] => Err(SigningError::bad_input(
            "The application's area takes part in no Post.",
        )),
        [(post, _)] => Ok(*post),
        several => match several
            .iter()
            .filter(|(_, labelled)| *labelled)
            .collect::<Vec<_>>()
            .as_slice()
        {
            [(post, _)] => Ok(*post),
            _ => Err(SigningError::invalid(
                InvalidReason::AmbiguousPost,
                "The application's area votes in more than one Post and its label doesn't say which.",
            )),
        },
    }
}

/// The subject of approving `application` as `registry_user_id`.
pub async fn voter_subject(
    keycloak_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    application: &ApplicationForSigning,
    registry_user_id: &str,
) -> Result<VoterApprovalSubject> {
    let realm = get_event_realm(&tenant_id.to_string(), &election_event_id.to_string());
    let names = registry_user_names(keycloak_transaction, &realm, registry_user_id).await?;
    let (last, first, username) = names.unwrap_or((None, None, None));
    let reason = application
        .annotations
        .as_ref()
        .and_then(|annotations| annotations.get("manual_verify_reason"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    Ok(VoterApprovalSubject {
        application_id: application.id.to_string(),
        applicant_registry_id: registry_user_id.to_owned(),
        decision: ApplicationDecision::Approve,
        status: application.status.clone(),
        submitted_at: application.created_at.format("%Y-%m-%d").to_string(),
        reason,
        registry_record: registry_line(
            last.as_deref(),
            first.as_deref(),
            username.as_deref(),
            registry_user_id,
        ),
    })
}

/// Whether approving an application runs now; see [`super::gate`].
#[allow(clippy::too_many_arguments)]
pub async fn gate_voter_approval(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    caller: &SigningCaller,
    tenant_id: &str,
    election_event_id: &str,
    application_id: &str,
    registry_user_id: &str,
) -> SigningResult<GuardOutcome> {
    let Some((tenant_id, election_event_id)) = event_ids(tenant_id, election_event_id) else {
        return Ok(GuardOutcome::Proceed);
    };
    gate(
        hasura_transaction,
        caller,
        SigningAction::ApproveVoter,
        tenant_id,
        election_event_id,
        || async move {
            let application_id = Uuid::parse_str(application_id)
                .map_err(|_| SigningError::bad_input("The application id is not a UUID."))?;
            let application = get_application_for_signing(
                hasura_transaction,
                tenant_id,
                election_event_id,
                application_id,
            )
            .await?
            .ok_or_else(|| SigningError::NotFound("There is no such application.".into()))?;
            let posts = match application.area_id {
                Some(area_id) => {
                    posts_of_area(
                        hasura_transaction,
                        tenant_id,
                        election_event_id,
                        area_id,
                        application.permission_label.as_deref(),
                    )
                    .await?
                }
                None => vec![],
            };
            let post = choose_post(&posts)?;
            let subject = voter_subject(
                keycloak_transaction,
                tenant_id,
                election_event_id,
                &application,
                registry_user_id,
            )
            .await?;
            Ok(GuardRequest {
                action: SigningAction::ApproveVoter,
                scope: RequestScope {
                    tenant_id,
                    election_event_id,
                    election_id: Some(post),
                    area_id: application.area_id,
                    trustee_id: None,
                    subject_key: Some(application.id.to_string()),
                },
                subject: serde_json::to_value(subject).map_err(anyhow::Error::from)?,
                document: None,
                config_revision: None,
            })
        },
    )
    .await
}

/// Cancels the request waiting to approve an application that is rejected
/// (PayloadChanged). It takes the event's signing lock first, so call it
/// before the route takes any other lock.
pub async fn cancel_for_rejection(
    hasura_transaction: &Transaction<'_>,
    caller: &SigningCaller,
    tenant_id: &str,
    election_event_id: &str,
    application_id: &str,
) -> SigningResult<usize> {
    let Some((tenant_id, election_event_id)) = event_ids(tenant_id, election_event_id) else {
        return Ok(0);
    };
    lock_signing_event(hasura_transaction, tenant_id, election_event_id).await?;
    let waiting = list_waiting_signing_requests(
        hasura_transaction,
        tenant_id,
        election_event_id,
        SigningAction::ApproveVoter,
    )
    .await?;
    let mut cancelled = 0;
    for candidate in waiting.iter().filter(|candidate| {
        candidate
            .subject
            .get("application_id")
            .and_then(Value::as_str)
            == Some(application_id)
    }) {
        if let Some(request) = lock_waiting_signing_request(
            hasura_transaction,
            tenant_id,
            election_event_id,
            SigningAction::ApproveVoter,
            &candidate.scope_key,
        )
        .await?
        {
            cancel_request(
                hasura_transaction,
                &request,
                CancelReason::PayloadChanged,
                caller.actor(),
                Some("The application was rejected."),
            )
            .await?;
            cancelled += 1;
        }
    }
    Ok(cancelled)
}

/// What approving a voter reaches outside the database: Keycloak, and the
/// messages that send the credentials.
#[async_trait]
pub trait VoterApprover: Send + Sync {
    /// Whether the registry record is already a verified voter.
    async fn is_verified(&self, realm: &str, user_id: &str) -> Result<bool>;

    /// The groups of the person approving, as the application records them.
    async fn group_names(&self, tenant_realm: &str, user_id: &str) -> Result<Vec<String>>;

    /// Confirms the application: updates the voter and sends the
    /// credentials.
    #[allow(clippy::too_many_arguments)]
    async fn confirm(
        &self,
        hasura_transaction: &Transaction<'_>,
        application_id: &str,
        tenant_id: &str,
        election_event_id: &str,
        registry_user_id: &str,
        admin_id: &str,
        admin_name: &str,
        group_names: &[String],
    ) -> Result<()>;
}

/// Today's services.
#[derive(Debug, Clone, Copy, Default)]
pub struct KeycloakVoterApprover;

#[async_trait]
impl VoterApprover for KeycloakVoterApprover {
    async fn is_verified(&self, realm: &str, user_id: &str) -> Result<bool> {
        let mut client = get_keycloak_pool()
            .await
            .get()
            .await
            .map_err(|error| anyhow!("Error getting the Keycloak client: {error:?}"))?;
        let transaction = client.transaction().await?;
        let verified = check_is_user_verified(&transaction, realm, user_id).await?;
        transaction.rollback().await?;
        Ok(verified)
    }

    async fn group_names(&self, tenant_realm: &str, user_id: &str) -> Result<Vec<String>> {
        get_group_names(tenant_realm, user_id).await
    }

    async fn confirm(
        &self,
        hasura_transaction: &Transaction<'_>,
        application_id: &str,
        tenant_id: &str,
        election_event_id: &str,
        registry_user_id: &str,
        admin_id: &str,
        admin_name: &str,
        group_names: &[String],
    ) -> Result<()> {
        confirm_application(
            hasura_transaction,
            application_id,
            tenant_id,
            election_event_id,
            registry_user_id,
            admin_id,
            admin_name,
            &group_names.to_vec(),
        )
        .await?;
        Ok(())
    }
}

/// The effect of a signed voter approval: once the application is as it was
/// signed and its registry record not verified yet, the person who started
/// it confirms it (and is its verifier), which issues the voter's
/// credentials; the application keeps the signing request and its signers.
pub async fn approve(
    hasura_transaction: &Transaction<'_>,
    request: &SigningRequestRow,
    approvals: &[SigningApprovalRow],
    voters: &dyn VoterApprover,
    progress: &EffectProgress,
) -> Result<Value> {
    let subject: VoterApprovalSubject = subject_of(request)?;
    let application_id = Uuid::parse_str(&subject.application_id)?;
    let application = get_application_for_signing(
        hasura_transaction,
        request.tenant_id,
        request.election_event_id,
        application_id,
    )
    .await?
    .ok_or_else(|| refuse("application-missing", "The application no longer exists."))?;
    if application.status != subject.status {
        return Err(refuse(
            "application-changed",
            format!(
                "The application is {} now, not {}.",
                application.status, subject.status
            ),
        ));
    }
    let tenant_id = request.tenant_id.to_string();
    let election_event_id = request.election_event_id.to_string();
    if voters
        .is_verified(
            &get_event_realm(&tenant_id, &election_event_id),
            &subject.applicant_registry_id,
        )
        .await?
    {
        return Err(refuse(
            "already-verified",
            "The registry record is already an approved voter.",
        ));
    }
    let group_names = voters
        .group_names(&get_tenant_realm(&tenant_id), &request.requested_by)
        .await?;
    let mut signers: Vec<&str> = approvals
        .iter()
        .map(|approval| approval.username.as_str())
        .collect();
    signers.sort();
    annotate_application(
        hasura_transaction,
        request.tenant_id,
        request.election_event_id,
        application_id,
        &json!({
            "signing_request_id": request.id,
            "signing_code": request.code,
            "signed_by": signers,
        }),
    )
    .await?;
    let admin_name = request
        .requested_by_name
        .clone()
        .unwrap_or_else(|| request.requested_by_username.clone());
    progress.reach_outside();
    voters
        .confirm(
            hasura_transaction,
            &subject.application_id,
            &tenant_id,
            &election_event_id,
            &subject.applicant_registry_id,
            &request.requested_by,
            &admin_name,
            &group_names,
        )
        .await?;
    Ok(json!({
        "application_id": subject.application_id,
        "applicant_registry_id": subject.applicant_registry_id,
    }))
}
