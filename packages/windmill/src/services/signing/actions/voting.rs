// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Opening and closing voting at a Post (`/update-election-voting-status`).
//!
//! The request signs the channels it changes and each one's status before
//! (`{channels, from}`; `from` entries read `CHANNEL=STATUS`). Without
//! channels, the event's enabled channels change. The gate refuses a change
//! the Post doesn't allow now, and, while closing needs signatures,
//! reopening a closed channel: voting closed under signatures stays closed.
//! Pausing, and changes of the whole event, are not signing actions.
//!
//! Executing one cancels the waiting requests of the opposite action at the
//! Post. Closing keeps a seal record (A4): the closing signatures, which the
//! SigningActionExecuted entry carries. Sealing the ballots belongs to the
//! close itself (`update_election_status`, once VOTE-FREEZE builds it), on
//! every close, signed or not. After a signed close, in its transaction, the
//! [`super::SealRecordSink`] hands the closing signatures to those seals,
//! and the record shows one [`SealSummary`] per country seal.

use super::{event_ids, gate, is_required, refuse, subject_of, EffectProgress};
use crate::postgres::election::get_election_by_id;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::signing::{
    list_waiting_signing_requests, lock_waiting_signing_request, SigningApprovalRow,
    SigningRequestRow,
};
use crate::services::election_event_status::{get_election_status, voting_transition_refusal};
use crate::services::signing::guard::{GuardOutcome, GuardRequest, RequestScope};
use crate::services::signing::requests::{cancel_request, certificate_names, requester};
use crate::services::signing::{InvalidReason, SigningCaller, SigningError, SigningResult};
use crate::services::voting_status::{resolve_voting_channels, update_election_status};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use sequent_core::ballot::{ElectionStatus, VotingStatus, VotingStatusChannel};
use sequent_core::signing::{CancelReason, SigningAction};
use sequent_core::types::hasura::core::Election;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::str::FromStr;
use uuid::Uuid;

/// The protected action a status change is, if it is one.
pub fn action_of(status: &VotingStatus) -> Option<SigningAction> {
    match status {
        VotingStatus::OPEN => Some(SigningAction::OpenVoting),
        VotingStatus::CLOSED => Some(SigningAction::CloseVoting),
        VotingStatus::NOT_STARTED | VotingStatus::PAUSED => None,
    }
}

/// The status a protected status change sets.
fn target_of(action: SigningAction) -> Result<VotingStatus> {
    match action {
        SigningAction::OpenVoting => Ok(VotingStatus::OPEN),
        SigningAction::CloseVoting => Ok(VotingStatus::CLOSED),
        other => Err(anyhow!("{other} is not a voting status change")),
    }
}

/// What an open or close voting request signs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VotingSubject {
    /// The channels it changes, sorted.
    pub channels: Vec<String>,
    /// Each channel's status before, as `CHANNEL=STATUS`, sorted.
    pub from: Vec<String>,
}

impl VotingSubject {
    /// The subject of changing `from`'s channels.
    pub fn new(from: &[(VotingStatusChannel, VotingStatus)]) -> Self {
        let mut pairs: Vec<(String, String)> = from
            .iter()
            .map(|(channel, status)| (channel.to_string(), status.to_string()))
            .collect();
        pairs.sort();
        pairs.dedup();
        VotingSubject {
            channels: pairs.iter().map(|(channel, _)| channel.clone()).collect(),
            from: pairs
                .iter()
                .map(|(channel, status)| format!("{channel}={status}"))
                .collect(),
        }
    }

    /// The channels and each one's status before.
    pub fn pairs(&self) -> Result<Vec<(VotingStatusChannel, VotingStatus)>> {
        self.from
            .iter()
            .map(|entry| {
                let (channel, status) = entry
                    .split_once('=')
                    .ok_or_else(|| anyhow!("malformed status entry {entry}"))?;
                Ok((
                    VotingStatusChannel::from_str(channel)
                        .map_err(|_| anyhow!("unknown voting channel {channel}"))?,
                    VotingStatus::from_str(status)
                        .map_err(|_| anyhow!("unknown voting status {status}"))?,
                ))
            })
            .collect()
    }

