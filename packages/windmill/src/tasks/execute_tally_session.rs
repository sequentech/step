// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use braid_messages::{
    artifact::Plaintexts, message::Message, statement::Statement, statement::StatementType,
};
use celery::prelude::TaskError;
use sequent_core::ballot::{BallotStyle, Contest, ContestPresentation};
use sequent_core::ballot_codec::{BigUIntCodec, PlaintextCodec};
use sequent_core::services::keycloak;
use strand::{backend::ristretto::RistrettoCtx, serialization::StrandDeserialize};
use tracing::{event, instrument, Level};

use crate::hasura;
use crate::services::celery_app::get_celery_app;
use crate::services::election_event_board::get_election_event_board;
use crate::types::error;
use crate::{services::protocol_manager, types::error::Result};

#[instrument]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn execute_tally_session(
    election_event_id: String,
    tenant_id: String,
    tally_session_id: String,
) -> Result<()> {
    // get credentials
    let auth_headers = keycloak::get_client_credentials().await?;

    // fetch election_event
    let election_events = hasura::election_event::get_election_event(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await?
    .data
    .expect("expected data")
    .sequent_backend_election_event;

    // check election event is found
    if 0 == election_events.len() {
        event!(
            Level::INFO,
            "Election Event not found {}",
            election_event_id.clone()
        );
        return Ok(());
    }

    let election_event = &election_events[0];

    // get name of bulletin board
    let bulletin_board_opt =
        get_election_event_board(election_event.bulletin_board_reference.clone());

    if bulletin_board_opt.is_none() {
        event!(
            Level::INFO,
            "Election Event {} has no bulletin board",
            election_event_id.clone()
        );
        return Ok(());
    }

    let bulletin_board = bulletin_board_opt.unwrap();

    // get all data for the execution: the last tally session execution,
    // the list of tally_session_contest, and the ballot styles
    let tally_session_data = hasura::tally_session_execution::get_last_tally_session_execution(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        tally_session_id.clone(),
    )
    .await?
    .data
    .expect("expected data");

    // if the execution is completed, we don't need to do anything
    if tally_session_data.sequent_backend_tally_session[0].is_execution_completed {
        event!(Level::INFO, "Tally session execution is completed",);
        return Ok(());
    }

    // get last message id
    let last_message_id = if tally_session_data
        .sequent_backend_tally_session_execution
        .len()
        > 0
    {
        tally_session_data.sequent_backend_tally_session_execution[0].current_message_id
    } else {
        -1
    };

    // get board messages
    let mut board_client = protocol_manager::get_board_client().await?;
    let board_messages = board_client.get_messages(&bulletin_board, -1).await?;

    // find a new board message
    let next_new_board_message_opt = board_messages
        .iter()
        .find(|board_message| board_message.id > last_message_id);

    if next_new_board_message_opt.is_none() {
        event!(Level::INFO, "Board has no new messages",);
        return Ok(());
    }

    // find the timestamp of the new board message.
    // We do this because once we convert into a Message, we lose the link to the board message id
    let next_timestamp =
        Message::strand_deserialize(&next_new_board_message_opt.clone().unwrap().message)?
            .statement
            .get_timestamp();

    // get the batch ids that are linked to this tally session
    let batch_ids = tally_session_data
        .sequent_backend_tally_session_contest
        .iter()
        .map(|tsc| tsc.session_id)
        .collect::<Vec<_>>();

    // convert board messages into messages
    let messages: Vec<Message> = protocol_manager::convert_board_messages(&board_messages)?;

    // find if there are new plaintexs (= with equal/higher timestamp) that have the batch ids we need
    let has_next_plaintext = messages
        .iter()
        .find(|message| {
            message.statement.get_timestamp() >= next_timestamp
                && message.statement.get_kind() == StatementType::Plaintexts
                && batch_ids.contains(&(message.statement.get_batch_number() as i64))
        })
        .is_some();

    if !has_next_plaintext {
        event!(Level::INFO, "Board has no new relevant plaintexs");
        return Ok(());
    }

    // find all plaintexs (even with lower ids/timestamps) for this tally session/batch ids
    let relevant_plaintexts: Vec<&Message> = messages
        .iter()
        .filter(|message| {
            message.statement.get_kind() == StatementType::Plaintexts
                && batch_ids.contains(&(message.statement.get_batch_number() as i64))
        })
        .collect();

    // get ballot styles, from where we'll get the Contest(s)
    let ballot_styles: Vec<BallotStyle> = tally_session_data
        .sequent_backend_ballot_style
        .iter()
        .map(|ballot_style_row| {
            let ballot_style_res: Result<BallotStyle, error::Error> = serde_json::from_str(
                ballot_style_row
                    .ballot_eml
                    .clone()
                    .unwrap_or("".into())
                    .as_str(),
            )
            .map_err(|error| error.into());
            ballot_style_res
        })
        .collect::<Result<Vec<BallotStyle>>>()?;

    // map plaintexts to contests
    let plaintexts_data: Vec<_> = relevant_plaintexts
        .iter()
        .map(|plaintexts_message| {
            plaintexts_message.artifact.clone().map(|artifact| {
                let plaintexts = Plaintexts::<RistrettoCtx>::strand_deserialize(&artifact)
                    .ok()
                    .map(|plaintexts| plaintexts.0 .0);
                let batch_num = plaintexts_message.statement.get_batch_number();
                let tally_session_contest_opt = tally_session_data
                    .sequent_backend_tally_session_contest
                    .iter()
                    .find(|tsc| tsc.session_id == batch_num as i64);
                let contest = tally_session_contest_opt
                    .map(|tally_session_contest| {
                        ballot_styles
                            .iter()
                            .find(|ballot_style| {
                                ballot_style.area_id == tally_session_contest.area_id
                            })
                            .map(|ballot_style| {
                                ballot_style
                                    .contests
                                    .iter()
                                    .find(|contest| contest.id == tally_session_contest.contest_id)
                            })
                            .flatten()
                    })
                    .flatten();
                (plaintexts, tally_session_contest_opt, contest)
            })
        })
        .filter_map(|s| s)
        .filter_map(|s| {
            let (plaintexts_opt, tally_session_contest_opt, contest_opt) = s;
            if plaintexts_opt.is_some()
                && tally_session_contest_opt.is_some()
                && contest_opt.is_some()
            {
                Some((
                    plaintexts_opt.unwrap(),
                    tally_session_contest_opt.unwrap(),
                    contest_opt.unwrap(),
                ))
            } else {
                None
            }
        })
        .collect();

    let _paths = plaintexts_data
        .iter()
        .map(|area_contest_plaintext| {
            let (plaintexts, tally_session_contest, contest) = area_contest_plaintext;

            // here if you need it
            let _area_id = tally_session_contest.area_id.clone();
            let _contest_id = contest.id.clone();
            let _election_id = contest.election_id.clone();
            let _election_event_id = contest.election_event_id.clone();

            let biguit_ballots = plaintexts
                .iter()
                .map(|plaintext| {
                    // TODO: handle unwraps
                    let biguint = contest
                        .decode_plaintext_contest_to_biguint(plaintext)
                        .unwrap();

                    // Testing decoded ballots here: to be removed
                    let _decoded_ballot =
                        contest.decode_plaintext_contest_bigint(&biguint).unwrap();
                    biguint.to_str_radix(10)
                })
                .collect::<Vec<_>>();

            let path = PathBuf::from("/tmp/ballots.csv");
            let mut file = File::create(path.clone()).expect("Could not create file");
            let buffer = biguit_ballots.join("\n").into_bytes();
            file.write_all(&buffer).expect("Cannot written to file");
            path
        })
        .collect::<Vec<_>>();

    Ok(())
}
