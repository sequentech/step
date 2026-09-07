// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres;
use crate::postgres::area::get_area_by_id;
use crate::postgres::election::{get_cast_vote_configuration, CastVoteConfiguration};
use crate::postgres::election_event::get_election_event_by_id;
use crate::services::cast_votes::{CastVote, CastVoteStatus};
use crate::services::database::get_hasura_pool;
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log::ElectoralLog;
use crate::services::external::utils::{
    datafix_annotations, external_voter_lock_key, is_datafix_election_event,
    DATAFIX_VOTER_LOCK_SECS,
};
use crate::services::pg_lock::PgLock;
use crate::services::protocol_manager::get_protocol_manager;
use anyhow::{anyhow, Context, Result};
use b4::messages::message::Signer;
use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Duration, Local};
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::*;
use sequent_core::ballot::verify_ballot_signature;
use sequent_core::ballot::ContestEncryptionPolicy;
use sequent_core::ballot::EGracePeriodPolicy;
use sequent_core::ballot::{
    AreaPresentation, EarlyVotingPolicy, ElectionPresentation, ElectionStatus, VoterSigningPolicy,
    VotingPeriodDates, VotingStatus, VotingStatusChannel,
};
use sequent_core::ballot::{HashableBallot, HashableBallotContest, SignedHashableBallot};
use sequent_core::encrypt::hash_ballot;
use sequent_core::encrypt::hash_ballot_sha512;
use sequent_core::encrypt::hash_multi_ballot;
use sequent_core::encrypt::hash_multi_ballot_sha512;
use sequent_core::encrypt::DEFAULT_PLAINTEXT_LABEL;
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
use strand::backend::ristretto::RistrettoCtx;
use strand::hash::{hash_to_array, Hash, HashWrapper};
use strand::serialization::StrandSerialize;
use strand::signature::StrandSignature;
use strand::signature::StrandSignaturePk;
use strand::signature::StrandSignatureSk;
use strand::util::StrandError;
use strand::zkp::Zkp;
use strum_macros::Display;
use tracing::{debug, error, info, instrument, trace};
use uuid::Uuid;

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

/// Maps a freshly inserted row to its `InsertCastVoteResult` from the persisted
/// status: `in-progress` still needs Datafix processing, `valid` is final. Any
/// other status is unreachable for a new insert and surfaces as an error.
#[instrument(skip_all, err)]
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

