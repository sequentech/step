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
    count_signing_approvals, list_waiting_signing_requests, lock_signing_event,
    lock_waiting_signing_request, SigningApprovalRow, SigningRequestRow,
};
use crate::services::election_event_status::{
    get_election_status, scheduled_transition_applies, voting_transition_refusal,
};
use crate::services::scheduled_outcome::{
    event_wide_targets, fire_time_state, fired_words, keys, mark_fired, record_fired_outcome,
    EventState, Moment, ScheduledRow,
};
use crate::services::signing::guard::{GuardOutcome, GuardRequest, RequestScope};
use crate::services::signing::log::{stage, Actor, LogStep, SystemOutcome};
use crate::services::signing::requests::{cancel_request, certificate_names, requester};
use crate::services::signing::{action_title, log_scope};
use crate::services::signing::{InvalidReason, SigningCaller, SigningError, SigningResult};
use crate::services::voting_status::{resolve_voting_channels, update_election_status};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::SigningStatementKind;
use sequent_core::ballot::{ElectionStatus, VotingStatus, VotingStatusChannel};
use sequent_core::signing::{CancelReason, SigningAction};
use sequent_core::types::hasura::core::Election;
use sequent_core::types::hasura::core::VotingChannels;
use sequent_core::types::scheduled_event::ManageElectionDatePayload;
use sequent_core::types::scheduled_event::{EventProcessors, ScheduledEvent};
use sequent_core::types::scheduled_outcome::{
    AuthorizedBy, CheckId, Explanation, ScheduledOutcomeKind, ScheduledTransition,
};
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

/// Why opening `election` is refused now with respect to its initialization
/// at the event's scope (VOTE-LIFECYCLE §9); `None` for other statuses.
async fn initialization_refusal_now(
    hasura_transaction: &Transaction<'_>,
    election: &Election,
    new_status: &VotingStatus,
) -> Result<Option<crate::services::election_event_status::TransitionRefusal>> {
    if *new_status != VotingStatus::OPEN {
        return Ok(None);
    }
    let election_event = get_election_event_by_id(
        hasura_transaction,
        &election.tenant_id,
        &election.election_event_id,
    )
    .await?;
    crate::services::initialization_scope::initialization_refusal_for(
        hasura_transaction,
        &election_event,
        election,
    )
    .await
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
            // Not before the Post is initialized at the event's scope (§9).
            if let Some(refusal) =
                initialization_refusal_now(hasura_transaction, &election, voting_status).await?
            {
                return Err(SigningError::invalid(
                    InvalidReason::Transition,
                    refusal.message(&election.id, voting_status, &VotingStatus::NOT_STARTED),
                ));
            }
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
    // Refused, not failed, while the Post isn't initialized at the scope.
    if let Some(refusal) =
        initialization_refusal_now(hasura_transaction, &election, &target).await?
    {
        return Err(refuse(
            refusal.code(),
            refusal.message(&election.id, &target, &VotingStatus::NOT_STARTED),
        ));
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
    /// The signing code of the closing request; empty (and left out) for a
    /// scheduled close, whose authorization is `authorized_by`.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub code: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub payload_sha256: String,
    pub signatures: Vec<ClosingSignature>,
    pub seals: Vec<SealSummary>,
    /// A scheduled close that a signed configuration authorized: that
    /// approval (VOTE-LIFECYCLE §5a). Its signers authorized the schedule;
    /// they are not closing signatures.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorized_by: Option<AuthorizedBy>,
    /// A scheduled close that ran at its deadline without signatures.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub unsigned: bool,
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
            authorized_by: None,
            unsigned: false,
        }
    }
}

/// Who a scheduled change is logged for.
pub const SCHEDULER: &str = "scheduled-event";

fn scheduler() -> Actor {
    Actor {
        user_id: SCHEDULER.to_owned(),
        username: SCHEDULER.to_owned(),
    }
}

/// What a scheduled change does at one Post.
#[derive(Debug, Clone, PartialEq)]
pub struct ScheduledDecision {
    pub election_id: Uuid,
    pub explanation: Explanation,
}

impl ScheduledDecision {
    pub fn runs(&self) -> bool {
        matches!(
            self.explanation.outcome,
            ScheduledOutcomeKind::Runs | ScheduledOutcomeKind::RunsUnsigned
        )
    }