    /// The key a request for these channels waits under.
    pub fn key(&self) -> String {
        self.channels.join(",")
    }
}

/// Each channel's status before a change of `channels` of `election` to
/// `new_status`, or why the change can't happen now.
pub fn plan(
    election: &Election,
    status: &ElectionStatus,
    channels: &[VotingStatusChannel],
    new_status: &VotingStatus,
) -> SigningResult<Vec<(VotingStatusChannel, VotingStatus)>> {
    if channels.is_empty() {
        return Err(SigningError::invalid(
            InvalidReason::Transition,
            "There is no channel to change.",
        ));
    }
    channels
        .iter()
        .map(|channel| {
            let current = status.status_by_channel(*channel);
            if current == *new_status {
                return Err(SigningError::invalid(
                    InvalidReason::Transition,
                    format!("The {channel} channel is already {new_status}."),
                ));
            }
            if let Some(refusal) = voting_transition_refusal(election, status, *channel, new_status)
            {
                return Err(SigningError::invalid(
                    InvalidReason::Transition,
                    format!(
                        "The {channel} channel can't change to {new_status} ({}).",
                        refusal.code()
                    ),
                ));
            }
            Ok((*channel, current))
        })
        .collect()
}

/// The Post and its status, and the channels a change applies to.
async fn read_post(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    election_id: Uuid,
    voting_channels: &Option<Vec<VotingStatusChannel>>,
) -> SigningResult<(Election, ElectionStatus, Vec<VotingStatusChannel>)> {
    let election_event = get_election_event_by_id(
        hasura_transaction,
        &tenant_id.to_string(),
        &election_event_id.to_string(),
    )
    .await?;
    let channels = resolve_voting_channels(&election_event, voting_channels)?;
    let election = get_election_by_id(
        hasura_transaction,
        &tenant_id.to_string(),
        &election_event_id.to_string(),
        &election_id.to_string(),
    )
    .await?
    .ok_or_else(|| SigningError::NotFound("There is no such Post.".into()))?;
    let status = get_election_status(election.status.clone()).unwrap_or_default();
    Ok((election, status, channels))
}

/// Whether a Post's status change runs now; see [`super::gate`]. While
/// closing needs signatures, reopening a closed channel is refused
/// (`closed-under-signatures`), whatever opening needs.
#[allow(clippy::too_many_arguments)]
pub async fn gate_election_status(
    hasura_transaction: &Transaction<'_>,
    caller: &SigningCaller,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    voting_status: &VotingStatus,
    voting_channels: &Option<Vec<VotingStatusChannel>>,
) -> SigningResult<GuardOutcome> {
    let Some(action) = action_of(voting_status) else {
        return Ok(GuardOutcome::Proceed);
    };
    let Some((tenant_id, election_event_id)) = event_ids(tenant_id, election_event_id) else {
        return Ok(GuardOutcome::Proceed);
    };
    let parsed_election = Uuid::parse_str(election_id).ok();
    if let (SigningAction::OpenVoting, Some(election_id)) = (action, parsed_election) {
        if is_required(
            hasura_transaction,
            tenant_id,
            election_event_id,
            SigningAction::CloseVoting,
        )
        .await?
        {
            let (_, status, channels) = read_post(
                hasura_transaction,
                tenant_id,
                election_event_id,
                election_id,
                voting_channels,
            )
            .await?;
            if let Some(channel) = channels
                .iter()
                .find(|channel| status.status_by_channel(**channel) == VotingStatus::CLOSED)
            {
                return Err(SigningError::invalid(
                    InvalidReason::ClosedUnderSignatures,
                    format!(
                        "The {channel} channel was closed under signatures: it can't be reopened through signing."
                    ),
                ));
            }
        }
    }
    gate(
        hasura_transaction,
        caller,
        action,
        tenant_id,
        election_event_id,
        || async move {
            let election_id = parsed_election
                .ok_or_else(|| SigningError::bad_input("The Post id is not a UUID."))?;
            let (election, status, channels) = read_post(
                hasura_transaction,
                tenant_id,
                election_event_id,
                election_id,
                voting_channels,
            )
            .await?;
            let subject = VotingSubject::new(&plan(&election, &status, &channels, voting_status)?);
            Ok(GuardRequest {
                action,
                scope: RequestScope {
                    tenant_id,
                    election_event_id,
                    election_id: Some(election_id),
                    area_id: None,
                    trustee_id: None,
                    subject_key: Some(subject.key()),
                },
                subject: serde_json::to_value(subject).map_err(anyhow::Error::from)?,
                document: None,
                config_revision: None,
            })
        },
    )
    .await
}

