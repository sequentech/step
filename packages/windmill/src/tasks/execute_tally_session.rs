// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

use braid_messages::{artifact::Plaintexts, message::Message, statement::StatementType};
use celery::prelude::TaskError;
use sequent_core::ballot::{BallotStyle, Contest, ContestPresentation};
use sequent_core::ballot_codec::{BigUIntCodec, PlaintextCodec};
use sequent_core::services::keycloak;
use strand::{backend::ristretto::RistrettoCtx, serialization::StrandDeserialize};
use tracing::{event, instrument, Level};
use velvet::cli::state::State;

use crate::hasura;
use crate::services::election_event_board::get_election_event_board;
use crate::types::error;
use crate::{services::protocol_manager, types::error::Result};
use velvet::cli::{self, CliRun};
use velvet::fixtures;

#[instrument]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn execute_tally_session(
    tenant_id: String,
    election_event_id: String,
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
    if election_events.is_empty() {
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
    let last_message_id = if !tally_session_data
        .sequent_backend_tally_session_execution
        .is_empty()
    {
        tally_session_data.sequent_backend_tally_session_execution[0].current_message_id
    } else {
        -1
    };

    // get board messages
    let mut board_client = protocol_manager::get_board_client().await?;
    let board_messages = board_client.get_messages(&bulletin_board, -1).await?;
    event!(
        Level::INFO,
        "FF 1 num board_messages {}",
        board_messages.len()
    );

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
    let next_timestamp = Message::strand_deserialize(&next_new_board_message_opt.unwrap().message)?
        .statement
        .get_timestamp();

    // get the batch ids that are linked to this tally session
    let batch_ids = tally_session_data
        .sequent_backend_tally_session_contest
        .iter()
        .map(|tsc| tsc.session_id)
        .collect::<Vec<_>>();
    event!(Level::INFO, "FF 2 num batch_ids {}", batch_ids.len());

    // convert board messages into messages
    let messages: Vec<Message> = protocol_manager::convert_board_messages(&board_messages)?;

    // find if there are new plaintexs (= with equal/higher timestamp) that have the batch ids we need
    let has_next_plaintext = messages.iter().any(|message| {
        message.statement.get_timestamp() >= next_timestamp
            && message.statement.get_kind() == StatementType::Plaintexts
            && batch_ids.contains(&(message.statement.get_batch_number() as i64))
    });

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
    event!(
        Level::INFO,
        "FF 3 num relevant_plaintexts {}",
        relevant_plaintexts.len()
    );

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
    event!(
        Level::INFO,
        "FF 4 num ballot_styles {}",
        ballot_styles.len()
    );

    // map plaintexts to contests
    let plaintexts_data: Vec<_> = relevant_plaintexts
        .iter()
        .filter_map(|plaintexts_message| {
            plaintexts_message.artifact.clone().map(|artifact| {
                let plaintexts = Plaintexts::<RistrettoCtx>::strand_deserialize(&artifact)
                    .ok()
                    .map(|plaintexts| plaintexts.0 .0);

                let batch_num = plaintexts_message.statement.get_batch_number();

                let tally_session_contest_opt = tally_session_data
                    .sequent_backend_tally_session_contest
                    .iter()
                    .find(|tsc| tsc.session_id == batch_num as i64);

                let contest = tally_session_contest_opt.and_then(|tally_session_contest| {
                    ballot_styles
                        .iter()
                        .find(|ballot_style| ballot_style.area_id == tally_session_contest.area_id)
                        .and_then(|ballot_style| {
                            ballot_style
                                .contests
                                .iter()
                                .find(|contest| contest.id == tally_session_contest.contest_id)
                        })
                });

                let ballot_style_opt = ballot_styles.iter().find(|b| {
                    if let Some(tally_session_contest) = tally_session_contest_opt {
                        if b.contests
                            .iter()
                            .any(|c| c.id == tally_session_contest.contest_id)
                        {
                            return true;
                        }
                        return false;
                    }

                    false
                });

                (
                    plaintexts,
                    tally_session_contest_opt,
                    contest,
                    ballot_style_opt,
                )
            })
        })
        .filter_map(|s| {
            let (plaintexts_opt, tally_session_contest_opt, contest_opt, ballot_style_opt) = s;
            if plaintexts_opt.is_some()
                && tally_session_contest_opt.is_some()
                && contest_opt.is_some()
                && ballot_style_opt.is_some()
            {
                Some((
                    plaintexts_opt.unwrap(),
                    tally_session_contest_opt.unwrap(),
                    contest_opt.unwrap(),
                    ballot_style_opt.unwrap(),
                ))
            } else {
                None
            }
        })
        .collect();
    event!(
        Level::INFO,
        "FF 5 num plaintexts_data {}",
        plaintexts_data.len()
    );

    let _ = plaintexts_data
        .iter()
        .try_for_each(|area_contest_plaintext| {
            let (plaintexts, tally_session_contest, contest, ballot_style) = area_contest_plaintext;

            // here if you need it
            let area_id = tally_session_contest.area_id.clone();
            let contest_id = contest.id.clone();
            let election_id = contest.election_id.clone();
            let election_event_id = contest.election_event_id.clone();

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
                    event!(Level::INFO, "FF 6 biguint {}", biguint.to_str_radix(10));
                    biguint.to_str_radix(10)
                })
                .collect::<Vec<_>>();

            //// Velvet input output dirs
            let velvet_input_dir = PathBuf::from("/tmp/velvet/input");
            let velvet_output_dir = PathBuf::from("/tmp/velvet/output");

            //// create ballots
            let mut path = velvet_input_dir.clone();
            path.push("default");
            path.push("ballots");
            path.push(format!("election__{election_id}"));
            path.push(format!("contest__{contest_id}"));
            path.push(format!("region__{area_id}"));
            fs::create_dir_all(&path).expect("Could not create dir");

            let file = path.join("ballots.csv");
            let mut file = File::create(file).expect("Could not create file");
            let buffer = biguit_ballots.join("\n").into_bytes();

            file.write_all(&buffer).expect("Cannot written to file");

            //// create velvet config
            let velvet_path_config = PathBuf::from("/tmp/velvet/input/config.json");
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .open(&velvet_path_config)
                .expect("Could not open file");

            writeln!(
                file,
                "{}",
                serde_json::to_string(&fixtures::get_config()).unwrap()
            )
            .expect("Could not write in file");

            //// create contest config file
            let mut path = PathBuf::from("/tmp/velvet/input/default/configs");
            path.push(format!("election__{election_id}"));
            fs::create_dir_all(&path).expect("Could not create dir");
            path.push("election-config.json");
            let mut file = fs::File::create(path).expect("Couldnt create file");
            writeln!(file, "{}", serde_json::to_string(&ballot_style).unwrap())
                .expect("Could not write in file");

            //// create contest config file
            let mut path = PathBuf::from("/tmp/velvet/input/default/configs");
            path.push(format!("election__{election_id}"));
            path.push(format!("contest__{contest_id}"));
            fs::create_dir_all(&path).expect("Could not create dir");
            path.push("contest-config.json");
            let mut file = fs::File::create(&path).expect("Couldnt create file");
            writeln!(file, "{}", serde_json::to_string(&contest).unwrap())
                .expect("Could not write in file");

            //// create region folder
            let mut path = PathBuf::from("/tmp/velvet/input/default/configs");
            path.push(format!("election__{election_id}"));
            path.push(format!("contest__{contest_id}"));
            path.push(format!("region__{area_id}"));
            fs::create_dir_all(&path).expect("Could not create dir");

            //// Run Velvet
            let cli = CliRun {
                stage: "main".to_string(),
                pipe_id: "decode-ballots".to_string(),
                config: velvet_path_config,
                input_dir: velvet_input_dir,
                output_dir: velvet_output_dir,
            };

            let config = cli.validate().unwrap();

            let mut state = State::new(&cli, &config).unwrap();

            // DecodeBallots
            event!(Level::INFO, "Exec Decode Ballots");
            state.exec_next().unwrap();

            // Do Tally
            event!(Level::INFO, "Exec Do Tally");
            state.exec_next().unwrap();

            // mark winners
            event!(Level::INFO, "Exec Mark Winners");
            state.exec_next().unwrap();

            // report
            event!(Level::INFO, "Exec Reports");
            state.exec_next().unwrap();

            Ok::<(), TaskError>(())
        });

    event!(Level::INFO, "FF 7 num paths {}", plaintexts_data.len());

    // Missing: insert tally_session_execution in hasura
    Ok(())
}