    /// Whether the action needed signatures: only then is the outcome a
    /// signing step.
    fn needed_signatures(&self) -> bool {
        self.explanation
            .checks
            .iter()
            .any(|check| check.id == CheckId::NeedsSignatures && !check.allows)
    }
}

/// The reason code a refused scheduled change logs: a signed transition
/// that already ran there (`replay`) or fires late (`late-fire`), one not
/// in the signed configuration, or one that needs signatures.
fn refusal_reason(explanation: &Explanation) -> &'static str {
    let covered = explanation
        .checks
        .iter()
        .find(|check| check.id == CheckId::Covered)
        .map(|check| check.current.message_key.as_str());
    match covered {
        Some(keys::COVERED_ALREADY_FIRED) => "replay",
        Some(keys::COVERED_LATE) => "late-fire",
        _ if explanation.deciding == CheckId::Covered => "not-in-signed-configuration",
        _ => "signing-required",
    }
}

/// The channels a scheduled change of a Post changes, each with its status
/// before, as the scheduled task computes them.
async fn scheduled_channels(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    election_id: Uuid,
    row: &ScheduledRow,
    voting_status: &VotingStatus,
) -> Result<Vec<(VotingStatusChannel, VotingStatus)>> {
    let Some(election) = get_election_by_id(
        hasura_transaction,
        &tenant_id.to_string(),
        &election_event_id.to_string(),
        &election_id.to_string(),
    )
    .await?
    else {
        return Ok(vec![]);
    };
    let configured = election
        .voting_channels
        .clone()
        .map(serde_json::from_value::<VotingChannels>)
        .transpose()?
        .unwrap_or_default();
    let status = get_election_status(election.status.clone()).unwrap_or_default();
    Ok(row
        .payload()
        .enabled_channels(&configured)
        .into_iter()
        .map(|channel| (channel, status.status_by_channel(channel)))
        .filter(|(_, current)| scheduled_transition_applies(current, voting_status))
        .collect())
}

/// Cancels waiting requests of `action` at the Post that `fits` keeps, with
/// `reason`; what each had.
async fn cancel_waiting(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    election_id: Uuid,
    action: SigningAction,
    fits: impl Fn(&SigningRequestRow) -> bool,
    reason: CancelReason,
    note: &str,
) -> Result<Vec<Value>> {
    let waiting =
        list_waiting_signing_requests(hasura_transaction, tenant_id, election_event_id, action)
            .await?;
    let mut cancelled = vec![];
    for candidate in waiting
        .iter()
        .filter(|candidate| candidate.election_id == Some(election_id) && fits(candidate))
    {
        let Some(locked) = lock_waiting_signing_request(
            hasura_transaction,
            tenant_id,
            election_event_id,
            action,
            &candidate.scope_key,
        )
        .await?
        else {
            continue;
        };
        let signatures =
            count_signing_approvals(hasura_transaction, tenant_id, election_event_id, locked.id)
                .await?;
        cancel_request(hasura_transaction, &locked, reason, scheduler(), Some(note)).await?;
        cancelled.push(json!({
            "request_id": locked.id,
            "action": action,
            "code": locked.code,
            "signatures": signatures,
            "required": locked.required,
        }));
    }
    Ok(cancelled)
}

/// The waiting requests a scheduled change makes moot at the Post: a close
/// cancels the close requests within the channels it closes
/// (`closed-on-schedule`; their signatures stay in the history and don't
/// count as a signed close), and, like a signed change, every waiting
/// request of the opposite action (`payload-changed`).
async fn cancel_for_scheduled_change(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    election_id: Uuid,
    action: SigningAction,
    changed: &[String],
) -> Result<Vec<Value>> {
    let mut cancelled = vec![];
    if action == SigningAction::CloseVoting {
        cancelled.extend(
            cancel_waiting(
                hasura_transaction,
                tenant_id,
                election_event_id,
                election_id,
                SigningAction::CloseVoting,
                |request| {
                    subject_of::<VotingSubject>(request).is_ok_and(|subject| {
                        subject
                            .channels
                            .iter()
                            .all(|channel| changed.contains(channel))
                    })
                },
                CancelReason::ClosedOnSchedule,
                "The schedule closed voting at its deadline.",
            )
            .await?,
        );
    }
    let opposite = match action {
        SigningAction::CloseVoting => SigningAction::OpenVoting,
        _ => SigningAction::CloseVoting,
    };
    cancelled.extend(
        cancel_waiting(
            hasura_transaction,
            tenant_id,
            election_event_id,
            election_id,
            opposite,
            |_| true,
            CancelReason::PayloadChanged,
            "The schedule changed voting first.",
        )
        .await?,
    );
    Ok(cancelled)
}