/// Decides the status a new vote is inserted with: Datafix events start
/// `in-progress` so the async pipeline can confirm eligibility, ordinary events
/// start `valid`. Fails closed if the Datafix configuration is malformed.
#[instrument(skip_all, err)]
fn initial_cast_vote_status(
    election_event: &ElectionEvent,
) -> Result<CastVoteStatus, CastVoteError> {
    match datafix_annotations(election_event) {
        Ok(Some(_)) => Ok(CastVoteStatus::InProgress),
        Ok(None) => Ok(CastVoteStatus::Valid),
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

/// Inserts a Datafix vote under the per-voter lease, owning the whole lock and
/// connection lifecycle so `try_insert_cast_vote` stays free of it.
///
/// The caller must already have released its read transaction and connection (a
/// `Transaction` borrows its `Client`, so the commit/drop can't move in here) —
/// this is also required for pool safety: no connection is held while blocking on
/// the lease. This acquires the `(tenant, event, voter)` lease on a fresh
/// connection, inserts + commits, and always releases the lease before
/// returning.
#[instrument(skip_all, err)]
#[allow(clippy::too_many_arguments)]
async fn insert_datafix_cast_vote_locked<'a>(
    input: InsertCastVoteInput,
    election_event: ElectionEvent,
    voting_channel: VotingStatusChannel,
    ids: CastVoteIds<'a>,
    signing_key: StrandSignatureSk,
    auth_time: &Option<i64>,
    voter_ip: &Option<String>,
    voter_country: &Option<String>,
    voter_signature_data: &Option<(StrandSignaturePk, StrandSignature)>,
    is_early_voting_area: bool,
    initial_status: CastVoteStatus,
) -> Result<(CastVote, VotingStatusChannel), CastVoteError> {
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

    let result = insert_cast_vote_and_commit(
        input,
        hasura_transaction,
        election_event,
        voting_channel,
        ids,
        signing_key,
        auth_time,
        voter_ip,
        voter_country,
        voter_signature_data,
        is_early_voting_area,
        initial_status,
    )
    .await;

    drop(hasura_db_client);
    release_datafix_voter_lock(lock).await;
    result
}

/// Maps a post-insert error to the caller's retry contract: an exceeded revote
/// limit is terminal and surfaced as `SkipRetryFailure`, every other error
/// propagates for the normal retry path.
#[instrument]
fn skip_or_propagate(cast_vote_err: CastVoteError) -> Result<InsertCastVoteResult, CastVoteError> {
    match cast_vote_err {
        CastVoteError::InsertFailedExceedsAllowedRevotes
        | CastVoteError::CheckVotesInOtherAreasFailed(_) => {
            Ok(InsertCastVoteResult::SkipRetryFailure(cast_vote_err))
        }
        _ => Err(cast_vote_err),
    }
}

#[derive(Debug)]
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
    #[serde(rename = "uuid_parse_failed")]
    UuidParseFailed(String, String),
    #[serde(rename = "ballot_id_mismatch")]
    #[strum(to_string = "ballot_id_mismatch")]
    BallotIdMismatch(String),
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

#[instrument(skip(input), err)]
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

    let initial_status = initial_cast_vote_status(&election_event)?;

    let presentation_opt = election_event
        .get_presentation()
        .map_err(|e| CastVoteError::ElectionEventNotFound(e.to_string()))?;

    let is_multi_contest = if let Some(presentation) = presentation_opt.clone() {
        presentation.contest_encryption_policy == Some(ContestEncryptionPolicy::MULTIPLE_CONTESTS)
    } else {
        false
    };

    let hash_result = if is_multi_contest {
        deserialize_and_check_multi_ballot(&input, voter_id)
    } else {
        deserialize_and_check_ballot(&input, voter_id)
    };

    let (pseudonym_h, vote_h, voter_signature_data) = match hash_result {
        Ok(hash) => hash,
        Err(cv_err) => {
            return Ok(InsertCastVoteResult::SkipRetryFailure(cv_err));
        }
    };

    let (electoral_log, signing_key) =
        get_electoral_log(&hasura_transaction, tenant_id, &election_event)
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

    let voter_signing_policy = election_event
        .get_presentation()
        .map_err(|e| CastVoteError::ElectionEventNotFound(e.to_string()))?
        .unwrap_or_default()
        .voter_signing_policy
        .unwrap_or_default();

    info!("voter signing policy {voter_signing_policy}");

    let area_presentation: AreaPresentation = match area.presentation {
        Some(presentation) => deserialize_value(presentation)
            .map_err(|e| CastVoteError::DeserializeAreaPresentationFailed(e.to_string()))?,
        None => AreaPresentation::default(),
    };
    let is_early_voting_area = area_presentation.is_early_voting();

    // Audit delivery remains post-commit and best-effort, as before. Reuse the
    // authenticated request's username and the already loaded signing key so
    // audit preparation needs neither Keycloak nor a second database checkout.
    let voter_electoral_log = ElectoralLog::for_voter_with_signing_key(
        &electoral_log.elog_database,
        voter_id,
        &signing_key,
    );

    // Datafix votes are inserted under a per-voter lease that owns its own
    // connection/lock lifecycle (see `insert_datafix_cast_vote_locked`); ordinary
    // votes reuse this read transaction directly. Either way the connection is
    // released before the audit below, which only enqueues its signed message.
    let result = match initial_status {
        CastVoteStatus::InProgress => {
            // Release the read transaction and its connection before locking: the
            // txn borrows the client, and a voter blocked on the lease must not
            // pin a pool connection.
            hasura_transaction
                .commit()
                .await
                .map_err(|err| CastVoteError::CommitFailed(err.to_string()))?;
            drop(hasura_db_client);
            insert_datafix_cast_vote_locked(
                input,
                election_event.clone(),
                voting_channel,
                ids,
                signing_key,
                auth_time,
                voter_ip,
                voter_country,
                &voter_signature_data,
                is_early_voting_area,
                initial_status,
            )
            .await
        }
        _ => {
            let result = insert_cast_vote_and_commit(
                input,
                hasura_transaction,
                election_event.clone(),
                voting_channel,
                ids,
                signing_key,
                auth_time,
                voter_ip,
                voter_country,
                &voter_signature_data,
                is_early_voting_area,
                initial_status,
            )
            .await;
            drop(hasura_db_client);
            result
        }
    };

    let ip = format!("ip: {}", voter_ip.as_deref().unwrap_or(""),);
    let country = format!("country: {}", voter_country.as_deref().unwrap_or(""),);
    let _audit_phase = CastVotePhase::start("audit");
    match result {
        Ok((inserted_cast_vote, effective_voting_channel)) => {
            let log_result = voter_electoral_log
                .post_cast_vote(
                    tenant_id.to_string(),
                    election_event_id.to_string(),
                    Some(election_id_string),
                    pseudonym_h,
                    vote_h,
                    ip,
                    country,
                    voter_id.to_string(),
                    username.clone(),
                    area_id.to_string().clone(),
                    effective_voting_channel.to_string(),
                )
                .await;
            if let Err(log_err) = log_result {
                error!("Error posting to the electoral log {:?}", log_err);
            }
            classify_inserted_cast_vote(inserted_cast_vote)
        }
        Err(cast_vote_err) => {
            error!(err=?cast_vote_err);

            let log_result = electoral_log
                .post_cast_vote_error(
                    tenant_id.to_string(),
                    election_event_id.to_string(),
                    Some(election_id_string),
                    pseudonym_h,
                    cast_vote_err.to_string(),
                    ip,
                    country,
                    voter_id.to_string(),
                    username.clone(),
                    area_id.to_string().clone(),
                )
                .await;

            if let Err(log_err) = log_result {
                error!("Error posting error to the electoral log {:?}", log_err);
            }

            skip_or_propagate(cast_vote_err)
        }
    }
}

