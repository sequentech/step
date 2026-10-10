// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres;
use crate::postgres::area::get_area_by_id;
use crate::postgres::ballot_style::{get_published_ballot_emls, PublishedBallotEml};
use crate::postgres::cast_vote::CastVoteReceipt;
use crate::postgres::election::{get_cast_vote_configuration, CastVoteConfiguration};
use crate::postgres::election_event::get_election_event_by_id;
use crate::services::cast_ballot::{
    prepare_received_cast, store_cast_receipt, CastBallotInput, CastBallotOutput, PreparedCast,
    ReceivedCast,
};
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
use crate::services::receive_ballot::must_be_received;
use anyhow::{anyhow, Context, Result};
use b4::messages::message::Signer;
use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Duration, Local};
use dashmap::DashMap;
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::*;
use once_cell::sync::Lazy;
use sequent_core::ballot::verify_ballot_signature;
use sequent_core::ballot::BallotStyle;
use sequent_core::ballot::ContestEncryptionPolicy;
// The tests build presentations with it; the grace period itself comes from
// `ballot_box_seal::deadline::grace_period`.
#[cfg(test)]
use sequent_core::ballot::EGracePeriodPolicy;
use sequent_core::ballot::{
    AreaPresentation, EarlyVotingPolicy, ElectionPresentation, ElectionStatus, VoterSigningPolicy,
    VotingPeriodDates, VotingStatus, VotingStatusChannel,
};
use sequent_core::ballot::{HashableBallot, HashableBallotContest, SignedHashableBallot};
use sequent_core::ballot_receipt::CastReceipt;
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
use std::collections::HashSet;
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

/// The votable contest ids of each published ballot style. A ballot style
/// never changes once it is published.
static PUBLISHED_STYLE_CONTEST_IDS: Lazy<DashMap<Uuid, HashSet<String>>> = Lazy::new(DashMap::new);

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
    received_cast: Option<&ReceivedCast>,
) -> Result<(CastVote, VotingStatusChannel, Option<CastReceipt>), CastVoteError> {
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
        received_cast,
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
        | CastVoteError::CheckVotesInOtherAreasFailed(_)
        | CastVoteError::BallotNotReceived
        | CastVoteError::BallotCastSignatureFailed(_)
        | CastVoteError::BallotAlreadyCast
        | CastVoteError::BallotAudited => Ok(InsertCastVoteResult::SkipRetryFailure(cast_vote_err)),
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
    )
    .await?;
    match outcome {
        CastOutcome::Inserted { result, .. } => Ok(result),
        CastOutcome::AlreadyCast(_) => Err(CastVoteError::UnknownError(
            "A ballot sent with the cast has no earlier receipt".to_string(),
        )),
    }
}

/// Checks a cast request and inserts the vote. A ballot has to carry exactly the
/// contests of a ballot style published for the voter's area and election. A
/// refusal that a retry cannot change is returned as a `CastOutcome` that holds
/// `InsertCastVoteResult::SkipRetryFailure`, any other error is returned for the
/// caller to retry.
#[instrument(skip(request), err)]
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

    let initial_status = initial_cast_vote_status(&election_event)?;

    let presentation_opt = election_event
        .get_presentation()
        .map_err(|e| CastVoteError::ElectionEventNotFound(e.to_string()))?;

    let is_multi_contest = if let Some(presentation) = presentation_opt.clone() {
        presentation.contest_encryption_policy == Some(ContestEncryptionPolicy::MULTIPLE_CONTESTS)
    } else {
        false
    };
    let presentation = presentation_opt.unwrap_or_default();

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
            let prepared =
                prepare_received_cast(&hasura_transaction, &election_event, voter_id, &cast).await;
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

    let published_styles = match get_published_style_contest_ids(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        &input.election_id.to_string(),
        area_id,
    )
    .await
    {
        Ok(published_styles) => published_styles,
        Err(error) => return error.into_cast_outcome(),
    };

    let hash_result = if is_multi_contest {
        deserialize_and_check_multi_ballot(&input, voter_id, &published_styles)
    } else {
        deserialize_and_check_ballot(&input, voter_id, &published_styles)
    };

    let (pseudonym_h, vote_h, voter_signature_data) = match hash_result {
        Ok(hash) => hash,
        Err(cv_err) => {
            return Ok(CastOutcome::refused(cv_err));
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
                received_cast.as_ref(),
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
                received_cast.as_ref(),
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
        Ok((inserted_cast_vote, effective_voting_channel, receipt)) => {
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
                .map(|result| CastOutcome::Inserted { result, receipt })
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

            skip_or_propagate(cast_vote_err).map(CastOutcome::refused_result)
        }
    }
}

