// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::area::get_area_by_id;
use crate::postgres::ballot_style::get_published_ballot_emls;
use crate::postgres::election::{get_cast_vote_configuration, CastVoteConfiguration};
use crate::postgres::election_event::get_election_event_by_id;
use crate::services::ballot_box::{accept_ballot, get_allowed_votes, BallotFormat};
use crate::services::ballot_box_reads::cast_vote_status;
use crate::services::cast_ballot::{
    prepare_received_cast, stored_cast, CastBallotInput, CastBallotOutput, PreparedCast,
    ReceivedCast,
};
use crate::services::cast_votes::{CastVote, CastVoteStatus};
use crate::services::database::get_hasura_pool;
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log::ElectoralLog;
use crate::services::electoral_log_board::get_board_signing_key;
use crate::services::external::utils::{
    datafix_annotations, external_voter_lock_key, is_datafix_election_event,
    DATAFIX_VOTER_LOCK_SECS,
};
use crate::services::pg_lock::PgLock;
use crate::services::receive_ballot::must_be_received;
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Duration, Local};
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use electoral_log::adapters::ballot_box::{AcceptBallot, AcceptOutcome, BallotStatus};
use electoral_log::messages::newtypes::*;
use sequent_core::ballot::verify_ballot_signature;
use sequent_core::ballot::BallotStyle;
use sequent_core::ballot::BallotTrackerPolicy;
use sequent_core::ballot::ContestEncryptionPolicy;
// The tests build presentations with it; the grace period itself comes from
// `ballot_box_seal::deadline::grace_period`.
use anyhow::Error;
#[cfg(test)]
use sequent_core::ballot::EGracePeriodPolicy;
use sequent_core::ballot::ElectionEventPresentation;
use sequent_core::ballot::LEGACY_TYPES_VERSION;
use sequent_core::ballot::{
    AreaPresentation, EarlyVotingPolicy, ElectionPresentation, ElectionStatus, VoterSigningPolicy,
    VotingPeriodDates, VotingStatus, VotingStatusChannel,
};
use sequent_core::ballot::{HashableBallot, HashableBallotContest, SignedHashableBallot};
use sequent_core::ballot_receipt::CastReceipt;
use sequent_core::encrypt::hash_ballot;
use sequent_core::encrypt::hash_ballot_sha512;
use sequent_core::encrypt::hash_ballot_style;
use sequent_core::encrypt::hash_multi_ballot;
use sequent_core::encrypt::hash_multi_ballot_sha512;
use sequent_core::error::BallotError;
use sequent_core::multi_ballot::verify_multi_ballot_signature;
use sequent_core::multi_ballot::HashableMultiBallot;
use sequent_core::multi_ballot::HashableMultiBallotContests;
use sequent_core::multi_ballot::SignedHashableMultiBallot;
use sequent_core::serialization::deserialize_with_path::*;
use sequent_core::services::date::ISO8601;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::hasura::core::{ElectionEvent, VotingChannels};
use sequent_core::types::scheduled_event::*;
use serde::{Deserialize, Serialize};
use serde_json::Serializer;
use std::time::Instant;
use strum_macros::Display;
use tracing::{debug, error, info, instrument, trace};
use uuid::Uuid;
use voting_crypto::hash::{hash_to_array, Hash, HashWrapper};
use voting_crypto::signing::Signature;
use voting_crypto::signing::VerificationKey;
use voting_crypto::RecordSerialize;
use voting_crypto::RistrettoCtx;

/// Emits one duration even when a phase fails or its future is cancelled.
struct CastVotePhase {
    phase: &'static str,
    started: Instant,
}

impl CastVotePhase {
    fn start(phase: &'static str) -> Self {
        Self {
            phase,
            started: Instant::now(),
        }
    }
}

