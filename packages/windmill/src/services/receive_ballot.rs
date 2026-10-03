// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The ballot box receives a ballot at review: it runs the cast checks, stores
//! the ballot and signs that it has it. It answers only after the commit, so
//! the Ballot ID a voter sees is proof of storage.

use crate::postgres::area::get_area_by_id;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::received_ballot::{
    format_received_at, get_published_ballot_eml, get_received_ballot, insert_received_ballot,
    ReceivedBallotScope,
};
use crate::services::ballot_box_key::{get_ballot_box_signing_key, published_key};
use crate::services::database::get_hasura_pool;
use crate::services::insert_cast_vote::{
    check_status, deserialize_and_check_ballot, deserialize_and_check_multi_ballot, CastVoteError,
    InsertCastVoteInput,
};
use chrono::{DateTime, Duration, TimeZone, Utc};
use dashmap::DashMap;
use deadpool_postgres::{Client as DbClient, Transaction};
use once_cell::sync::Lazy;
use sequent_core::ballot::{
    AreaPresentation, BallotStyle, ContestEncryptionPolicy, ReceiptsPolicy, VotingStatusChannel,
};
use sequent_core::ballot_receipt::{sign_received_ballot, ReceivedBallot, ReceivedStatement};
use sequent_core::encrypt::hash_ballot_style;
use sequent_core::serialization::deserialize_with_path::{deserialize_str, deserialize_value};
use sequent_core::services::uuid_validation::parse_uuid_v4;
use serde::Deserialize;
use strand::signature::StrandSignatureSk;
use tracing::instrument;
use uuid::Uuid;

pub type ReceiveBallotInput = InsertCastVoteInput;
pub type ReceiveBallotOutput = ReceivedBallot;

/// A taken Ballot ID is about one in a million at 1.7 million ballots; each
/// attempt signs one millisecond later.
const MAX_BALLOT_ID_ATTEMPTS: i64 = 8;

/// The hash of a ballot style never changes once it is published.
static BALLOT_STYLE_HASHES: Lazy<DashMap<Uuid, String>> = Lazy::new(DashMap::new);

/// What a ballot says about the ballot style it was made with and about its
/// voter's key. Single and multi-contest ballots share these fields.
#[derive(Deserialize)]
struct BallotEnvelope {
    config: String,
    ballot_style_hash: String,
    voter_signing_pk: Option<String>,
    voter_ballot_signature: Option<String>,
}

/// Whether a ballot must have been received by the ballot box before it is
/// cast. Telephone voting has no voter device to check a receipt, so it keeps
/// casting directly.
pub fn must_be_received(policy: &ReceiptsPolicy, voting_channel: VotingStatusChannel) -> bool {
    match policy {
        ReceiptsPolicy::DISABLED => false,
        ReceiptsPolicy::SIGNED_BY_BALLOT_BOX => voting_channel != VotingStatusChannel::TELEPHONE,
    }
}

fn voter_signature(envelope: &BallotEnvelope) -> Result<(String, String), CastVoteError> {
    match (
        envelope.voter_signing_pk.clone(),
        envelope.voter_ballot_signature.clone(),
    ) {
        (Some(public_key), Some(signature)) => Ok((public_key, signature)),
        _ => Err(CastVoteError::BallotVoterSignatureRequired),
    }
}

fn check_ballot_style_hash(
    envelope: &BallotEnvelope,
    published_hash: &str,
) -> Result<(), CastVoteError> {
    if envelope.ballot_style_hash == published_hash {
        Ok(())
    } else {
        Err(CastVoteError::BallotStyleMismatch(format!(
            "Ballot style {} has hash {published_hash}, the ballot says {}",
            envelope.config, envelope.ballot_style_hash
        )))
    }
}

fn hash_published_ballot_eml(ballot_eml: &str) -> Result<String, CastVoteError> {
    let ballot_style: BallotStyle = deserialize_str(ballot_eml)
        .map_err(|err| CastVoteError::CheckStatusInternalFailed(err.to_string()))?;
    hash_ballot_style(&ballot_style)
        .map_err(|err| CastVoteError::CheckStatusInternalFailed(err.to_string()))
}

/// The ballot must name the ballot style published for the voter's area and
/// election, and carry that style's hash.
#[instrument(skip_all, err)]
async fn check_ballot_style(
    hasura_transaction: &Transaction<'_>,
    scope: &ReceivedBallotScope<'_>,
    area_id: &Uuid,
    envelope: &BallotEnvelope,
) -> Result<(), CastVoteError> {
    let unknown_style = || {
        CastVoteError::BallotStyleMismatch(format!(
            "Ballot style {} is not published for the voter",
            envelope.config
        ))
    };
    let ballot_style_id = parse_uuid_v4(&envelope.config).map_err(|_| unknown_style())?;
    let cached_hash = BALLOT_STYLE_HASHES
        .get(&ballot_style_id)
        .map(|hash| hash.value().clone());

    let published = get_published_ballot_eml(
        hasura_transaction,
        scope.tenant_id,
        scope.election_event_id,
        scope.election_id,
        area_id,
        &ballot_style_id,
        cached_hash.is_none(),
    )
    .await
    .map_err(|err| CastVoteError::CheckStatusInternalFailed(err.to_string()))?
    .ok_or_else(unknown_style)?;

    let published_hash = match (cached_hash, published) {
        (Some(hash), _) => hash,
        (None, Some(ballot_eml)) => {
            let hash = hash_published_ballot_eml(&ballot_eml)?;
            BALLOT_STYLE_HASHES.insert(ballot_style_id, hash.clone());
            hash
        }
        (None, None) => return Err(unknown_style()),
    };
    check_ballot_style_hash(envelope, &published_hash)
}