/// Logs (and returns for the row's record) what a scheduled change does at
/// a Post that runs it. A change with no channel to change (the Post is
/// already open or closed) records that and cancels nothing.
#[allow(clippy::too_many_arguments)]
async fn run_at_post(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    state: &EventState,
    row: &ScheduledRow,
    decision: &ScheduledDecision,
    voting_status: &VotingStatus,
) -> Result<Value> {
    let action = row.action();
    let post = decision.election_id;
    let explanation = &decision.explanation;
    let pairs = scheduled_channels(
        hasura_transaction,
        tenant_id,
        election_event_id,
        post,
        row,
        voting_status,
    )
    .await?;
    let subject = VotingSubject::new(&pairs);
    let mut details = json!({
        "action": action,
        "election_id": post,
        "scheduled_event_id": row.transition.scheduled_event_id,
        "outcome": explanation.outcome,
        "authorized_by": explanation.authorized_by,
        "unsigned": explanation.outcome == ScheduledOutcomeKind::RunsUnsigned,
        "fingerprint": row.transition.fingerprint,
        "channels": subject.channels,
        "explanation": explanation,
    });
    if explanation.outcome == ScheduledOutcomeKind::RunsUnsigned {
        details["reason"] = json!("unsigned-scheduled-close");
    }
    let post_name = state.post_name(Some(post));
    // Once: the same signed transition doesn't run again at this Post, even
    // when it had nothing to change.
    mark_fired(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &row.transition,
        post,
        &pairs
            .iter()
            .map(|(channel, _)| *channel)
            .collect::<Vec<_>>(),
    )
    .await?;
    let description = if pairs.is_empty() {
        details["nothing_to_change"] = json!(true);
        format!(
            "Nothing to {} at {post_name} on schedule: no channel can change",
            action_title(action).to_lowercase()
        )
    } else {
        let cancelled = cancel_for_scheduled_change(
            hasura_transaction,
            tenant_id,
            election_event_id,
            post,
            action,
            &subject.channels,
        )
        .await?;
        details["cancelled"] = json!(cancelled);
        if action == SigningAction::CloseVoting {
            let closed_at: DateTime<Utc> = hasura_transaction
                .query_one("SELECT clock_timestamp()", &[])
                .await?
                .try_get(0)?;
            // No closing signatures: the approval that authorized the
            // schedule, or none at all.
            let record = SealRecord {
                closed_at,
                election_id: Some(post),
                channels: subject.channels.clone(),
                from: subject.from.clone(),
                code: String::new(),
                payload_sha256: String::new(),
                signatures: vec![],
                seals: vec![],
                authorized_by: explanation.authorized_by.clone(),
                unsigned: explanation.outcome == ScheduledOutcomeKind::RunsUnsigned,
            };
            details["record"] = serde_json::to_value(&record)?;
        }
        fired_words(action, explanation, &post_name)
    };
    if decision.needed_signatures() {
        stage(
            hasura_transaction,
            &LogStep {
                kind: SigningStatementKind::SigningActionExecuted,
                user: scheduler(),
                system: SystemOutcome::Info,
                scope: log_scope(tenant_id, election_event_id, Some(post), None),
                description,
                details: details.clone(),
            },
        )
        .await?;
    }
    Ok(details)
}

/// Logs a scheduled change that doesn't run at a Post (a SYSTEM error);
/// `explanation` is the Post's own, or `None` when another Post's refusal
/// held it back.
async fn refuse_at_post(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    row: &ScheduledRow,
    election_id: Uuid,
    explanation: Option<&Explanation>,
    reason: &str,
    description: String,
) -> Result<Value> {
    let details = json!({
        "action": row.action(),
        "election_id": election_id,
        "scheduled_event_id": row.transition.scheduled_event_id,
        "outcome": ScheduledOutcomeKind::Refused,
        "authorized_by": Value::Null,
        "unsigned": false,
        "fingerprint": row.transition.fingerprint,
        "reason": reason,
        "explanation": explanation,
    });
    tracing::warn!(%election_event_id, %election_id, scheduled_event_id = %row.transition.scheduled_event_id, "{description}");
    stage(
        hasura_transaction,
        &LogStep {
            kind: SigningStatementKind::SigningActionExecuted,
            user: scheduler(),
            system: SystemOutcome::Error,
            scope: log_scope(tenant_id, election_event_id, Some(election_id), None),
            description,
            details: details.clone(),
        },
    )
    .await?;
    Ok(details)
}