/// Checks a ballot that encrypts each contest on its own: its id is the hash of
/// its content, it carries exactly the contests of one of the `published_styles`,
/// the proofs of knowledge hold and its voter signature, if it has one, is valid.
/// Returns the voter pseudonym hash, the ballot hash and the signature data.
#[instrument(skip(input), err)]
pub fn deserialize_and_check_ballot(
    input: &InsertCastVoteInput,
    voter_id: &str,
    published_styles: &[HashSet<String>],
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

    check_ballot_contests(
        hashable_ballot_contests
            .iter()
            .map(|contest| &contest.contest_id),
        published_styles,
    )?;

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

/// Checks a ballot that encrypts all its contests together, as
/// `deserialize_and_check_ballot` does for one that encrypts each contest on its
/// own.
#[instrument(skip(input), err)]
pub fn deserialize_and_check_multi_ballot(
    input: &InsertCastVoteInput,
    voter_id: &str,
    published_styles: &[HashSet<String>],
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

    check_ballot_contests(
        &hashable_multi_ballot_contests.contest_ids,
        published_styles,
    )?;

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
        voter_signature_data,
        received_cast
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
    received_cast: Option<&ReceivedCast>,
) -> Result<(CastVote, VotingStatusChannel, Option<CastReceipt>), CastVoteError> {
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
        &area_uuid,
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

    // The cast vote takes the Ballot ID the voter saw at review. The ballot
    // box signs the receipt of a cast it stores in this transaction, so the
    // receipt leaves only with the commit.
    let cast = match received_cast {
        Some(received_cast) => {
            let (cast_at, receipt) = store_cast_receipt(
                &hasura_transaction,
                &tenant_uuid,
                &election_event_uuid,
                received_cast,
            )
            .await?;
            Some((&received_cast.received_ballot, cast_at, receipt))
        }
        None => None,
    };
    let ballot_id = cast
        .as_ref()
        .map(|(received_ballot, _, _)| received_ballot.received.ballot_id.as_str())
        .unwrap_or(&input.ballot_id);
    let cast_vote_receipt =
        cast.as_ref()
            .map(|(received_ballot, cast_at, receipt)| CastVoteReceipt {
                received_ballot_id: &received_ballot.id,
                cast_at,
                cast_receipt_signature: &receipt.cast_receipt_signature,
            });

    let insert_phase = CastVotePhase::start("insert");
    let insert = postgres::cast_vote::insert_cast_vote(
        &hasura_transaction,
        &tenant_uuid,
        &election_event_uuid,
        &election_uuid,
        &area_uuid,
        &input.content,
        ids.voter_id,
        ballot_id,
        &ballot_signature,
        voter_ip,
        voter_country,
        effective_voting_channel,
        initial_status,
        cast_vote_receipt.as_ref(),
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

    Ok((
        cast_vote,
        effective_voting_channel,
        cast.map(|(_, _, receipt)| receipt),
    ))
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

    let electoral_log = ElectoralLog::for_system_with_signing_key(board_name.as_str(), sk);
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

#[instrument(skip_all, err)]
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

/// Inspect the database error itself: tokio-postgres Display only says
/// "db error", and matching that string loses the trigger's public error code.
pub fn map_insert_error(error: anyhow::Error) -> CastVoteError {
    // The seal guard refuses a write to a sealed ballot box (VOTE-FREEZE):
    // voting is closed there.
    if crate::services::ballot_box_seal::is_ballot_box_sealed(&error) {
        return CastVoteError::CheckStatusFailed(
            crate::services::ballot_box_seal::BALLOT_BOX_SEALED_MESSAGE.to_string(),
        );
    }
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

/// Why the contests of the published ballot styles could not be determined.
#[derive(Debug)]
pub(crate) enum PublishedStylesError {
    /// The styles could not be read from the database, so a retry can succeed.
    Lookup(CastVoteError),
    /// A published ballot style cannot be used, so a retry finds the same data.
    Unreadable(CastVoteError),
}

impl PublishedStylesError {
    /// Wraps a failure to read the published ballot styles, which a retry can
    /// fix.
    fn lookup_failed(error: anyhow::Error) -> Self {
        Self::Lookup(CastVoteError::CheckStatusInternalFailed(error.to_string()))
    }

    /// Maps the failure to the caller's retry contract: an unreadable style is
    /// terminal and surfaced as a refused cast, a failed lookup propagates for
    /// the normal retry path.
    fn into_cast_outcome(self) -> Result<CastOutcome, CastVoteError> {
        match self {
            Self::Unreadable(cv_err) => Ok(CastOutcome::refused(cv_err)),
            Self::Lookup(cv_err) => Err(cv_err),
        }
    }
}

impl From<PublishedStylesError> for CastVoteError {
    /// The cast error that the failure to determine the published styles
    /// stands for.
    fn from(error: PublishedStylesError) -> Self {
        match error {
            PublishedStylesError::Lookup(cv_err) | PublishedStylesError::Unreadable(cv_err) => {
                cv_err
            }
        }
    }
}

/// The ids of the contests that a ballot has to carry for a ballot style, read
/// from its EML. Acclaimed contests are never encoded, so they are left out.
fn votable_contest_ids(ballot_eml: &str) -> Result<HashSet<String>, CastVoteError> {
    let ballot_style: BallotStyle = deserialize_str(ballot_eml)
        .map_err(|e| CastVoteError::CheckStatusInternalFailed(e.to_string()))?;
    Ok(ballot_style
        .votable_contests()
        .map(|contest| contest.id.clone())
        .collect())
}

/// The votable contest ids of a published ballot style, from the EML the lookup
/// returned for it.
fn read_style_contest_ids(
    style: &PublishedBallotEml,
) -> Result<HashSet<String>, PublishedStylesError> {
    let ballot_eml = style.ballot_eml.as_deref().ok_or_else(|| {
        PublishedStylesError::Unreadable(CastVoteError::CheckStatusInternalFailed(format!(
            "Published ballot style {} has no ballot EML",
            style.id
        )))
    })?;
    votable_contest_ids(ballot_eml).map_err(PublishedStylesError::Unreadable)
}

/// The votable contests of each ballot style published for this area and
/// election.
#[instrument(skip(hasura_transaction), err(Debug))]
pub(crate) async fn get_published_style_contest_ids(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    area_id: &str,
) -> Result<Vec<HashSet<String>>, PublishedStylesError> {
    let mut styles = get_published_ballot_emls(
        hasura_transaction,
        tenant_id,
        election_event_id,
        election_id,
        area_id,
        false,
    )
    .await
    .map_err(PublishedStylesError::lookup_failed)?;
    if styles
        .iter()
        .any(|style| !PUBLISHED_STYLE_CONTEST_IDS.contains_key(&style.id))
    {
        styles = get_published_ballot_emls(
            hasura_transaction,
            tenant_id,
            election_event_id,
            election_id,
            area_id,
            true,
        )
        .await
        .map_err(PublishedStylesError::lookup_failed)?;
    }

    let mut published_styles = Vec::with_capacity(styles.len());
    for style in styles {
        if let Some(cached) = PUBLISHED_STYLE_CONTEST_IDS.get(&style.id) {
            published_styles.push(cached.clone());
            continue;
        }
        let style_contest_ids = read_style_contest_ids(&style)?;
        PUBLISHED_STYLE_CONTEST_IDS.insert(style.id, style_contest_ids.clone());
        published_styles.push(style_contest_ids);
    }
    Ok(published_styles)
}

/// The tally reads one ciphertext for each votable contest of the ballot style
/// published for the voter's area and election. A ballot has to carry exactly
/// the contests of one of the published styles, each of them once.
fn check_ballot_contests<'a>(
    ballot_contest_ids: impl IntoIterator<Item = &'a String>,
    published_styles: &[HashSet<String>],
) -> Result<(), CastVoteError> {
    if published_styles.is_empty() {
        return Err(CastVoteError::BallotStyleMismatch(
            "There is no ballot style published for the voter's area and election".to_string(),
        ));
    }

    let mut ballot_contests: HashSet<&str> = HashSet::new();
    let mut repeated: Vec<&str> = Vec::new();
    for contest_id in ballot_contest_ids {
        if !ballot_contests.insert(contest_id.as_str()) {
            repeated.push(contest_id.as_str());
        }
    }
    if !repeated.is_empty() {
        repeated.sort();
        repeated.dedup();
        return Err(CastVoteError::BallotStyleMismatch(format!(
            "The ballot includes contests {repeated:?} more than once"
        )));
    }

    let matches_ballot = |style: &HashSet<String>| {
        style.len() == ballot_contests.len()
            && style
                .iter()
                .all(|contest_id| ballot_contests.contains(contest_id.as_str()))
    };
    if published_styles.iter().any(matches_ballot) {
        return Ok(());
    }

    let (mut missing, mut unexpected): (Vec<&str>, Vec<&str>) = published_styles
        .iter()
        .map(|style| {
            let missing: Vec<&str> = style
                .iter()
                .map(String::as_str)
                .filter(|contest_id| !ballot_contests.contains(contest_id))
                .collect();
            let unexpected: Vec<&str> = ballot_contests
                .iter()
                .copied()
                .filter(|contest_id| !style.contains(*contest_id))
                .collect();
            (missing, unexpected)
        })
        .min_by_key(|(missing, unexpected)| missing.len() + unexpected.len())
        .unwrap_or_default();
    missing.sort();
    unexpected.sort();
    Err(CastVoteError::BallotStyleMismatch(format!(
        "The ballot does not match the contests of the voter's ballot style: \
         missing {missing:?}, not part of the style {unexpected:?}"
    )))
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
#[path = "insert_cast_vote_tests.rs"]
mod tests;
