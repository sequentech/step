// SPDX-FileCopyrightText: 2024 David Ruescas <david@sequentech.io>
// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres;
use crate::postgres::area::get_area_by_id;
use crate::postgres::election::get_election_by_id;
use crate::postgres::election::get_election_max_revotes;
use crate::postgres::election_event::{get_election_event_by_id, ElectionEventDatafix};
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::cast_votes::get_voter_signing_key;
use crate::services::cast_votes::CastVote;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::datafix;
use crate::services::datafix::types::SoapRequest;
use crate::services::datafix::utils::is_datafix_election_event;
use crate::services::datafix::utils::voted_via_internet;
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log::ElectoralLog;
use crate::services::protocol_manager::get_protocol_manager;
use crate::services::users::{get_username_by_id, list_users, ListUsersFilter};
use anyhow::{anyhow, Context, Result};
use b3::messages::message::Signer;
use chrono::{DateTime, Duration, Local};
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::*;
use futures::try_join;
use sequent_core::ballot::ContestEncryptionPolicy;
use sequent_core::ballot::EGracePeriodPolicy;

use sequent_core::ballot::{
    AreaPresentation, EarlyVotingPolicy, ElectionPresentation, ElectionStatus, VoterSigningPolicy,
    VotingPeriodDates, VotingStatus, VotingStatusChannel,
};
use sequent_core::ballot::{HashableBallot, HashableBallotContest};
use sequent_core::encrypt::hash_ballot_sha512;
use sequent_core::encrypt::hash_multi_ballot_sha512;
use sequent_core::encrypt::DEFAULT_PLAINTEXT_LABEL;
use sequent_core::multi_ballot::HashableMultiBallot;
use sequent_core::multi_ballot::HashableMultiBallotContests;
use sequent_core::serialization::deserialize_with_path::*;
use sequent_core::services::date::ISO8601;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::types::hasura::core::{ElectionEvent, VotingChannels};
use sequent_core::types::keycloak::{VOTED_CHANNEL, VOTED_CHANNEL_INTERNET_VALUE};
use sequent_core::types::scheduled_event::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strand::backend::ristretto::RistrettoCtx;
use strand::hash::{hash_to_array, Hash, HashWrapper};
use strand::serialization::StrandSerialize;
use strand::signature::StrandSignatureSk;
use strand::util::StrandError;
use strand::zkp::Zkp;
use strum_macros::Display;
use tracing::info;
use tracing::{error, event, instrument, Level};
use uuid::Uuid;
// Added imports
use sequent_core::encrypt::hash_ballot;
use sequent_core::encrypt::hash_multi_ballot;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InsertCastVoteInput {
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

pub enum InsertCastVoteResult {
    Success(InsertCastVoteOutput),
    SkipRetryFailure(CastVoteError),
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
    #[serde(rename = "electoral_log_not_found")]
    ElectoralLogNotFound(String),
    #[serde(rename = "check_status_failed")]
    CheckStatusFailed(String),
    #[serde(rename = "check_status_internal_failed")]
    CheckStatusInternalFailed(String),
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

    let mut keycloak_db_client: DbClient = get_keycloak_pool().await.get().await.map_err(|e| {
        error!("Error getting keycloak db client {}", e);
        CastVoteError::GetDbClientFailed(e.to_string())
    })?;

    let keycloak_transaction = keycloak_db_client.transaction().await.map_err(|e| {
        error!("Error getting keycloak transaction {}", e);
        CastVoteError::GetDbClientFailed(e.to_string())
    })?;

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

    let presentation_opt = election_event
        .get_presentation()
        .map_err(|e| CastVoteError::ElectionEventNotFound(e.to_string()))?;

    let is_multi_contest = if let Some(presentation) = presentation_opt.clone() {
        presentation.contest_encryption_policy == Some(ContestEncryptionPolicy::MULTIPLE_CONTESTS)
    } else {
        false
    };

    match verify_ballot_id_matches_content(&input, is_multi_contest).map_err(|e| e) {
        Ok(()) => {}
        Err(cv_err) => {
            return Ok(InsertCastVoteResult::SkipRetryFailure(cv_err));
        }
    }

    let (pseudonym_h, vote_h) = if is_multi_contest {
        deserialize_and_check_multi_ballot(&input.content, voter_id)?
    } else {
        deserialize_and_check_ballot(&input.content, voter_id)?
    };

    let (electoral_log, signing_key) =
        get_electoral_log(&hasura_transaction, &tenant_id, &election_event)
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

    let voter_signing_key: Option<StrandSignatureSk> = if VoterSigningPolicy::WITH_SIGNATURE
        == voter_signing_policy
    {
        let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
            .with_context(|| "missing bulletin board")
            .map_err(|e| CastVoteError::BallotVoterSignatureFailed(e.to_string()))?;

        let voter_signing_key = get_voter_signing_key(
            &hasura_transaction,
            &board_name,
            ids.tenant_id,
            ids.election_event_id,
            ids.voter_id,
            area_id,
        )
        .await
        .map_err(|e| CastVoteError::BallotVoterSignatureFailed(e.to_string()))?;
        Some(voter_signing_key)
    } else {
        None
    };

    let area_presentation: AreaPresentation = match area.presentation {
        Some(presentation) => deserialize_value(presentation)
            .map_err(|e| CastVoteError::DeserializeAreaPresentationFailed(e.to_string()))?,
        None => AreaPresentation::default(),
    };
    let is_early_voting_area = area_presentation
        .allow_early_voting
        .clone()
        .unwrap_or_default()
        == EarlyVotingPolicy::AllowEarlyVoting;

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
        &voter_signing_key,
        is_early_voting_area,
    )
    .await;

    let ip = format!("ip: {}", voter_ip.as_deref().unwrap_or(""),);
    let country = format!("country: {}", voter_country.as_deref().unwrap_or(""),);
    let realm = get_event_realm(tenant_id, election_event_id);
    let username = get_username_by_id(&keycloak_transaction, &realm, voter_id)
        .await
        .map_err(|e| CastVoteError::UnknownError(format!("Error get_username_by_id {e:?}")))?;

    match result {
        Ok(inserted_cast_vote) => {
            let mut after_result_hasura_client: DbClient = get_hasura_pool()
                .await
                .get()
                .await
                .map_err(|e| CastVoteError::GetDbClientFailed(e.to_string()))?;
            let after_result_hasura_transaction = after_result_hasura_client
                .transaction()
                .await
                .map_err(|e| CastVoteError::GetTransactionFailed(e.to_string()))?;

            if is_datafix_election_event(&election_event) {
                // If insert_cast_vote_and_commit fails then we will not send SetVoted to VoterView.
                // However if the one failing is voterview_requests::send returning an error here would be problematic
                // because the vote is already casted.
                // But it will be a good idea to log the error in the electoral_log.
                let filter = ListUsersFilter {
                    tenant_id: tenant_id.to_string(),
                    election_event_id: Some(election_event_id.to_string()),
                    realm: realm.clone(),
                    user_ids: Some(vec![voter_id.to_string()]),
                    area_id: Some(area_id.to_string()),
                    ..ListUsersFilter::default()
                };
                let hasura_transaction = hasura_db_client
                    .transaction()
                    .await
                    .map_err(|e| CastVoteError::GetTransactionFailed(e.to_string()))?;
                let user =
                    match list_users(&hasura_transaction, &keycloak_transaction, filter).await {
                        Ok((users, 1)) => users
                            .last()
                            .map(|val_ref| val_ref.to_owned())
                            .unwrap_or_default(),
                        Ok(_) => {
                            return Err(CastVoteError::UnknownError(format!(
                                "Multiple users found with id {voter_id}"
                            )));
                        }
                        Err(_) => {
                            return Err(CastVoteError::UnknownError(format!(
                                "Voter not found with id {voter_id}"
                            )));
                        }
                    };
                let attributes = user.attributes.clone().unwrap_or_default();
                if !voted_via_internet(&attributes) {
                    let result = datafix::voterview_requests::send(
                        SoapRequest::SetVoted,
                        ElectionEventDatafix(election_event),
                        &username,
                    )
                    .await;

                    // TODO: Post the result in the electoral_log

                    let client = KeycloakAdminClient::new().await.map_err(|e| {
                        CastVoteError::UnknownError(format!(
                            "Error obtaining keycloak admin client: {e:?}"
                        ))
                    })?;

                    // Set the attribute to avoid sending it again on the next vote.
                    let mut hash_map = HashMap::new();
                    hash_map.insert(
                        VOTED_CHANNEL.to_string(),
                        vec![VOTED_CHANNEL_INTERNET_VALUE.to_string()],
                    );
                    let attributes = Some(hash_map);
                    let _user = client
                        .edit_user(
                            &realm, voter_id, None, attributes, None, None, None, None, None, None,
                        )
                        .await
                        .map_err(|e| {
                            error!("Error editing user Internet channel: {e:?}");
                        });
                }
            }
            let electoral_log_res = ElectoralLog::for_voter(
                &after_result_hasura_transaction,
                &electoral_log.elog_database,
                tenant_id,
                election_event_id,
                voter_id,
                &voter_signing_key,
            )
            .await;

            let electoral_log = match electoral_log_res {
                Ok(electoral_log) => electoral_log,
                Err(err) => {
                    error!("Error posting to the electoral log {:?}", err);
                    return Ok(InsertCastVoteResult::Success(inserted_cast_vote));
                }
            };

            let log_result = electoral_log
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
                )
                .await;
            if let Err(log_err) = log_result {
                error!("Error posting to the electoral log {:?}", log_err);
            }
            Ok(InsertCastVoteResult::Success(inserted_cast_vote))
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
                    username,
                    area_id.to_string().clone(),
                )
                .await;

            if let Err(log_err) = log_result {
                error!("Error posting error to the electoral log {:?}", log_err);
            }

            match cast_vote_err {
                CastVoteError::InsertFailedExceedsAllowedRevotes => {
                    Ok(InsertCastVoteResult::SkipRetryFailure(cast_vote_err))
                }
                _ => Err(cast_vote_err),
            }
        }
    }
}