#[instrument(skip(input), err)]
pub fn deserialize_and_check_ballot(
    input: &InsertCastVoteInput,
    voter_id: &str,
) -> Result<
    (
        PseudonymHash,
        CastVoteHash,
        Option<(StrandSignaturePk, StrandSignature)>,
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

    let pseudonym_hash_bytes = hash_voter_id(voter_id)
        .map_err(|e| CastVoteError::SerializeVoterIdFailed(e.to_string()))?;

    let vote_hash_bytes = hash_ballot_sha512(&hashable_ballot)
        .map_err(|e| CastVoteError::SerializeBallotFailed(e.to_string()))?;

    let pseudonym_h = PseudonymHash(HashWrapper::new(pseudonym_hash_bytes));
    let vote_h = CastVoteHash(HashWrapper::new(vote_hash_bytes));

    let hashable_ballot_contests = hashable_ballot
        .deserialize_contests()
        .map_err(|e| CastVoteError::DeserializeContestsFailed(e.to_string()))?;

    hashable_ballot_contests
        .iter()
        .map(check_popk)
        .collect::<Result<Vec<()>>>()
        .map_err(|e| CastVoteError::PokValidationFailed(e.to_string()))?;

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

#[instrument(skip(input), err)]
pub fn deserialize_and_check_multi_ballot(
    input: &InsertCastVoteInput,
    voter_id: &str,
) -> Result<
    (
        PseudonymHash,
        CastVoteHash,
        Option<(StrandSignaturePk, StrandSignature)>,
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

    let pseudonym_hash_bytes = hash_voter_id(voter_id)
        .map_err(|e| CastVoteError::SerializeVoterIdFailed(e.to_string()))?;

    let vote_hash_bytes = hash_multi_ballot_sha512(&hashable_multi_ballot)
        .map_err(|e| CastVoteError::SerializeBallotFailed(e.to_string()))?;

    let pseudonym_h = PseudonymHash(HashWrapper::new(pseudonym_hash_bytes));
    let vote_h = CastVoteHash(HashWrapper::new(vote_hash_bytes));

    let hashable_multi_ballot_contests = hashable_multi_ballot
        .deserialize_contests()
        .map_err(|e| CastVoteError::DeserializeContestsFailed(e.to_string()))?;

    check_popk_multi(&hashable_multi_ballot_contests)
        .map_err(|e| CastVoteError::PokValidationFailed(e.to_string()))?;

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

#[instrument(
    skip(
        input,
        hasura_transaction,
        election_event,
        signing_key,
        voter_signature_data
    ),
    err
)]
pub async fn insert_cast_vote_and_commit<'a>(
    input: InsertCastVoteInput,
    hasura_transaction: Transaction<'_>,
    election_event: ElectionEvent,
    voting_channel: VotingStatusChannel,
    ids: CastVoteIds<'a>,
    signing_key: StrandSignatureSk,
    auth_time: &Option<i64>,
    voter_ip: &Option<String>,
    voter_country: &Option<String>,
    voter_signature_data: &Option<(StrandSignaturePk, StrandSignature)>,
    is_early_voting_area: bool,
    initial_status: CastVoteStatus,
) -> Result<(CastVote, VotingStatusChannel), CastVoteError> {
    let election_id_string = input.election_id.to_string();
    let election_id = election_id_string.as_str();
    let tenant_uuid = parse_uuid_v4(ids.tenant_id)
        .map_err(|e| CastVoteError::UuidParseFailed(e.to_string(), "tenant_id".to_string()))?;
    let election_event_uuid = parse_uuid_v4(ids.election_event_id).map_err(|e| {
        CastVoteError::UuidParseFailed(e.to_string(), "election_event_id".to_string())
    })?;
    let election_uuid = parse_uuid_v4(election_id)
        .map_err(|e| CastVoteError::UuidParseFailed(e.to_string(), "election_id".to_string()))?;
    let area_uuid = parse_uuid_v4(ids.area_id)
        .map_err(|e| CastVoteError::UuidParseFailed(e.to_string(), "area_id".to_string()))?;
    // The database trigger enforces both revote limits and cross-area
    // exclusivity under the same per-voter lock, including in-progress votes.
    let effective_voting_channel = check_status(
        ids.tenant_id,
        ids.election_event_id,
        election_id,
        &hasura_transaction,
        &election_event,
        auth_time,
        voting_channel,
        is_early_voting_area,
    )
    .await?;

    let voter_signature = voter_signature_data.clone().map(|val| val.1);

    let ballot_signature: [u8; 64] = voter_signature
        .map(|signature| signature.to_bytes())
        .unwrap_or([0u8; 64]);

    let insert_phase = CastVotePhase::start("insert");
    let insert = postgres::cast_vote::insert_cast_vote(
        &hasura_transaction,
        &tenant_uuid,
        &election_event_uuid,
        &election_uuid,
        &area_uuid,
        &input.content,
        ids.voter_id,
        &input.ballot_id,
        &ballot_signature,
        voter_ip,
        voter_country,
        effective_voting_channel,
        initial_status,
    );

    let cast_vote = insert.await.map_err(map_insert_error)?;

    drop(insert_phase);
    let _commit_phase = CastVotePhase::start("commit");
    // Keep INSERT last before COMMIT: its per-voter advisory lock must not
    // cover audit delivery or unrelated queries.
    hasura_transaction
        .commit()
        .await
        .map_err(|e| CastVoteError::CommitFailed(e.to_string()))?;

    Ok((cast_vote, effective_voting_channel))
}

