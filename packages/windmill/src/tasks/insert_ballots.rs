// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Context;
use braid_messages::newtypes::BatchNumber;
use celery::error::TaskError;
use celery::prelude::*;
use sequent_core::ballot::ElectionEventStatus;
use sequent_core::ballot::HashableBallot;
use sequent_core::serialization::base64::Base64Deserialize;
use sequent_core::services::keycloak;
use serde::{Deserialize, Serialize};
use strand::backend::ristretto::RistrettoCtx;
use strand::elgamal::Ciphertext;
use strand::signature::StrandSignaturePk;
use tracing::{event, instrument, Level};

use crate::hasura;
use crate::hasura::tally_session_contest::get_tally_session_contest;
use crate::hasura::trustee::get_trustees_by_id;
use crate::services::election_event_board::get_election_event_board;
use crate::services::protocol_manager::*;
use crate::services::public_keys::deserialize_pk;
use crate::types::error::{Error, Result};

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct InsertBallotsPayload {
    pub trustee_pks: Vec<String>,
}

#[instrument]
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

    let trustees = get_trustees_by_id(
        auth_headers.clone(),
        tenant_id.clone(),
        body.trustee_pks.clone(),
    )
    .await?
    .data
    .with_context(|| "can't find trustees")?
    .sequent_backend_trustee;

    event!(Level::INFO, "trustees len: {:?}", trustees.len());

    // 4. create trustees keys from input strings
    let deserialized_trustee_pks: Vec<StrandSignaturePk> = trustees
        .clone()
        .into_iter()
        .map(|trustee| deserialize_pk(trustee.public_key.unwrap()))
        .collect();

    event!(
        Level::INFO,
        "deserialized_trustee_pks len: {:?}",
        deserialized_trustee_pks.len()
    );

    // check config is already created
    let status: Option<ElectionEventStatus> = match election_event.status.clone() {
        Some(value) => serde_json::from_value(value)?,
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

    let cast_ballots_response = hasura::cast_ballot::find_ballots(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        tally_session_contest.area_id.clone(),
    )
    .await?;

    let ballots_list = &cast_ballots_response
        .data
        .expect("expected data".into())
        .sequent_backend_cast_vote;

    event!(Level::INFO, "ballots_list len: {:?}", ballots_list.len());

    let insertable_ballots: Vec<Ciphertext<RistrettoCtx>> = ballots_list
        .iter()
        .map(|ballot| {
            ballot
                .content
                .clone()
                .map(|ballot_str| {
                    let hashable_ballot: HashableBallot<RistrettoCtx> =
                        Base64Deserialize::deserialize(ballot_str).unwrap();
                    hashable_ballot
                        .contests
                        .iter()
                        .find(|contest| contest.contest_id == tally_session_contest.contest_id)
                        .map(|contest| contest.ciphertext.clone())
                })
                .flatten()
        })
        .filter(|ballot| ballot.is_some())
        .map(|ballot| ballot.clone().unwrap())
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
