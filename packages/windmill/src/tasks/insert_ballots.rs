// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context};
use board_messages::braid::newtypes::BatchNumber;
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use sequent_core::ballot::ElectionEventStatus;
use sequent_core::ballot::HashableBallot;
use sequent_core::serialization::base64::Base64Deserialize;
use sequent_core::serialization::deserialize_with_path::*;
use sequent_core::services::keycloak;
use serde::{Deserialize, Serialize};
use strand::backend::ristretto::RistrettoCtx;
use strand::elgamal::Ciphertext;
use strand::signature::StrandSignaturePk;
use tracing::{event, instrument, Level};

use crate::hasura;
use crate::hasura::tally_session_contest::get_tally_session_contest;
use crate::hasura::trustee::get_trustees_by_name;
use crate::services::cast_votes::{find_area_ballots, CastVote};
use crate::services::database::get_hasura_pool;
use crate::services::election_event_board::get_election_event_board;
use crate::services::protocol_manager::*;
use crate::services::public_keys::deserialize_public_key;
use crate::types::error::{Error, Result};

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct InsertBallotsPayload {
    pub trustee_names: Vec<String>,
}

fn deserialize_cast_vote(
    ballot: &CastVote,
    contest_id: String,
) -> Result<Ciphertext<RistrettoCtx>> {
    let ballot_str = ballot
        .content
        .clone()
        .ok_or(anyhow!("Missing ballot string"))?;

    let hashable_ballot: HashableBallot = serde_json::from_str(&ballot_str)?;
    let contests = hashable_ballot
        .deserialize_contests()
        .map_err(|err| anyhow!("{:?}", err))?;
    let contest = contests
        .iter()
        .find(|contest| contest.contest_id == contest_id)
        .ok_or(anyhow!("Can't find contest in ballot"))?;

    Ok(contest.ciphertext.clone())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn insert_ballots(
    body: InsertBallotsPayload,
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
    tally_session_contest_id: String,
) -> Result<()> {
    let auth_headers = keycloak::get_client_credentials().await?;
    let tally_session_contest = &get_tally_session_contest(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        tally_session_id.clone(),
        tally_session_contest_id.clone(),
    )
    .await?
    .data
    .expect("expected data".into())
    .sequent_backend_tally_session_contest[0];
    // fetch election_event
    let hasura_response = hasura::election_event::get_election_event(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await?;
    let election_event = &hasura_response
        .data
        .expect("expected data".into())
        .sequent_backend_election_event[0];

    let trustees = get_trustees_by_name(&auth_headers, &tenant_id, &body.trustee_names)
        .await?
        .data
        .with_context(|| "can't find trustees")?
        .sequent_backend_trustee;

    event!(Level::INFO, "trustees len: {:?}", trustees.len());

    // 4. create trustees keys from input strings
    let deserialized_trustee_pks: Vec<StrandSignaturePk> = trustees
        .clone()
        .into_iter()
        .map(|trustee| deserialize_public_key(trustee.public_key.unwrap()))
        .collect();

    event!(
        Level::INFO,
        "deserialized_trustee_pks len: {:?}",
        deserialized_trustee_pks.len()
    );

    // check config is already created
    let status: Option<ElectionEventStatus> = match election_event.status.clone() {
        Some(value) => deserialize_value(value)?,
        None => None,
    };
    if !status
        .clone()
        .map(|val| val.is_config_created())
        .unwrap_or(false)
    {
        return Err(Error::String("bulletin board config missing".into()));
    }
    /*if !status.map(|val| val.is_stopped()).unwrap_or(false) {
        return Err(Error::String("election event is not stopped".into()));
    }*/

    let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
        .with_context(|| "missing bulletin board")?;

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .with_context(|| "Error acquiring hasura db client")?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .with_context(|| "Error acquiring hasura transaction")?;

    let ballots_list = find_area_ballots(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &tally_session_contest.area_id,
    )
    .await?;

    event!(Level::INFO, "ballots_list len: {:?}", ballots_list.len());

    let deserialized_ballots: Vec<(CastVote, Result<Ciphertext<RistrettoCtx>>)> = ballots_list
        .clone()
        .into_iter()
        .map(|ballot| -> (CastVote, Result<Ciphertext<RistrettoCtx>>) {
            let ciphertext =
                deserialize_cast_vote(&ballot, tally_session_contest.contest_id.clone());
            (ballot, ciphertext)
        })
        .collect();

    let (insertable_ballots, ballot_errors): (Vec<_>, Vec<_>) = deserialized_ballots
        .into_iter()
        .partition(|element| element.1.is_ok());


    // Vec<(id, error)>
    let ballot_errors: Vec<(Uuid, String)> = ballot_errors
        .into_iter()
        .map(|element| -> Result<(Uuid, String)> {
            let error = format!("#{:?}", element.1.unwrap_err());
            let id_uuid = Uuid::parse_str(element.0.id)?;
            Ok((id_uuid, error))
        })
        .collect()?;

    insert_tally_session_contest_vote_error(
        &hasura_transaction,
        Uuid::parse_str(tally_session_contest.tenant_id)?,
        Uuid::parse_str(tally_session_contest.election_event_id)?,
        Uuid::parse_str(tally_session_contest.contest_id)?,
        Uuid::parse_str(tally_session_contest.tally_session_id)?,
        Uuid::parse_str(tally_session_contest.area_id)?,
        Uuid::parse_str(tally_session_contest.id)?,
        &ballot_errors
    ).await?;

    let insertable_ballots: Vec<Ciphertext<RistrettoCtx>> = insertable_ballots
        .into_iter()
        .map(|element| element.1.unwrap())
        .collect();

    let batch = tally_session_contest.session_id.clone() as BatchNumber;
    add_ballots_to_board(
        board_name.as_str(),
        insertable_ballots,
        batch,
        deserialized_trustee_pks,
    )
    .await?;

    Ok(())
}