fn parse_uuid_field(value: &str, field: &str) -> Result<Uuid, CastVoteError> {
    parse_uuid_v4(value)
        .map_err(|e| CastVoteError::UuidParseFailed(e.to_string(), field.to_string()))
}

pub(crate) fn truncate_to_milliseconds(time: DateTime<Utc>) -> Result<DateTime<Utc>, CastVoteError> {
    Utc.timestamp_millis_opt(time.timestamp_millis())
        .single()
        .ok_or_else(|| CastVoteError::BallotSignFailed("Invalid received time".to_string()))
}

fn sign_received(
    signing_key: &StrandSignatureSk,
    scope: &ReceivedBallotScope<'_>,
    voter_signing_pk: &str,
    voter_ballot_signature: &str,
    received_at: &DateTime<Utc>,
) -> Result<ReceivedBallot, CastVoteError> {
    let key_id = published_key(signing_key)
        .map_err(|err| CastVoteError::BallotSignFailed(err.to_string()))?
        .key_id;
    sign_received_ballot(
        signing_key,
        ReceivedStatement {
            tenant_id: scope.tenant_id.to_string(),
            election_event_id: scope.election_event_id.to_string(),
            election_id: scope.election_id.to_string(),
            ballot_hash: scope.ballot_hash.to_string(),
            voter_signing_pk: voter_signing_pk.to_string(),
            voter_ballot_signature: voter_ballot_signature.to_string(),
            received_at: format_received_at(received_at),
            key_id,
        },
    )
    .map_err(|err| CastVoteError::BallotSignFailed(err.to_string()))
}

#[instrument(skip(input), err)]
pub async fn try_receive_ballot(
    input: ReceiveBallotInput,
    tenant_id: &str,
    voter_id: &str,
    area_id: &str,
    voting_channel: VotingStatusChannel,
    auth_time: &Option<i64>,
) -> Result<ReceiveBallotOutput, CastVoteError> {
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

    let is_multi_contest =
        presentation.contest_encryption_policy == Some(ContestEncryptionPolicy::MULTIPLE_CONTESTS);
    if is_multi_contest {
        deserialize_and_check_multi_ballot(&input, voter_id)?;
    } else {
        deserialize_and_check_ballot(&input, voter_id)?;
    }
    let envelope: BallotEnvelope = deserialize_str(&input.content)
        .map_err(|e| CastVoteError::DeserializeBallotFailed(e.to_string()))?;
    let (voter_signing_pk, voter_ballot_signature) = voter_signature(&envelope)?;

    let election_id = input.election_id.to_string();
    let tenant_uuid = parse_uuid_field(tenant_id, "tenant_id")?;
    let election_event_uuid = parse_uuid_field(&election_event.id, "election_event_id")?;
    let area_uuid = parse_uuid_field(area_id, "area_id")?;
    let scope = ReceivedBallotScope {
        tenant_id: &tenant_uuid,
        election_event_id: &election_event_uuid,
        election_id: &input.election_id,
        voter_id,
        ballot_hash: &input.ballot_id,
    };

    check_ballot_style(&hasura_transaction, &scope, &area_uuid, &envelope).await?;

    let area_presentation: AreaPresentation = match area.presentation {
        Some(presentation) => deserialize_value(presentation)
            .map_err(|e| CastVoteError::DeserializeAreaPresentationFailed(e.to_string()))?,
        None => AreaPresentation::default(),
    };
    check_status(
        tenant_id,
        &election_event.id,
        &election_id,
        &hasura_transaction,
        &election_event,
        auth_time,
        voting_channel,
        area_presentation.is_early_voting(),
    )
    .await?;

    let database_error = |error: anyhow::Error| CastVoteError::InsertFailed(format!("{error:#}"));
    if let Some(stored) = get_received_ballot(&hasura_transaction, &scope)
        .await
        .map_err(database_error)?
    {
        return Ok(stored.received);
    }

    let signing_key = get_ballot_box_signing_key(&hasura_transaction, &election_event)
        .await
        .map_err(|e| CastVoteError::BallotSignFailed(e.to_string()))?
        .ok_or_else(|| {
            CastVoteError::BallotSignFailed(
                "The election event has no ballot box key; publish its ballots".to_string(),
            )
        })?;
    let first_received_at = truncate_to_milliseconds(Utc::now())?;

    for attempt in 0..MAX_BALLOT_ID_ATTEMPTS {
        let received_at = first_received_at + Duration::milliseconds(attempt);
        let received = sign_received(
            &signing_key,
            &scope,
            &voter_signing_pk,
            &voter_ballot_signature,
            &received_at,
        )?;
        let inserted = insert_received_ballot(
            &hasura_transaction,
            &scope,
            &area_uuid,
            &input.content,
            &received_at,
            &received,
        )
        .await
        .map_err(database_error)?;

        if inserted.is_some() {
            hasura_transaction
                .commit()
                .await
                .map_err(|e| CastVoteError::CommitFailed(e.to_string()))?;
            return Ok(received);
        }
        // Either this voter's same ballot arrived on another request, or the
        // Ballot ID is taken in the election event.
        if let Some(stored) = get_received_ballot(&hasura_transaction, &scope)
            .await
            .map_err(database_error)?
        {
            return Ok(stored.received);
        }
    }

    Err(CastVoteError::InsertFailed(
        "No free Ballot ID for the received ballot".to_string(),
    ))
}

#[cfg(test)]
#[path = "receive_ballot_tests.rs"]
mod tests;
