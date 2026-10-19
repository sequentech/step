// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The ballot box receives a ballot at review: it runs the cast checks, stores
//! the ballot in the event's ballot box and signs that it has it. It answers
//! only once the ballot is stored, so the Ballot ID a voter sees is proof of
//! storage.

use crate::postgres::area::get_area_by_id;
use crate::postgres::ballot_style::get_published_ballot_eml;
use crate::postgres::election_event::get_election_event_by_id;
use crate::services::ballot_box_key::{get_ballot_box_signing_key, published_key};
use crate::services::database::get_hasura_pool;
use crate::services::electoral_log_board::get_event_store;
use crate::services::insert_cast_vote::{
    ballot_tracker_context, check_status, deserialize_and_check_ballot,
    deserialize_and_check_multi_ballot, CastVoteError, InsertCastVoteInput,
};
use chrono::{DateTime, TimeZone, Utc};
use dashmap::DashMap;
use deadpool_postgres::{Client as DbClient, Transaction};
use electoral_log::adapters::ballot_box_received::{
    ReceiveBallot, ReceiveOutcome, ReceivedBallot as StoredReceivedBallot, ReceivedSignature,
};
use once_cell::sync::Lazy;
use sequent_core::ballot::{
    AreaPresentation, BallotStyle, ContestEncryptionPolicy, ReceiptsPolicy, VotingStatusChannel,
};
use sequent_core::ballot_receipt::{sign_received_ballot, ReceivedBallot, ReceivedStatement};
use sequent_core::encrypt::hash_ballot_style;
use sequent_core::serialization::deserialize_with_path::{deserialize_str, deserialize_value};
use sequent_core::services::uuid_validation::parse_uuid_v4;
use serde::Deserialize;
use tracing::instrument;
use uuid::Uuid;
use voting_crypto::signing::SigningKey;

pub type ReceiveBallotInput = InsertCastVoteInput;
pub type ReceiveBallotOutput = ReceivedBallot;

/// The format of the times the ballot box signs: `ReceivedStatement::received_at`
/// and `CastReceiptStatement::cast_at`.
const SIGNED_TIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.3fZ";

pub fn format_signed_time(time: &DateTime<Utc>) -> String {
    time.format(SIGNED_TIME_FORMAT).to_string()
}

/// The election a ballot is received in, and the ballot's hash as the voter
/// sent it.
pub struct ReceivedBallotScope<'a> {
    pub tenant_id: &'a Uuid,
    pub election_event_id: &'a Uuid,
    pub election_id: &'a Uuid,
    pub ballot_hash: &'a str,
}

/// The hash of a ballot style never changes once it is published.
static BALLOT_STYLE_HASHES: Lazy<DashMap<Uuid, (String, BallotStyle)>> = Lazy::new(DashMap::new);

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
#[instrument(skip_all)]
async fn check_ballot_style(
    hasura_transaction: &Transaction<'_>,
    scope: &ReceivedBallotScope<'_>,
    area_id: &Uuid,
    envelope: &BallotEnvelope,
) -> Result<BallotStyle, CastVoteError> {
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

    let (published_hash, style) = match (cached_hash, published) {
        (Some(cached), _) => cached,
        (None, Some(ballot_eml)) => {
            let hash = hash_published_ballot_eml(&ballot_eml)?;
            let style: BallotStyle = deserialize_str(&ballot_eml)
                .map_err(|err| CastVoteError::CheckStatusInternalFailed(err.to_string()))?;
            BALLOT_STYLE_HASHES.insert(ballot_style_id, (hash.clone(), style.clone()));
            (hash, style)
        }
        (None, None) => return Err(unknown_style()),
    };
    check_ballot_style_hash(envelope, &published_hash)?;
    Ok(style)
}

/// Shared by receive and cast, including casting without signed receipts.
#[instrument(skip_all)]
pub(crate) async fn check_published_ballot(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    area_id: &str,
    input: &InsertCastVoteInput,
    policy: ContestEncryptionPolicy,
) -> Result<(), CastVoteError> {
    let envelope: BallotEnvelope = deserialize_str(&input.content)
        .map_err(|error| CastVoteError::DeserializeBallotFailed(error.to_string()))?;
    let tenant = parse_uuid_field(tenant_id, "tenant_id")?;
    let event = parse_uuid_field(election_event_id, "election_event_id")?;
    let area = parse_uuid_field(area_id, "area_id")?;
    let scope = ReceivedBallotScope {
        tenant_id: &tenant,
        election_event_id: &event,
        election_id: &input.election_id,
        ballot_hash: &input.ballot_id,
    };
    let style = check_ballot_style(transaction, &scope, &area, &envelope).await?;
    let entries = match policy {
        ContestEncryptionPolicy::SINGLE_CONTEST => {
            let ballot: sequent_core::ballot::SignedHashableBallot =
                deserialize_str(&input.content)
                    .map_err(|error| CastVoteError::DeserializeBallotFailed(error.to_string()))?;
            ballot
                .deserialize_contests()
                .map_err(|error| CastVoteError::DeserializeContestsFailed(error.to_string()))?
        }
        ContestEncryptionPolicy::MULTIPLE_CONTESTS => {
            let ballot: sequent_core::multi_ballot::SignedHashableMultiBallot =
                deserialize_str(&input.content)
                    .map_err(|error| CastVoteError::DeserializeBallotFailed(error.to_string()))?;
            vec![ballot
                .deserialize_contests()
                .map_err(|error| CastVoteError::DeserializeContestsFailed(error.to_string()))?]
        }
    };
    sequent_core::encrypt::verify_ballot_ciphertexts(&entries, &style, policy)
        .map_err(|error| CastVoteError::PokValidationFailed(error.to_string()))
}