/// The database's time, as the fired outcome records it.
async fn fired_at(hasura_transaction: &Transaction<'_>) -> Result<DateTime<Utc>> {
    Ok(hasura_transaction
        .query_one("SELECT clock_timestamp()", &[])
        .await?
        .try_get(0)?)
}

/// A scheduled change whose row isn't a scheduled opening or closing of
/// this action (it can't be checked against a signed configuration): it
/// runs only while neither copy needs signatures; otherwise it is refused
/// and logged once (`signing-required`), for the event or each Post.
async fn unknown_row(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    posts: &[String],
    action: SigningAction,
    scheduled_event_id: &str,
) -> Result<Vec<String>> {
    let state = EventState::read(hasura_transaction, tenant_id, election_event_id).await?;
    let scopes: Vec<Option<Uuid>> = if posts.is_empty() {
        vec![None]
    } else {
        posts
            .iter()
            .map(|post| Uuid::parse_str(post).ok())
            .collect()
    };
    if !scopes
        .iter()
        .any(|scope| state.needs_signatures(action, *scope))
    {
        return Ok(posts.to_vec());
    }
    for election in scopes {
        let description = format!(
            "Did not {} on schedule: it needs signatures",
            action_title(action).to_lowercase()
        );
        tracing::warn!(%election_event_id, ?election, %scheduled_event_id, "{description}");
        stage(
            hasura_transaction,
            &LogStep {
                kind: SigningStatementKind::SigningActionExecuted,
                user: scheduler(),
                system: SystemOutcome::Error,
                scope: log_scope(tenant_id, election_event_id, election, None),
                description,
                details: json!({
                    "action": action,
                    "election_id": election,
                    "scheduled_event_id": scheduled_event_id,
                    "outcome": ScheduledOutcomeKind::Refused,
                    "authorized_by": Value::Null,
                    "reason": "signing-required",
                }),
            },
        )
        .await?;
    }
    Ok(vec![])
}

/// What a scheduled change did at one Post: whether it runs there (the
/// caller changes only those), why, and what was logged and recorded.
#[derive(Debug, Clone, PartialEq)]
pub struct PostDecision {
    pub election_id: String,
    pub runs: bool,
    /// `None` for a row that couldn't be read as a scheduled opening or
    /// closing (decided by the rules alone).
    pub explanation: Option<Explanation>,
    pub details: Value,
}