impl Drop for CastVotePhase {
    fn drop(&mut self) {
        info!(
            phase = self.phase,
            duration_us = self.started.elapsed().as_micros() as u64,
            "cast-vote phase completed"
        );
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InsertCastVoteInput {
    // Here is the class used for voting
    pub ballot_id: String,
    pub election_id: Uuid,
    pub content: String,
}
impl InsertCastVoteInput {
    /// Returns a byte representation of this object suitable for hashing
    /// and then signing.
    ///
    /// To avoid adding the borsh dependency we do the serialization
    /// manually. This requires an invertible map which we get
    /// by prepending a fixed length prefix to each field
    /// with its size. Because the maximum representation of a usize is
    /// 8, we use 8 as the fixed size length prefix.
    pub(crate) fn get_bytes_for_signing(&self) -> Vec<u8> {
        let mut ret: Vec<u8> = vec![];

        let bytes = self.ballot_id.as_bytes();
        let mut length = [0u8; 8];
        let b = bytes.len().to_le_bytes();
        let l = b.len();
        length[0..l].copy_from_slice(&bytes[0..l]);

        ret.extend(&length);
        ret.extend(bytes);

        let bytes = self.election_id.as_bytes();
        let mut length = [0u8; 8];
        let b = bytes.len().to_le_bytes();
        let l = b.len();
        length[0..l].copy_from_slice(&bytes[0..l]);

        ret.extend(&length);
        ret.extend(bytes);

        let bytes = self.content.as_bytes();
        let mut length = [0u8; 8];
        let b = bytes.len().to_le_bytes();
        let l = b.len();
        length[0..l].copy_from_slice(&bytes[0..l]);

        ret.extend(&length);
        ret.extend(bytes);

        ret
    }
}

pub type InsertCastVoteOutput = CastVote;

/// Outcome of a cast-vote insert, distinguishing the follow-up each needs:
/// `Success` is a final `valid` vote, `PendingDatafix` is an `in-progress` vote
/// the caller must enqueue for the async Datafix pipeline, and
/// `SkipRetryFailure` is a terminal error the caller should surface as-is.
pub enum InsertCastVoteResult {
    Success(InsertCastVoteOutput),
    PendingDatafix(InsertCastVoteOutput),
    SkipRetryFailure(CastVoteError),
}

/// What a voter asks the ballot box to cast: a ballot sent with the request,
/// or one the ballot box received at review, named by its Ballot ID.
pub(crate) enum CastRequest {
    Ballot(InsertCastVoteInput),
    Received(CastBallotInput),
}

pub(crate) enum CastOutcome {
    Inserted {
        result: InsertCastVoteResult,
        /// Signed by the ballot box for a ballot it had received.
        receipt: Option<CastReceipt>,
    },
    /// The same cast was stored by an earlier request.
    AlreadyCast(CastBallotOutput),
}

impl CastOutcome {
    fn refused(error: CastVoteError) -> Self {
        Self::refused_result(InsertCastVoteResult::SkipRetryFailure(error))
    }

    fn refused_result(result: InsertCastVoteResult) -> Self {
        CastOutcome::Inserted {
            result,
            receipt: None,
        }
    }
}

/// A ballot accepted into the ballot box, with the receipt the ballot box
/// signed when it casts a ballot it had received.
type Accepted = (CastVote, VotingStatusChannel, Option<CastReceipt>);

/// Maps a freshly inserted row to its `InsertCastVoteResult` from the persisted
/// status: `in-progress` still needs Datafix processing, `valid` is final. Any
/// other status is unreachable for a new insert and surfaces as an error.
#[instrument(skip_all)]
fn classify_inserted_cast_vote(
    cast_vote: InsertCastVoteOutput,
) -> Result<InsertCastVoteResult, CastVoteError> {
    match cast_vote.status {
        CastVoteStatus::InProgress => Ok(InsertCastVoteResult::PendingDatafix(cast_vote)),
        CastVoteStatus::Valid => Ok(InsertCastVoteResult::Success(cast_vote)),
        status => Err(CastVoteError::UnknownError(format!(
            "Unexpected initial cast vote status: {status}"
        ))),
    }
}

/// Decides the status a new ballot is accepted with: Datafix events' ballots start
/// pending so the async pipeline can confirm eligibility, ordinary events' ballots
/// start valid. Fails closed if the Datafix configuration is malformed.
#[instrument(skip_all)]
fn initial_ballot_status(election_event: &ElectionEvent) -> Result<BallotStatus, CastVoteError> {
    match datafix_annotations(election_event) {
        Ok(Some(_)) => Ok(BallotStatus::Pending),
        Ok(None) => Ok(BallotStatus::Valid),
        Err(err) => Err(CastVoteError::InvalidDatafixConfiguration(err.to_string())),
    }
}

/// Releases a Datafix voter lock, logging on failure. Lock cleanup is
/// best-effort — a failed release only delays reacquisition until the lock
/// expires, so it must never mask the caller's own error.
#[instrument(skip_all)]
async fn release_datafix_voter_lock(lock: PgLock) {
    if let Err(err) = lock.release().await {
        error!("Error releasing the Datafix voter lock: {err}");
    }
}

/// Accepts a Datafix vote under the per-voter lease, owning the whole lock and
/// connection lifecycle so `try_insert_cast_vote` stays free of it.
///
/// The caller must already have released its read transaction and connection (a
/// `Transaction` borrows its `Client`, so the commit/drop can't move in here) —
/// this is also required for pool safety: no connection is held while blocking on
/// the lease. This acquires the `(tenant, event, voter)` lease, runs the status
/// checks on a fresh connection, accepts the ballot, and always releases the lease
/// before returning.
#[instrument(skip_all)]
async fn accept_datafix_ballot_locked(
    ballot: &BallotToAccept<'_>,
) -> Result<Accepted, CastVoteError> {
    let ids = ballot.ids;
    let voter_id_uuid = parse_uuid_v4(ids.voter_id)
        .map_err(|err| CastVoteError::VoterStateLocked(format!("Invalid voter id: {err}")))?;
    let lock = PgLock::acquire(
        external_voter_lock_key(ids.tenant_id, ids.election_event_id, &voter_id_uuid),
        Uuid::new_v4().to_string(),
        ISO8601::now() + Duration::seconds(DATAFIX_VOTER_LOCK_SECS),
    )
    .await
    .map_err(|err| CastVoteError::VoterStateLocked(err.to_string()))?;

    let mut hasura_db_client = match get_hasura_pool().await.get().await {
        Ok(client) => client,
        Err(err) => {
            release_datafix_voter_lock(lock).await;
            return Err(CastVoteError::GetDbClientFailed(err.to_string()));
        }
    };
    let hasura_transaction = match hasura_db_client.transaction().await {
        Ok(transaction) => transaction,
        Err(err) => {
            release_datafix_voter_lock(lock).await;
            return Err(CastVoteError::GetTransactionFailed(err.to_string()));
        }
    };

    let result = accept_into_ballot_box(hasura_transaction, ballot).await;

    drop(hasura_db_client);
    release_datafix_voter_lock(lock).await;
    result
}

/// Maps a post-insert error to the caller's retry contract: a refusal that a
/// retry cannot change, such as an exceeded revote limit or voting being
/// closed, is terminal and surfaced as `SkipRetryFailure`, every other error
/// propagates for the normal retry path.
#[instrument(skip_all)]
fn skip_or_propagate(cast_vote_err: CastVoteError) -> Result<InsertCastVoteResult, CastVoteError> {
    match cast_vote_err {
        CastVoteError::InsertFailedExceedsAllowedRevotes
        | CastVoteError::CheckVotesInOtherAreasFailed(_)
        | CastVoteError::CheckStatusFailed(_)
        | CastVoteError::VotingChannelNotEnabled(_)
        | CastVoteError::BallotIdMismatch(_)
        | CastVoteError::BallotNotReceived
        | CastVoteError::BallotCastSignatureFailed(_)
        | CastVoteError::BallotAlreadyCast
        | CastVoteError::BallotAudited => Ok(InsertCastVoteResult::SkipRetryFailure(cast_vote_err)),
        _ => Err(cast_vote_err),
    }
}

#[derive(Debug, Clone, Copy)]
struct CastVoteIds<'a> {
    election_event_id: &'a str,
    tenant_id: &'a str,
    voter_id: &'a str,
    area_id: &'a str,
}

#[derive(Serialize, Deserialize, Debug, Display)]
pub enum CastVoteError {
    #[serde(rename = "voting_channel_not_enabled")]
    VotingChannelNotEnabled(String),
    #[serde(rename = "area_not_found")]
    AreaNotFound,
    #[serde(rename = "election_event_not_found")]
    ElectionEventNotFound(String),
    #[serde(rename = "invalid_datafix_configuration")]
    InvalidDatafixConfiguration(String),
    #[serde(rename = "electoral_log_not_found")]
    ElectoralLogNotFound(String),
    #[serde(rename = "check_status_failed")]
    CheckStatusFailed(String),
    #[serde(rename = "check_status_internal_failed")]
    CheckStatusInternalFailed(String),
    #[serde(rename = "voter_state_locked")]
    VoterStateLocked(String),
    #[serde(rename = "check_previous_votes_failed")]
    CheckPreviousVotesFailed(String),
    #[serde(rename = "check_revotes_failed")]
    CheckRevotesFailed(String),
    #[serde(rename = "check_votes_in_other_areas_failed")]
    CheckVotesInOtherAreasFailed(String),
    #[serde(rename = "insert_failed")]
    InsertFailed(String),
    #[serde(rename = "insert_failed_exceeds_allowed_revotes")]
    #[strum(to_string = "insert_failed_exceeds_allowed_revotes")]
    InsertFailedExceedsAllowedRevotes,
    #[serde(rename = "commit_failed")]
    CommitFailed(String),
    #[serde(rename = "get_db_client_failed")]
    GetDbClientFailed(String),
    #[serde(rename = "get_client_credentials_failed")]
    GetClientCredentialsFailed(String),
    #[serde(rename = "get_area_id_failed")]
    GetAreaIdFailed(String),
    #[serde(rename = "get_transaction_failed")]
    GetTransactionFailed(String),
    #[serde(rename = "deserialize_ballot_failed")]
    DeserializeBallotFailed(String),
    #[serde(rename = "deserialize_contests_failed")]
    DeserializeContestsFailed(String),
    #[serde(rename = "deserialize_area_presentation_failed")]
    DeserializeAreaPresentationFailed(String),
    #[serde(rename = "serialize_voter_id_failed")]
    SerializeVoterIdFailed(String),
    #[serde(rename = "serialize_ballot_failed")]
    SerializeBallotFailed(String),
    #[serde(rename = "pok_validation_failed")]
    PokValidationFailed(String),
    #[serde(rename = "ballot_sign_failed")]
    BallotSignFailed(String),
    #[serde(rename = "ballot_voter_signature_failed")]
    BallotVoterSignatureFailed(String),
    #[serde(rename = "ballot_voter_signature_required")]
    #[strum(to_string = "ballot_voter_signature_required")]
    BallotVoterSignatureRequired,
    #[serde(rename = "ballot_style_mismatch")]
    BallotStyleMismatch(String),
    #[serde(rename = "ballot_not_received")]
    #[strum(to_string = "ballot_not_received")]
    BallotNotReceived,
    #[serde(rename = "ballot_cast_signature_required")]
    #[strum(to_string = "ballot_cast_signature_required")]
    BallotCastSignatureRequired,
    #[serde(rename = "ballot_cast_signature_failed")]
    BallotCastSignatureFailed(String),
    #[serde(rename = "ballot_already_cast")]
    #[strum(to_string = "ballot_already_cast")]
    BallotAlreadyCast,
    #[serde(rename = "ballot_audited")]
    #[strum(to_string = "ballot_audited")]
    BallotAudited,
    #[serde(rename = "uuid_parse_failed")]
    UuidParseFailed(String, String),
    #[serde(rename = "ballot_id_mismatch")]
    #[strum(to_string = "ballot_id_mismatch")]
    BallotIdMismatch(String),
    #[serde(rename = "get_ballot_style_failed")]
    GetBallotStyleFailed(String),
    #[serde(rename = "unknown_error")]
    UnknownError(String),
}

impl CastVoteError {
    pub fn new(error: anyhow::Error) -> Self {
        match error.downcast::<CastVoteError>() {
            Ok(e) => e,
            Err(e) => CastVoteError::UnknownError(e.to_string()),
        }
    }
}

#[instrument(skip_all)]
pub async fn try_insert_cast_vote(
    input: InsertCastVoteInput,
    tenant_id: &str,
    voter_id: &str,
    area_id: &str,
    voting_channel: VotingStatusChannel,
    auth_time: &Option<i64>,
    voter_ip: &Option<String>,
    voter_country: &Option<String>,
    username: &Option<String>,
) -> Result<InsertCastVoteResult, CastVoteError> {
    let outcome = try_cast(
        CastRequest::Ballot(input),
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
    .await?;
    match outcome {
        CastOutcome::Inserted { result, .. } => Ok(result),
        CastOutcome::AlreadyCast(_) => Err(CastVoteError::UnknownError(
            "A ballot sent with the cast has no earlier receipt".to_string(),
        )),
    }
}

#[instrument(skip_all)]
pub(crate) async fn try_cast(
    request: CastRequest,
    tenant_id: &str,
    voter_id: &str,
    area_id: &str,
    voting_channel: VotingStatusChannel,
    auth_time: &Option<i64>,
    voter_ip: &Option<String>,
    voter_country: &Option<String>,
    username: &Option<String>,
    user_agent: &Option<String>,
) -> Result<CastOutcome, CastVoteError> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| CastVoteError::GetDbClientFailed(e.to_string()))?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| CastVoteError::GetTransactionFailed(e.to_string()))?;

    let area_opt = get_area_by_id(&hasura_transaction, tenant_id, area_id)
        .await
        .map_err(|e| CastVoteError::GetAreaIdFailed(e.to_string()))?;

    let area = if let Some(area) = area_opt {
        area
    } else {
        return Err(CastVoteError::AreaNotFound);
    };
    let election_event_id: &str = area.election_event_id.as_str();
    let election_event =
        get_election_event_by_id(&hasura_transaction, tenant_id, election_event_id)
            .await
            .map_err(|e| CastVoteError::ElectionEventNotFound(e.to_string()))?;

    let initial_status = initial_ballot_status(&election_event)?;

    let presentation_opt = election_event
        .get_presentation()
        .map_err(|e| CastVoteError::ElectionEventNotFound(e.to_string()))?;

    let is_multi_contest = if let Some(presentation) = presentation_opt.clone() {
        presentation.contest_encryption_policy == Some(ContestEncryptionPolicy::MULTIPLE_CONTESTS)
    } else {
        false
    };
    let presentation = presentation_opt.unwrap_or_default();

    crate::services::vote_privacy::require_signed_receipts(&election_event, Some(voting_channel))
        .map_err(|_| CastVoteError::CheckStatusFailed("Incompatible ballot privacy policy".into()))?;
    let private_ballot = presentation.ballot_identity
        == Some(sequent_core::types::vote_privacy::BallotIdentity::Pseudonym);

    // With receipts on, the ballot box casts only what it has received from
    // this voter, on the voter's signed request.
    let received_ballot_required =
        must_be_received(&presentation.receipts_policy(), voting_channel);
    let (input, received_cast) = match request {
        CastRequest::Ballot(_) if received_ballot_required => {
            return Ok(CastOutcome::refused(
                CastVoteError::BallotCastSignatureRequired,
            ));
        }
        CastRequest::Ballot(input) => (input, None),
        CastRequest::Received(_) if !received_ballot_required => {
            return Ok(CastOutcome::refused(CastVoteError::CheckStatusFailed(
                "The ballot box does not sign receipts in this election event".to_string(),
            )));
        }
        CastRequest::Received(cast) => {
            let prepared = prepare_received_cast(
                &hasura_transaction,
                &election_event,
                voter_id,
                area_id,
                &cast,
            )
            .await;
            match prepared {
                Ok(PreparedCast::ToCast(input, received_cast)) => (input, Some(received_cast)),
                Ok(PreparedCast::AlreadyCast(cast)) => {
                    return Ok(CastOutcome::AlreadyCast(cast));
                }
                Err(cast_vote_err) => {
                    return skip_or_propagate(cast_vote_err).map(CastOutcome::refused_result);
                }
            }
        }
    };

    let storage_identity = received_cast
        .as_ref()
        .map(|cast| cast.received_ballot.voter_id.clone())
        .unwrap_or_else(|| voter_id.to_owned());
    let voter_id = storage_identity.as_str();
    // Only the owned receipt's pseudonym enters ballot objects and evidence.
    let private_username = Some(storage_identity.clone());
    let username = if private_ballot {
        &private_username
    } else {
        username
    };

    crate::services::receive_ballot::check_published_ballot(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        area_id,
        &input,
        presentation
            .contest_encryption_policy
            .clone()
            .unwrap_or_default(),
    )
    .await?;

    let (ballot_tracker_policy, published_ballot_styles) = ballot_tracker_context(
        &hasura_transaction,
        Some(&presentation),
        tenant_id,
        election_event_id,
        area_id,
        &input.election_id.to_string(),
    )
    .await?;

    let hash_result = if is_multi_contest {
        deserialize_and_check_multi_ballot(
            &input,
            voter_id,
            &ballot_tracker_policy,
            &published_ballot_styles,
        )
    } else {
        deserialize_and_check_ballot(
            &input,
            voter_id,
            &ballot_tracker_policy,
            &published_ballot_styles,
        )
    };

    let (mut pseudonym_h, vote_h, voter_signature_data) = match hash_result {
        Ok(hash) => hash,
        Err(cv_err) => {
            return Ok(CastOutcome::refused(cv_err));
        }
    };

    if private_ballot {
        pseudonym_h = PseudonymHash(HashWrapper::new(
            voting_crypto::hash::hash_to_array(voter_id.as_bytes()).map_err(|_| {
                CastVoteError::SerializeVoterIdFailed("Unable to hash pseudonym".into())
            })?,
        ));
    }

    let electoral_log = get_electoral_log(&hasura_transaction, tenant_id, &election_event)
        .await
        .map_err(|e| CastVoteError::ElectoralLogNotFound(e.to_string()))?;

    // From this point on, we have all variables needed to do post_cat_vote_error
    let election_id_string = input.election_id.to_string();

    let ids = CastVoteIds {
        election_event_id,
        tenant_id,
        voter_id,
        area_id,
    };

    let voter_signing_policy = presentation
        .voter_signing_policy
        .clone()
        .unwrap_or_default();

    info!("voter signing policy {voter_signing_policy}");

    let area_presentation: AreaPresentation = match area.presentation {
        Some(presentation) => deserialize_value(presentation)
            .map_err(|e| CastVoteError::DeserializeAreaPresentationFailed(e.to_string()))?,
        None => AreaPresentation::default(),
    };
    let is_early_voting_area = area_presentation.is_early_voting();

    // Datafix votes are accepted under a per-voter lease that owns its own
    // connection/lock lifecycle (see `accept_datafix_ballot_locked`); ordinary
    // votes reuse this read transaction directly. Either way the connection is
    // released before a refusal is logged below. The sequencer appends an accepted
    // ballot's record to the log.
    let format = if is_multi_contest {
        BallotFormat::HashableMultiBallot
    } else {
        BallotFormat::HashableBallot
    };
    let ballot = BallotToAccept {
        input: &input,
        election_event: &election_event,
        voting_channel,
        ids,
        format,
        status: initial_status,
        auth_time,
        voter_ip,
        voter_country,
        username,
        user_agent,
        voter_signature_data: &voter_signature_data,
        is_early_voting_area,
        pseudonym_hash: &pseudonym_h,
        ballot_hash: &vote_h,
        received: received_cast.as_ref(),
    };
    let result = if initial_status == BallotStatus::Pending {
        // Release the read transaction and its connection before locking: the
        // txn borrows the client, and a voter blocked on the lease must not
        // pin a pool connection.
        hasura_transaction
            .commit()
            .await
            .map_err(|err| CastVoteError::CommitFailed(err.to_string()))?;
        drop(hasura_db_client);
        accept_datafix_ballot_locked(&ballot).await
    } else {
        let result = accept_into_ballot_box(hasura_transaction, &ballot).await;
        drop(hasura_db_client);
        result
    };

    let _audit_phase = CastVotePhase::start("audit");
    match (result, received_cast.as_ref()) {
        (Ok((accepted, _, receipt)), _) => classify_inserted_cast_vote(accepted)
            .map(|result| CastOutcome::Inserted { result, receipt }),
        // Another request cast this received ballot first: when it was this
        // same cast, the voter gets the receipt the ballot box signed then.
        (Err(CastVoteError::BallotAlreadyCast), Some(received)) => {
            match stored_cast(received).await? {
                Some(cast) => Ok(CastOutcome::AlreadyCast(cast)),
                None => Ok(CastOutcome::refused(CastVoteError::BallotAlreadyCast)),
            }
        }
        (Err(cast_vote_err), _) => {
            error!("Ballot acceptance refused");

            let log_result = if private_ballot {
                crate::services::vote_privacy::queue_rejection(
                    tenant_id,
                    election_event_id,
                    &election_id_string,
                    voter_id,
                )
                .await
            } else {
                electoral_log
                    .post_cast_vote_error(
                        tenant_id.to_string(),
                        election_event_id.to_string(),
                        Some(election_id_string),
                        pseudonym_h,
                        "Ballot acceptance refused".into(),
                        String::new(),
                        String::new(),
                        voter_id.to_string(),
                        username.clone(),
                        area_id.to_string(),
                    )
                    .await
            };
            if log_result.is_err() {
                error!("Could not queue ballot refusal audit");
            }

            skip_or_propagate(cast_vote_err).map(CastOutcome::refused_result)
        }
    }
}

