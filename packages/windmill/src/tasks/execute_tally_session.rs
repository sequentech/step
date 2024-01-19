// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>, Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura;
use crate::hasura::election_event::get_election_event_helper;
use crate::hasura::election_event::update_election_event_status;
use crate::hasura::results_event::insert_results_event;
use crate::hasura::tally_session::set_tally_session_completed;
use crate::hasura::tally_session_execution::get_last_tally_session_execution::{
    GetLastTallySessionExecutionSequentBackendTallySessionContest, ResponseData,
};
use crate::hasura::tally_session_execution::{
    get_last_tally_session_execution, insert_tally_session_execution,
};
use crate::services::cast_votes::{count_cast_votes_election, ElectionCastVotes};
use crate::services::ceremonies::results::populate_results_tables;
use crate::services::ceremonies::serialize_logs::generate_logs;
use crate::services::ceremonies::serialize_logs::sort_logs;
use crate::services::ceremonies::tally_ceremony::find_last_tally_session_execution;
use crate::services::ceremonies::tally_ceremony::get_tally_ceremony_status;
use crate::services::ceremonies::tally_progress::generate_tally_progress;
use crate::services::ceremonies::velvet_tally::run_velvet_tally;
use crate::services::compress::compress_folder;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::date::ISO8601;
use crate::services::documents::upload_and_return_document;
use crate::services::election_event_board::get_election_event_board;
use crate::services::election_event_status::get_election_event_status;
use crate::services::pg_lock::PgLock;
use crate::services::protocol_manager;
use crate::services::users::list_users;
use crate::services::users::ListUsersFilter;
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context};
use board_messages::braid::{artifact::Plaintexts, message::Message, statement::StatementType};
use celery::prelude::TaskError;
use chrono::Duration;
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use sequent_core::ballot::{BallotStyle, Contest};
use sequent_core::services::connection;
use sequent_core::services::connection::AuthHeaders;
use sequent_core::services::keycloak;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::ceremonies::TallyCeremonyStatus;
use sequent_core::types::ceremonies::TallyExecutionStatus;
use std::str::FromStr;
use std::string::ToString;
use strand::{backend::ristretto::RistrettoCtx, context::Ctx, serialization::StrandDeserialize};
use tempfile::tempdir;
use tokio::time::{interval, Duration as ChronoDuration};
use tracing::{event, instrument, Level};
use uuid::Uuid;

type AreaContestDataType = (
    Vec<<RistrettoCtx as Ctx>::P>,
    GetLastTallySessionExecutionSequentBackendTallySessionContest,
    Contest,
    BallotStyle,
    u64,
);

#[instrument(skip_all, err)]
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
async fn process_plaintexts(
    auth_headers: AuthHeaders,
    relevant_plaintexts: Vec<&Message>,
    ballot_styles: Vec<BallotStyle>,
    tally_session_data: ResponseData,
) -> Result<Vec<AreaContestDataType>> {
    let almost_vec: Vec<AreaContestDataType> = relevant_plaintexts
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

                let ballot_style_opt =
                    if let Some(tally_session_contest) = tally_session_contest_opt {
                        ballot_styles.iter().find(|ballot_style| {
                            ballot_style.area_id == tally_session_contest.area_id
                                && ballot_style.election_id == tally_session_contest.election_id
                                && ballot_style
                                    .contests
                                    .iter()
                                    .any(|contest| contest.id == tally_session_contest.contest_id)
                        })
                    } else {
                        None
                    };

                let contest = if let Some(tally_session_contest) = tally_session_contest_opt {
                    ballot_style_opt
                        .map(|ballot_style| {
                            ballot_style
                                .contests
                                .iter()
                                .find(|contest| contest.id == tally_session_contest.contest_id)
                        })
                        .flatten()
                } else {
                    None
                };

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
                    0,
                ))
            }
            _ => None,
        })
        .collect();

    let mut keycloak_db_client: DbClient = get_keycloak_pool()
        .await
        .get()
        .await
        .with_context(|| "Error acquiring keycloak db client")?;
    let keycloak_transaction = keycloak_db_client
        .transaction()
        .await
        .with_context(|| "Error acquiring keycloak transaction")?;
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .with_context(|| "Error acquiring hasura db client")?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .with_context(|| "Error acquiring hasura transaction")?;

    let mut data: Vec<AreaContestDataType> = vec![];

    for almost in almost_vec {
        let (_plaintexts, tally_session_contest, contest, _ballots_style, _count) = almost.clone();
        let count = get_eligible_voters(
            auth_headers.clone(),
            &hasura_transaction,
            &keycloak_transaction,
            &contest.tenant_id,
            &contest.election_event_id,
            &contest.election_id,
            &tally_session_contest.area_id,
        )
        .await?;

        let mut with_count = almost.clone();
        with_count.4 = count;
        data.push(with_count);
    }
    keycloak_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")?;
    Ok(data)
}

