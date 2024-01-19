// SPDX-FileCopyrightText: 2024 David Ruescas <david@sequentech.io>
// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura;
use crate::postgres;
use crate::postgres::area::get_area_by_id;
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
use chrono::{DateTime, Utc};
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use rocket::futures::TryFutureExt;
use rocket::serde::json::Json;
use sequent_core::ballot::ElectionEventStatus;
use sequent_core::ballot::ElectionStatus;
use sequent_core::ballot::VotingStatus;
use sequent_core::services::connection::AuthHeaders;
use sequent_core::services::keycloak;
use serde::{Deserialize, Serialize};
use strand::backend::ristretto::RistrettoCtx;
use strand::elgamal::Ciphertext;
use strand::hash::HashWrapper;
use strand::serialization::StrandDeserialize;
use strand::signature::StrandSignatureSk;
use strand::zkp::Schnorr;
use strand::zkp::Zkp;
use tracing::instrument;
use tracing::{event, Level};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct InsertCastVoteInput {
    pub ballot_id: String,
    pub election_event_id: Uuid,
    pub election_id: Uuid,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InsertCastVoteOutput {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub cast_ballot_signature: Vec<u8>,
}

#[instrument(err)]
pub async fn try_insert_cast_vote(
    input: InsertCastVoteInput,
    tenant_id: &str,
    voter_id: &str,
    area_id: &str,
) -> Result<InsertCastVoteOutput> {
    let mut hasura_db_client: DbClient = get_hasura_pool().await.get().await?;
    let hasura_transaction = hasura_db_client.transaction().await?;
    // TODO performance of serializable
    hasura_transaction
        .simple_query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE;")
        .await
        .with_context(|| "Cannot set transaction isolation level")?;

    let election_event_id_string = input.election_event_id.to_string();
    let election_id_string = input.election_id.to_string();
    let election_event_id = &election_event_id_string;
    let election_id = &election_id_string;
    let area_id_opt =
        get_area_by_id(&hasura_transaction, tenant_id, election_event_id, area_id).await?;

    // TODO get the voter id from somewhere
    let pseudonym_h = [0u8; 64];
    let vote_h = [0u8; 64];

    // get this from the incoming data
    let ciphertext_bytes = vec![];
    // get this from the incoming data
    let proof_bytes = vec![];
    // must match that used on the proving side (voting client)
    let label = vec![];

    check_popk(&ciphertext_bytes, &proof_bytes, &label)?;

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
    ////check_previous_votes(voter_id, &input, &hasura_transaction);

    // TODO signature must include more information
    let ballot_signature = signing_key.sign(input.content.as_bytes())?;
    let ballot_signature = ballot_signature.to_bytes().to_vec();
    let insert = insert(
        tenant_id,
        election_event_id,
        election_id,
        area_id,
        &input.content,
        voter_id,
        &input.ballot_id,
        ballot_signature,
        &hasura_transaction,
    );
    //insert(&input, voter_id, ballot_signature, &hasura_transaction);

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
            electoral_log
                .post_cast_vote(
                    input.election_event_id.to_string(),
                    Some(input.election_id.to_string()),
                    pseudonym_h,
                    vote_h,
                )
                .await
                .with_context(|| "Error posting to the electoral log")?;
            Ok(result)
        }
        Err(e) => {
            // TODO error message may leak implementation details
            electoral_log
                .post_cast_vote_error(
                    input.election_event_id.to_string(),
                    Some(input.election_id.to_string()),
                    pseudonym_h,
                    e.to_string(),
                )
                .await
                .with_context(|| "Error posting to the electoral log")?;
            Err(e)
        }
    }
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

    let status = election_event
        .status
        .clone()
        .ok_or(anyhow!("Could not retrieve election event status"))?;
    let status: ElectionEventStatus =
        serde_json::from_value(status).context("Failed to deserialize election event status")?;
    if status.voting_status != VotingStatus::OPEN {
        return Err(anyhow!("Election event voting status is not open"));
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

    let status = election
        .status
        .clone()
        .ok_or(anyhow!("Could not retrieve election status"))?;
    let status: ElectionStatus =
        serde_json::from_value(status).context("Failed to deserialize election status")?;
    if status.voting_status != VotingStatus::OPEN {
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
    // TODO
    let max_revotes = 1;

    let result = postgres::cast_vote::get_cast_votes(
        &hasura_transaction,
        &Uuid::parse_str(tenant_id)?,
        &Uuid::parse_str(election_event_id)?,
        &Uuid::parse_str(election_id)?,
        &Uuid::parse_str(area_id)?,
        voter_id_string,
    )
    .await?;

    let (same, other): (Vec<Uuid>, Vec<Uuid>) = result
        .into_iter()
        .map(|(_, _, area_id)| area_id)
        .partition(|area_id| area_id.to_string() == area_id.to_string());

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

#[instrument(skip(ballot_signature, hasura_transaction), err)]
async fn insert(
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    area_id: &str,
    content: &str,
    voter_id: &str,
    ballot_id: &str,
    ballot_signature: Vec<u8>,
    hasura_transaction: &Transaction<'_>,
) -> anyhow::Result<InsertCastVoteOutput> {
    let (id, created_at) = postgres::cast_vote::insert_cast_vote(
        &hasura_transaction,
        &Uuid::parse_str(tenant_id)?,
        &Uuid::parse_str(election_event_id)?,
        &Uuid::parse_str(election_id)?,
        &Uuid::parse_str(area_id)?,
        content,
        voter_id,
        ballot_id,
        &ballot_signature,
    )
    .await?;

    let ret = InsertCastVoteOutput {
        id: id.to_string(),
        created_at: created_at,
        cast_ballot_signature: ballot_signature,
    };

    Ok(ret)
}

#[instrument(skip_all, err)]
fn check_popk(ciphertext_bytes: &[u8], proof_bytes: &[u8], label: &[u8]) -> Result<()> {
    let zkp = Zkp::new(&RistrettoCtx);
    let proof = Schnorr::<RistrettoCtx>::strand_deserialize(&proof_bytes)?;
    let ciphertext = Ciphertext::<RistrettoCtx>::strand_deserialize(&ciphertext_bytes)?;
    let popk_ok = zkp.encryption_popk_verify(&ciphertext.mhr, &ciphertext.gr, &proof, &label)?;

    if !popk_ok {
        return Err(anyhow!("Popk validation failed"));
    }

    Ok(())
}