/// The ballot tracker policy of the election event and the ballot styles a
/// ballot can be encrypted with to be accepted for the voter's area and
/// election.
pub async fn ballot_tracker_context(
    hasura_transaction: &Transaction<'_>,
    presentation: Option<&ElectionEventPresentation>,
    tenant_id: &str,
    election_event_id: &str,
    area_id: &str,
    election_id: &str,
) -> Result<(BallotTrackerPolicy, Vec<PublishedBallotStyleHash>), CastVoteError> {
    let ballot_tracker_policy = presentation
        .and_then(|presentation| presentation.ballot_tracker_policy.clone())
        .unwrap_or_default();
    let published_ballot_styles = get_published_ballot_emls(
        hasura_transaction,
        tenant_id,
        election_event_id,
        area_id,
        election_id,
    )
    .await
    .map_err(|e| CastVoteError::GetBallotStyleFailed(e.to_string()))?
    .iter()
    .map(|ballot_eml| PublishedBallotStyleHash::from_ballot_eml(ballot_eml))
    .collect::<Result<Vec<_>, _>>()?;
    Ok((ballot_tracker_policy, published_ballot_styles))
}

/// The id and hash of a ballot style a ballot can be encrypted with to be
/// accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedBallotStyleHash {
    pub id: String,
    pub hash: String,
}