fn parse_uuid_field(value: &str, field: &str) -> Result<Uuid, CastVoteError> {
    parse_uuid_v4(value)
        .map_err(|e| CastVoteError::UuidParseFailed(e.to_string(), field.to_string()))
}

pub(crate) fn truncate_to_milliseconds(
    time: DateTime<Utc>,
) -> Result<DateTime<Utc>, CastVoteError> {
    Utc.timestamp_millis_opt(time.timestamp_millis())
        .single()
        .ok_or_else(|| CastVoteError::BallotSignFailed("Invalid received time".to_string()))
}

fn sign_received(
    signing_key: &SigningKey,
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
            received_at: format_signed_time(received_at),
            key_id,
        },
    )
    .map_err(|err| CastVoteError::BallotSignFailed(err.to_string()))
}

/// The receipt the ballot box signed for a stored received ballot.
pub(crate) fn stored_receipt(tenant_id: &Uuid, stored: StoredReceivedBallot) -> ReceivedBallot {
    ReceivedBallot {
        statement: ReceivedStatement {
            tenant_id: tenant_id.to_string(),
            election_event_id: stored.election_event_id,
            election_id: stored.election_id,
            ballot_hash: stored.ballot_hash,
            voter_signing_pk: stored.voter_signing_pk,
            voter_ballot_signature: stored.voter_ballot_signature,
            received_at: format_signed_time(&stored.received_at),
            key_id: stored.key_id,
        },
        received_signature: stored.received_signature,
        ballot_id: stored.ballot_id,
    }
}

/// Unknown envelope fields are outside the voter's verified ballot signature.
/// Private evidence therefore stores only the typed signed envelope, preventing
/// client-added identity/network fields from being copied into public log bytes.
fn storage_ballot_content(
    content: &str,
    mode: sequent_core::types::vote_privacy::BallotIdentity,
    multi: bool,
) -> Result<String, CastVoteError> {
    if mode == sequent_core::types::vote_privacy::BallotIdentity::VoterId {
        return Ok(content.to_owned());
    }
    let sanitized = if multi {
        serde_json::from_str::<sequent_core::multi_ballot::SignedHashableMultiBallot>(content)
            .and_then(|ballot| serde_json::to_string(&ballot))
    } else {
        serde_json::from_str::<sequent_core::ballot::SignedHashableBallot>(content)
            .and_then(|ballot| serde_json::to_string(&ballot))
    };
    sanitized.map_err(|_| {
        CastVoteError::DeserializeBallotFailed("Invalid signed ballot envelope".into())
    })
}

#[instrument(skip_all)]
pub async fn try_receive_ballot(
    input: ReceiveBallotInput,
    tenant_id: &str,
    voter_id: &str,
    area_id: &str,
    voting_channel: VotingStatusChannel,
    auth_time: &Option<i64>,
) -> Result<ReceiveBallotOutput, CastVoteError> {
    try_receive_ballot_with_network(
        input,
        tenant_id,
        voter_id,
        area_id,
        voting_channel,
        auth_time,
        None,
    )
    .await
}

