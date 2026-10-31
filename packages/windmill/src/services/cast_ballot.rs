// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The voter casts a ballot the ballot box received at review by signing its
//! Ballot ID with the key that signed the ballot. The ballot box stores the
//! cast and signs a receipt for it, which it gives out only once the cast is
//! stored.

use crate::services::ballot_box_key::{get_ballot_box_signing_key, published_key};
use crate::services::electoral_log_board::{get_event_databases, get_event_store};
use crate::services::insert_cast_vote::{
    try_cast, CastOutcome, CastRequest, CastVoteError, InsertCastVoteInput, InsertCastVoteOutput,
    InsertCastVoteResult,
};
use crate::services::receive_ballot::{format_signed_time, truncate_to_milliseconds};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use electoral_log::adapters::ballot_box::{AcceptBallot, AcceptOutcome, AcceptReceived};
use electoral_log::adapters::ballot_box_reads::BallotIdMatch;
use electoral_log::adapters::ballot_box_received::{ReceivedBallot, ReceivedStatus};
use electoral_log::adapters::events::ActivityMark;
use electoral_log::adapters::postgres::PostgresStore;
use sequent_core::ballot::VotingStatusChannel;
use sequent_core::ballot_receipt::{
    normalize_ballot_id, sign_cast_receipt, verify_cast_signature, CastReceipt,
    CastReceiptStatement,
};
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::hasura::core::ElectionEvent;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use uuid::Uuid;
use voting_crypto::signing::{Signature, SigningKey};

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

fn store_error(error: anyhow::Error) -> CastVoteError {
    CastVoteError::InsertFailed(format!("{error:#}"))
}

fn receipt_statement(
    received: &ReceivedBallot,
    cast_at: &DateTime<Utc>,
    key_id: &str,
    cast_signature: &str,
) -> CastReceiptStatement {
    CastReceiptStatement {
        election_event_id: received.election_event_id.clone(),
        election_id: received.election_id.clone(),
        ballot_id: received.ballot_id.clone(),
        received_at: format_signed_time(&received.received_at),
        cast_at: format_signed_time(cast_at),
        key_id: key_id.to_string(),
        cast_signature: cast_signature.to_string(),
    }
}

/// The receipt the ballot box signed when it stored the cast, if it did.
pub(crate) fn stored_receipt(received: &ReceivedBallot) -> Option<CastReceipt> {
    match (
        &received.cast_at,
        &received.cast_signature,
        &received.cast_receipt_signature,
    ) {
        (Some(cast_at), Some(cast_signature), Some(cast_receipt_signature)) => Some(CastReceipt {
            statement: receipt_statement(received, cast_at, &received.key_id, cast_signature),
            cast_receipt_signature: cast_receipt_signature.clone(),
        }),
        _ => None,
    }
}

/// The signature as the ballot box writes it, once it verifies against the
/// key that signed the received ballot.
fn check_cast_signature(
    received: &ReceivedBallot,
    cast_signature: &str,
) -> Result<String, CastVoteError> {
    let refused = |error: &dyn std::fmt::Display| {
        CastVoteError::BallotCastSignatureFailed(format!("Cast signature check failed: {error}"))
    };
    verify_cast_signature(
        &received.voter_signing_pk,
        &received.election_id,
        &received.ballot_id,
        cast_signature,
    )
    .map_err(|error| refused(&error))?;
    Signature::from_b64_string(cast_signature)
        .and_then(|signature| signature.to_b64_string())
        .map_err(|error| refused(&error))
}

/// What to do with a received ballot whose Cast signature verifies. A ballot
/// is cast once: the same signature again is a retry of that cast.
fn cast_or_stored_receipt(
    received: &ReceivedBallot,
    cast_signature: &str,
) -> Result<Option<CastReceipt>, CastVoteError> {
    match received.status {
        ReceivedStatus::Received => Ok(None),
        ReceivedStatus::Audited => Err(CastVoteError::BallotAudited),
        ReceivedStatus::Cast => match stored_receipt(received) {
            Some(receipt) if receipt.statement.cast_signature == cast_signature => {
                Ok(Some(receipt))
            }
            _ => Err(CastVoteError::BallotAlreadyCast),
        },
    }
}

/// A received ballot about to be cast: the voter's Cast signature over its
/// Ballot ID, and the key the ballot box signs the receipt with.
pub struct ReceivedCast {
    pub received_ballot: ReceivedBallot,
    pub cast_signature: String,
    pub ballot_box_key: SigningKey,
    pub key_id: String,
}