impl PublishedBallotStyleHash {
    pub fn from_ballot_eml(ballot_eml: &str) -> Result<Self, CastVoteError> {
        let ballot_style: BallotStyle = deserialize_str(ballot_eml)
            .map_err(|e| CastVoteError::GetBallotStyleFailed(e.to_string()))?;
        let hash = hash_ballot_style(&ballot_style)
            .map_err(|e| CastVoteError::GetBallotStyleFailed(e.to_string()))?;
        Ok(PublishedBallotStyleHash {
            id: ballot_style.id,
            hash,
        })
    }
}

/// The tracker of a current ballot covers the ballot style it names, so that
/// style must be one published for the voter's area and election. A legacy
/// ballot's tracker does not cover it, and it is only accepted when the
/// election event's [`BallotTrackerPolicy`] allows it.
pub fn check_ballot_style_binding(
    version: u32,
    ballot_style_id: &str,
    ballot_style_hash: &str,
    ballot_tracker_policy: &BallotTrackerPolicy,
    published_ballot_styles: &[PublishedBallotStyleHash],
) -> Result<(), CastVoteError> {
    if version == LEGACY_TYPES_VERSION {
        return match ballot_tracker_policy {
            BallotTrackerPolicy::ALLOW_LEGACY => Ok(()),
            BallotTrackerPolicy::STYLE_BOUND => Err(CastVoteError::BallotStyleMismatch(format!(
                "Ballot version {version} is not accepted by the ballot tracker policy \
                 {ballot_tracker_policy}"
            ))),
        };
    }

    if published_ballot_styles
        .iter()
        .any(|published| published.id == ballot_style_id && published.hash == ballot_style_hash)
    {
        return Ok(());
    }
    Err(CastVoteError::BallotStyleMismatch(format!(
        "Ballot style {ballot_style_id} with hash {ballot_style_hash} is not published for this \
         election and area"
    )))
}