/// What a scheduled change (a START/END_VOTING_PERIOD row) does at each of
/// `posts`, under [`crate::services::scheduled_outcome`] at fire time, with
/// its log entries: a covered change runs, authorized by the signed
/// configuration; an uncovered opening is refused; an uncovered close runs
/// without signatures only when both copies say `RUN_AS_SYSTEM`. A close
/// that runs cancels the Post's waiting close requests within its channels
/// (`closed-on-schedule`), and any change the waiting requests of the
/// opposite action. Each Post gets one SYSTEM SigningActionExecuted entry,
/// with its own explanation, when the action needed signatures, and the row
/// keeps what happened (`annotations.fired_outcome`).
pub async fn scheduled_decisions(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    posts: &[String],
    voting_status: &VotingStatus,
    scheduled_event_id: &str,
) -> Result<Vec<PostDecision>> {
    let runs_everywhere = || {
        posts
            .iter()
            .map(|post| PostDecision {
                election_id: post.clone(),
                runs: true,
                explanation: None,
                details: Value::Null,
            })
            .collect::<Vec<_>>()
    };
    let Some(action) = action_of(voting_status) else {
        return Ok(runs_everywhere());
    };
    let Some((tenant, event)) = event_ids(tenant_id, election_event_id) else {
        return Ok(runs_everywhere());
    };
    // The signing lock before any read or row lock.
    lock_signing_event(hasura_transaction, tenant, event).await?;
    let Some((state, row)) = fire_time_state(hasura_transaction, tenant, event, scheduled_event_id)
        .await?
        .filter(|(_, row)| row.action() == action)
    else {
        let runs = unknown_row(
            hasura_transaction,
            tenant,
            event,
            posts,
            action,
            scheduled_event_id,
        )
        .await?;
        return Ok(posts
            .iter()
            .map(|post| PostDecision {
                election_id: post.clone(),
                runs: runs.contains(post),
                explanation: None,
                details: Value::Null,
            })
            .collect());
    };
    let targets = state.posts_of(&row);
    let mut decisions = vec![];
    for post in posts {
        // Only canonical ids, as stored.
        let Some(post_id) = Uuid::parse_str(post)
            .ok()
            .filter(|post_id| post_id.to_string() == *post)
        else {
            continue;
        };
        if !targets.contains(&post_id) {
            continue;
        }
        let decision = ScheduledDecision {
            election_id: post_id,
            explanation: state.explain(&row, Some(post_id), Moment::FireTime),
        };
        let runs = decision.runs();
        let details =
            if decision.explanation.outcome == ScheduledOutcomeKind::WaitingForInitialization {
                serde_json::to_value(&decision.explanation)?
            } else if runs {
                run_at_post(
                    hasura_transaction,
                    tenant,
                    event,
                    &state,
                    &row,
                    &decision,
                    voting_status,
                )
                .await?
            } else {
                refuse_at_post(
                    hasura_transaction,
                    tenant,
                    event,
                    &row,
                    post_id,
                    Some(&decision.explanation),
                    refusal_reason(&decision.explanation),
                    fired_words(
                        action,
                        &decision.explanation,
                        &state.post_name(Some(post_id)),
                    ),
                )
                .await?
            };
        decisions.push(PostDecision {
            election_id: post.clone(),
            runs,
            explanation: Some(decision.explanation),
            details,
        });
    }
    if decisions.iter().all(|decision| {
        decision.explanation.as_ref().is_some_and(|explanation| {
            explanation.outcome == ScheduledOutcomeKind::WaitingForInitialization
        })
    }) {
        return Ok(decisions);
    }
    record_fired_outcome(
        hasura_transaction,
        tenant,
        event,
        scheduled_event_id,
        &json!({
            "at": fired_at(hasura_transaction).await?,
            "posts": decisions.iter().filter(|decision| !decision.explanation.as_ref().is_some_and(
                |explanation| explanation.outcome == ScheduledOutcomeKind::WaitingForInitialization
            )).map(|decision| decision.details.clone()).collect::<Vec<_>>(),
        }),
    )
    .await?;
    Ok(decisions)
}

/// [`scheduled_decisions`]: the Posts it runs at.
pub async fn scheduled_change_for_posts(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    posts: &[String],
    voting_status: &VotingStatus,
    scheduled_event_id: &str,
) -> Result<Vec<String>> {
    Ok(scheduled_decisions(
        hasura_transaction,
        tenant_id,
        election_event_id,
        posts,
        voting_status,
        scheduled_event_id,
    )
    .await?
    .into_iter()
    .filter(|decision| decision.runs)
    .map(|decision| decision.election_id)
    .collect())
}

/// What an event-wide START/END_VOTING_PERIOD does, decided and logged at
/// each Post it changes ([`event_wide_targets`]): the event-wide task
/// changes only the Posts whose decision `runs`.
pub async fn event_wide_decision(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    scheduled_event: &ScheduledEvent,
) -> Result<Vec<PostDecision>> {
    let voting_status = match scheduled_event.event_processor {
        Some(EventProcessors::START_VOTING_PERIOD) => VotingStatus::OPEN,
        Some(EventProcessors::END_VOTING_PERIOD) => VotingStatus::CLOSED,
        _ => return Ok(vec![]),
    };
    if let Some((tenant, event)) = event_ids(tenant_id, election_event_id) {
        lock_signing_event(hasura_transaction, tenant, event).await?;
    }
    let posts = event_wide_targets(
        hasura_transaction,
        tenant_id,
        election_event_id,
        scheduled_event,
    )
    .await?;
    scheduled_decisions(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &posts,
        &voting_status,
        &scheduled_event.id,
    )
    .await
}

