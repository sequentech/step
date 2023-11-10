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
        event!(Level::INFO, "Board has no new relevant plaintexs",);
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

    // TODO: fetch contest from Hasura
    let contest = Contest {
        id: "63b1-f93b-4151-93d6-bbe0ea5eac46 69f2f987-460c-48ac-ac7a-4d44d99b37e6".into(),
        tenant_id: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5".into(),
        election_event_id: "33f18502-a67c-4853-8333-a58630663559".into(),
        election_id: "f2f1065e-b784-46d1-b81a-c71bfeb9ad55".into(),
        name: Some("Secretario General".into()),
        description: Some(
            "Elige quien quieres que sea tu Secretario General en tu municipio".into(),
        ),
        max_votes: 1,
        min_votes: 0,
        winning_candidates_num: 1,
        voting_type: Some("first-past-the-post".into()),
        counting_algorithm: Some("plurality-at-large".into()),
        is_encrypted: true,
        candidates: vec![],
        presentation: Some(ContestPresentation {
            allow_writeins: false,
            base32_writeins: true,
            invalid_vote_policy: "allowed".into(),
            cumulative_number_of_checkboxes: None,
            shuffle_categories: true,
            shuffle_all_options: true,
            shuffle_category_list: None,
            show_points: false,
            enable_checkable_lists: None,
        }),
    };

    let encoded_ballots = messages
        .into_iter()
        .filter(|m| matches!(m.statement, Statement::Plaintexts(..)))
        .map(|m| {
            // TODO: handle unwraps
            let artifact = m.artifact.unwrap();
            let res = Plaintexts::<RistrettoCtx>::strand_deserialize(&artifact).unwrap();
            let res = res.0 .0;
            let biguit_ballots = contest
                .decode_plaintext_contest_to_biguint(&res[0])
                .unwrap();

            // Testing decoded ballots here: to be removed
            let decoded_ballots = contest.decode_plaintext_contest_bigint(&biguit_ballots);
            dbg!(&decoded_ballots);

            biguit_ballots.to_str_radix(10)
        })
        .collect::<Vec<_>>();

    let path = PathBuf::from("/tmp/ballots.csv");
    let mut file = File::create(path).expect("Could not create file");
    let buffer = encoded_ballots.join("\n").into_bytes();
    file.write_all(&buffer).expect("Cannot written to file");

    Ok(())
}
