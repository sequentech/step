// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The voter audits a ballot the ballot box received at review. Before the
//! voting portal shows the ballot's secret values, the ballot box marks the
//! received ballot audited and records that on the election event's board, in
//! one write. An audited ballot is never cast, and a cast one is never
//! audited.

use crate::postgres::area::get_area_by_id;
use crate::postgres::election_event::get_election_event_by_id;
use crate::services::database::get_hasura_pool;
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log::ElectoralLog;
use crate::services::electoral_log_board::{get_board_signing_key, get_event_store};
use crate::services::insert_cast_vote::{
    ballot_tracker_context, check_status, deserialize_and_check_ballot,
    deserialize_and_check_multi_ballot, CastVoteError, InsertCastVoteInput,
};
use crate::services::receive_ballot::must_be_received;
use deadpool_postgres::Client as DbClient;
use electoral_log::adapters::ballot_box_received::{AuditOutcome, ReceivedBallot, ReceivedStatus};
use electoral_log::messages::message::{BallotAudit, Message};
use electoral_log::messages::newtypes::{
    CastVoteHash, ElectionIdString, EventIdString, PseudonymHash, VoterCountryString,
    VoterIpString, VotingChannelString,
};
use electoral_log::LogEntry;
use sequent_core::ballot::{AreaPresentation, ContestEncryptionPolicy, VotingStatusChannel};
use sequent_core::ballot_receipt::normalize_ballot_id;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use uuid::Uuid;
use voting_crypto::signing::SigningKey;

/// The voter's request to audit a received ballot.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuditBallotInput {
    /// The election the ballot was received in.
    pub election_id: Uuid,
    /// The Ballot ID the ballot box signed when it received the ballot.
    pub ballot_id: String,
}

/// The received ballot the ballot box marked audited.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AuditBallotOutput {
    /// The Ballot ID of its receipt.
    pub ballot_id: String,
    /// The ballot's hash, as the voter sent it.
    pub ballot_hash: String,
}

/// Where the request to audit came from, for the record.
pub struct AuditRequest<'a> {
    /// The voting channel `check_status` accepted the request for.
    pub voting_channel: VotingStatusChannel,
    /// The voter's IP address, if known.
    pub voter_ip: &'a Option<String>,
    /// The voter's country, if known.
    pub voter_country: &'a Option<String>,
    /// The voter's username.
    pub username: &'a Option<String>,
}

fn store_error(error: anyhow::Error) -> CastVoteError {
    CastVoteError::InsertFailed(format!("{error:#}"))
}

/// Delivery ID of a received ballot's audit record, so that appending it again
/// stores nothing new.
pub fn audit_delivery_id(election_event_id: &str, received_id: &str) -> String {
    format!("ballot-audit:{election_event_id}:{received_id}")
}

/// What the voter is told about a received ballot as found before marking it:
/// `None` when it waits to be cast and is to be marked now. A ballot audited
/// before is audited again, so that a voter who lost the answer can retry.
pub fn found_received(
    received: &ReceivedBallot,
) -> Option<Result<AuditBallotOutput, CastVoteError>> {
    match received.status {
        ReceivedStatus::Received => None,
        ReceivedStatus::Audited => Some(Ok(audited(received))),
        ReceivedStatus::Cast => Some(Err(CastVoteError::BallotAlreadyCast)),
    }
}

/// The answer for the outcome of marking a received ballot audited.
pub fn audit_answer(
    outcome: AuditOutcome,
    received: &ReceivedBallot,
) -> Result<AuditBallotOutput, CastVoteError> {
    match outcome {
        AuditOutcome::Audited | AuditOutcome::AlreadyAudited => Ok(audited(received)),
        AuditOutcome::Cast => Err(CastVoteError::BallotAlreadyCast),
        AuditOutcome::NotReceived => Err(CastVoteError::BallotNotReceived),
    }
}

fn audited(received: &ReceivedBallot) -> AuditBallotOutput {
    AuditBallotOutput {
        ballot_id: received.ballot_id.clone(),
        ballot_hash: received.ballot_hash.clone(),
    }
}

/// The record of auditing a received ballot, signed with the event's
/// protocol-manager key as a cast-vote record is, for the voter.
pub fn audit_record(
    board: &str,
    received: &ReceivedBallot,
    pseudonym_hash: PseudonymHash,
    ballot_hash: CastVoteHash,
    request: &AuditRequest<'_>,
    signing_key: &SigningKey,
) -> anyhow::Result<LogEntry> {
    let log = ElectoralLog::for_voter_with_signing_key(board, &received.voter_id, signing_key);
    let message = Message::ballot_audited_message(
        BallotAudit {
            event: EventIdString(received.election_event_id.clone()),
            election: ElectionIdString(Some(received.election_id.clone())),
            pseudonym_h: pseudonym_hash,
            vote_h: ballot_hash,
            received_ballot_id: received.ballot_id.clone(),
            ip: VoterIpString(format!("ip: {}", request.voter_ip.as_deref().unwrap_or(""))),
            country: VoterCountryString(format!(
                "country: {}",
                request.voter_country.as_deref().unwrap_or("")
            )),
            voting_channel: VotingChannelString(request.voting_channel.to_string()),
            voter_id: Some(received.voter_id.clone()),
            voter_username: request.username.clone(),
            area_id: received.area_id.clone(),
        },
        &log.sd,
    )?;
    Ok(LogEntry {
        delivery_id: audit_delivery_id(&received.election_event_id, &received.id),
        message: (&message).try_into()?,
    })
}