/// Whether a scheduled status change (a start or end date) must leave the
/// change to people; see [`scheduled_change_for_posts`], which logs each
/// Post's outcome. A change for the whole event (`election_id` `None`) is
/// decided at each Post it applies to and, since this answer is for the
/// whole event, runs only if it runs at every one of them: when it is
/// refused at one, the others log a refusal too (`event-wide-refused`,
/// without an explanation of their own). The per-Post event-wide close
/// calls [`scheduled_change_for_posts`] instead.
pub async fn scheduled_change_needs_signatures(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<&str>,
    voting_status: &VotingStatus,
    scheduled_event_id: &str,
) -> Result<bool> {
    if let Some(election_id) = election_id {
        let runs = scheduled_change_for_posts(
            hasura_transaction,
            tenant_id,
            election_event_id,
            &[election_id.to_owned()],
            voting_status,
            scheduled_event_id,
        )
        .await?;
        return Ok(runs.is_empty());
    }
    let Some(action) = action_of(voting_status) else {
        return Ok(false);
    };
    let Some((tenant, event)) = event_ids(tenant_id, election_event_id) else {
        return Ok(false);
    };
    lock_signing_event(hasura_transaction, tenant, event).await?;
    let Some((state, row)) = fire_time_state(hasura_transaction, tenant, event, scheduled_event_id)
        .await?
        .filter(|(_, row)| row.action() == action)
    else {
        // Checked once for the event, logged when refused.
        let refused = state_refuses(hasura_transaction, tenant, event, action).await?;
        unknown_row(
            hasura_transaction,
            tenant,
            event,
            &[],
            action,
            scheduled_event_id,
        )
        .await?;
        return Ok(refused);
    };
    let posts = state.posts_of(&row);
    let decisions: Vec<ScheduledDecision> = posts
        .iter()
        .map(|post| ScheduledDecision {
            election_id: *post,
            explanation: state.explain(&row, Some(*post), Moment::FireTime),
        })
        .collect();
    let refused = decisions.iter().filter(|decision| !decision.runs()).count();
    if refused == 0 {
        let posts: Vec<String> = posts.iter().map(Uuid::to_string).collect();
        scheduled_change_for_posts(
            hasura_transaction,
            tenant_id,
            election_event_id,
            &posts,
            voting_status,
            scheduled_event_id,
        )
        .await?;
        return Ok(false);
    }
    let mut fired = vec![];
    for decision in &decisions {
        let post_name = state.post_name(Some(decision.election_id));
        let details = if decision.runs() {
            refuse_at_post(
                hasura_transaction,
                tenant,
                event,
                &row,
                decision.election_id,
                None,
                "event-wide-refused",
                format!(
                    "Did not {} at {post_name} on schedule: the event-wide change was refused at {refused} Post(s)",
                    action_title(action).to_lowercase()
                ),
            )
            .await?
        } else {
            refuse_at_post(
                hasura_transaction,
                tenant,
                event,
                &row,
                decision.election_id,
                Some(&decision.explanation),
                refusal_reason(&decision.explanation),
                fired_words(action, &decision.explanation, &post_name),
            )
            .await?
        };
        fired.push(details);
    }
    record_fired_outcome(
        hasura_transaction,
        tenant,
        event,
        scheduled_event_id,
        &json!({ "at": fired_at(hasura_transaction).await?, "posts": fired }),
    )
    .await?;
    Ok(true)
}

/// Whether either copy of the event's rule for `action` needs signatures.
async fn state_refuses(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    action: SigningAction,
) -> Result<bool> {
    Ok(
        EventState::read(hasura_transaction, tenant_id, election_event_id)
            .await?
            .needs_signatures(action, None),
    )
}

/// Closes voting on some channels of a Post, as a signed close authorizes.
#[async_trait::async_trait]
pub trait PostCloser: Send + Sync {
    async fn close(
        &self,
        hasura_transaction: &Transaction<'_>,
        tenant_id: Uuid,
        election_event_id: Uuid,
        election_id: Uuid,
        channels: &[VotingStatusChannel],
    ) -> Result<()>;
}

/// Closes through the voting status service, as the scheduler's close does.
#[derive(Debug, Clone, Copy, Default)]
pub struct StatusCloser;

#[async_trait::async_trait]
impl PostCloser for StatusCloser {
    async fn close(
        &self,
        hasura_transaction: &Transaction<'_>,
        tenant_id: Uuid,
        election_event_id: Uuid,
        election_id: Uuid,
        channels: &[VotingStatusChannel],
    ) -> Result<()> {
        update_election_status(
            tenant_id.to_string(),
            None,
            None,
            hasura_transaction,
            &election_event_id.to_string(),
            &election_id.to_string(),
            &VotingStatus::CLOSED,
            &Some(channels.to_vec()),
        )
        .await
    }
}

/// When a signed transition is due.
fn signed_instant(transition: &ScheduledTransition) -> Option<DateTime<Utc>> {
    transition
        .scheduled_date
        .as_deref()
        .and_then(|date| DateTime::parse_from_rfc3339(date).ok())
        .map(|date| date.with_timezone(&Utc))
}

