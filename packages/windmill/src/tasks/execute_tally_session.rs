// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>, Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura;
use crate::hasura::results_area_contest::insert_results_area_contest;
use crate::hasura::results_area_contest_candidate::insert_results_area_contest_candidate;
use crate::hasura::results_contest::insert_results_contest;
use crate::hasura::results_contest_candidate::insert_results_contest_candidate;
use crate::hasura::results_election::insert_results_election;
use crate::hasura::results_event::insert_results_event;
use crate::hasura::tally_session::set_tally_session_completed;
use crate::hasura::tally_session_execution::get_last_tally_session_execution::{
    GetLastTallySessionExecutionSequentBackendTallySessionContest, ResponseData,
};
use crate::hasura::tally_session_execution::{
    get_last_tally_session_execution, insert_tally_session_execution,
};
use crate::services::compress::compress_folder;
use crate::services::documents::upload_and_return_document;
use crate::services::election_event_board::get_election_event_board;
use crate::services::pg_lock::PgLock;
use crate::services::protocol_manager;
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context};
use braid_messages::{artifact::Plaintexts, message::Message, statement::StatementType};
use celery::prelude::TaskError;
use chrono::{Duration, Utc};
use sequent_core::ballot::{BallotStyle, Contest};
use sequent_core::ballot_codec::PlaintextCodec;
use sequent_core::services::connection;
use sequent_core::services::keycloak;
use sequent_core::types::ceremonies::TallyExecutionStatus;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
use std::string::ToString;
use strand::{backend::ristretto::RistrettoCtx, context::Ctx, serialization::StrandDeserialize};
use tempfile::tempdir;
use tracing::{event, instrument, Level};
use uuid::Uuid;
use velvet::cli::state::State;
use velvet::cli::CliRun;
use velvet::fixtures;
use velvet::pipes::generate_reports::ElectionReportDataComputed;

type AreaContestDataType = (
    Vec<<RistrettoCtx as Ctx>::P>,
    GetLastTallySessionExecutionSequentBackendTallySessionContest,
    Contest,
    BallotStyle,
);

#[instrument(skip_all)]
fn get_ballot_styles(tally_session_data: &ResponseData) -> Result<Vec<BallotStyle>> {
    // get ballot styles, from where we'll get the Contest(s)
    tally_session_data
        .sequent_backend_ballot_style
        .iter()
        .map(|ballot_style_row| {
            let ballot_style_res: Result<BallotStyle, Error> = serde_json::from_str(
                ballot_style_row
                    .ballot_eml
                    .clone()
                    .unwrap_or("".into())
                    .as_str(),
            )
            .map_err(|error| error.into());
            ballot_style_res
        })
        .collect::<Result<Vec<BallotStyle>>>()
}

#[instrument(skip_all)]
fn process_plaintexts(
    relevant_plaintexts: Vec<&Message>,
    ballot_styles: Vec<BallotStyle>,
    tally_session_data: ResponseData,
) -> Vec<AreaContestDataType> {
    relevant_plaintexts
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
        .filter_map(|s| match s {
            (Some(plaintexts), Some(tally_session_contest), Some(contest), Some(ballots_style)) => {
                Some((
                    plaintexts,
                    tally_session_contest.clone(),
                    contest.clone(),
                    ballots_style.clone(),
                ))
            }
            _ => None,
        })
        .collect()
}

#[instrument]
fn get_execution_status(execution_status: Option<String>) -> Option<TallyExecutionStatus> {
    let Some(execution_status_str) = execution_status.clone() else {
        event!(Level::INFO, "Missing execution status");

        return None;
    };
    let Some(execution_status) = TallyExecutionStatus::from_str(&execution_status_str).ok() else {
        event!(Level::INFO, "Tally session can't continue the tally with unexpected execution status {}", execution_status_str);

        return None;
    };
    let valid_status: Vec<TallyExecutionStatus> = vec![
        TallyExecutionStatus::CONNECTED,
        TallyExecutionStatus::IN_PROGRESS,
    ];
    if !valid_status.contains(&execution_status) {
        event!(
            Level::INFO,
            "Tally session can't continue the tally with unexpected execution status {}",
            execution_status_str
        );

        return None;
    };
    Some(execution_status)
}