pub enum FoundCast {
    /// The ballot waits to be cast; the Cast signature verifies.
    ToCast(ReceivedBallot, String),
    /// The ballot box already stored this cast: the receipt it signed then.
    AlreadyCast(CastBallotOutput),
}

/// The cast vote a cast received ballot was stored as, with its receipt.
async fn cast_output(
    store: &PostgresStore,
    received: &ReceivedBallot,
    receipt: CastReceipt,
) -> Result<CastBallotOutput, CastVoteError> {
    let cast = store
        .received_cast_ballots(
            &received.election_event_id,
            &received.voter_id,
            &received.election_id,
            BallotIdMatch::Exact(&received.ballot_id),
        )
        .await
        .map_err(store_error)?;
    let cast = cast
        .into_iter()
        .next()
        .ok_or(CastVoteError::BallotAlreadyCast)?;
    Ok(CastBallotOutput {
        receipt,
        cast_vote_id: cast.ballot.ballot.id,
    })
}

/// Finds the ballot the voter asks to cast and checks the request against it.
/// Another voter's Ballot ID is not found, as an unknown one.
#[instrument(skip_all)]
pub async fn find_received_cast(
    store: &PostgresStore,
    election_event_id: &str,
    voter_id: &str,
    input: &CastBallotInput,
) -> Result<FoundCast, CastVoteError> {
    let ballot_id =
        normalize_ballot_id(&input.ballot_id).ok_or(CastVoteError::BallotNotReceived)?;
    let received = store
        .received_ballot_by_id(
            election_event_id,
            &input.election_id.to_string(),
            voter_id,
            &ballot_id,
        )
        .await
        .map_err(store_error)?
        .ok_or(CastVoteError::BallotNotReceived)?;

    let cast_signature = check_cast_signature(&received, &input.cast_signature)?;
    match cast_or_stored_receipt(&received, &cast_signature)? {
        None => Ok(FoundCast::ToCast(received, cast_signature)),
        Some(receipt) => Ok(FoundCast::AlreadyCast(
            cast_output(store, &received, receipt).await?,
        )),
    }
}

/// The authenticated voter is resolved inside the existing receipt ownership
/// query. Only the receipt's storage identity leaves that query.
#[instrument(skip_all)]
pub async fn find_received_cast_for_owner(
    store: &PostgresStore,
    event: &str,
    area: &str,
    voter: &str,
    mode: electoral_log::adapters::privacy::BallotIdentity,
    input: &CastBallotInput,
) -> Result<FoundCast, CastVoteError> {
    let ballot_id =
        normalize_ballot_id(&input.ballot_id).ok_or(CastVoteError::BallotNotReceived)?;
    let received = store
        .received_ballot_for_owner(
            event,
            &input.election_id.to_string(),
            area,
            voter,
            &ballot_id,
            mode,
        )
        .await
        .map_err(store_error)?
        .ok_or(CastVoteError::BallotNotReceived)?;
    let signature = check_cast_signature(&received, &input.cast_signature)?;
    match cast_or_stored_receipt(&received, &signature)? {
        None => Ok(FoundCast::ToCast(received, signature)),
        Some(receipt) => Ok(FoundCast::AlreadyCast(
            cast_output(store, &received, receipt).await?,
        )),
    }
}

/// What casting a received ballot takes: the ballot as it was received, and
/// the key that signs the receipt.
pub fn received_cast(
    received: ReceivedBallot,
    cast_signature: String,
    ballot_box_key: SigningKey,
) -> Result<(InsertCastVoteInput, ReceivedCast), CastVoteError> {
    let key_id = published_key(&ballot_box_key)
        .map_err(|e| CastVoteError::BallotSignFailed(e.to_string()))?
        .key_id;
    // A stored receipt is read back with the key of its Received statement.
    if key_id != received.key_id {
        return Err(CastVoteError::BallotSignFailed(
            "The ballot box key is not the one that received the ballot".to_string(),
        ));
    }
    let election_id = parse_uuid_v4(&received.election_id)
        .map_err(|e| CastVoteError::UuidParseFailed(e.to_string(), "election_id".to_string()))?;

    Ok((
        InsertCastVoteInput {
            ballot_id: received.ballot_hash.clone(),
            election_id,
            content: received.content.clone(),
        },
        ReceivedCast {
            received_ballot: received,
            cast_signature,
            ballot_box_key,
            key_id,
        },
    ))
}