#[instrument]
fn get_execution_status(execution_status: Option<String>) -> Option<TallyExecutionStatus> {
    let Some(execution_status_str) = execution_status.clone() else {
        event!(Level::INFO, "Missing execution status");

        return None;
    };
    let Some(execution_status) = TallyExecutionStatus::from_str(&execution_status_str).ok() else {
        event!(
            Level::INFO,
            "Tally session can't continue the tally with unexpected execution status {}",
            execution_status_str
        );

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

#[instrument(err)]
pub async fn count_cast_votes_election_with_census(
    auth_headers: AuthHeaders,
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<ElectionCastVotes>> {
    let mut cast_votes =
        count_cast_votes_election(&hasura_transaction, &tenant_id, &election_event_id).await?;

    for cast_vote in &mut cast_votes {
        let realm = get_event_realm(tenant_id, election_event_id);

        let (_users, census) = list_users(
            &hasura_transaction,
            &keycloak_transaction,
            ListUsersFilter {
                tenant_id: tenant_id.to_string(),
                election_event_id: Some(election_event_id.to_string()),
                election_id: Some(cast_vote.election_id.clone()),
                area_id: None,
                realm: realm.clone(),
                search: None,
                first_name: None,
                last_name: None,
                username: None,
                email: None,
                limit: Some(1),
                offset: None,
                user_ids: None,
            },
        )
        .await?;
        cast_vote.census = census as i64;
    }
    //keycloak_transaction
    //    .commit()
    //    .await
    //    .with_context(|| "error comitting transaction")?;

    Ok(cast_votes)
}

#[instrument(skip_all, err)]
pub async fn get_eligible_voters(
    auth_headers: connection::AuthHeaders,
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    area_id: &str,
) -> Result<u64> {
    let realm = get_event_realm(tenant_id, election_event_id);

    let (_users, census) = list_users(
        &hasura_transaction,
        &keycloak_transaction,
        ListUsersFilter {
            tenant_id: tenant_id.to_string(),
            election_event_id: Some(election_event_id.to_string()),
            election_id: Some(election_id.to_string()),
            area_id: Some(area_id.to_string()),
            realm: realm.clone(),
            search: None,
            first_name: None,
            last_name: None,
            username: None,
            email: None,
            limit: Some(1),
            offset: None,
            user_ids: None,
        },
    )
    .await?;
    Ok(census as u64)
}

#[instrument(skip_all, err)]
async fn map_plaintext_data(
    auth_headers: AuthHeaders,
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
) -> Result<
    Option<(
        Vec<AreaContestDataType>,
        i64,
        bool,
        TallyCeremonyStatus,
        Option<Vec<i64>>,
        Vec<ElectionCastVotes>,
    )>,
> {
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

    let tally_session = &tally_session_data.sequent_backend_tally_session[0];

    let Some(execution_status) = get_execution_status(tally_session.execution_status.clone())
    else {
        return Ok(None);
    };

    if execution_status != TallyExecutionStatus::IN_PROGRESS {
        event!(
            Level::INFO,
            "Skipping tally session {} for event {} as execution status '{}' is not '{}'",
            tally_session.id,
            tally_session.election_event_id,
            execution_status.to_string(),
            TallyExecutionStatus::IN_PROGRESS.to_string()
        );
        return Ok(None);
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
    }

    let initial_status = if tally_session_data
        .sequent_backend_tally_session_execution
        .is_empty()
    {
        None
    } else {
        tally_session_data.sequent_backend_tally_session_execution[0]
            .status
            .clone()
    };

    let mut new_status = get_tally_ceremony_status(initial_status)?;

    let new_tally_progress = generate_tally_progress(&tally_session_data, &messages).await?;
    let mut new_logs = generate_logs(&messages, next_timestamp.clone(), &batch_ids)?;

    new_status.elections_status = new_tally_progress;

    let mut logs = new_status.logs.clone();
    logs.append(&mut new_logs);
    new_status.logs = sort_logs(&logs);

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
    let session_ids: Vec<i64> = relevant_plaintexts
        .iter()
        .map(|message| message.statement.get_batch_number() as i64)
        .collect();
    // we have all plaintexts
    let is_execution_completed = relevant_plaintexts.len() == batch_ids.len();

    let plaintexts_data: Vec<AreaContestDataType> = process_plaintexts(
        auth_headers.clone(),
        relevant_plaintexts,
        ballot_styles,
        tally_session_data,
    )
    .await?;

    let cast_votes_count = count_cast_votes_election_with_census(
        auth_headers.clone(),
        &hasura_transaction,
        &keycloak_transaction,
        &tenant_id,
        &election_event_id,
    )
    .await?;
    Ok(Some((
        plaintexts_data,
        newest_message_id,
        is_execution_completed,
        new_status,
        Some(session_ids),
        cast_votes_count,
    )))
}

#[instrument(skip(auth_headers), err)]
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

#[instrument(err, skip(auth_headers))]
pub async fn execute_tally_session_wrapped(
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
    auth_headers: AuthHeaders,
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
) -> Result<()> {
    let (tally_session_execution, _tally_session) = find_last_tally_session_execution(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        tally_session_id.clone(),
    )
    .await?
    .unwrap();
    // map plaintexts to contests
    let plaintexts_data_opt = map_plaintext_data(
        auth_headers.clone(),
        &hasura_transaction,
        &keycloak_transaction,
        tenant_id.clone(),
        election_event_id.clone(),
        tally_session_id.clone(),
    )
    .await?;

    let Some((
        plaintexts_data,
        newest_message_id,
        is_execution_completed,
        new_status,
        session_ids,
        cast_votes_count,
    )) = plaintexts_data_opt else {
        return Ok(());
    };

    event!(Level::INFO, "Num plaintexts_data {}", plaintexts_data.len());

    // base temp folder
    let base_tempdir = tempdir()?;
    // get credentials
    // map_plaintext_data also calls this but at this point the credentials
    // could be expired
    let auth_headers = keycloak::get_client_credentials().await?;

    let status = run_velvet_tally(
        base_tempdir.path().to_path_buf(),
        &plaintexts_data,
        &cast_votes_count,
    )?;

    let results_event_id = populate_results_tables(
        auth_headers.clone(),
        hasura_transaction,
        &base_tempdir.path().to_path_buf(),
        status,
        &tenant_id,
        &election_event_id,
        session_ids.clone(),
        tally_session_execution.clone(),
    )
    .await?;

    // insert tally_session_execution
    insert_tally_session_execution(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        newest_message_id,
        tally_session_id.clone(),
        Some(new_status),
        results_event_id,
        session_ids,
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
        // get the election event
        let election_event = get_election_event_helper(
            auth_headers.clone(),
            tenant_id.clone(),
            election_event_id.clone(),
        )
        .await?;
        let current_status = get_election_event_status(election_event.status).unwrap();
        let mut new_event_status = current_status.clone();
        new_event_status.tally_ceremony_finished = Some(true);
        let new_status_js = serde_json::to_value(new_event_status)?;
        update_election_event_status(
            auth_headers.clone(),
            tenant_id.clone(),
            election_event_id.clone(),
            new_status_js,
        )
        .await?;
    }

    Ok(())
}

#[instrument(err)]
pub async fn transactions_wrapper(
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
) -> Result<()> {
    let auth_headers = keycloak::get_client_credentials().await?;
    let mut keycloak_db_client: DbClient = get_keycloak_pool()
        .await
        .get()
        .await
        .with_context(|| "Error acquiring keycloak connection pool")?;
    let keycloak_transaction = keycloak_db_client
        .transaction()
        .await
        .with_context(|| "Error acquiring keycloak transaction")?;
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .with_context(|| "Error acquiring hasura connection pool")?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .with_context(|| "Error acquiring hasura transaction")?;

    let res =
        execute_tally_session_wrapped(
            tenant_id.clone(),
            election_event_id.clone(),
            tally_session_id.clone(),
            auth_headers.clone(),
            &hasura_transaction,
            &keycloak_transaction,
        )
        .await
        .with_context(|| "Error executing tally session")?;

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")?;

    Ok(res)
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 1200000, max_retries = 0)]
pub async fn execute_tally_session(
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
) -> Result<()> {
    let lock = PgLock::acquire(
        format!(
            "execute_tally_session-{}-{}-{}",
            tenant_id, election_event_id, tally_session_id
        ),
        Uuid::new_v4().to_string(),
        ISO8601::now() + Duration::seconds(120),
    )
    .await?;
    let mut interval = tokio::time::interval(ChronoDuration::from_secs(30));
    let mut current_task = tokio::spawn(transactions_wrapper(
        tenant_id.clone(),
        election_event_id.clone(),
        tally_session_id.clone(),
    ));
    let res = loop {
        tokio::select! {
            _ = interval.tick() => {
                // Execute the callback function here
                lock.update_expiry().await?;
            }
            res = &mut current_task => {

                break res.map_err(|err| Error::String(format!("Error executing loop: {:?}", err))).flatten();
            }
        }
    };
    lock.release().await?;
    res
}