pub(crate) fn hash_voter_id(voter_id: &str) -> Result<Hash, StrandError> {
    let bytes = voter_id.to_string().strand_serialize()?;
    hash_to_array(&bytes)
}

#[instrument(skip_all, err)]
async fn get_electoral_log(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event: &ElectionEvent,
) -> anyhow::Result<(ElectoralLog, StrandSignatureSk)> {
    let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
        .with_context(|| "missing bulletin board")?;

    let protocol_manager = get_protocol_manager::<RistrettoCtx>(
        hasura_transaction,
        tenant_id,
        Some(&election_event.id),
        &board_name,
    )
    .await?;
    let sk = protocol_manager.get_signing_key();

    let electoral_log = ElectoralLog::for_voter_with_signing_key(board_name.as_str(), "", sk);
    Ok((electoral_log, sk.clone()))
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
    let grace_period_secs = election_presentation.grace_period_secs.unwrap_or(0);
    let grace_period_policy = election_presentation
        .grace_period_policy
        .clone()
        .unwrap_or(EGracePeriodPolicy::NO_GRACE_PERIOD);
    let apply_grace_period = grace_period_policy != EGracePeriodPolicy::NO_GRACE_PERIOD
        && voting_channel == VotingStatusChannel::ONLINE
        && current_voting_status != VotingStatus::PAUSED;
    let grace_period_duration = Duration::seconds(grace_period_secs as i64);

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

#[instrument(skip_all, err)]
async fn check_status(
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
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

    let auth_time_local: DateTime<Local> = if let Some(auth_time_int) = *auth_time {
        if let Ok(auth_time_parsed) = ISO8601::timestamp_ms_utc_to_date_opt(auth_time_int) {
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

    // Always read the writer: a TTL alone cannot invalidate an administrative
    // close, channel change, or reschedule. One narrow row keeps those updates
    // visible without transferring election EML or unrelated scheduled tasks.
    let CastVoteConfiguration {
        presentation,
        status,
        voting_channels,
        scheduled_events,
    } = get_cast_vote_configuration(
        hasura_transaction,
        tenant_id,
        election_event_id,
        election_id,
    )
    .await
    .map_err(|e| CastVoteError::CheckStatusInternalFailed(e.to_string()))?;
    let election_presentation = parse_election_presentation(presentation)?;

    // these dates are used to check by scheduled event date
    // (even if the even hasn't been executed)
    let dates: VotingPeriodDates = generate_voting_period_dates(
        scheduled_events,
        &tenant_id,
        &election_event_id,
        Some(election_id),
    )
    .unwrap_or(Default::default());

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

/// Inspect the database error itself: tokio-postgres Display only says
/// "db error", and matching that string loses the trigger's public error code.
fn map_insert_error(error: anyhow::Error) -> CastVoteError {
    let message = error
        .downcast_ref::<tokio_postgres::Error>()
        .and_then(|error| error.as_db_error())
        .filter(|error| error.code() == &tokio_postgres::error::SqlState::RAISE_EXCEPTION)
        .map(|error| error.message());
    match message {
        Some("insert_failed_exceeds_allowed_revotes") => {
            CastVoteError::InsertFailedExceedsAllowedRevotes
        }
        Some("check_votes_in_other_areas_failed") => CastVoteError::CheckVotesInOtherAreasFailed(
            "Cannot insert cast vote, votes already present in other area(s)".to_string(),
        ),
        _ => CastVoteError::InsertFailed(format!("{error:#}")),
    }
}

#[instrument(skip_all, err)]
fn check_popk(ballot_contest: &HashableBallotContest<RistrettoCtx>) -> Result<()> {
    let zkp = Zkp::new(&RistrettoCtx);
    let popk_ok = zkp.encryption_popk_verify(
        &ballot_contest.ciphertext.mhr,
        &ballot_contest.ciphertext.gr,
        &ballot_contest.proof,
        &DEFAULT_PLAINTEXT_LABEL,
    )?;

    if !popk_ok {
        return Err(anyhow!(
            "Popk validation failed for contest {}",
            ballot_contest.contest_id
        ));
    }

    Ok(())
}

#[instrument(skip_all, err)]
fn check_popk_multi(ballot_contest: &HashableMultiBallotContests<RistrettoCtx>) -> Result<()> {
    let zkp = Zkp::new(&RistrettoCtx);
    let popk_ok = zkp.encryption_popk_verify(
        &ballot_contest.ciphertext.mhr,
        &ballot_contest.ciphertext.gr,
        &ballot_contest.proof,
        &DEFAULT_PLAINTEXT_LABEL,
    )?;

    if !popk_ok {
        return Err(anyhow!(
            "Popk validation failed for contest ids {:?}",
            ballot_contest.contest_ids
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn election_event(annotations: Option<serde_json::Value>) -> ElectionEvent {
        ElectionEvent {
            id: "event".to_string(),
            created_at: None,
            updated_at: None,
            labels: None,
            annotations,
            tenant_id: "tenant".to_string(),
            description: None,
            presentation: None,
            bulletin_board_reference: None,
            is_archived: false,
            voting_channels: None,
            status: None,
            user_boards: None,
            encryption_protocol: "protocol".to_string(),
            is_audit: None,
            audit_election_event_id: None,
            public_key: None,
            statistics: None,
            external_id: None,
        }
    }

    #[test]
    fn malformed_presentation_is_an_internal_error_instead_of_default_policy() {
        assert!(matches!(
            parse_election_presentation(Some(json!({"grace_period_secs": "invalid"}))),
            Err(CastVoteError::CheckStatusInternalFailed(message))
                if message.contains("Failed to deserialize election presentation")
        ));
    }

    #[test]
    fn missing_and_valid_presentation_remain_supported() {
        assert!(parse_election_presentation(None).is_ok());
        let presentation =
            parse_election_presentation(Some(json!({"grace_period_secs": 120}))).unwrap();
        assert_eq!(presentation.grace_period_secs, Some(120));
    }

    #[test]
    fn scheduled_close_is_checked_at_each_submission_without_stale_acceptance() {
        let close = ISO8601::to_date("2026-01-01T12:00:00Z").unwrap();
        let status = ElectionStatus {
            voting_status: VotingStatus::OPEN,
            ..Default::default()
        };
        for offset in [-30, -1, 0, 1, 30] {
            let result = check_status_with_loaded_election(
                close + Duration::seconds(offset),
                close - Duration::minutes(1),
                VotingStatusChannel::ONLINE,
                false,
                VotingPeriodDates {
                    start_date: None,
                    end_date: Some("2026-01-01T12:00:00Z".into()),
                },
                &status,
                &ElectionPresentation::default(),
                "election",
            );
            // Preserve existing boundary semantics: at the deadline is accepted.
            assert_eq!(result.is_ok(), offset <= 0, "offset={offset}");
        }
    }

    #[test]
    fn administrative_pause_is_not_hidden_by_a_future_scheduled_close() {
        let now = ISO8601::to_date("2026-01-01T12:00:00Z").unwrap();
        for voting_status in [
            VotingStatus::NOT_STARTED,
            VotingStatus::PAUSED,
            VotingStatus::CLOSED,
        ] {
            let status = ElectionStatus {
                voting_status,
                ..Default::default()
            };
            assert!(check_status_with_loaded_election(
                now,
                now - Duration::minutes(1),
                VotingStatusChannel::ONLINE,
                false,
                VotingPeriodDates {
                    start_date: None,
                    end_date: Some("2026-01-02T12:00:00Z".into())
                },
                &status,
                &ElectionPresentation::default(),
                "election",
            )
            .is_err());
        }
    }

    #[tokio::test]
    #[ignore = "requires CAST_VOTE_TEST_DATABASE_URL pointing to a test PostgreSQL database"]
    async fn database_trigger_error_preserves_public_error_and_retry_contract() {
        let url = std::env::var("CAST_VOTE_TEST_DATABASE_URL").expect("test database URL");
        let (client, connection) = tokio_postgres::connect(&url, tokio_postgres::NoTls)
            .await
            .unwrap();
        let connection = tokio::spawn(connection);
        let error = client
            .batch_execute(
                "DO $$ BEGIN RAISE EXCEPTION 'insert_failed_exceeds_allowed_revotes'; END $$;",
            )
            .await
            .unwrap_err();
        let mapped =
            map_insert_error(anyhow::Error::new(error).context("Error inserting cast vote"));
        assert_eq!(
            serde_json::to_value(&mapped).unwrap(),
            "insert_failed_exceeds_allowed_revotes"
        );
        assert!(matches!(
            skip_or_propagate(mapped),
            Ok(InsertCastVoteResult::SkipRetryFailure(
                CastVoteError::InsertFailedExceedsAllowedRevotes
            ))
        ));
        let mut config = deadpool_postgres::Config::new();
        config.url = Some(url);
        let pool = config
            .create_pool(
                Some(deadpool_postgres::Runtime::Tokio1),
                tokio_postgres::NoTls,
            )
            .unwrap();
        let mut db = pool.get().await.unwrap();
        let transaction = db.transaction().await.unwrap();
        transaction.batch_execute("ALTER TABLE sequent_backend.election ADD COLUMN presentation jsonb, ADD COLUMN status jsonb, ADD COLUMN voting_channels jsonb; CREATE TABLE sequent_backend.scheduled_event (id uuid, tenant_id uuid, election_event_id uuid, archived_at timestamptz, task_id text, event_payload jsonb, cron_config jsonb);").await.unwrap();
        let tenant = Uuid::new_v4().to_string();
        let event = Uuid::new_v4().to_string();
        let election = Uuid::new_v4().to_string();
        transaction.execute("INSERT INTO sequent_backend.election (id,tenant_id,election_event_id,status) VALUES ($1,$2,$3,'{\"voting_status\":\"OPEN\"}')", &[&parse_uuid_v4(&election).unwrap(), &parse_uuid_v4(&tenant).unwrap(), &parse_uuid_v4(&event).unwrap()]).await.unwrap();
        let task = generate_manage_date_task_name(
            &tenant,
            &event,
            Some(&election),
            &EventProcessors::END_VOTING_PERIOD,
        );
        let payload = json!({"election_id": election});
        let cron = json!({"scheduled_date": "2026-01-01T12:00:00Z"});
        transaction.execute("INSERT INTO sequent_backend.scheduled_event (id,tenant_id,election_event_id,task_id,event_payload,cron_config) VALUES ($1,$2,$3,$4,$5,$6)", &[&Uuid::new_v4(), &parse_uuid_v4(&tenant).unwrap(), &parse_uuid_v4(&event).unwrap(), &task, &payload, &cron]).await.unwrap();
        let loaded = get_cast_vote_configuration(&transaction, &tenant, &event, &election)
            .await
            .unwrap();
        assert_eq!(loaded.scheduled_events.len(), 1);
        assert_eq!(
            generate_voting_period_dates(loaded.scheduled_events, &tenant, &event, Some(&election))
                .unwrap()
                .end_date,
            Some("2026-01-01T12:00:00Z".into())
        );
        transaction.batch_execute("UPDATE sequent_backend.election SET status='{\"voting_status\":\"PAUSED\"}'; UPDATE sequent_backend.scheduled_event SET archived_at=now();").await.unwrap();
        let loaded = get_cast_vote_configuration(&transaction, &tenant, &event, &election)
            .await
            .unwrap();
        assert_eq!(loaded.status.unwrap()["voting_status"], "PAUSED");
        assert!(loaded.scheduled_events.is_empty());
        assert!(get_cast_vote_configuration(
            &transaction,
            &Uuid::new_v4().to_string(),
            &event,
            &election
        )
        .await
        .is_err());
        transaction.batch_execute("ALTER TABLE sequent_backend.cast_vote ADD COLUMN ballot_id text, ADD COLUMN cast_ballot_signature bytea, ADD COLUMN annotations jsonb, ADD COLUMN created_at timestamptz DEFAULT now(), ADD COLUMN last_updated_at timestamptz DEFAULT now();").await.unwrap();
        let content = "encrypted-ballot".repeat(1000);
        let tenant_uuid = parse_uuid_v4(&tenant).unwrap();
        let event_uuid = parse_uuid_v4(&event).unwrap();
        let election_uuid = parse_uuid_v4(&election).unwrap();
        let area_uuid = Uuid::new_v4();
        let inserted = postgres::cast_vote::insert_cast_vote(
            &transaction,
            &tenant_uuid,
            &event_uuid,
            &election_uuid,
            &area_uuid,
            &content,
            "voter",
            "ballot",
            &[0; 64],
            &None,
            &None,
            VotingStatusChannel::ONLINE,
            CastVoteStatus::Valid,
        )
        .await
        .unwrap();
        assert_eq!(inserted.content.as_deref(), Some(content.as_str()));
        assert_eq!(inserted.status, CastVoteStatus::Valid);
        assert_eq!(inserted.area_id, Some(area_uuid.to_string()));
        assert_eq!(inserted.ballot_id.as_deref(), Some("ballot"));
        let error = postgres::cast_vote::insert_cast_vote(
            &transaction,
            &tenant_uuid,
            &event_uuid,
            &election_uuid,
            &area_uuid,
            &content,
            "voter",
            "ballot",
            &[0; 64],
            &None,
            &None,
            VotingStatusChannel::ONLINE,
            CastVoteStatus::Valid,
        )
        .await
        .unwrap_err();
        assert!(matches!(
            map_insert_error(error),
            CastVoteError::InsertFailedExceedsAllowedRevotes
        ));
        transaction.rollback().await.unwrap();
        drop(client);
        connection.await.unwrap().unwrap();
    }

    #[test]
    fn ordinary_events_insert_valid_votes_without_async_processing() {
        let status = initial_cast_vote_status(&election_event(None)).unwrap();
        assert_eq!(status, CastVoteStatus::Valid);
    }

    #[test]
    fn configured_datafix_events_insert_pending_votes() {
        let annotations = json!({
            "datafix:id": "external-event",
            "datafix:password_policy": r#"{"base":"password-only","size":6,"characters":"numeric"}"#,
            "datafix:voterview_request": r#"{"url":"https://example.invalid","usr":"user","psw":"secret","county_mun":"county"}"#
        });
        let status = initial_cast_vote_status(&election_event(Some(annotations))).unwrap();
        assert_eq!(status, CastVoteStatus::InProgress);
    }

    #[test]
    fn malformed_datafix_configuration_fails_closed() {
        let annotations = json!({"datafix:id": "external-event"});
        assert!(matches!(
            initial_cast_vote_status(&election_event(Some(annotations))),
            Err(CastVoteError::InvalidDatafixConfiguration(_))
        ));
    }

    #[test]
    fn online_votes_in_open_early_voting_areas_use_early_voting_channel() {
        let election_status = ElectionStatus {
            voting_status: VotingStatus::NOT_STARTED,
            early_voting_status: VotingStatus::OPEN,
            ..Default::default()
        };

        assert_eq!(
            effective_voting_channel_for_status(
                VotingStatusChannel::ONLINE,
                true,
                &election_status,
            ),
            VotingStatusChannel::EARLY_VOTING
        );
    }

    #[test]
    fn online_close_date_keeps_existing_status_rejection_for_early_voting_area() {
        let election_status = ElectionStatus {
            voting_status: VotingStatus::NOT_STARTED,
            early_voting_status: VotingStatus::OPEN,
            ..Default::default()
        };
        let now = ISO8601::to_date("2026-01-01T12:00:00Z").unwrap();
        let auth_time = ISO8601::to_date("2026-01-01T11:00:00Z").unwrap();
        let dates = VotingPeriodDates {
            start_date: None,
            end_date: Some("2026-01-02T00:00:00Z".to_string()),
        };

        let result = check_status_with_loaded_election(
            now,
            auth_time,
            VotingStatusChannel::ONLINE,
            true,
            dates,
            &election_status,
            &ElectionPresentation::default(),
            "election-id",
        );

        assert!(matches!(result, Err(CastVoteError::CheckStatusFailed(_))));
    }

    #[test]
    fn accepted_early_vote_without_online_close_date_is_labelled_early_voting() {
        let election_status = ElectionStatus {
            voting_status: VotingStatus::NOT_STARTED,
            early_voting_status: VotingStatus::OPEN,
            ..Default::default()
        };
        let now = ISO8601::to_date("2026-01-01T12:00:00Z").unwrap();
        let auth_time = ISO8601::to_date("2026-01-01T11:00:00Z").unwrap();

        let channel = check_status_with_loaded_election(
            now,
            auth_time,
            VotingStatusChannel::ONLINE,
            true,
            VotingPeriodDates::default(),
            &election_status,
            &ElectionPresentation::default(),
            "election-id",
        )
        .unwrap();

        assert_eq!(channel, VotingStatusChannel::EARLY_VOTING);
    }

    #[test]
    fn early_voting_area_does_not_overwrite_transport_channels() {
        let election_status = ElectionStatus {
            voting_status: VotingStatus::NOT_STARTED,
            kiosk_voting_status: VotingStatus::NOT_STARTED,
            early_voting_status: VotingStatus::OPEN,
            telephone_voting_status: VotingStatus::NOT_STARTED,
            ..Default::default()
        };

        for channel in [VotingStatusChannel::KIOSK, VotingStatusChannel::TELEPHONE] {
            assert_eq!(
                effective_voting_channel_for_status(channel, true, &election_status,),
                channel
            );
        }
    }
}