#[instrument(skip_all)]
async fn map_plaintext_data(
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
) -> Result<Option<(Vec<AreaContestDataType>, i64, bool)>> {
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

        return Ok(None);
    }

    let election_event = &election_events[0];

    // get name of bulletin board
    let bulletin_board_opt =
        get_election_event_board(election_event.bulletin_board_reference.clone());

    let Some(bulletin_board) = bulletin_board_opt else {
        event!(
            Level::INFO,
            "Election Event {} has no bulletin board",
            election_event_id.clone()
        );

        return Ok(None);
    };

    // get all data for the execution: the last tally session execution,
    // the list of tally_session_contest, and the ballot styles
    let tally_session_data = get_last_tally_session_execution(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        tally_session_id.clone(),
    )
    .await?
    .data
    .expect("expected data");

    let Some(_execution_status) = get_execution_status(tally_session_data.sequent_backend_tally_session[0].execution_status.clone()) else {
        return Ok(None);
    };

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
    event!(Level::INFO, "Num board_messages {}", board_messages.len());

    // find a new board message
    let next_new_board_message_opt = board_messages
        .iter()
        .find(|board_message| board_message.id > last_message_id);

    let newest_message_id = board_messages
        .last()
        .map(|board_message| board_message.id)
        .unwrap_or(-1);

    if next_new_board_message_opt.is_none() {
        event!(Level::INFO, "Board has no new messages",);
        return Ok(None);
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
    event!(Level::INFO, "Num batch_ids {}", batch_ids.len());

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
        return Ok(None);
    }

    // get ballot styles, from where we'll get the Contest(s)
    let ballot_styles: Vec<BallotStyle> = get_ballot_styles(&tally_session_data)?;
    event!(Level::INFO, "Num ballot_styles {}", ballot_styles.len());

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
        "Num relevant_plaintexts {}",
        relevant_plaintexts.len()
    );
    // we have all plaintexts
    let is_execution_completed = relevant_plaintexts.len() == batch_ids.len();

    let plaintexts_data: Vec<AreaContestDataType> =
        process_plaintexts(relevant_plaintexts, ballot_styles, tally_session_data);
    Ok(Some((
        plaintexts_data,
        newest_message_id,
        is_execution_completed,
    )))
}

#[instrument(skip_all)]
async fn save_results(
    results: Vec<ElectionReportDataComputed>,
    tenant_id: &str,
    election_event_id: &str,
    results_event_id: &str,
) -> Result<()> {
    let auth_headers = keycloak::get_client_credentials().await?;
    for election in &results {
        insert_results_election(
            &auth_headers,
            tenant_id,
            election_event_id,
            results_event_id,
            &election.election_id,
            &None, // name
            &None, // elegible_census,
            &None, // total_valid_votes,
            &None, // explicit_invalid_votes,
            &None, // implicit_invalid_votes,
            &None, // blank_votes,
        )
        .await?;

        for contest in &election.reports {
            if let Some(area_id) = &contest.area_id {
                insert_results_area_contest(
                    &auth_headers,
                    tenant_id,
                    election_event_id,
                    &election.election_id,
                    &contest.contest.id,
                    area_id,
                    results_event_id,
                    None, // elegible_census
                    Some(contest.contest_result.total_votes as i64),
                    // missing total valid votes
                    Some(contest.contest_result.total_invalid_votes as i64),
                    None, // implicit_invalid_votes
                    None, // blank_votes
                    contest.contest.voting_type.clone(),
                    contest.contest.counting_algorithm.clone(),
                )
                .await?;

                for candidate in &contest.candidate_result {
                    insert_results_area_contest_candidate(
                        &auth_headers,
                        tenant_id,
                        election_event_id,
                        &election.election_id,
                        &contest.contest.id,
                        area_id,
                        &candidate.candidate.id,
                        results_event_id,
                        Some(candidate.total_count as i64),
                        candidate.winning_position.map(|val| val as i64),
                        None, // points
                    )
                    .await?;
                }
            } else {
                insert_results_contest(
                    &auth_headers,
                    tenant_id,
                    election_event_id,
                    &election.election_id,
                    &contest.contest.id,
                    results_event_id,
                    None, // elegible_census
                    Some(contest.contest_result.total_votes as i64),
                    // missing total valid votes
                    Some(contest.contest_result.total_invalid_votes as i64),
                    None, // implicit_invalid_votes
                    None, // blank_votes
                    contest.contest.voting_type.clone(),
                    contest.contest.counting_algorithm.clone(),
                    contest.contest.name.clone(),
                )
                .await?;

                for candidate in &contest.candidate_result {
                    insert_results_contest_candidate(
                        &auth_headers,
                        tenant_id,
                        election_event_id,
                        &election.election_id,
                        &contest.contest.id,
                        &candidate.candidate.id,
                        results_event_id,
                        Some(candidate.total_count as i64),
                        candidate.winning_position.map(|val| val as i64),
                        None, // points
                    )
                    .await?;
                }
            }
        }
    }
    Ok(())
}