/// Marks the voter's received ballot audited, with the same checks as
/// receiving it: the event receives ballots at review on this channel, the
/// election accepts votes now and the voter's session is recent enough. The
/// Ballot ID of another voter's ballot is not found, as an unknown one.
#[expect(
    clippy::too_many_arguments,
    reason = "Keep the voter's request inputs as try_insert_cast_vote and try_cast_ballot take them, so that the harvest adapter calls them alike."
)]
#[instrument(skip(input), err)]
pub async fn try_audit_ballot(
    input: AuditBallotInput,
    tenant_id: &str,
    voter_id: &str,
    area_id: &str,
    voting_channel: VotingStatusChannel,
    auth_time: &Option<i64>,
    voter_ip: &Option<String>,
    voter_country: &Option<String>,
    username: &Option<String>,
) -> Result<AuditBallotOutput, CastVoteError> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| CastVoteError::GetDbClientFailed(e.to_string()))?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| CastVoteError::GetTransactionFailed(e.to_string()))?;

    let area = get_area_by_id(&hasura_transaction, tenant_id, area_id)
        .await
        .map_err(|e| CastVoteError::GetAreaIdFailed(e.to_string()))?
        .ok_or(CastVoteError::AreaNotFound)?;
    let election_event =
        get_election_event_by_id(&hasura_transaction, tenant_id, &area.election_event_id)
            .await
            .map_err(|e| CastVoteError::ElectionEventNotFound(e.to_string()))?;
    let presentation = election_event
        .get_presentation()
        .map_err(|e| CastVoteError::ElectionEventNotFound(e.to_string()))?
        .unwrap_or_default();
    if !must_be_received(&presentation.receipts_policy(), voting_channel) {
        return Err(CastVoteError::CheckStatusFailed(
            "The ballot box does not receive ballots at review in this election event".to_string(),
        ));
    }

    let election_id = input.election_id.to_string();
    let area_uuid = parse_uuid_v4(area_id)
        .map_err(|e| CastVoteError::UuidParseFailed(e.to_string(), "area_id".to_string()))?;
    let area_presentation: AreaPresentation = match area.presentation {
        Some(presentation) => deserialize_value(presentation)
            .map_err(|e| CastVoteError::DeserializeAreaPresentationFailed(e.to_string()))?,
        None => AreaPresentation::default(),
    };
    let effective_voting_channel = check_status(
        tenant_id,
        &election_event.id,
        &election_id,
        &area_uuid,
        &hasura_transaction,
        &election_event,
        auth_time,
        voting_channel,
        area_presentation.is_early_voting(),
    )
    .await?;

    let store = get_event_store(&election_event.id)
        .await
        .map_err(store_error)?;
    let ballot_id =
        normalize_ballot_id(&input.ballot_id).ok_or(CastVoteError::BallotNotReceived)?;
    let received = store
        .received_ballot_by_id(&election_event.id, &election_id, voter_id, &ballot_id)
        .await
        .map_err(store_error)?
        .ok_or(CastVoteError::BallotNotReceived)?;
    if let Some(answer) = found_received(&received) {
        return answer;
    }

    // The record names the ballot by the hashes its cast-vote record would carry.
    let (ballot_tracker_policy, published_ballot_styles) = ballot_tracker_context(
        &hasura_transaction,
        Some(&presentation),
        tenant_id,
        &election_event.id,
        area_id,
        &election_id,
    )
    .await?;
    let stored_ballot = InsertCastVoteInput {
        ballot_id: received.ballot_hash.clone(),
        election_id: input.election_id,
        content: received.content.clone(),
    };
    let (pseudonym_hash, ballot_hash, _) = if presentation.contest_encryption_policy
        == Some(ContestEncryptionPolicy::MULTIPLE_CONTESTS)
    {
        deserialize_and_check_multi_ballot(
            &stored_ballot,
            voter_id,
            &ballot_tracker_policy,
            &published_ballot_styles,
        )?
    } else {
        deserialize_and_check_ballot(
            &stored_ballot,
            voter_id,
            &ballot_tracker_policy,
            &published_ballot_styles,
        )?
    };

    let board = get_election_event_board(election_event.bulletin_board_reference.clone())
        .ok_or_else(|| {
            CastVoteError::ElectoralLogNotFound("Election event has no audit board".to_string())
        })?;
    let signing_key = get_board_signing_key(
        &hasura_transaction,
        tenant_id,
        Some(&election_event.id),
        &board,
    )
    .await
    .map_err(|e| CastVoteError::BallotSignFailed(e.to_string()))?;
    let entry = audit_record(
        &board,
        &received,
        pseudonym_hash,
        ballot_hash,
        &AuditRequest {
            voting_channel: effective_voting_channel,
            voter_ip,
            voter_country,
            username,
        },
        &signing_key,
    )
    .map_err(|e| CastVoteError::BallotSignFailed(e.to_string()))?;
    let outcome = store
        .audit_received_ballot(&received, &board, &entry)
        .await
        .map_err(store_error)?;

    // Keep the shared lifecycle lock `check_status` took until the event store
    // has committed the audit.
    hasura_transaction
        .commit()
        .await
        .map_err(|e| CastVoteError::CommitFailed(e.to_string()))?;
    drop(hasura_db_client);
    info!(?outcome, "received ballot audit");
    audit_answer(outcome, &received)
}

#[cfg(test)]
#[path = "audit_ballot_tests.rs"]
mod tests;