/// The effect of a signed status change: the change the route would have
/// made, by the person who started it, once every channel is still as it
/// was signed and may change.
pub async fn change_status(
    hasura_transaction: &Transaction<'_>,
    request: &SigningRequestRow,
    progress: &EffectProgress,
) -> Result<Value> {
    let target = target_of(request.action)?;
    let subject: VotingSubject = subject_of(request)?;
    let signed = subject.pairs()?;
    let election_id = request
        .election_id
        .ok_or_else(|| anyhow!("signing request {} has no Post", request.id))?;
    let election = get_election_by_id(
        hasura_transaction,
        &request.tenant_id.to_string(),
        &request.election_event_id.to_string(),
        &election_id.to_string(),
    )
    .await?
    .ok_or_else(|| refuse("post-missing", "The Post no longer exists."))?;
    let status = get_election_status(election.status.clone()).unwrap_or_default();
    for (channel, from) in &signed {
        if status.status_by_channel(*channel) != *from {
            return Err(refuse(
                "state-changed",
                format!("The {channel} channel is no longer {from}."),
            ));
        }
        if let Some(refusal) = voting_transition_refusal(&election, &status, *channel, &target) {
            return Err(refuse(
                refusal.code(),
                format!("The {channel} channel can't change."),
            ));
        }
    }
    let channels: Vec<VotingStatusChannel> = signed.iter().map(|(channel, _)| *channel).collect();
    progress.reach_outside();
    update_election_status(
        request.tenant_id.to_string(),
        Some(&request.requested_by),
        Some(&request.requested_by_username),
        hasura_transaction,
        &request.election_event_id.to_string(),
        &election_id.to_string(),
        &target,
        &Some(channels),
    )
    .await?;
    Ok(json!({
        "voting_status": target.to_string(),
        "channels": subject.channels,
        "from": subject.from,
    }))
}

/// Whether a status change's result changed any channel.
pub fn transitioned(result: &Value) -> bool {
    result
        .get("channels")
        .and_then(Value::as_array)
        .is_some_and(|channels| !channels.is_empty())
}

/// Cancels the waiting requests of the opposite action at the request's
/// Post, under the event's signing lock: opening voting cancels a waiting
/// close, and closing it a waiting open.
pub async fn cancel_opposite(
    hasura_transaction: &Transaction<'_>,
    request: &SigningRequestRow,
) -> Result<usize> {
    let opposite = match request.action {
        SigningAction::OpenVoting => SigningAction::CloseVoting,
        SigningAction::CloseVoting => SigningAction::OpenVoting,
        _ => return Ok(0),
    };
    let waiting = list_waiting_signing_requests(
        hasura_transaction,
        request.tenant_id,
        request.election_event_id,
        opposite,
    )
    .await?;
    let mut cancelled = 0;
    for candidate in waiting
        .iter()
        .filter(|candidate| candidate.election_id == request.election_id)
    {
        if let Some(locked) = lock_waiting_signing_request(
            hasura_transaction,
            request.tenant_id,
            request.election_event_id,
            opposite,
            &candidate.scope_key,
        )
        .await?
        {
            cancel_request(
                hasura_transaction,
                &locked,
                CancelReason::PayloadChanged,
                requester(request),
                Some(&format!(
                    "Signing request {} changed voting first.",
                    request.code
                )),
            )
            .await?;
            cancelled += 1;
        }
    }
    Ok(cancelled)
}

/// A closing signature in the seal record.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ClosingSignature {
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub signed_at: DateTime<Utc>,
    pub certificate_fingerprint: String,
    pub certificate_subject: Option<String>,
    pub certificate_cn: Option<String>,
}