#[instrument(skip_all)]
async fn tally_area_contest(
    area_contest_plaintext: AreaContestDataType,
    base_tempdir: PathBuf,
    results_event_id: &str,
) -> Result<()> {
    let (plaintexts, tally_session_contest, contest, ballot_style) = area_contest_plaintext;

    let area_id = tally_session_contest.area_id.clone();
    let contest_id = contest.id.clone();
    let election_id = contest.election_id.clone();

    let biguit_ballots = plaintexts
        .iter()
        .filter_map(|plaintext| {
            let biguint = contest.decode_plaintext_contest_to_biguint(plaintext);

            match biguint {
                Ok(v) => {
                    let biguit_str = v.to_str_radix(10);
                    event!(Level::INFO, "Decoded biguint {biguit_str}");

                    Some(biguit_str)
                }
                Err(e) => {
                    event!(Level::WARN, "Decoding plaintext has failed: {e}");
                    None
                }
            }
        })
        .collect::<Vec<_>>();

    let velvet_input_dir = base_tempdir.join("input");
    let velvet_output_dir = base_tempdir.join("output");

    //// create ballots
    let ballots_path = velvet_input_dir.join(format!(
        "default/ballots/election__{election_id}/contest__{contest_id}/area__{area_id}"
    ));
    fs::create_dir_all(&ballots_path).map_err(|e| Error::FileAccess(ballots_path.clone(), e))?;

    let csv_ballots_path = ballots_path.join("ballots.csv");
    let mut csv_ballots_file = File::create(&csv_ballots_path)
        .map_err(|e| Error::FileAccess(csv_ballots_path.clone(), e))?;
    let buffer = biguit_ballots.join("\n").into_bytes();

    csv_ballots_file
        .write_all(&buffer)
        .map_err(|e| Error::FileAccess(csv_ballots_path.clone(), e))?;

    //// create velvet config
    let velvet_path_config: PathBuf = velvet_input_dir.join("config.json");
    let mut config_file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(&velvet_path_config)
        .map_err(|e| Error::FileAccess(velvet_path_config.clone(), e))?;

    writeln!(
        config_file,
        "{}",
        serde_json::to_string(&fixtures::get_config())?
    )
    .map_err(|e| Error::FileAccess(velvet_path_config.clone(), e))?;

    //// create area folder
    let area_path: PathBuf = velvet_input_dir.join(format!(
        "default/configs/election__{election_id}/contest__{contest_id}/area__{area_id}"
    ));
    fs::create_dir_all(&area_path).map_err(|e| Error::FileAccess(area_path.clone(), e))?;

    //// create contest config file
    let ballot_style_path: PathBuf = velvet_input_dir.join(format!(
        "default/configs/election__{election_id}/election-config.json"
    ));
    let mut ballot_style_file = fs::File::create(&ballot_style_path)
        .map_err(|e| Error::FileAccess(ballot_style_path.clone(), e))?;

    writeln!(
        ballot_style_file,
        "{}",
        serde_json::to_string(&ballot_style)?
    )
    .map_err(|e| Error::FileAccess(ballot_style_path.clone(), e))?;

    //// create contest config file
    let contest_config_path: PathBuf = velvet_input_dir.join(format!(
        "default/configs/election__{election_id}/contest__{contest_id}/contest-config.json"
    ));
    let mut contest_config_file = fs::File::create(contest_config_path)
        .map_err(|e| Error::FileAccess(ballot_style_path.clone(), e))?;

    writeln!(contest_config_file, "{}", serde_json::to_string(&contest)?)
        .map_err(|e| Error::FileAccess(ballot_style_path.clone(), e))?;

    //// Run Velvet
    let cli = CliRun {
        stage: "main".to_string(),
        pipe_id: "decode-ballots".to_string(),
        config: velvet_path_config,
        input_dir: velvet_input_dir,
        output_dir: velvet_output_dir,
    };

    let config = cli.validate().map_err(|e| Error::String(e.to_string()))?;

    let mut state = State::new(&cli, &config).map_err(|e| Error::String(e.to_string()))?;

    while let Some(next_stage) = state.get_next() {
        let stage_name = next_stage.to_string();
        event!(Level::INFO, "Exec {}", stage_name);
        state.exec_next().map_err(|e| {
            Error::String(format!("Error during {}: {}", stage_name, e.to_string()))
        })?;
    }
    if let Ok(results) = state.get_results() {
        save_results(
            results,
            &contest.tenant_id,
            &contest.election_event_id,
            results_event_id,
        )
        .await?;
    }

    Ok(())
}