/// The receipt the ballot box signs for casting the received ballot now. It
/// is good only once the cast it names is stored.
pub fn sign_receipt(
    received_cast: &ReceivedCast,
) -> Result<(DateTime<Utc>, CastReceipt), CastVoteError> {
    let cast_at = truncate_to_milliseconds(Utc::now())?;
    let receipt = sign_cast_receipt(
        &received_cast.ballot_box_key,
        receipt_statement(
            &received_cast.received_ballot,
            &cast_at,
            &received_cast.key_id,
            &received_cast.cast_signature,
        ),
    )
    .map_err(|err| CastVoteError::BallotSignFailed(err.to_string()))?;
    Ok((cast_at, receipt))
}

/// Accepts the vote that casts a received ballot and marks the received
/// ballot cast with the receipt the ballot box signs for it, in one write. A
/// received ballot that no longer waits to be cast is refused as audited,
/// already cast or not received.
#[instrument(skip_all)]
pub(crate) async fn accept_received_cast(
    accept: &AcceptBallot<'_>,
    received_cast: &ReceivedCast,
) -> Result<(AcceptOutcome, DateTime<Utc>, CastReceipt), CastVoteError> {
    accept_received_cast_with_network(accept, received_cast, None).await
}

pub(crate) async fn accept_received_cast_with_network(
    accept: &AcceptBallot<'_>,
    received_cast: &ReceivedCast,
    network: Option<(&str, Option<&str>, Option<&str>)>,
) -> Result<(AcceptOutcome, DateTime<Utc>, CastReceipt), CastVoteError> {
    let (cast_at, receipt) = sign_receipt(received_cast)?;
    let received = &received_cast.received_ballot;
    let databases = get_event_databases().await.map_err(store_error)?;
    databases
        .mark_ballot_activity(accept.election_event_id, ActivityMark::Throttled)
        .await
        .map_err(store_error)?;
    let store = databases
        .store(accept.election_event_id)
        .await
        .map_err(store_error)?;
    let accepted = AcceptReceived {
        ballot_id: &received.ballot_id,
        cast_at,
        cast_signature: &received_cast.cast_signature,
        cast_receipt_signature: &receipt.cast_receipt_signature,
    };
    let delivery_id = format!("cast:{}:{}", received.election_event_id, received.ballot_id);
    let outcome = match network {
        Some((ip, country, user_agent)) => {
            store
                .accept_received_ballot_with_forensics(
                    accept,
                    &accepted,
                    &electoral_log::adapters::privacy::ForensicNetworkRecord {
                        election_event_id: accept.election_event_id,
                        election_id: Some(accept.election_id),
                        area_id: Some(accept.area_id),
                        event_kind: "cast",
                        delivery_id: &delivery_id,
                        pseudonym: accept.voter_id,
                        ballot_id: Some(&received.ballot_id),
                        ip,
                        country,
                        user_agent,
                        recorded_at: cast_at,
                    },
                )
                .await
        }
        None => store.accept_received_ballot(accept, &accepted).await,
    }
    .map_err(store_error)?;
    if outcome != AcceptOutcome::NotReceivable {
        return Ok((outcome, cast_at, receipt));
    }
    let stored = store
        .received_ballot_by_id(
            &received.election_event_id,
            &received.election_id,
            &received.voter_id,
            &received.ballot_id,
        )
        .await
        .map_err(store_error)?;
    match stored.map(|stored| stored.status) {
        Some(ReceivedStatus::Audited) => Err(CastVoteError::BallotAudited),
        Some(ReceivedStatus::Cast) => Err(CastVoteError::BallotAlreadyCast),
        Some(ReceivedStatus::Received) | None => Err(CastVoteError::BallotNotReceived),
    }
}

/// The receipt of a received ballot another request already cast with the
/// same Cast signature; `None` when it was cast with another one.
#[instrument(skip_all)]
pub(crate) async fn stored_cast(
    received_cast: &ReceivedCast,
) -> Result<Option<CastBallotOutput>, CastVoteError> {
    let received = &received_cast.received_ballot;
    let store = get_event_store(&received.election_event_id)
        .await
        .map_err(store_error)?;
    let Some(stored) = store
        .received_ballot_by_id(
            &received.election_event_id,
            &received.election_id,
            &received.voter_id,
            &received.ballot_id,
        )
        .await
        .map_err(store_error)?
    else {
        return Ok(None);
    };
    match cast_or_stored_receipt(&stored, &received_cast.cast_signature) {
        Ok(Some(receipt)) => Ok(Some(cast_output(&store, &stored, receipt).await?)),
        Ok(None) | Err(_) => Ok(None),
    }
}