/// The signed rows effective at a Post, shared by execution and the read model.
fn effective_signed_transitions<'a>(
    approval: &'a crate::services::scheduled_outcome::Approval,
    post: Uuid,
    processor: &str,
) -> Vec<&'a ScheduledTransition> {
    let post_text = post.to_string();
    let mut own: Vec<_> = approval
        .schedule
        .iter()
        .filter(|row| {
            row.event_processor == processor
                && row.election_id.as_deref() == Some(post_text.as_str())
        })
        .collect();
    let overrides_event = own
        .iter()
        .any(|transition| signed_channels(transition).contains(&VotingStatusChannel::ONLINE));
    if !overrides_event {
        own.extend(
            approval
                .schedule
                .iter()
                .filter(|row| row.event_processor == processor && row.election_id.is_none()),
        );
    }
    own
}

fn signed_channels(transition: &ScheduledTransition) -> Vec<VotingStatusChannel> {
    ScheduledRow {
        transition: transition.clone(),
        annotations: json!({}),
        written_now: false,
    }
    .payload()
    .channels()
}

fn remaining_signed_close_channels(
    state: &EventState,
    approval: &crate::services::scheduled_outcome::Approval,
    post: Uuid,
    close: &ScheduledTransition,
    due: DateTime<Utc>,
) -> Vec<VotingStatusChannel> {
    let opens = effective_signed_transitions(approval, post, "START_VOTING_PERIOD");
    signed_channels(close)
        .into_iter()
        .filter(|channel| {
            !opens.iter().any(|open| {
                signed_instant(open).is_some_and(|at| {
                    due < at
                        && at <= state.now
                        && signed_channels(open).contains(channel)
                        && state
                            .fired_effects
                            .get(&(
                                open.scheduled_event_id.clone(),
                                post,
                                open.fingerprint.clone(),
                            ))
                            .is_some_and(|effects| {
                                effects.iter().any(|effect| {
                                    effect.channels.contains(channel)
                                        && effect.fired_at >= at
                                        && effect.fired_at > due
                                        && effect.fired_at <= state.now
                                })
                            })
                })
            })
        })
        .collect()
}

/// The authoritative close stays visible even if its editable row is removed.
#[derive(Debug, Clone, Serialize)]
pub struct RetainedSignedClose {
    pub fingerprint: String,
    pub scheduled_event_id: String,
    pub election_id: Uuid,
    pub scheduled_at: String,
    pub channels: Vec<VotingStatusChannel>,
    pub authorized_by: AuthorizedBy,
    pub fired_at: Option<String>,
}

pub async fn retained_signed_closes(
    transaction: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
) -> Result<Vec<RetainedSignedClose>> {
    if get_election_event_by_id(transaction, &tenant.to_string(), &event.to_string())
        .await?
        .is_archived
    {
        return Ok(vec![]);
    }
    let state = EventState::read_for_closes(transaction, tenant, event).await?;
    let mut retained = vec![];
    for post in &state.posts {
        let Some(approval) = state.approval(Some(post.id)) else {
            continue;
        };
        for close in effective_signed_transitions(approval, post.id, "END_VOTING_PERIOD") {
            let Some(due) = signed_instant(close) else {
                continue;
            };
            let channels = remaining_signed_close_channels(&state, approval, post.id, close, due);
            let fired_at = state.fired.get(&(
                close.scheduled_event_id.clone(),
                post.id,
                close.fingerprint.clone(),
            ));
            if channels.is_empty() && fired_at.is_none() {
                continue;
            }
            retained.push(RetainedSignedClose {
                fingerprint: close.fingerprint.clone(),
                scheduled_event_id: close.scheduled_event_id.clone(),
                election_id: post.id,
                scheduled_at: due.to_rfc3339(),
                channels,
                authorized_by: approval.authorized_by(),
                fired_at: fired_at.map(DateTime::to_rfc3339),
            });
        }
    }
    retained.sort_by(|a, b| {
        (&a.scheduled_at, &a.election_id, &a.scheduled_event_id).cmp(&(
            &b.scheduled_at,
            &b.election_id,
            &b.scheduled_event_id,
        ))
    });
    Ok(retained)
}

