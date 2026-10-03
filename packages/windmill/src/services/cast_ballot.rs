// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The voter casts a ballot the ballot box received at review by signing its
//! Ballot ID with the key that signed the ballot. The ballot box stores the
//! cast and signs a receipt for it, which it gives out only after the commit.

use crate::postgres::cast_vote::get_cast_vote_id_of_received_ballot;
use crate::postgres::received_ballot::{
    format_cast_at, get_received_ballot_to_cast, mark_received_ballot_cast, ReceivedBallotStatus,
    ReceivedBallotToCast, StoredCast, StoredReceivedBallot,
};
use crate::services::ballot_box_key::{get_ballot_box_signing_key, published_key};
use crate::services::insert_cast_vote::{
    try_cast, CastOutcome, CastRequest, CastVoteError, InsertCastVoteInput, InsertCastVoteOutput,
    InsertCastVoteResult,
};
use crate::services::receive_ballot::truncate_to_milliseconds;
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use sequent_core::ballot::VotingStatusChannel;
use sequent_core::ballot_receipt::{
    normalize_ballot_id, sign_cast_receipt, verify_cast_signature, CastReceipt,
    CastReceiptStatement,
};
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::hasura::core::ElectionEvent;
use serde::{Deserialize, Serialize};
use strand::signature::{StrandSignature, StrandSignatureSk};
use tracing::instrument;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CastBallotInput {
    pub election_id: Uuid,
    /// The Ballot ID the ballot box signed when it received the ballot.
    pub ballot_id: String,
    /// The voter's signature over "cast this Ballot ID".
    pub cast_signature: String,
}

/// The ballot box's receipt for a cast, with the cast vote it was stored as.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CastBallotOutput {
    #[serde(flatten)]
    pub receipt: CastReceipt,
    pub cast_vote_id: String,
}

pub enum CastBallotResult {
    Cast(CastBallotOutput),
    /// Cast, and still to be confirmed by the Datafix pipeline.
    PendingDatafix(CastBallotOutput, InsertCastVoteOutput),
    SkipRetryFailure(CastVoteError),
}

pub(crate) enum PreparedCast {
    ToCast(InsertCastVoteInput, ReceivedCast),
    /// The ballot box already stored this cast: the receipt it signed then.
    AlreadyCast(CastBallotOutput),
}

fn stored_receipt(received_ballot: &ReceivedBallotToCast, cast: &StoredCast) -> CastReceipt {
    let received = &received_ballot.stored.received;
    CastReceipt {
        statement: CastReceiptStatement {
            election_event_id: received.statement.election_event_id.clone(),
            election_id: received.statement.election_id.clone(),
            ballot_id: received.ballot_id.clone(),
            received_at: received.statement.received_at.clone(),
            cast_at: format_cast_at(&cast.cast_at),
            key_id: received.statement.key_id.clone(),
            cast_signature: cast.cast_signature.clone(),
        },
        cast_receipt_signature: cast.cast_receipt_signature.clone(),
    }
}

/// The signature as the ballot box writes it, once it verifies against the
/// key that signed the received ballot.
fn check_cast_signature(
    received_ballot: &ReceivedBallotToCast,
    cast_signature: &str,
) -> Result<String, CastVoteError> {
    let received = &received_ballot.stored.received;
    let refused = |error: &dyn std::fmt::Display| {
        CastVoteError::BallotCastSignatureFailed(format!("Cast signature check failed: {error}"))
    };
    verify_cast_signature(
        &received.statement.voter_signing_pk,
        &received.statement.election_id,
        &received.ballot_id,
        cast_signature,
    )
    .map_err(|error| refused(&error))?;
    StrandSignature::from_b64_string(cast_signature)
        .and_then(|signature| signature.to_b64_string())
        .map_err(|error| refused(&error))
}

/// What to do with a received ballot whose Cast signature verifies. A ballot
/// is cast once: the same signature again is a retry of that cast.
fn cast_or_stored_receipt(
    received_ballot: &ReceivedBallotToCast,
    cast_signature: &str,
) -> Result<Option<CastReceipt>, CastVoteError> {
    match (received_ballot.stored.status, &received_ballot.cast) {
        (ReceivedBallotStatus::Received, _) => Ok(None),
        (ReceivedBallotStatus::Audited, _) => Err(CastVoteError::BallotAudited),
        (ReceivedBallotStatus::Cast, Some(cast)) if cast.cast_signature == cast_signature => {
            Ok(Some(stored_receipt(received_ballot, cast)))
        }
        (ReceivedBallotStatus::Cast, _) => Err(CastVoteError::BallotAlreadyCast),
    }
}