#[instrument(skip_all)]
pub fn deserialize_and_check_ballot(
    input: &InsertCastVoteInput,
    voter_id: &str,
    ballot_tracker_policy: &BallotTrackerPolicy,
    published_ballot_styles: &[PublishedBallotStyleHash],
) -> Result<
    (
        PseudonymHash,
        CastVoteHash,
        Option<(VerificationKey, Signature)>,
    ),
    CastVoteError,
> {
    let signed_hashable_ballot: SignedHashableBallot = deserialize_str(&input.content)
        .map_err(|e| CastVoteError::DeserializeBallotFailed(e.to_string()))?;

    let hashable_ballot: HashableBallot = (&signed_hashable_ballot)
        .try_into()
        .map_err(|e: BallotError| CastVoteError::DeserializeBallotFailed(e.to_string()))?;

    let computed_hash = hash_ballot(&hashable_ballot)
        .map_err(|e| CastVoteError::SerializeBallotFailed(e.to_string()))?;

    /// Verifies that the ballot_id corresponds to the hash of the ballot content
    /// The function serves as a security check to ensure that
    /// a ballot's content matches its claimed ID.
    /// This is crucial for maintaining the integrity of the voting system
    /// by preventing ballot tampering or substitution.
    if computed_hash != input.ballot_id {
        return Err(CastVoteError::BallotIdMismatch(format!(
            "Expected {} but got {}",
            computed_hash, input.ballot_id
        )));
    }

    check_ballot_style_binding(
        hashable_ballot.version,
        &hashable_ballot.config,
        &hashable_ballot.ballot_style_hash,
        ballot_tracker_policy,
        published_ballot_styles,
    )?;

    let pseudonym_hash_bytes = hash_voter_id(voter_id)
        .map_err(|e| CastVoteError::SerializeVoterIdFailed(e.to_string()))?;

    let vote_hash_bytes = hash_ballot_sha512(&hashable_ballot)
        .map_err(|e| CastVoteError::SerializeBallotFailed(e.to_string()))?;

    let pseudonym_h = PseudonymHash(HashWrapper::new(pseudonym_hash_bytes));
    let vote_h = CastVoteHash(HashWrapper::new(vote_hash_bytes));

    // Check ballot signature
    let election_id = input.election_id.to_string();
    let signature_opt = verify_ballot_signature(
        &input.ballot_id,
        &election_id,
        &signed_hashable_ballot,
    )
    .map_err(|err| {
        CastVoteError::BallotVoterSignatureFailed(format!("Ballot signature check failed: {err}"))
    })?;
    info!("is_signature_verified =  {}", signature_opt.is_some());

    Ok((pseudonym_h, vote_h, signature_opt))
}

#[instrument(skip_all)]
pub fn deserialize_and_check_multi_ballot(
    input: &InsertCastVoteInput,
    voter_id: &str,
    ballot_tracker_policy: &BallotTrackerPolicy,
    published_ballot_styles: &[PublishedBallotStyleHash],
) -> Result<
    (
        PseudonymHash,
        CastVoteHash,
        Option<(VerificationKey, Signature)>,
    ),
    CastVoteError,
> {
    let signed_hashable_multi_ballot: SignedHashableMultiBallot =
        deserialize_str(&input.content)
            .map_err(|e| CastVoteError::DeserializeBallotFailed(e.to_string()))?;

    let hashable_multi_ballot: HashableMultiBallot = (&signed_hashable_multi_ballot)
        .try_into()
        .map_err(|e: BallotError| CastVoteError::DeserializeBallotFailed(e.to_string()))?;

    let computed_hash = hash_multi_ballot(&hashable_multi_ballot)
        .map_err(|e| CastVoteError::SerializeBallotFailed(e.to_string()))?;

    /// Verifies that the ballot_id corresponds to the hash of the ballot content
    /// The function serves as a security check to ensure that
    /// a ballot's content matches its claimed ID.
    /// This is crucial for maintaining the integrity of the voting system
    /// by preventing ballot tampering or substitution.
    if computed_hash != input.ballot_id {
        return Err(CastVoteError::BallotIdMismatch(format!(
            "Expected {} but got {}",
            computed_hash, input.ballot_id
        )));
    }

    check_ballot_style_binding(
        hashable_multi_ballot.version,
        &hashable_multi_ballot.config,
        &hashable_multi_ballot.ballot_style_hash,
        ballot_tracker_policy,
        published_ballot_styles,
    )?;

    let pseudonym_hash_bytes = hash_voter_id(voter_id)
        .map_err(|e| CastVoteError::SerializeVoterIdFailed(e.to_string()))?;

    let vote_hash_bytes = hash_multi_ballot_sha512(&hashable_multi_ballot)
        .map_err(|e| CastVoteError::SerializeBallotFailed(e.to_string()))?;

    let pseudonym_h = PseudonymHash(HashWrapper::new(pseudonym_hash_bytes));
    let vote_h = CastVoteHash(HashWrapper::new(vote_hash_bytes));

    // Check ballot signature
    let election_id = input.election_id.to_string();
    let voter_signature_opt = verify_multi_ballot_signature(
        &input.ballot_id,
        &election_id,
        &signed_hashable_multi_ballot,
    )
    .map_err(|err| {
        CastVoteError::BallotVoterSignatureFailed(format!("Ballot signature check failed: {err}"))
    })?;
    info!("is_signature_verified =  {}", voter_signature_opt.is_some());

    Ok((pseudonym_h, vote_h, voter_signature_opt))
}