#[instrument(skip_all)]
pub(crate) async fn prepare_received_cast(
    hasura_transaction: &Transaction<'_>,
    election_event: &ElectionEvent,
    voter_id: &str,
    area_id: &str,
    input: &CastBallotInput,
) -> Result<PreparedCast, CastVoteError> {
    let store = get_event_store(&election_event.id)
        .await
        .map_err(store_error)?;
    let mode = match crate::services::vote_privacy::identity_mode(election_event)
        .map_err(|_| CastVoteError::CheckStatusFailed("Invalid ballot privacy policy".into()))?
    {
        sequent_core::types::vote_privacy::BallotIdentity::VoterId => {
            electoral_log::adapters::privacy::BallotIdentity::VoterId
        }
        sequent_core::types::vote_privacy::BallotIdentity::Pseudonym => {
            electoral_log::adapters::privacy::BallotIdentity::Pseudonym
        }
    };
    let found =
        find_received_cast_for_owner(&store, &election_event.id, area_id, voter_id, mode, input)
            .await?;
    let (received, cast_signature) = match found {
        FoundCast::AlreadyCast(cast) => return Ok(PreparedCast::AlreadyCast(cast)),
        FoundCast::ToCast(received, cast_signature) => (received, cast_signature),
    };

    let ballot_box_key = get_ballot_box_signing_key(hasura_transaction, election_event)
        .await
        .map_err(|e| CastVoteError::BallotSignFailed(e.to_string()))?
        .ok_or_else(|| {
            CastVoteError::BallotSignFailed("The election event has no ballot box key".to_string())
        })?;
    let (input, received_cast) = received_cast(received, cast_signature, ballot_box_key)?;
    Ok(PreparedCast::ToCast(input, received_cast))
}

#[instrument(skip_all)]
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
    try_cast_ballot_with_network(
        input,
        tenant_id,
        voter_id,
        area_id,
        voting_channel,
        auth_time,
        voter_ip,
        voter_country,
        username,
        &None,
    )
    .await
}

#[instrument(skip_all)]
pub async fn try_cast_ballot_with_network(
    input: CastBallotInput,
    tenant_id: &str,
    voter_id: &str,
    area_id: &str,
    voting_channel: VotingStatusChannel,
    auth_time: &Option<i64>,
    voter_ip: &Option<String>,
    voter_country: &Option<String>,
    username: &Option<String>,
    user_agent: &Option<String>,
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
        user_agent,
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
            InsertCastVoteResult::Success(cast_vote, _) => {
                Ok(CastBallotResult::Cast(output(receipt, &cast_vote)?))
            }
            InsertCastVoteResult::PendingDatafix(cast_vote, _) => Ok(
                CastBallotResult::PendingDatafix(output(receipt, &cast_vote)?, cast_vote),
            ),
        },
    }
}

#[cfg(test)]
#[path = "cast_ballot_tests.rs"]
mod tests;

#[cfg(test)]
mod postgres_wiring_tests {
    use super::*;
    use crate::services::ballot_box::BallotFormat;
    use crate::services::receive_ballot::format_signed_time;
    use electoral_log::adapters::ballot_box::BallotStatus;
    use electoral_log::adapters::ballot_box_received::{
        ReceiveBallot, ReceiveOutcome, ReceivedSignature,
    };
    use sequent_core::ballot_receipt::{
        sign_cast_statement, sign_received_ballot, ReceivedStatement,
    };
    use voting_crypto::signing::VerificationKey;

    struct Event {
        id: String,
        election: String,
        area: String,
        store: PostgresStore,
        ballot_box_sk: SigningKey,
    }

    impl Event {
        async fn create() -> Self {
            let id = Uuid::new_v4().to_string();
            let store = get_event_databases()
                .await
                .unwrap()
                .create_event(&Uuid::new_v4().to_string(), &id)
                .await
                .unwrap();
            store.create_ballot_box(&id).await.unwrap();
            Self {
                id,
                election: Uuid::new_v4().to_string(),
                area: Uuid::new_v4().to_string(),
                store,
                ballot_box_sk: SigningKey::generate().unwrap(),
            }
        }