/// A received ballot about to be cast: the voter's Cast signature over its
/// Ballot ID, and the key the ballot box signs the receipt with.
pub struct ReceivedCast {
    pub received_ballot: StoredReceivedBallot,
    pub cast_signature: String,
    pub ballot_box_key: StrandSignatureSk,
    pub key_id: String,
}

pub enum FoundCast {
    /// The ballot waits to be cast; the Cast signature verifies.
    ToCast(ReceivedBallotToCast, String),
    /// The ballot box already stored this cast: the receipt it signed then.
    AlreadyCast(CastBallotOutput),
}

/// Finds the ballot the voter asks to cast and checks the request against it.
/// Another voter's Ballot ID is not found, as an unknown one.
#[instrument(skip_all, err)]
pub async fn find_received_cast(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    voter_id: &str,
    input: &CastBallotInput,
) -> Result<FoundCast, CastVoteError> {
    let ballot_id =
        normalize_ballot_id(&input.ballot_id).ok_or(CastVoteError::BallotNotReceived)?;
    let received_ballot = get_received_ballot_to_cast(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &input.election_id,
        voter_id,
        &ballot_id,
    )
    .await
    .map_err(|e| CastVoteError::InsertFailed(format!("{e:#}")))?
    .ok_or(CastVoteError::BallotNotReceived)?;

    let cast_signature = check_cast_signature(&received_ballot, &input.cast_signature)?;
    let Some(receipt) = cast_or_stored_receipt(&received_ballot, &cast_signature)? else {
        return Ok(FoundCast::ToCast(received_ballot, cast_signature));
    };
    let cast_vote_id = get_cast_vote_id_of_received_ballot(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &input.election_id,
        voter_id,
        &received_ballot.stored.id,
    )
    .await
    .map_err(|e| CastVoteError::InsertFailed(format!("{e:#}")))?
    .ok_or(CastVoteError::BallotAlreadyCast)?;
    Ok(FoundCast::AlreadyCast(CastBallotOutput {
        receipt,
        cast_vote_id: cast_vote_id.to_string(),
    }))
}

/// What casting a received ballot takes: the ballot as it was received, and
/// the key that signs the receipt.
pub fn received_cast(
    received_ballot: ReceivedBallotToCast,
    cast_signature: String,
    ballot_box_key: StrandSignatureSk,
) -> Result<(InsertCastVoteInput, ReceivedCast), CastVoteError> {
    let statement = &received_ballot.stored.received.statement;
    let key_id = published_key(&ballot_box_key)
        .map_err(|e| CastVoteError::BallotSignFailed(e.to_string()))?
        .key_id;
    // A stored receipt is read back with the key of its Received statement.
    if key_id != statement.key_id {
        return Err(CastVoteError::BallotSignFailed(
            "The ballot box key is not the one that received the ballot".to_string(),
        ));
    }
    let election_id = parse_uuid_v4(&statement.election_id)
        .map_err(|e| CastVoteError::UuidParseFailed(e.to_string(), "election_id".to_string()))?;

    Ok((
        InsertCastVoteInput {
            ballot_id: statement.ballot_hash.clone(),
            election_id,
            content: received_ballot.content,
        },
        ReceivedCast {
            received_ballot: received_ballot.stored,
            cast_signature,
            ballot_box_key,
            key_id,
        },
    ))
}