/// A vote to accept into the ballot box, once its ballot has been checked.
struct BallotToAccept<'a> {
    input: &'a InsertCastVoteInput,
    election_event: &'a ElectionEvent,
    voting_channel: VotingStatusChannel,
    ids: CastVoteIds<'a>,
    format: BallotFormat,
    status: BallotStatus,
    auth_time: &'a Option<i64>,
    voter_ip: &'a Option<String>,
    voter_country: &'a Option<String>,
    username: &'a Option<String>,
    user_agent: &'a Option<String>,
    voter_signature_data: &'a Option<(VerificationKey, Signature)>,
    is_early_voting_area: bool,
    pseudonym_hash: &'a PseudonymHash,
    ballot_hash: &'a CastVoteHash,
    /// The received ballot it casts, under the receipts policy.
    received: Option<&'a ReceivedCast>,
}

/// Accept a vote into the event's ballot box, after checking that its election
/// accepts votes through its channel. The ballot box enforces the revote limit and
/// cross-area exclusivity.
#[instrument(skip_all)]
async fn accept_into_ballot_box(
    hasura_transaction: Transaction<'_>,
    ballot: &BallotToAccept<'_>,
) -> Result<Accepted, CastVoteError> {
    let BallotToAccept {
        input,
        election_event,
        voting_channel,
        ids,
        format,
        status,
        auth_time,
        voter_ip,
        voter_country,
        username,
        user_agent,
        voter_signature_data,
        is_early_voting_area,
        pseudonym_hash,
        ballot_hash,
        received,
    } = *ballot;
    let election_id = input.election_id.to_string();
    let area_uuid = parse_uuid_v4(ids.area_id)
        .map_err(|e| CastVoteError::UuidParseFailed(e.to_string(), "area_id".to_string()))?;
    let effective_voting_channel = check_status(
        ids.tenant_id,
        ids.election_event_id,
        &election_id,
        &area_uuid,
        &hasura_transaction,
        election_event,
        auth_time,
        voting_channel,
        is_early_voting_area,
    )
    .await?;
    let allowed_votes = get_allowed_votes(
        &hasura_transaction,
        ids.tenant_id,
        ids.election_event_id,
        &election_id,
    )
    .await
    .map_err(|e| CastVoteError::CheckRevotesFailed(e.to_string()))?;
    hasura_transaction
        .commit()
        .await
        .map_err(|e| CastVoteError::CommitFailed(e.to_string()))?;

    let voter_signature: Option<Vec<u8>> = voter_signature_data
        .as_ref()
        .map(|(_, signature)| signature.to_bytes().to_vec());
    let pseudonym_hash = pseudonym_hash.0.clone().to_inner();
    let ballot_hash = ballot_hash.0.clone().to_inner();
    let format = format.to_string();
    let channel = effective_voting_channel.to_string();
    let accept = AcceptBallot {
        election_event_id: ids.election_event_id,
        election_id: &election_id,
        area_id: ids.area_id,
        voter_id: ids.voter_id,
        ballot_id: &input.ballot_id,
        format: &format,
        content: &input.content,
        voter_signature: voter_signature.as_deref(),
        pseudonym_hash: &pseudonym_hash,
        ballot_hash: &ballot_hash,
        voting_channel: &channel,
        status,
        voter_ip: None,
        voter_country: None,
        username: username.as_deref(),
        allowed_votes,
    };
    let _phase = CastVotePhase::start("accept");
    let (outcome, receipt) = match received {
        Some(received) => {
            let forensic = election_event
                .get_presentation()
                .map_err(|_| CastVoteError::CheckStatusFailed("Invalid privacy policy".into()))?
                .unwrap_or_default()
                .network_data_policy
                .unwrap_or_default();
            let network = if crate::services::vote_privacy::identity_mode(election_event)
                .map_err(|_| CastVoteError::BallotSignFailed("Invalid privacy policy".into()))?
                == sequent_core::types::vote_privacy::BallotIdentity::Pseudonym
                && forensic == sequent_core::types::vote_privacy::NetworkDataPolicy::ForensicStore
            {
                voter_ip
                    .as_deref()
                    .map(|ip| (ip, voter_country.as_deref(), user_agent.as_deref()))
            } else {
                None
            };
            let (outcome, cast_at, receipt) =
                crate::services::cast_ballot::accept_received_cast_with_network(
                    &accept, received, network,
                )
                .await?;
            (outcome, Some((cast_at, receipt)))
        }
        None => (
            accept_ballot(&accept)
                .await
                .map_err(|e| CastVoteError::InsertFailed(format!("{e:#}")))?,
            None,
        ),
    };
    match outcome {
        AcceptOutcome::Accepted { id, .. } => {
            let accepted_at = receipt
                .as_ref()
                .map(|(cast_at, _)| *cast_at)
                .unwrap_or_else(chrono::Utc::now);
            Ok((
                CastVote {
                    id,
                    tenant_id: ids.tenant_id.to_string(),
                    election_id: Some(election_id),
                    area_id: Some(ids.area_id.to_string()),
                    created_at: Some(accepted_at),
                    last_updated_at: Some(accepted_at),
                    content: Some(input.content.clone()),
                    voter_id_string: Some(ids.voter_id.to_string()),
                    election_event_id: ids.election_event_id.to_string(),
                    ballot_id: Some(input.ballot_id.clone()),
                    cast_ballot_signature: voter_signature,
                    status: cast_vote_status(status),
                },
                effective_voting_channel,
                receipt.map(|(_, receipt)| receipt),
            ))
        }
        AcceptOutcome::TooManyVotes => Err(CastVoteError::InsertFailedExceedsAllowedRevotes),
        AcceptOutcome::VotedInOtherArea => Err(CastVoteError::CheckVotesInOtherAreasFailed(
            "The voter already voted in another area".to_string(),
        )),
        AcceptOutcome::DuplicateBallotId if received.is_some() => {
            Err(CastVoteError::BallotAlreadyCast)
        }
        AcceptOutcome::DuplicateBallotId => Err(CastVoteError::InsertFailed(
            "The ballot ID was already used in this election event".to_string(),
        )),
        AcceptOutcome::BallotBoxSealed => Err(CastVoteError::CheckStatusFailed(
            crate::services::ballot_box_seal::BALLOT_BOX_SEALED_MESSAGE.to_string(),
        )),
        AcceptOutcome::NotReceivable => Err(CastVoteError::BallotNotReceived),
    }
}