#[instrument(skip(auth_headers))]
async fn create_results_event(
    auth_headers: &connection::AuthHeaders,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<String> {
    let results_event = &insert_results_event(auth_headers, &tenant_id, &election_event_id)
        .await?
        .data
        .with_context(|| "can't find results_event")?
        .insert_sequent_backend_results_event
        .with_context(|| "can't find results_event")?
        .returning[0];

    Ok(results_event.id.clone())
}

#[instrument]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 120000)]
pub async fn execute_tally_session(
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
) -> Result<()> {
    let auth_headers = keycloak::get_client_credentials().await?;
    let lock = PgLock::acquire(
        auth_headers.clone(),
        format!(
            "execute_tally_session-{}-{}-{}",
            tenant_id, election_event_id, tally_session_id
        ),
        Uuid::new_v4().to_string(),
        Some(Utc::now().naive_utc() + Duration::seconds(120)),
    )
    .await?;
    // map plaintexts to contests
    let plaintexts_data_opt = map_plaintext_data(
        tenant_id.clone(),
        election_event_id.clone(),
        tally_session_id.clone(),
    )
    .await?;

    if plaintexts_data_opt.is_none() {
        lock.release(auth_headers.clone()).await?;
        return Ok(());
    }

    let (plaintexts_data, newest_message_id, is_execution_completed) = plaintexts_data_opt.unwrap();

    event!(Level::INFO, "Num plaintexts_data {}", plaintexts_data.len());

    // base temp folder
    let base_tempdir = tempdir()?;

    let results_event_id =
        create_results_event(&auth_headers, &tenant_id, &election_event_id).await?;

    // perform tallies with velvet
    for area_contest_plaintext in plaintexts_data.iter() {
        tally_area_contest(
            area_contest_plaintext.clone(),
            base_tempdir.path().to_path_buf(),
            &results_event_id,
        )
        .await
        .map_err(|err| {
            event!(Level::ERROR, "Tally area contest: {err}");
            err
        })?;
    }
 
    // compressed file with the tally
    let data = compress_folder(base_tempdir.path())?;

    // get credentials
    // map_plaintext_data also calls this but at this point the credentials
    // could be expired
    let auth_headers = keycloak::get_client_credentials().await?;

    // upload binary data into a document (s3 and hasura)
    let document = upload_and_return_document(
        data,
        "application/gzip".to_string(),
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        "tally.tar.gz".into(),
    )
    .await?;

    // insert tally_session_execution
    insert_tally_session_execution(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        newest_message_id,
        tally_session_id.clone(),
        Some(document.id.clone()),
        None,
        None,
    )
    .await?;

    if is_execution_completed {
        // update tally session to flag it as completed
        set_tally_session_completed(
            auth_headers.clone(),
            tenant_id.clone(),
            election_event_id.clone(),
            tally_session_id.clone(),
        )
        .await?;
    }
    lock.release(auth_headers.clone()).await?;

    Ok(())
}
