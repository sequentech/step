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
use sequent_core::services::openid;
use serde::{Deserialize, Serialize};
use strand::backend::ristretto::RistrettoCtx;
use strand::elgamal::Ciphertext;
use tracing::instrument;

use crate::hasura;
use crate::services::election_event_board::get_election_event_board;
use crate::services::protocol_manager::*;
use crate::types::task_error::into_task_error;

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct InsertBallotsPayload {
    pub trustee_pks: Vec<String>,
}

#[instrument]
#[celery::task]
pub async fn insert_ballots(
    body: InsertBallotsPayload,
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
    tally_session_contest_id: String,
) -> TaskResult<()> {
    let auth_headers = openid::get_client_credentials()
        .await
        .map_err(into_task_error)?;
    let tally_session_contest = get_tally_session_contest(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        tally_session_id.clone(),
        tally_session_contest_id.clone(),
    )
    .await
    .map_err(into_task_error)?;
    .data
    .expect("expected data".into())
    .sequent_backend_tally_session_contest[0];
    // fetch election_event
    let hasura_response = hasura::election_event::get_election_event(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await
    .map_err(into_task_error)?;
    let election_event = &hasura_response
        .data
        .expect("expected data".into())
        .sequent_backend_election_event[0];

    // check config is already created
    let status: Option<ElectionEventStatus> = match election_event.status.clone() {
        Some(value) => serde_json::from_value(value).map_err(into_task_error)?,
        None => None,
    };
    if !status
        .clone()
        .map(|val| val.is_config_created())
        .unwrap_or(false)
    {
        return Err(TaskError::UnexpectedError(
            "bulletin board config missing".into(),
        ));
    }
    if !status.map(|val| val.is_stopped()).unwrap_or(false) {
        return Err(TaskError::UnexpectedError(
            "election event is not stopped".into(),
        ));
    }

    let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
        .with_context(|| "missing bulletin board")
        .map_err(into_task_error)?;

    let cast_ballots_response = hasura::cast_ballot::find_ballots(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        area_id.clone(),
    )
    .await
    .map_err(into_task_error)?;

    let ballots_list = &cast_ballots_response
        .data
        .expect("expected data".into())
        .sequent_backend_cast_vote;

    let insertable_ballots: Vec<Ciphertext<RistrettoCtx>> = ballots_list
        .iter()
        .map(|ballot| {
            ballot
                .content
                .clone()
                .map(|ballot_str| {
                    let hashable_ballot: Option<HashableBallot<RistrettoCtx>> =
                        Base64Deserialize::deserialize(ballot_str).ok();
                    hashable_ballot
                        .map(|value| {
                            value
                                .contests
                                .iter()
                                .find(|contest| contest.contest_id == contest_id)
                                .map(|contest| contest.ciphertext.clone())
                        })
                        .flatten()
                })
                .flatten()
        })
        .filter(|ballot| ballot.is_some())
        .map(|ballot| ballot.clone().unwrap())
        .collect();

    add_ballots_to_board(board_name.as_str(), insertable_ballots, batch)
        .await
        .map_err(into_task_error)?;

    Ok(())
}