        /// A ballot of the voter the ballot box received, with the voter's
        /// key and the Cast signature for it.
        async fn receive(&self, ballot_hash: &str) -> (ReceivedBallot, String) {
            let voter_sk = SigningKey::generate().unwrap();
            let voter_signing_pk = VerificationKey::from_sk(&voter_sk)
                .unwrap()
                .to_b64_string()
                .unwrap();
            let voter_ballot_signature = voter_sk.sign(b"ballot").unwrap().to_b64_string().unwrap();
            let outcome = self
                .store
                .receive_ballot(
                    &ReceiveBallot {
                        election_event_id: &self.id,
                        election_id: &self.election,
                        area_id: &self.area,
                        voter_id: "voter",
                        ballot_hash,
                        content: "ciphertext",
                        voter_signing_pk: &voter_signing_pk,
                        voter_ballot_signature: &voter_ballot_signature,
                    },
                    truncate_to_milliseconds(Utc::now()).unwrap(),
                    |received_at| {
                        let received = sign_received_ballot(
                            &self.ballot_box_sk,
                            ReceivedStatement {
                                tenant_id: Uuid::new_v4().to_string(),
                                election_event_id: self.id.clone(),
                                election_id: self.election.clone(),
                                ballot_hash: ballot_hash.to_string(),
                                voter_signing_pk: voter_signing_pk.clone(),
                                voter_ballot_signature: voter_ballot_signature.clone(),
                                received_at: format_signed_time(&received_at),
                                key_id: published_key(&self.ballot_box_sk)?.key_id,
                            },
                        )?;
                        Ok(ReceivedSignature {
                            ballot_id: received.ballot_id,
                            key_id: received.statement.key_id,
                            received_signature: received.received_signature,
                        })
                    },
                )
                .await
                .unwrap();
            let ReceiveOutcome::Received(received) = outcome else {
                panic!("the ballot is received: {outcome:?}");
            };
            let cast_signature =
                sign_cast_statement(&voter_sk, &self.election, &received.ballot_id).unwrap();
            (received, cast_signature)
        }

        fn cast(&self, received: &ReceivedBallot, cast_signature: &str) -> ReceivedCast {
            received_cast(
                received.clone(),
                cast_signature.to_string(),
                self.ballot_box_sk.clone(),
            )
            .unwrap()
            .1
        }

        async fn accept(
            &self,
            received_cast: &ReceivedCast,
        ) -> Result<(AcceptOutcome, DateTime<Utc>, CastReceipt), CastVoteError> {
            let format = BallotFormat::HashableBallot.to_string();
            accept_received_cast(
                &AcceptBallot {
                    election_event_id: &self.id,
                    election_id: &self.election,
                    area_id: &self.area,
                    voter_id: "voter",
                    ballot_id: &received_cast.received_ballot.ballot_hash,
                    format: &format,
                    content: &received_cast.received_ballot.content,
                    voter_signature: None,
                    pseudonym_hash: &[0; 64],
                    ballot_hash: &[0; 64],
                    voting_channel: "ONLINE",
                    status: BallotStatus::Valid,
                    voter_ip: None,
                    voter_country: None,
                    username: None,
                    allowed_votes: 0,
                    vitrine: None,
                },
                received_cast,
            )
            .await
        }
    }

    #[tokio::test]
    #[ignore = "requires a disposable ELECTORAL_LOG_PG_* database"]
    async fn a_received_ballot_is_cast_once_and_a_retry_gets_its_receipt() {
        let event = Event::create().await;
        let (received, cast_signature) = event.receive("0abc12").await;
        let received_cast = event.cast(&received, &cast_signature);

        let (AcceptOutcome::Accepted { id, .. }, _, receipt) =
            event.accept(&received_cast).await.unwrap()
        else {
            panic!("the received ballot is cast");
        };
        assert!(matches!(
            event.accept(&received_cast).await.unwrap().0,
            AcceptOutcome::DuplicateBallotId
        ));

        let stored = stored_cast(&received_cast).await.unwrap().unwrap();
        assert_eq!(stored.cast_vote_id, id);
        assert_eq!(stored.receipt, receipt);
        let another = event.cast(&received, "another-signature");
        assert_eq!(stored_cast(&another).await.unwrap(), None);
    }

    #[tokio::test]
    #[ignore = "requires a disposable ELECTORAL_LOG_PG_* database"]
    async fn an_audited_or_unknown_received_ballot_is_not_cast() {
        let event = Event::create().await;
        let (received, cast_signature) = event.receive("0abc12").await;
        event
            .store
            .client()
            .await
            .unwrap()
            .execute(
                "UPDATE ballot_box_received SET status = 'audited' \
                 WHERE election_event_id = $1::text::uuid AND ballot_id = $2",
                &[&event.id, &received.ballot_id],
            )
            .await
            .unwrap();
        assert!(matches!(
            event.accept(&event.cast(&received, &cast_signature)).await,
            Err(CastVoteError::BallotAudited)
        ));

        let mut unknown = received.clone();
        unknown.ballot_id = "0000-0000".to_string();
        assert!(matches!(
            event.accept(&event.cast(&unknown, &cast_signature)).await,
            Err(CastVoteError::BallotNotReceived)
        ));
    }
}
