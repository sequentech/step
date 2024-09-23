// SPDX-FileCopyrightText: 2024 David Ruescas <david@sequentech.io>
// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura;
use crate::postgres;
use crate::postgres::area::get_area_by_id;
use crate::postgres::election::get_election_max_revotes;
use crate::services::cast_votes::CastVote;
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log::ElectoralLog;
use crate::services::protocol_manager::get_protocol_manager;
use crate::{
    hasura::election_event::get_election_event::GetElectionEventSequentBackendElectionEvent,
    services::database::get_hasura_pool,
};
use anyhow::{anyhow, Context, Result};
use board_messages::braid::message::Signer;
use board_messages::electoral_log::newtypes::*;
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use rocket::futures::TryFutureExt;
use sequent_core::ballot::ElectionEventStatus;
use sequent_core::ballot::ElectionPresentation;
use sequent_core::ballot::ElectionStatus;
use sequent_core::ballot::VotingStatus;
use sequent_core::ballot::{HashableBallot, HashableBallotContest};
use sequent_core::encrypt::hash_ballot_sha512;
use sequent_core::encrypt::DEFAULT_PLAINTEXT_LABEL;
use sequent_core::serialization::base64::Base64Deserialize;
use sequent_core::serialization::deserialize_with_path::*;
use sequent_core::services::connection::AuthHeaders;
use sequent_core::services::keycloak;
use serde::{Deserialize, Serialize};
use serde_json;
use serde_json::Value;
use strand::backend::ristretto::RistrettoCtx;
use strand::hash::{hash_to_array, Hash, HashWrapper};
use strand::serialization::StrandSerialize;
use strand::signature::StrandSignatureSk;
use strand::util::StrandError;
use strand::zkp::Zkp;
use tracing::{event, instrument, Level};
use uuid::Uuid;

use super::date::ISO8601;

#[derive(Serialize, Deserialize, Debug)]
pub struct InsertCastVoteInput {
    pub ballot_id: String,
    pub election_id: Uuid,
    pub content: String,
}

pub type InsertCastVoteOutput = CastVote;

#[instrument(skip(input), err)]
pub async fn try_insert_cast_vote(
    hasura_transaction: &Transaction<'_>,
    input: InsertCastVoteInput,
    tenant_id: &str,
    voter_id: &str,
    area_id: &str,
) -> Result<InsertCastVoteOutput> {
    // TODO performance of serializable
    /*hasura_transaction
    .simple_query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE;")
    .await
    .with_context(|| "Cannot set transaction isolation level")?;*/

    let election_id_string = input.election_id.to_string();
    let election_id = election_id_string.as_str();
    let area_opt = get_area_by_id(&hasura_transaction, tenant_id, area_id).await?;

    let area = if let Some(area) = area_opt {
        area
    } else {
        return Err(anyhow!("Area id not found"));
    };
    let election_event_id = area.election_event_id.as_str();

    let hashable_ballot: HashableBallot = deserialize_str(&input.content)
        .map_err(|err| anyhow!("Error deserializing ballot content: {}", err))?;

    let pseudonym_h =
        hash_voter_id(voter_id).map_err(|err| anyhow!("Error hashing voter id: {:?}", err))?;
    let vote_h = hash_ballot_sha512(&hashable_ballot)
        .map_err(|err| anyhow!("Error hashing ballot: {:?}", err))?;

    let hashable_ballot_contests = hashable_ballot
        .deserialize_contests()
        .map_err(|err| anyhow!("Error deserializing ballot content: {:?}", err))?;
    hashable_ballot_contests
        .iter()
        .map(|contest| check_popk(contest))
        .collect::<Result<Vec<()>>>()?;

    let auth_headers = keycloak::get_client_credentials().await?;

    let election_event = get_election_event(&auth_headers, tenant_id, election_event_id).await?;
    let (electoral_log, signing_key) = get_electoral_log(&election_event).await?;

    let check_status = check_status(
        tenant_id,
        election_event_id,
        election_id,
        auth_headers,
        &election_event,
    );

    // Transaction isolation begins at this future (unless above methods are
    // switched from hasura to direct sql)
    let check_previous_votes = check_previous_votes(
        voter_id,
        tenant_id,
        election_event_id,
        election_id,
        area_id,
        &hasura_transaction,
    );

    // TODO signature must include more information
    let ballot_signature = signing_key.sign(input.content.as_bytes())?;
    let ballot_signature = ballot_signature.to_bytes().to_vec();
    let tenant_uuid = Uuid::parse_str(tenant_id)?;
    let election_event_uuid = Uuid::parse_str(election_event_id)?;
    let election_uuid = Uuid::parse_str(election_id)?;
    let area_uuid = Uuid::parse_str(area_id)?;
    let insert = postgres::cast_vote::insert_cast_vote(
        &hasura_transaction,
        &tenant_uuid,
        &election_event_uuid,
        &election_uuid,
        &area_uuid,
        &input.content,
        voter_id,
        &input.ballot_id,
        &ballot_signature,
    );

    let result = check_status
        .and_then(|_| check_previous_votes)
        .and_then(|_| insert)
        .await;

    let commit = hasura_transaction
        .commit()
        .await
        .map_err(|e| anyhow!("Commit failed: {}", e));
    let result = commit.and(result);

    let pseudonym_h = PseudonymHash(HashWrapper::new(pseudonym_h));
    let vote_h = CastVoteHash(HashWrapper::new(vote_h));

    match result {
        Ok(result) => {
            let log_result = electoral_log
                .post_cast_vote(
                    election_event_id.to_string(),
                    Some(input.election_id.to_string()),
                    pseudonym_h,
                    vote_h,
                )
                .await;
            if let Err(log_err) = log_result {
                event!(
                    Level::ERROR,
                    "Error posting to the electoral log {:?}",
                    log_err
                );
            }
            Ok(result.into())
        }
        Err(err) => {
            // TODO error message may leak implementation details
            let log_result = electoral_log
                .post_cast_vote_error(
                    election_event_id.to_string(),
                    Some(input.election_id.to_string()),
                    pseudonym_h,
                    err.to_string(),
                )
                .await;

            if let Err(log_err) = log_result {
                event!(
                    Level::ERROR,
                    "Error posting error to the electoral log {:?}",
                    log_err
                );
            }
            Err(err)
        }
    }
}