#[instrument(err)]
pub fn deserialize_and_check_ballot(
    content: &str,
    voter_id: &str,
) -> Result<(PseudonymHash, CastVoteHash), CastVoteError> {
    let hashable_ballot: HashableBallot = deserialize_str(&content)
        .map_err(|e| CastVoteError::DeserializeBallotFailed(e.to_string()))?;

    let pseudonym_h = hash_voter_id(voter_id)
        .map_err(|e| CastVoteError::SerializeVoterIdFailed(e.to_string()))?;

    let vote_h = hash_ballot_sha512(&hashable_ballot)
        .map_err(|e| CastVoteError::SerializeBallotFailed(e.to_string()))?;

    let pseudonym_h = PseudonymHash(HashWrapper::new(pseudonym_h));
    let vote_h = CastVoteHash(HashWrapper::new(vote_h));

    let hashable_ballot_contests = hashable_ballot
        .deserialize_contests()
        .map_err(|e| CastVoteError::DeserializeContestsFailed(e.to_string()))?;

    hashable_ballot_contests
        .iter()
        .map(check_popk)
        .collect::<Result<Vec<()>>>()
        .map_err(|e| CastVoteError::PokValidationFailed(e.to_string()))?;

    Ok((pseudonym_h, vote_h))
}

#[instrument(skip(content), err)]
pub fn deserialize_and_check_multi_ballot(
    content: &str,
    voter_id: &str,
) -> Result<(PseudonymHash, CastVoteHash), CastVoteError> {
    let hashable_multi_ballot: HashableMultiBallot = deserialize_str(content)
        .map_err(|e| CastVoteError::DeserializeBallotFailed(e.to_string()))?;

    let pseudonym_h = hash_voter_id(voter_id)
        .map_err(|e| CastVoteError::SerializeVoterIdFailed(e.to_string()))?;

    let vote_h = hash_multi_ballot_sha512(&hashable_multi_ballot)
        .map_err(|e| CastVoteError::SerializeBallotFailed(e.to_string()))?;

    let pseudonym_h = PseudonymHash(HashWrapper::new(pseudonym_h));
    let vote_h = CastVoteHash(HashWrapper::new(vote_h));

    let hashable_multi_ballot_contests = hashable_multi_ballot
        .deserialize_contests()
        .map_err(|e| CastVoteError::DeserializeContestsFailed(e.to_string()))?;

    check_popk_multi(&hashable_multi_ballot_contests)
        .map_err(|e| CastVoteError::PokValidationFailed(e.to_string()))?;

    Ok((pseudonym_h, vote_h))
}