/// A voter's pseudonym, stored with the voter's ballots and in their records.
pub fn hash_voter_id(voter_id: &str) -> Result<Hash> {
    let bytes = voter_id.to_string().record_serialize()?;
    hash_to_array(&bytes)
}

#[instrument(skip_all)]
async fn get_electoral_log(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event: &ElectionEvent,
) -> anyhow::Result<ElectoralLog> {
    let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
        .with_context(|| "missing bulletin board")?;

    let protocol_manager = get_board_signing_key(
        hasura_transaction,
        tenant_id,
        Some(&election_event.id),
        &board_name,
    )
    .await?;
    Ok(ElectoralLog::for_system_with_signing_key(
        board_name.as_str(),
        &protocol_manager,
    ))
}

fn effective_voting_channel_for_status(
    voting_channel: VotingStatusChannel,
    is_early_voting_area: bool,
    election_status: &ElectionStatus,
) -> VotingStatusChannel {
    let allow_early_voting = voting_channel == VotingStatusChannel::ONLINE
        && is_early_voting_area
        && election_status.status_by_channel(VotingStatusChannel::EARLY_VOTING)
            == VotingStatus::OPEN
        && election_status.status_by_channel(VotingStatusChannel::ONLINE)
            == VotingStatus::NOT_STARTED;

    if allow_early_voting {
        VotingStatusChannel::EARLY_VOTING
    } else {
        voting_channel
    }
}

/// Non-online signed deadlines remain binding while the scheduler is delayed.
/// Existing grace policy applies only to ONLINE; other channels stop at the instant.
fn check_signed_channel_deadline(
    now: DateTime<Local>,
    channel: VotingStatusChannel,
    signed_close: Option<&str>,
) -> Result<(), CastVoteError> {
    if channel == VotingStatusChannel::ONLINE {
        return Ok(()); // The authoritative ONLINE bound is merged into dates.end_date.
    }
    if let Some(signed_close) = signed_close {
        let close = ISO8601::to_date(signed_close).map_err(|error| {
            CastVoteError::CheckStatusInternalFailed(format!(
                "Invalid signed closing date: {error}"
            ))
        })?;
        if now >= close {
            return Err(CastVoteError::CheckStatusFailed(
                "The signed closing deadline for this voting channel has passed".to_string(),
            ));
        }
    }
    Ok(())
}

/// Applies the existing vote-acceptance policy after `check_status` has loaded
/// the election state. The requested channel continues to drive status, date,
/// and grace-period checks; the effective channel is derived only after the
/// vote has passed those checks so channel persistence cannot broaden access.
fn check_status_with_loaded_election(
    now: DateTime<Local>,
    auth_time_local: DateTime<Local>,
    voting_channel: VotingStatusChannel,
    is_early_voting_area: bool,
    mut dates: VotingPeriodDates,
    election_status: &ElectionStatus,
    election_presentation: &ElectionPresentation,
    election_id: &str,
) -> Result<VotingStatusChannel, CastVoteError> {
    if voting_channel != VotingStatusChannel::ONLINE {
        dates.end_date = None;
    }

    let close_date_esq_event_opt: Option<DateTime<Local>> =
        if let Some(end_date_str) = dates.end_date {
            match ISO8601::to_date(&end_date_str) {
                Ok(close_date) => {
                    info!("Parsed end_date: {}", close_date);
                    Some(close_date)
                }
                Err(err) => {
                    info!("Failed to parse end_date: {}", err);
                    None
                }
            }
        } else {
            None
        };

    let current_voting_status = election_status.status_by_channel(voting_channel);
    let dates_by_channel = election_status.dates_by_channel(voting_channel);
    // The seal deadline uses the same grace period (VOTE-FREEZE).
    let grace_period =
        crate::services::ballot_box_seal::deadline::grace_period(election_presentation);
    let apply_grace_period = grace_period.is_some()
        && voting_channel == VotingStatusChannel::ONLINE
        && current_voting_status != VotingStatus::PAUSED;
    let grace_period_duration = grace_period.unwrap_or_else(Duration::zero);

    if let Some(close_date_esq_event) = close_date_esq_event_opt {
        let close_date_plus_grace_period = close_date_esq_event + grace_period_duration;

        if apply_grace_period {
            if now > close_date_plus_grace_period || auth_time_local > close_date_esq_event {
                return Err(CastVoteError::CheckStatusFailed(
                    "Cannot vote outside grace period".to_string(),
                ));
            }

            if now <= close_date_esq_event && current_voting_status != VotingStatus::OPEN {
                return Err(CastVoteError::CheckStatusFailed(
                    format!("Election voting status is not open (={current_voting_status:?}) while voting before the closing date of the election"),
                ));
            }
        } else {
            if now > close_date_esq_event {
                return Err(CastVoteError::CheckStatusFailed(
                    "Election close date passed and grace period does not apply or is not set"
                        .to_string(),
                ));
            }

            if current_voting_status != VotingStatus::OPEN {
                return Err(CastVoteError::CheckStatusFailed(format!(
                    "Election Voting Status for voting_channel={voting_channel:?} is {current_voting_status:?} instead of Open and grace_period_policy does not apply or is not set"
                )));
            }
        }
    } else {
        // Preserve the pre-ticket acceptance rule: this exception is only
        // consulted when there is no configured online close date.
        let allow_early_voting = is_early_voting_area
            && election_status.status_by_channel(VotingStatusChannel::EARLY_VOTING)
                == VotingStatus::OPEN
            && election_status.status_by_channel(VotingStatusChannel::ONLINE)
                == VotingStatus::NOT_STARTED;
        let last_stopped_at = dates_by_channel
            .last_stopped_at
            .map(|val| val.with_timezone(&Local));
        let allow_grace_period_voting = match last_stopped_at {
            Some(close_date) => {
                apply_grace_period
                    && now < close_date + grace_period_duration
                    && auth_time_local < close_date
            }
            None => false,
        };

        match current_voting_status {
            VotingStatus::NOT_STARTED if allow_early_voting => {}
            VotingStatus::NOT_STARTED | VotingStatus::PAUSED => {
                return Err(CastVoteError::CheckStatusFailed(format!(
                    "Voting Status for voting_channel={voting_channel:?} is {current_voting_status:?}"
                )));
            }
            VotingStatus::OPEN => {
                debug!("Allowing cast vote for election id {election_id}");
            }
            VotingStatus::CLOSED if allow_grace_period_voting => {
                info!("Allowing grace period vote at {now}");
            }
            VotingStatus::CLOSED => {
                return Err(CastVoteError::CheckStatusFailed(format!(
                    "Voting Status for voting_channel={voting_channel:?} is {current_voting_status:?}"
                )));
            }
        }
    }

    let effective_voting_channel =
        effective_voting_channel_for_status(voting_channel, is_early_voting_area, election_status);
    if effective_voting_channel != voting_channel {
        debug!("Allowing early voting for election id {election_id}");
    }
    Ok(effective_voting_channel)
}