fn hash_voter_id(voter_id: &str) -> Result<Hash, StrandError> {
    let bytes = voter_id.to_string().strand_serialize()?;
    hash_to_array(&bytes)
}

#[instrument(skip_all, err)]
async fn get_electoral_log(
    election_event: &GetElectionEventSequentBackendElectionEvent,
) -> anyhow::Result<(ElectoralLog, StrandSignatureSk)> {
    let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
        .with_context(|| "missing bulletin board")?;

    let protocol_manager = get_protocol_manager::<RistrettoCtx>(&board_name).await?;
    let sk = protocol_manager.get_signing_key();

    let electoral_log = ElectoralLog::new_from_sk(board_name.as_str(), &sk).await;

    Ok((electoral_log?, sk.clone()))
}

#[instrument(skip_all, err)]
async fn get_election_event(
    auth_headers: &AuthHeaders,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<GetElectionEventSequentBackendElectionEvent> {
    let hasura_response = hasura::election_event::get_election_event(
        auth_headers.clone(),
        tenant_id.to_string(),
        election_event_id.to_string(),
    )
    .await
    .context("Cannot retrieve election event data")?;

    // TODO expect
    let election_event = &hasura_response
        .data
        .expect("expected data".into())
        .sequent_backend_election_event[0];

    Ok(election_event.clone())
}

#[instrument(skip_all, err)]
async fn check_status(
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    auth_headers: AuthHeaders,
    election_event: &GetElectionEventSequentBackendElectionEvent,
) -> anyhow::Result<()> {
    if election_event.is_archived {
        return Err(anyhow!("Election event is archived"));
    }

    let hasura_response = hasura::election::get_election(
        auth_headers.clone(),
        tenant_id.to_string(),
        election_event_id.to_string(),
        election_id.to_string(),
    )
    .await
    .context("Cannot retrieve election data")?;

    // TODO expect
    let election = &hasura_response
        .data
        .expect("expected data".into())
        .sequent_backend_election[0];

    let election_presentation: ElectionPresentation = election
        .presentation
        .clone()
        .map(|value| deserialize_value(value).ok())
        .flatten()
        .unwrap_or(Default::default());

    let close_date_opt = election_presentation
        .dates
        .clone()
        .map(|dates| dates.end_date)
        .flatten()
        .map(|end_date| ISO8601::to_date(&end_date).ok())
        .flatten();

    if let Some(close_date) = close_date_opt {
        if ISO8601::now() > close_date {
            return Err(anyhow!("Election is closed"));
        }
    };

    let election_status: ElectionStatus = election
        .status
        .clone()
        .map(|value| deserialize_value(value).context("Failed to deserialize election status"))
        .transpose()
        .map(|value| value.unwrap_or(Default::default()))?;

    if election_status.voting_status != VotingStatus::OPEN {
        return Err(anyhow!("Election voting status is not open"));
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
) -> anyhow::Result<()> {
    let max_revotes = get_election_max_revotes(
        hasura_transaction,
        tenant_id,
        election_event_id,
        election_id,
    )
    .await?;

    let result = postgres::cast_vote::get_cast_votes(
        &hasura_transaction,
        &Uuid::parse_str(tenant_id)?,
        &Uuid::parse_str(election_event_id)?,
        &Uuid::parse_str(election_id)?,
        voter_id_string,
    )
    .await?;

    let (same, other): (Vec<Uuid>, Vec<Uuid>) = result
        .into_iter()
        .filter_map(|cv| cv.area_id.and_then(|id| Uuid::parse_str(&id).ok()))
        .partition(|cv_area_id| cv_area_id.to_string() == area_id.to_string());

    event!(Level::INFO, "get cast votes returns same: {:?}", same);
    if same.len() >= max_revotes {
        return Err(anyhow!(
            "Cannot insert cast vote, maximum votes reached ({}, {})",
            voter_id_string,
            same.len()
        ));
    }
    if other.len() > 0 {
        return Err(anyhow!(
            "Cannot insert cast vote, votes already present in other area(s) ({}, {:?})",
            voter_id_string,
            other
        ));
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