#[instrument(skip(input, signing_key, voter_signing_key), err)]
pub async fn get_ballot_signature(
    input: &InsertCastVoteInput,
    voter_signing_key: &Option<StrandSignatureSk>,
    signing_key: StrandSignatureSk,
) -> Result<Vec<u8>, CastVoteError> {
    // These are unhashed bytes, the signing code will hash it first.
    let ballot_bytes = input.get_bytes_for_signing();

    if let Some(voter_signing_key) = voter_signing_key.clone() {
        // TODO do something with this
        let voter_ballot_signature = voter_signing_key
            .sign(&ballot_bytes)
            .map_err(|e| CastVoteError::BallotVoterSignatureFailed(e.to_string()))?;
    };

    let ballot_signature = signing_key
        .sign(&ballot_bytes)
        .map_err(|e| CastVoteError::BallotSignFailed(e.to_string()))?;

    Ok(ballot_signature.to_bytes().to_vec())
}

#[instrument(
    skip(
        input,
        hasura_transaction,
        election_event,
        signing_key,
        voter_signing_key
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
    voter_signing_key: &Option<StrandSignatureSk>,
    is_early_voting_area: bool,
) -> Result<CastVote, CastVoteError> {
    let election_id_string = input.election_id.to_string();
    let election_id = election_id_string.as_str();
    let tenant_uuid = Uuid::parse_str(ids.tenant_id)
        .map_err(|e| CastVoteError::UuidParseFailed(e.to_string(), "tenant_id".to_string()))?;
    let election_event_uuid = Uuid::parse_str(ids.election_event_id).map_err(|e| {
        CastVoteError::UuidParseFailed(e.to_string(), "election_event_id".to_string())
    })?;
    let election_uuid = Uuid::parse_str(election_id)
        .map_err(|e| CastVoteError::UuidParseFailed(e.to_string(), "election_id".to_string()))?;
    let area_uuid = Uuid::parse_str(ids.area_id)
        .map_err(|e| CastVoteError::UuidParseFailed(e.to_string(), "area_id".to_string()))?;
    let (check_status, check_previous_votes, ballot_signature) = try_join!(
        // Check status is the most expensive call here, it takes around 2/3 of the time of the whole insert_cast_vote
        check_status(
            ids.tenant_id,
            ids.election_event_id,
            election_id,
            &hasura_transaction,
            &election_event,
            auth_time,
            voting_channel,
            is_early_voting_area,
        ),
        // Transaction isolation begins at this future (unless above methods are
        // switched from hasura to direct sql)
        check_previous_votes(
            ids.voter_id,
            ids.tenant_id,
            ids.election_event_id,
            election_id,
            ids.area_id,
            &hasura_transaction,
            &tenant_uuid,
            &election_event_uuid,
            &election_uuid,
        ),
        get_ballot_signature(&input, voter_signing_key, signing_key)
    )?;

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
        &voter_ip,
        &voter_country,
    );

    let cast_vote = insert.await.map_err(|e| {
        let err_str = e.to_string();
        if err_str.contains(
            CastVoteError::InsertFailedExceedsAllowedRevotes
                .to_string()
                .as_str(),
        ) {
            CastVoteError::InsertFailedExceedsAllowedRevotes
        } else {
            CastVoteError::InsertFailed(err_str)
        }
    })?;

    hasura_transaction
        .commit()
        .await
        .map_err(|e| CastVoteError::CommitFailed(e.to_string()))?;

    Ok(cast_vote)
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

    let electoral_log = ElectoralLog::new_from_sk(
        hasura_transaction,
        tenant_id,
        &election_event.id,
        board_name.as_str(),
        &sk,
    )
    .await;

    Ok((electoral_log?, sk.clone()))
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
) -> Result<(), CastVoteError> {
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

    let election_opt = get_election_by_id(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        election_id,
    )
    .await
    .context("Cannot retrieve election data")
    .map_err(|e| CastVoteError::CheckStatusInternalFailed(e.to_string()))?;
    let election = election_opt.ok_or(CastVoteError::CheckStatusInternalFailed(
        "Election not found".into(),
    ))?;

    let election_presentation: ElectionPresentation = election
        .presentation
        .clone()
        .map(|value| deserialize_value(value).ok())
        .flatten()
        .unwrap_or(Default::default());

    let scheduled_events = find_scheduled_event_by_election_event_id(
        &hasura_transaction,
        tenant_id,
        election_event_id,
    )
    .await
    .map_err(|e| CastVoteError::CheckStatusInternalFailed(e.to_string()))?;

    // these dates are used to check by scheduled event date
    // (even if the even hasn't been executed)
    let mut dates: VotingPeriodDates = generate_voting_period_dates(
        scheduled_events.clone(),
        &tenant_id,
        &election_event_id,
        Some(election_id),
    )
    .unwrap_or(Default::default());

    if VotingStatusChannel::ONLINE != voting_channel.clone() {
        dates.end_date = None;
    }

    let close_date_opt: Option<DateTime<Local>> = if let Some(end_date_str) = dates.end_date {
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

    let election_status: ElectionStatus = election
        .status
        .clone()
        .map(|value| deserialize_value(value).context("Failed to deserialize election status"))
        .transpose()
        .map(|value| value.unwrap_or_default())
        .map_err(|e| CastVoteError::CheckStatusInternalFailed(e.to_string()))?;

    let election_voting_channels: VotingChannels = election
        .voting_channels
        .clone()
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

    let current_voting_status = election_status.status_by_channel(voting_channel);
    let dates_by_channel = election_status.dates_by_channel(voting_channel);

    // calculate if we need to apply the grace period
    let grace_period_secs = election_presentation.grace_period_secs.unwrap_or(0);
    let grace_period_policy = election_presentation
        .grace_period_policy
        .unwrap_or(EGracePeriodPolicy::NO_GRACE_PERIOD);

    let allow_early_voting = is_early_voting_area
        && election_status.status_by_channel(VotingStatusChannel::EARLY_VOTING)
            == VotingStatus::OPEN
        && election_status.status_by_channel(VotingStatusChannel::ONLINE)
            == VotingStatus::NOT_STARTED;

    // We can only calculate grace period if there's a close date
    if let Some(close_date) = close_date_opt {
        // We only apply the grace period if:
        // 1. Grace period policy is not NO_GRACE_PERIOD
        // 2. Voting Channel is ONLINE
        // 3. Current Voting Status is not PAUSED
        let apply_grace_period: bool = grace_period_policy != EGracePeriodPolicy::NO_GRACE_PERIOD
            && voting_channel == VotingStatusChannel::ONLINE
            && current_voting_status != VotingStatus::PAUSED;
        let grace_period_duration = Duration::seconds(grace_period_secs as i64);
        let close_date_plus_grace_period = close_date + grace_period_duration;

        if apply_grace_period {
            // a voter cannot cast a vote after the grace period or if the voter
            // authenticated after the closing date
            if now > close_date_plus_grace_period || auth_time_local > close_date {
                return Err(CastVoteError::CheckStatusFailed(
                    "Cannot vote outside grace period".to_string(),
                ));
            }

            // if voting before the closing date, we don't apply the grace
            // period so current voting status needs to be open
            if now <= close_date && current_voting_status != VotingStatus::OPEN {
                return Err(CastVoteError::CheckStatusFailed(
                    format!("Election voting status is not open (={current_voting_status:?}) while voting before the closing date of the election"),
                ));
            }
        } else {
            // if grace period does not apply and there's a closing date, to
            // cast a vote you need to do it before the closing date
            if now > close_date {
                return Err(CastVoteError::CheckStatusFailed(
                    "Election close date passed and grace period does not apply or is not set"
                        .to_string(),
                ));
            }

            // if no grace period, election needs to be open to cast a vote
            // period
            if current_voting_status != VotingStatus::OPEN {
                return Err(CastVoteError::CheckStatusFailed(
                    format!("Election Voting Status for voting_channel={voting_channel:?} is {current_voting_status:?} instead of Open and grace_period_policy does not apply or is not set"),
                ));
            }
        }

    // if there's no closing date, election needs to be open to cast a vote
    } else if allow_early_voting {
        info!("Allowing early voting for election id {election_id}");
    } else {
        if current_voting_status != VotingStatus::OPEN {
            return Err(CastVoteError::CheckStatusFailed(
                format!("Voting Status for voting_channel={voting_channel:?} is {current_voting_status:?} instead of Open"),
            ));
        }
        let last_stopped_at = dates_by_channel
            .last_stopped_at
            .map(|val| val.with_timezone(&Local));

        if let Some(close_date) = last_stopped_at {
            if now > close_date {
                return Err(CastVoteError::CheckStatusFailed(format!(
                    "Election close date passed for channel {}",
                    voting_channel
                )));
            }
        }
    }
    Ok(())
}

#[instrument(skip_all, err)]
async fn check_previous_votes(
    voter_id_string: &str,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    area_id: &str,
    hasura_transaction: &Transaction<'_>,
    tenant_uuid: &Uuid,
    election_event_uuid: &Uuid,
    election_uuid: &Uuid,
) -> Result<(), CastVoteError> {
    let (max_revotes, result) = try_join!(
        get_election_max_revotes(
            hasura_transaction,
            tenant_id,
            election_event_id,
            election_id,
        ),
        postgres::cast_vote::get_cast_votes(
            &hasura_transaction,
            tenant_uuid,
            election_event_uuid,
            election_uuid,
            voter_id_string,
        )
    )
    .map_err(|e| CastVoteError::CheckPreviousVotesFailed(e.to_string()))?;

    let (same, other): (Vec<Uuid>, Vec<Uuid>) = result
        .into_iter()
        .filter_map(|cv| cv.area_id.and_then(|id| Uuid::parse_str(&id).ok()))
        .partition(|cv_area_id| cv_area_id.to_string() == area_id.to_string());

    event!(Level::INFO, "get cast votes returns same: {:?}", same);

    // Skip max votes check if max_revotes is 0, allowing unlimited votes
    if max_revotes > 0 && same.len() >= max_revotes {
        return Err(CastVoteError::CheckRevotesFailed(format!(
            "Cannot insert cast vote, maximum votes reached ({}, {})",
            voter_id_string,
            same.len()
        )));
    }
    if other.len() > 0 {
        return Err(CastVoteError::CheckVotesInOtherAreasFailed(format!(
            "Cannot insert cast vote, votes already present in other area(s) ({}, {:?})",
            voter_id_string, other
        )));
    }
    Ok(())
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

/// Verifies that the ballot_id corresponds to the hash of the ballot content
/// The function serves as a security check to ensure that
/// a ballot's content matches its claimed ID.
/// This is crucial for maintaining the integrity of the voting system
/// by preventing ballot tampering or substitution.
pub fn verify_ballot_id_matches_content(
    input: &InsertCastVoteInput,
    is_multi_contest: bool,
) -> Result<(), CastVoteError> {
    let computed_hash = if is_multi_contest {
        let hashable_ballot: HashableMultiBallot = deserialize_str(&input.content)
            .map_err(|e| CastVoteError::DeserializeBallotFailed(e.to_string()))?;

        hash_multi_ballot(&hashable_ballot)
            .map_err(|e| CastVoteError::SerializeBallotFailed(e.to_string()))?
    } else {
        let hashable_ballot: HashableBallot = deserialize_str(&input.content)
            .map_err(|e| CastVoteError::DeserializeBallotFailed(e.to_string()))?;

        hash_ballot(&hashable_ballot)
            .map_err(|e| CastVoteError::SerializeBallotFailed(e.to_string()))?
    };

    if computed_hash != input.ballot_id {
        return Err(CastVoteError::BallotIdMismatch(format!(
            "Expected {} but got {}",
            computed_hash, input.ballot_id
        )));
    }

    Ok(())
}