/// What the seal record shows of one seal the close made; the seal itself
/// is VOTE-FREEZE's, one per country.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SealSummary {
    /// The country (area) whose ballot box is sealed.
    pub area_id: Option<Uuid>,
    pub area_name: Option<String>,
    /// The hash algorithm, as the seal states it.
    pub hash_algorithm: String,
    pub hash: String,
    pub ballots: Option<u64>,
    pub signed_by: Option<String>,
}

/// The seal record of closing voting at a Post: when, which channels from
/// which status, under which signing code, the closing signatures, and what
/// it shows of the seals the close made (none until VOTE-FREEZE).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SealRecord {
    pub closed_at: DateTime<Utc>,
    pub election_id: Option<Uuid>,
    pub channels: Vec<String>,
    pub from: Vec<String>,
    pub code: String,
    pub payload_sha256: String,
    pub signatures: Vec<ClosingSignature>,
    pub seals: Vec<SealSummary>,
}

impl SealRecord {
    /// The record of a closing request and its approvals; `effect` is what
    /// the status change answered, `closed_at` the database's time.
    pub fn new(
        request: &SigningRequestRow,
        approvals: &[SigningApprovalRow],
        effect: &Value,
        closed_at: DateTime<Utc>,
    ) -> Self {
        let strings = |field: &str| -> Vec<String> {
            effect
                .get(field)
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default()
        };
        let mut signatures: Vec<ClosingSignature> = approvals
            .iter()
            .map(|approval| {
                let (subject, common_name) = certificate_names(&approval.certificate_pem);
                ClosingSignature {
                    user_id: approval.user_id.clone(),
                    username: approval.username.clone(),
                    display_name: approval
                        .display_name
                        .clone()
                        .unwrap_or_else(|| approval.username.clone()),
                    signed_at: approval.signed_at,
                    certificate_fingerprint: approval.fingerprint_sha256.clone(),
                    certificate_subject: subject,
                    certificate_cn: common_name,
                }
            })
            .collect();
        signatures.sort_by(|a, b| {
            a.signed_at
                .cmp(&b.signed_at)
                .then(a.user_id.cmp(&b.user_id))
        });
        SealRecord {
            closed_at,
            election_id: request.election_id,
            channels: strings("channels"),
            from: strings("from"),
            code: request.code.clone(),
            payload_sha256: request.payload_sha256.clone(),
            signatures,
            seals: vec![],
        }
    }
}

/// Who a scheduled change is logged for.
pub const SCHEDULER: &str = "scheduled-event";

/// Whether a scheduled status change (a start or end date) must leave the
/// change to people, because the action's rule needs signatures; when it
/// must, logs that it did not run (a SYSTEM error entry) and warns. A
/// change for the whole event (`election_id` `None`) is checked once.
pub async fn scheduled_change_needs_signatures(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<&str>,
    voting_status: &VotingStatus,
    scheduled_event_id: &str,
) -> Result<bool> {
    let Some(action) = action_of(voting_status) else {
        return Ok(false);
    };
    let Some((tenant, event)) = event_ids(tenant_id, election_event_id) else {
        return Ok(false);
    };
    if !is_required(hasura_transaction, tenant, event, action).await? {
        return Ok(false);
    }
    let election = election_id.and_then(|id| Uuid::parse_str(id).ok());
    let description = format!(
        "Did not {} on schedule: it needs signatures",
        crate::services::signing::action_title(action).to_lowercase()
    );
    tracing::warn!(%election_event_id, ?election_id, %scheduled_event_id, "{description}");
    crate::services::signing::log::stage(
        hasura_transaction,
        &crate::services::signing::log::LogStep {
            kind: electoral_log::messages::newtypes::SigningStatementKind::SigningActionExecuted,
            user: crate::services::signing::log::Actor {
                user_id: SCHEDULER.to_owned(),
                username: SCHEDULER.to_owned(),
            },
            system: crate::services::signing::log::SystemOutcome::Error,
            scope: crate::services::signing::log_scope(tenant, event, election, None),
            description,
            details: json!({
                "action": action,
                "election_id": election,
                "scheduled_event_id": scheduled_event_id,
                "reason": "signing-required",
            }),
        },
    )
    .await?;
    Ok(true)
}