/// Marks the received ballot cast with the receipt the ballot box signs for
/// it. The receipt is good only if the caller's transaction commits.
#[instrument(skip_all, err)]
pub async fn store_cast_receipt(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    received_cast: &ReceivedCast,
) -> Result<(DateTime<Utc>, CastReceipt), CastVoteError> {
    let received = &received_cast.received_ballot.received;
    let cast_at = truncate_to_milliseconds(Utc::now())?;
    let receipt = sign_cast_receipt(
        &received_cast.ballot_box_key,
        CastReceiptStatement {
            election_event_id: received.statement.election_event_id.clone(),
            election_id: received.statement.election_id.clone(),
            ballot_id: received.ballot_id.clone(),
            received_at: received.statement.received_at.clone(),
            cast_at: format_cast_at(&cast_at),
            key_id: received_cast.key_id.clone(),
            cast_signature: received_cast.cast_signature.clone(),
        },
    )
    .map_err(|err| CastVoteError::BallotSignFailed(err.to_string()))?;

    let cast_now = mark_received_ballot_cast(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &received_cast.received_ballot.id,
        &cast_at,
        &received_cast.cast_signature,
        &receipt.cast_receipt_signature,
    )
    .await
    .map_err(|e| CastVoteError::InsertFailed(format!("{e:#}")))?;
    if !cast_now {
        return Err(CastVoteError::BallotAlreadyCast);
    }
    Ok((cast_at, receipt))
}

#[instrument(skip_all, err)]
pub(crate) async fn prepare_received_cast(
    hasura_transaction: &Transaction<'_>,
    election_event: &ElectionEvent,
    voter_id: &str,
    input: &CastBallotInput,
) -> Result<PreparedCast, CastVoteError> {
    let parse_uuid = |value: &str, field: &str| {
        parse_uuid_v4(value)
            .map_err(|e| CastVoteError::UuidParseFailed(e.to_string(), field.to_string()))
    };
    let found = find_received_cast(
        hasura_transaction,
        &parse_uuid(&election_event.tenant_id, "tenant_id")?,
        &parse_uuid(&election_event.id, "election_event_id")?,
        voter_id,
        input,
    )
    .await?;
    let (received_ballot, cast_signature) = match found {
        FoundCast::AlreadyCast(cast) => return Ok(PreparedCast::AlreadyCast(cast)),
        FoundCast::ToCast(received_ballot, cast_signature) => (received_ballot, cast_signature),
    };

    let ballot_box_key = get_ballot_box_signing_key(hasura_transaction, election_event)
        .await
        .map_err(|e| CastVoteError::BallotSignFailed(e.to_string()))?
        .ok_or_else(|| {
            CastVoteError::BallotSignFailed("The election event has no ballot box key".to_string())
        })?;
    let (input, received_cast) = received_cast(received_ballot, cast_signature, ballot_box_key)?;
    Ok(PreparedCast::ToCast(input, received_cast))
}

#[instrument(skip(input), err)]
pub async fn try_cast_ballot(
    input: CastBallotInput,
    tenant_id: &str,
    voter_id: &str,
    area_id: &str,
    voting_channel: VotingStatusChannel,
    auth_time: &Option<i64>,
    voter_ip: &Option<String>,
    voter_country: &Option<String>,
    username: &Option<String>,
) -> Result<CastBallotResult, CastVoteError> {
    let outcome = try_cast(
        CastRequest::Received(input),
        tenant_id,
        voter_id,
        area_id,
        voting_channel,
        auth_time,
        voter_ip,
        voter_country,
        username,
    )
    .await?;
    let no_receipt =
        || CastVoteError::BallotSignFailed("The ballot was cast without a receipt".to_string());

    let output = |receipt: Option<CastReceipt>, cast_vote: &InsertCastVoteOutput| {
        receipt
            .map(|receipt| CastBallotOutput {
                receipt,
                cast_vote_id: cast_vote.id.clone(),
            })
            .ok_or_else(no_receipt)
    };

    match outcome {
        CastOutcome::AlreadyCast(cast) => Ok(CastBallotResult::Cast(cast)),
        CastOutcome::Inserted { result, receipt } => match result {
            InsertCastVoteResult::SkipRetryFailure(error) => {
                Ok(CastBallotResult::SkipRetryFailure(error))
            }
            InsertCastVoteResult::Success(cast_vote) => {
                Ok(CastBallotResult::Cast(output(receipt, &cast_vote)?))
            }
            InsertCastVoteResult::PendingDatafix(cast_vote) => Ok(
                CastBallotResult::PendingDatafix(output(receipt, &cast_vote)?, cast_vote),
            ),
        },
    }
}

#[cfg(test)]
#[path = "cast_ballot_tests.rs"]
mod tests;