/// Missing presentation uses defaults; malformed configured policy must not
/// silently become a different grace-period policy.
fn parse_election_presentation(
    presentation: Option<serde_json::Value>,
) -> Result<ElectionPresentation, CastVoteError> {
    presentation
        .map(|value| {
            deserialize_value(value).context("Failed to deserialize election presentation")
        })
        .transpose()
        .map(|value| value.unwrap_or_default())
        .map_err(|error| CastVoteError::CheckStatusInternalFailed(error.to_string()))
}

fn parse_voter_auth_time(auth_time: Option<i64>) -> Result<DateTime<Local>, CastVoteError> {
    let parsed = if let Some(auth_time_int) = auth_time {
        if let Ok(auth_time_parsed) = ISO8601::timestamp_secs_utc_to_date_opt(auth_time_int) {
            auth_time_parsed
        } else {
            return Err(CastVoteError::CheckStatusFailed(
                "Invalid auth_time timestamp".to_string(),
            ));
        }
    } else {
        return Err(CastVoteError::CheckStatusFailed(
            "auth_time is not a valid integer".to_string(),
        ));
    };
    Ok(parsed)
}

#[instrument(skip_all)]
pub(crate) async fn check_status(
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    area_id: &Uuid,
    hasura_transaction: &Transaction<'_>,
    election_event: &ElectionEvent,
    auth_time: &Option<i64>,
    voting_channel: VotingStatusChannel,
    is_early_voting_area: bool,
) -> Result<VotingStatusChannel, CastVoteError> {
    let _phase = CastVotePhase::start("check_status");
    if election_event.is_archived {
        return Err(CastVoteError::CheckStatusFailed(
            "Election event is archived".to_string(),
        ));
    }
    let now = ISO8601::now();

    let auth_time_local = parse_voter_auth_time(*auth_time)?;

    // Always read the writer: a TTL alone cannot invalidate an administrative
    // close, channel change, or reschedule. One narrow row keeps those updates
    // visible without transferring election EML or unrelated scheduled tasks.
    let CastVoteConfiguration {
        presentation,
        status,
        voting_channels,
        dates,
        signed_close_dates,
    } = get_cast_vote_configuration(
        hasura_transaction,
        tenant_id,
        election_event_id,
        election_id,
    )
    .await
    .map_err(|e| CastVoteError::CheckStatusInternalFailed(e.to_string()))?;
    let election_presentation = parse_election_presentation(presentation)?;

    let election_status: ElectionStatus = status
        .map(|value| deserialize_value(value).context("Failed to deserialize election status"))
        .transpose()
        .map(|value| value.unwrap_or_default())
        .map_err(|e| CastVoteError::CheckStatusInternalFailed(e.to_string()))?;

    let election_voting_channels: VotingChannels = voting_channels
        .map(|value| {
            deserialize_value(value).context("Failed to deserialize election voting_channels")
        })
        .transpose()
        .map(|value| value.unwrap_or_default())
        .map_err(|e| CastVoteError::CheckStatusInternalFailed(e.to_string()))?;

    // we check that the voting channel coming from the JWT is enabled in this
    // election
    if voting_channel.channel_from(&election_voting_channels) != Some(true) {
        return Err(CastVoteError::VotingChannelNotEnabled(format!(
            "Voting Channel {voting_channel:?} is not enabled in the election"
        )));
    }

    check_signed_channel_deadline(
        now,
        voting_channel,
        signed_close_dates.get(&voting_channel).map(String::as_str),
    )?;
    let effective_channel =
        effective_voting_channel_for_status(voting_channel, is_early_voting_area, &election_status);
    if effective_channel != voting_channel {
        check_signed_channel_deadline(
            now,
            effective_channel,
            signed_close_dates
                .get(&effective_channel)
                .map(String::as_str),
        )?;
    }

    check_seal_deadline(
        hasura_transaction,
        election_event,
        election_id,
        area_id,
        now,
        voting_channel,
        &election_status,
        &election_voting_channels,
        &election_presentation,
    )
    .await?;

    check_status_with_loaded_election(
        now,
        auth_time_local,
        voting_channel,
        is_early_voting_area,
        dates,
        &election_status,
        &election_presentation,
        election_id,
    )
}

/// With the Ballot Box Seal Policy set to Seal at close, voting into a
/// ballot box ends at its seal deadline: the one its seal row fixed, else
/// the one the election's status gives now (VOTE-FREEZE). This keeps the
/// cast check and the seal on the same deadline, also when the grace
/// period of an end date would run later. Policy off: no check.
#[allow(clippy::too_many_arguments)]
pub async fn check_seal_deadline(
    hasura_transaction: &Transaction<'_>,
    election_event: &ElectionEvent,
    election_id: &str,
    area_id: &Uuid,
    now: DateTime<Local>,
    voting_channel: VotingStatusChannel,
    election_status: &ElectionStatus,
    election_voting_channels: &VotingChannels,
    election_presentation: &ElectionPresentation,
) -> Result<(), CastVoteError> {
    use crate::services::ballot_box_seal::{
        box_grace_deadline, deadline::seal_deadline, seal_policy,
    };
    if seal_policy(election_event) != sequent_core::ballot::BallotBoxSealPolicy::SEAL_AT_CLOSE {
        return Ok(());
    }
    let parse = |id: &str| {
        Uuid::parse_str(id).map_err(|e| CastVoteError::CheckStatusInternalFailed(e.to_string()))
    };
    let deadline = match box_grace_deadline(
        hasura_transaction,
        &parse(&election_event.tenant_id)?,
        &parse(&election_event.id)?,
        &parse(election_id)?,
        area_id,
    )
    .await
    .map_err(|e| CastVoteError::CheckStatusInternalFailed(e.to_string()))?
    {
        Some(deadline) => Some(deadline),
        None => seal_deadline(
            election_status,
            election_voting_channels,
            election_presentation,
        )
        .map(|(_, deadline)| deadline),
    };
    match deadline {
        Some(deadline) if now.with_timezone(&chrono::Utc) >= deadline => {
            Err(CastVoteError::CheckStatusFailed(format!(
                "Voting Status for voting_channel={voting_channel:?} is {:?}: the ballot box's seal deadline has passed",
                VotingStatus::CLOSED
            )))
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
#[path = "insert_cast_vote_tests.rs"]
mod tests;