/// Signed closes are authoritative: at each Post, the newest executed
/// configuration approval's effective closes (ONLINE own rows replace the
/// event-wide rows) whose signed times have passed close each channel that
/// no later effective signed opening (also passed) supersedes and that is
/// still open, whatever happened to the live row since (edited, stopped,
/// archived, deleted) and whatever the live channels say. Only a newer
/// approved configuration moves it. Each close runs once (it is marked as
/// run), cancels what it makes moot, and logs a SYSTEM SigningActionExecuted
/// with `reason: "signed-close"` and `authorized_by`. The scheduler calls it
/// on every tick; the Posts it closed.
pub async fn enforce_signed_closes(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    closer: &dyn PostCloser,
) -> Result<Vec<PostDecision>> {
    let Some((tenant, event)) = event_ids(tenant_id, election_event_id) else {
        return Ok(vec![]);
    };
    crate::postgres::scheduled_event::lock_elections(
        hasura_transaction,
        tenant_id,
        election_event_id,
        None,
    )
    .await?;
    if get_election_event_by_id(hasura_transaction, tenant_id, election_event_id)
        .await?
        .is_archived
    {
        return Ok(vec![]);
    }
    let state = EventState::read_for_closes(hasura_transaction, tenant, event).await?;
    let mut closed = vec![];
    for post in state.posts.iter().map(|post| post.id) {
        let Some(approval) = state.approval(Some(post)) else {
            continue;
        };
        let post_text = post.to_string();
        let mut closes: Vec<_> = effective_signed_transitions(approval, post, "END_VOTING_PERIOD")
            .into_iter()
            .filter_map(|close| Some((signed_instant(close)?, close)))
            .filter(|(due, _)| *due <= state.now)
            .collect();
        closes.sort_by_key(|(due, close)| (*due, close.scheduled_event_id.clone()));
        for (due, close) in closes {
            let key = (
                close.scheduled_event_id.clone(),
                post,
                close.fingerprint.clone(),
            );
            if state.fired.contains_key(&key) {
                continue;
            }
            let Some(election) =
                get_election_by_id(hasura_transaction, tenant_id, election_event_id, &post_text)
                    .await?
            else {
                continue;
            };
            let status = get_election_status(election.status.clone()).unwrap_or_default();
            let pairs: Vec<(VotingStatusChannel, VotingStatus)> =
                remaining_signed_close_channels(&state, approval, post, close, due)
                    .into_iter()
                    .map(|channel| (channel, status.status_by_channel(channel)))
                    .filter(|(_, current)| {
                        scheduled_transition_applies(current, &VotingStatus::CLOSED)
                    })
                    .collect();
            if pairs.is_empty() {
                continue;
            }
            let subject = VotingSubject::new(&pairs);
            let channels: Vec<VotingStatusChannel> =
                pairs.iter().map(|(channel, _)| *channel).collect();
            closer
                .close(hasura_transaction, tenant, event, post, &channels)
                .await?;
            mark_fired(hasura_transaction, tenant, event, close, post, &channels).await?;
            let cancelled = cancel_for_scheduled_change(
                hasura_transaction,
                tenant,
                event,
                post,
                SigningAction::CloseVoting,
                &subject.channels,
            )
            .await?;
            let authorized_by = approval.authorized_by();
            let closed_at = fired_at(hasura_transaction).await?;
            let record = SealRecord {
                closed_at,
                election_id: Some(post),
                channels: subject.channels.clone(),
                from: subject.from.clone(),
                code: String::new(),
                payload_sha256: String::new(),
                signatures: vec![],
                seals: vec![],
                authorized_by: Some(authorized_by.clone()),
                unsigned: false,
            };
            let details = json!({
                "action": SigningAction::CloseVoting,
                "election_id": post,
                "scheduled_event_id": close.scheduled_event_id,
                "outcome": ScheduledOutcomeKind::Runs,
                "authorized_by": authorized_by,
                "unsigned": false,
                "fingerprint": close.fingerprint,
                "channels": subject.channels,
                "signed_time": due,
                "reason": "signed-close",
                "record": record,
                "cancelled": cancelled,
            });
            stage(
            hasura_transaction,
            &LogStep {
                kind: SigningStatementKind::SigningActionExecuted,
                user: scheduler(),
                system: SystemOutcome::Info,
                scope: log_scope(tenant, event, Some(post), None),
                description: format!(
                    "Closed voting at {} at its signed time, authorized by the signed configuration {}",
                    state.post_name(Some(post)),
                    approval.code
                ),
                details: details.clone(),
            },
        )
        .await?;
            closed.push(PostDecision {
                election_id: post_text.clone(),
                runs: true,
                explanation: None,
                details,
            });
        }
    }
    Ok(closed)
}