#[instrument(skip_all)]
pub async fn try_receive_ballot_with_network(
    input: ReceiveBallotInput,
    tenant_id: &str,
    voter_id: &str,
    area_id: &str,
    voting_channel: VotingStatusChannel,
    auth_time: &Option<i64>,
    network: Option<(&str, Option<&str>, Option<&str>)>,
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
    let (ballot_tracker_policy, published_ballot_styles) = ballot_tracker_context(
        &hasura_transaction,
        Some(&presentation),
        tenant_id,
        &area.election_event_id,
        area_id,
        &input.election_id.to_string(),
    )
    .await?;
    if is_multi_contest {
        deserialize_and_check_multi_ballot(
            &input,
            voter_id,
            &ballot_tracker_policy,
            &published_ballot_styles,
        )?;
    } else {
        deserialize_and_check_ballot(
            &input,
            voter_id,
            &ballot_tracker_policy,
            &published_ballot_styles,
        )?;
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
        ballot_hash: &input.ballot_id,
    };

    check_published_ballot(
        &hasura_transaction,
        tenant_id,
        &election_event.id,
        area_id,
        &input,
        presentation
            .contest_encryption_policy
            .clone()
            .unwrap_or_default(),
    )
    .await?;

    let area_presentation: AreaPresentation = match area.presentation {
        Some(presentation) => deserialize_value(presentation)
            .map_err(|e| CastVoteError::DeserializeAreaPresentationFailed(e.to_string()))?,
        None => AreaPresentation::default(),
    };
    check_status(
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

    let signing_key = get_ballot_box_signing_key(&hasura_transaction, &election_event)
        .await
        .map_err(|e| CastVoteError::BallotSignFailed(e.to_string()))?
        .ok_or_else(|| {
            CastVoteError::BallotSignFailed(
                "The election event has no ballot box key; publish its ballots".to_string(),
            )
        })?;
    hasura_transaction
        .commit()
        .await
        .map_err(|e| CastVoteError::CommitFailed(e.to_string()))?;
    drop(hasura_db_client);

    let store = get_event_store(&election_event.id)
        .await
        .map_err(|e| CastVoteError::InsertFailed(format!("{e:#}")))?;
    crate::services::vote_privacy::require_signed_receipts(&election_event, Some(voting_channel))
        .map_err(|_| CastVoteError::CheckStatusFailed("Incompatible ballot privacy policy".into()))?;
    let storage_identity =
        crate::services::vote_privacy::ballot_identity(&election_event, voter_id)
            .await
            .map_err(|_| {
                CastVoteError::CheckStatusFailed("Voter privacy registration is not ready".into())
            })?;
    crate::services::electoral_log_board::get_event_databases()
        .await
        .map_err(|_| CastVoteError::InsertFailed("Ballot registration unavailable".into()))?
        .mark_ballot_activity(
            &election_event.id,
            electoral_log::adapters::events::ActivityMark::Throttled,
        )
        .await
        .map_err(|_| CastVoteError::InsertFailed("Ballot registration unavailable".into()))?;
    let stored_content = storage_ballot_content(
        &input.content,
        crate::services::vote_privacy::identity_mode(&election_event).map_err(|_| {
            CastVoteError::CheckStatusFailed("Invalid ballot privacy policy".into())
        })?,
        is_multi_contest,
    )?;
    let first_received_at = truncate_to_milliseconds(Utc::now())?;
    let outcome = store
        .receive_ballot(
            &ReceiveBallot {
                election_event_id: &election_event.id,
                election_id: &election_id,
                area_id,
                voter_id: &storage_identity,
                ballot_hash: &input.ballot_id,
                content: &stored_content,
                voter_signing_pk: &voter_signing_pk,
                voter_ballot_signature: &voter_ballot_signature,
            },
            first_received_at,
            |received_at| {
                let received = sign_received(
                    &signing_key,
                    &scope,
                    &voter_signing_pk,
                    &voter_ballot_signature,
                    &received_at,
                )
                .map_err(anyhow::Error::msg)?;
                Ok(ReceivedSignature {
                    ballot_id: received.ballot_id,
                    key_id: received.statement.key_id,
                    received_signature: received.received_signature,
                })
            },
        )
        .await
        .map_err(|error| match error.downcast::<CastVoteError>() {
            Ok(error) => error,
            Err(error) => CastVoteError::InsertFailed(format!("{error:#}")),
        })?;

    match outcome {
        ReceiveOutcome::Received(stored) | ReceiveOutcome::AlreadyReceived(stored) => {
            let policy = election_event
                .get_presentation()
                .map_err(|_| CastVoteError::CheckStatusFailed("Invalid privacy policy".into()))?
                .unwrap_or_default();
            if policy.ballot_identity
                == Some(sequent_core::types::vote_privacy::BallotIdentity::Pseudonym)
                && policy.network_data_policy
                    == Some(sequent_core::types::vote_privacy::NetworkDataPolicy::ForensicStore)
            {
                if let Some((ip, country, user_agent)) = network {
                    store
                        .record_forensic_network(
                            &electoral_log::adapters::privacy::ForensicNetworkRecord {
                                election_event_id: &stored.election_event_id,
                                election_id: Some(&stored.election_id),
                                area_id: Some(&stored.area_id),
                                event_kind: "receive",
                                delivery_id: &format!(
                                    "receive:{}:{}",
                                    stored.election_event_id, stored.ballot_id
                                ),
                                pseudonym: &stored.voter_id,
                                ballot_id: Some(&stored.ballot_id),
                                ip,
                                country,
                                user_agent,
                                recorded_at: stored.received_at,
                            },
                        )
                        .await
                        .map_err(|_| {
                            CastVoteError::InsertFailed("Unable to record forensic receive".into())
                        })?;
                }
            }
            Ok(stored_receipt(&tenant_uuid, stored))
        }
        ReceiveOutcome::NoFreeBallotId => Err(CastVoteError::InsertFailed(
            "No free Ballot ID for the received ballot".to_string(),
        )),
    }
}

#[cfg(test)]
#[path = "receive_ballot_tests.rs"]
mod tests;
