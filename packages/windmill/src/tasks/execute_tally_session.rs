// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>, Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura;
use crate::hasura::election_event::get_election_event_helper;
use crate::hasura::election_event::update_election_event_status;
use crate::hasura::keys_ceremony::get_keys_ceremonies;
use crate::hasura::tally_session_execution::get_last_tally_session_execution;
use crate::hasura::tally_session_execution::get_last_tally_session_execution::ResponseData;
use crate::postgres::area::get_event_areas;
use crate::postgres::contest::export_contests;
use crate::postgres::election::set_election_initialization_report_generated;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::keys_ceremony::get_keys_ceremony_by_id;
use crate::postgres::reports::get_template_alias_for_report;
use crate::postgres::reports::ReportType;
use crate::postgres::results_event::insert_results_event;
use crate::postgres::tally_session::get_tally_session_by_id;
use crate::postgres::tally_session_execution::insert_tally_session_execution;
use crate::postgres::tally_sheet::get_published_tally_sheets_by_event;
use crate::postgres::template::get_template_by_alias;
use crate::services::cast_votes::{count_cast_votes_election, ElectionCastVotes};
use crate::services::ceremonies::insert_ballots::{
    count_auditable_ballots, get_elections_end_dates, insert_ballots_messages,
};
use crate::services::ceremonies::keys_ceremony::get_keys_ceremony_board;
use crate::services::ceremonies::results::populate_results_tables;
use crate::services::ceremonies::serialize_logs::{
    append_tally_finished, generate_logs, print_messages, sort_logs,
};
use crate::services::ceremonies::tally_ceremony::{
    find_last_tally_session_execution, get_tally_ceremony_status, set_tally_session_completed,
};
use crate::services::ceremonies::tally_progress::generate_tally_progress;
use crate::services::ceremonies::tally_session_error::handle_tally_session_error;
use crate::services::ceremonies::velvet_tally::run_velvet_tally;
use crate::services::ceremonies::velvet_tally::AreaContestDataType;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::election::get_election_event_elections;
use crate::services::election_event_status::get_election_event_status;
use crate::services::pg_lock::PgLock;
use crate::services::protocol_manager;
use crate::services::reports::electoral_results::ElectoralResults;
use crate::services::reports::initialization::InitializationTemplate;
use crate::services::reports::template_renderer::{
    ReportOriginatedFrom, ReportOrigins, TemplateRenderer,
};
use crate::services::reports::utils::get_public_asset_template;
use crate::services::tally_sheets::validation::validate_tally_sheet;
use crate::services::tasks_semaphore::acquire_semaphore;
use crate::services::temp_path::{
    PUBLIC_ASSETS_ELECTORAL_RESULTS_TEMPLATE_SYSTEM, PUBLIC_ASSETS_INITIALIZATION_TEMPLATE_SYSTEM,
};
use crate::services::users::list_users;
use crate::services::users::ListUsersFilter;
use crate::tasks::execute_tally_session::get_last_tally_session_execution::{
    GetLastTallySessionExecutionSequentBackendTallySession,
    GetLastTallySessionExecutionSequentBackendTallySessionContest,
};
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context, Result as AnyhowResult};
use b3::messages::{artifact::Plaintexts, message::Message, statement::StatementType};
use celery::prelude::TaskError;
use chrono::{DateTime, Duration, Utc};
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use sequent_core::ballot::BallotStyle;
use sequent_core::ballot::Contest;
use sequent_core::ballot::ContestEncryptionPolicy;
use sequent_core::serialization::deserialize_with_path::*;
use sequent_core::services::area_tree::TreeNode;
use sequent_core::services::area_tree::TreeNodeArea;
use sequent_core::services::connection;
use sequent_core::services::connection::AuthHeaders;
use sequent_core::services::date::ISO8601;
use sequent_core::services::keycloak;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::ceremonies::TallyCeremonyStatus;
use sequent_core::types::ceremonies::TallyExecutionStatus;
use sequent_core::types::ceremonies::TallyTrusteeStatus;
use sequent_core::types::ceremonies::TallyType;
use sequent_core::types::hasura::core::Area;
use sequent_core::types::hasura::core::ElectionEvent;
use sequent_core::types::hasura::core::KeysCeremony;
use sequent_core::types::hasura::core::TallySession;
use sequent_core::types::hasura::core::TallySheet;
use sequent_core::types::templates::PrintToPdfOptionsLocal;
use sequent_core::types::templates::ReportExtraConfig;
use sequent_core::types::templates::SendTemplateBody;
use std::collections::HashMap;
use std::collections::HashSet;
use std::str::FromStr;
use strand::{backend::ristretto::RistrettoCtx, context::Ctx, serialization::StrandDeserialize};
use tempfile::tempdir;
use tokio::time::Duration as ChronoDuration;
use tracing::{event, info, instrument, warn, Level};
use uuid::Uuid;

#[instrument(skip_all, err)]
fn get_ballot_styles(tally_session_data: &ResponseData) -> Result<Vec<BallotStyle>> {
    // get ballot styles, from where we'll get the Contest(s)
    tally_session_data
        .sequent_backend_ballot_style
        .iter()
        .map(|ballot_style_row| {
            let ballot_style_res: Result<BallotStyle, Error> = deserialize_str(
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

#[instrument(skip_all, err)]
async fn generate_area_contests_mc(
    hasura_transaction: &Transaction<'_>,
    relevant_plaintexts: &Vec<&Message>,
    ballot_styles: &Vec<BallotStyle>,
    tally_session_data: &ResponseData,
    areas: &Vec<Area>,
    tenant_id: &str,
    election_event_id: &str,
) -> AnyhowResult<Vec<AreaContestDataType>> {
    let all_contests = export_contests(hasura_transaction, tenant_id, election_event_id).await?;
    let areas_map: HashMap<String, Area> = areas
        .clone()
        .into_iter()
        .map(|area: Area| (area.id.clone(), area.clone()))
        .collect();
    let mut almost_vec: Vec<AreaContestDataType> = vec![];
    for session_election in tally_session_data
        .sequent_backend_tally_session_contest
        .clone()
    {
        // contest ids for this election
        let contest_ids = all_contests
            .iter()
            .filter_map(|contest| {
                if contest.election_id != session_election.election_id {
                    return None;
                }
                Some(contest.id.clone())
            })
            .collect::<Vec<_>>();
        for (i, contest_id) in contest_ids.iter().enumerate() {
            let area_id = session_election.area_id.clone();
            let election_id = session_election.election_id.clone();

            let Some(ballot_style) = ballot_styles.iter().find(|ballot_style| {
                ballot_style.area_id == area_id
                    && ballot_style.election_id == election_id
                    && ballot_style
                        .contests
                        .iter()
                        .any(|contest| contest.id == *contest_id)
            }) else {
                event!(Level::WARN, "IGNORING: Ballot Style not found for area id = {}, election id = {}, contest id = {}", area_id, election_id, contest_id);
                continue;
            };

            let Some(contest) = ballot_style
                .contests
                .iter()
                .find(|contest| contest.election_id == election_id && contest.id == *contest_id)
            else {
                event!(
                    Level::WARN,
                    "IGNORING: Contest not found for contest id = {}",
                    contest_id
                );
                continue;
            };

            let plaintexts = if 0 == i {
                let batch_num: i64 = session_election.session_id;
                let Some(plaintexts) = relevant_plaintexts
                    .iter()
                    .find(|plaintexts_message| {
                        batch_num == plaintexts_message.statement.get_batch_number() as i64
                    })
                    .map(|plaintexts_message| {
                        plaintexts_message
                            .artifact
                            .clone()
                            .map(|artifact| -> Option<Vec<<RistrettoCtx as Ctx>::P>> {
                                Plaintexts::<RistrettoCtx>::strand_deserialize(&artifact)
                                    .ok()
                                    .map(|plaintexts| plaintexts.0 .0)
                            })
                            .flatten()
                    })
                    .flatten()
                else {
                    event!(Level::INFO, "Expected: Plaintexts not found yet for session contest = {}, batch number = {}", session_election.id, batch_num );
                    continue;
                };
                info!(
                    "Multi Contests: Adding {} plaintexts for area {} and election {}",
                    plaintexts.len(),
                    area_id,
                    election_id
                );
                plaintexts
            } else {
                vec![]
            };

            let Some(area) = areas_map.get(&ballot_style.area_id) else {
                event!(Level::INFO, "Area not found {}", ballot_style.area_id);
                continue;
            };

            almost_vec.push(AreaContestDataType {
                plaintexts,
                last_tally_session_execution: session_election.clone(),
                contest: contest.clone(),
                ballot_style: ballot_style.clone(),
                eligible_voters: 0,
                auditable_votes: 0,
                area: area.clone(),
            })
        }
    }

    Ok(almost_vec)
}

#[instrument(skip_all, err)]
fn generate_area_contests(
    relevant_plaintexts: &Vec<&Message>,
    ballot_styles: &Vec<BallotStyle>,
    tally_session_data: &ResponseData,
    areas: &Vec<Area>,
) -> AnyhowResult<Vec<AreaContestDataType>> {
    let areas_map: HashMap<String, Area> = areas
        .clone()
        .into_iter()
        .map(|area: Area| (area.id.clone(), area.clone()))
        .collect();

    event!(
        Level::WARN,
        "Num sequent_backend_tally_session_contest = {}",
        tally_session_data
            .sequent_backend_tally_session_contest
            .len()
    );

    let almost_vec: Vec<AreaContestDataType> = tally_session_data
        .sequent_backend_tally_session_contest
        .iter()
        .filter_map(|session_contest| {
            let Some(ballot_style) = ballot_styles.iter().find(|ballot_style| {
                ballot_style.area_id == session_contest.area_id
                    && ballot_style.election_id == session_contest.election_id
                    && ballot_style
                        .contests
                        .iter()
                        .any(|contest| contest.id == session_contest.contest_id.clone().unwrap_or_default())
            }) else {
                event!(Level::WARN, "IGNORING: Ballot Style not found for area id = {}, election id = {}, contest id = {}", session_contest.area_id, session_contest.election_id, session_contest.contest_id.clone().unwrap_or_default());
                return None;
            };

            let Some(contest) = ballot_style
                .contests
                .iter()
                .find(|contest| contest.election_id == session_contest.election_id &&
                    contest.id == session_contest.contest_id.clone().unwrap_or_default() ) else {
                    event!(Level::WARN, "IGNORING: Contest not found for contest id = {}", session_contest.contest_id.clone().unwrap_or_default());
                    return None;
                };

            let batch_num: i64 = session_contest.session_id;
            let Some(plaintexts) = relevant_plaintexts
                .iter()
                .find(|plaintexts_message|
                    batch_num == plaintexts_message.statement.get_batch_number() as i64
                )
                .map(|plaintexts_message| {
                    plaintexts_message.artifact
                        .clone()
                        .map(|artifact| -> Option<Vec<<RistrettoCtx as Ctx>::P>> {
                            Plaintexts::<RistrettoCtx>::strand_deserialize(&artifact)
                                .ok()
                                .map(|plaintexts| plaintexts.0 .0)
                        })
                        .flatten()
                })
                .flatten() else {
                    event!(Level::INFO, "Expected: Plaintexts not found yet for session contest = {}, batch number = {}", session_contest.id, batch_num );
                    return None;
                };
            let Some(area) = areas_map.get(&ballot_style.area_id) else {
                event!(Level::INFO, "Area not found {}", ballot_style.area_id);
                return None;
            };

            Some(AreaContestDataType {
                plaintexts,
                last_tally_session_execution: session_contest.clone(),
                contest: contest.clone(),
                ballot_style: ballot_style.clone(),
                eligible_voters: 0,
                auditable_votes: 0,
                area: area.clone(),
            })
        })
        .collect();

    Ok(almost_vec)
}

#[instrument(skip_all, err)]
async fn process_plaintexts(
    auth_headers: AuthHeaders,
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    relevant_plaintexts: Vec<&Message>,
    ballot_styles: Vec<BallotStyle>,
    tally_session_data: ResponseData,
    areas: &Vec<Area>,
    tenant_id: &str,
    election_event_id: &str,
    contest_encryption_policy: ContestEncryptionPolicy,
) -> Result<Vec<AreaContestDataType>> {
    event!(
        Level::WARN,
        "Num sequent_backend_tally_session_contest = {}",
        tally_session_data
            .sequent_backend_tally_session_contest
            .len()
    );
    let almost_vec = match contest_encryption_policy {
        ContestEncryptionPolicy::MULTIPLE_CONTESTS => {
            generate_area_contests_mc(
                hasura_transaction,
                &relevant_plaintexts,
                &ballot_styles,
                &tally_session_data,
                areas,
                tenant_id,
                election_event_id,
            )
            .await?
        }
        ContestEncryptionPolicy::SINGLE_CONTEST => generate_area_contests(
            &relevant_plaintexts,
            &ballot_styles,
            &tally_session_data,
            areas,
        )?,
    };
    event!(Level::WARN, "Num almost_vec = {}", almost_vec.len());
    let treenode_areas: Vec<TreeNodeArea> = areas.iter().map(|area| area.into()).collect();

    let areas_tree = TreeNode::<()>::from_areas(treenode_areas)?;

    // set<area_id, contest_id>
    let found_area_contests: HashSet<(String, String)> = almost_vec
        .iter()
        .map(|val| (val.area.id.clone(), val.contest.id.clone()))
        .collect();
    event!(
        Level::WARN,
        "Num found_area_contests = {}",
        found_area_contests.len()
    );

    let filtered_area_contests: Vec<AreaContestDataType> = almost_vec
        .clone()
        .into_iter()
        .filter(|area_contest| {
            event!(Level::WARN, "find_path_to_area {}", area_contest.area.id);
            let Some(tree_path) = areas_tree.find_path_to_area(&area_contest.area.id) else {
                event!(Level::WARN, "NOT FOUND");
                return false;
            };
            /*tree_path.iter().all(|tree_node| {
                found_area_contests
                    .contains(&(tree_node.id.clone(), area_contest.contest.id.clone()))
            })*/
            true
        })
        .collect();
    event!(
        Level::WARN,
        "Num filtered_area_contests = {}",
        filtered_area_contests.len()
    );

    let elections_end_dates = get_elections_end_dates(&auth_headers, tenant_id, election_event_id)
        .await
        .with_context(|| "error getting elections end_date")?;

    let mut data: Vec<AreaContestDataType> = vec![];

    let election_ids_alias: HashMap<String, String> =
        get_election_event_elections(&hasura_transaction, tenant_id, election_event_id)
            .await?
            .into_iter()
            .filter_map(|election| election.alias.map(|x| (election.id.clone(), x)))
            .collect();

    // fill in the eligible voters data
    // FIXME: For election level data
    for almost in filtered_area_contests {
        let mut area_contest = almost.clone();

        let election_alias = match election_ids_alias.get(&area_contest.contest.election_id) {
            Some(alias) => alias,
            None => "",
        }
        .to_string();

        let eligible_voters = get_eligible_voters(
            auth_headers.clone(),
            &hasura_transaction,
            &keycloak_transaction,
            &area_contest.contest.tenant_id,
            &area_contest.contest.election_event_id,
            &area_contest.contest.election_id,
            &area_contest.last_tally_session_execution.area_id,
            &election_alias,
        )
        .await?;
        let auditable_votes = count_auditable_ballots(
            &elections_end_dates,
            &auth_headers,
            &hasura_transaction,
            &keycloak_transaction,
            &area_contest.contest.tenant_id,
            &area_contest.contest.election_event_id,
            &area_contest.contest.election_id,
            &area_contest.contest.id,
            &area_contest.last_tally_session_execution.area_id,
        )
        .await
        .with_context(|| "Error counting auditable ballots")?;

        let contest_name = &area_contest.contest.name;
        let area_id = &area_contest.last_tally_session_execution.area_id;
        info!(
            r#"
            Setting:
                eligible_voters={eligible_voters},
                auditable_votes={auditable_votes},
            for area_contest with:
                contest_name={contest_name:?} & and area_id={area_id}
        "#
        );
        area_contest.eligible_voters = eligible_voters;
        area_contest.auditable_votes = auditable_votes
            .try_into()
            .with_context(|| "Too many auditable ballots")?;
        data.push(area_contest);
    }
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

#[instrument(skip_all, err)]
pub async fn count_cast_votes_election_with_census(
    auth_headers: AuthHeaders,
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<ElectionCastVotes>> {
    let mut cast_votes =
        count_cast_votes_election(&hasura_transaction, &tenant_id, &election_event_id, None)
            .await?;

    let election_ids_alias: HashMap<String, String> =
        get_election_event_elections(&hasura_transaction, tenant_id, election_event_id)
            .await?
            .into_iter()
            .filter_map(|election| election.alias.map(|x| (election.id.clone(), x)))
            .collect();

    for cast_vote in &mut cast_votes {
        let realm = get_event_realm(tenant_id, election_event_id);

        let election_alias = match election_ids_alias.get(&cast_vote.election_id) {
            Some(alias) => alias,
            None => "",
        }
        .to_string();

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
                attributes: None,
                enabled: None,
                email_verified: None,
                sort: None,
                has_voted: None,
                authorized_to_election_alias: Some(election_alias.to_string()),
            },
        )
        .await?;
        cast_vote.census = census as i64;
    }

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
    election_alias: &str,
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
            attributes: None,
            enabled: None,
            email_verified: None,
            sort: None,
            has_voted: None,
            authorized_to_election_alias: Some(election_alias.to_string()),
        },
    )
    .await?;
    Ok(census as u64)
}

#[instrument(skip_all, err)]
pub async fn upsert_ballots_messages(
    auth_headers: &AuthHeaders,
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    board_name: &str,
    trustee_names: Vec<String>,
    messages: &Vec<Message>,
    tally_session_contests: &Vec<GetLastTallySessionExecutionSequentBackendTallySessionContest>,
    tally_session_hasura: &TallySession,
) -> Result<Vec<GetLastTallySessionExecutionSequentBackendTallySessionContest>> {
    let contest_encryption_policy = tally_session_hasura
        .configuration
        .clone()
        .unwrap_or_default()
        .get_contest_encryption_policy();
    let expected_batch_ids: Vec<i64> = tally_session_contests
        .clone()
        .into_iter()
        .map(|tally_session_contest| tally_session_contest.session_id.clone())
        .collect();
    let existing_ballots_batches: Vec<i64> = messages
        .iter()
        .filter(|message| {
            expected_batch_ids.contains(&(message.statement.get_batch_number() as i64))
                && StatementType::Ballots == message.statement.get_kind()
        })
        .map(|message| message.statement.get_batch_number() as i64)
        .collect();
    event!(
        Level::INFO,
        "existing_ballots_batches: '{:?}'",
        existing_ballots_batches
    );
    let missing_ballots_batches: Vec<
        GetLastTallySessionExecutionSequentBackendTallySessionContest,
    > = tally_session_contests
        .clone()
        .into_iter()
        .filter(|tally_session_contest| {
            !existing_ballots_batches.contains(&tally_session_contest.session_id)
        })
        .collect();

    event!(
        Level::INFO,
        "missing_ballots_batches num: {}",
        missing_ballots_batches.len()
    );
    if missing_ballots_batches.len() > 0 {
        insert_ballots_messages(
            &auth_headers,
            hasura_transaction,
            keycloak_transaction,
            tenant_id,
            election_event_id,
            board_name,
            trustee_names,
            missing_ballots_batches.clone(),
            contest_encryption_policy,
        )
        .await?;
    }
    Ok(missing_ballots_batches)
}

fn get_tally_session_created_at_timestamp_secs(
    tally_session: &GetLastTallySessionExecutionSequentBackendTallySession,
) -> Result<i64> {
    let Some(created_at) = &tally_session.created_at.clone() else {
        return Err(Error::String(format!(
            "Missing created_at for tally_session"
        )));
    };
    let tally_session_created_at = ISO8601::to_date(&created_at)?;
    Ok(tally_session_created_at.timestamp())
}

#[instrument(skip_all, err)]
pub fn clean_tally_sheets(
    tally_sheet_rows: &Vec<TallySheet>,
    plaintexts_data: &Vec<AreaContestDataType>,
) -> Result<Vec<TallySheet>> {
    let contests_map: HashMap<String, Contest> = plaintexts_data
        .clone()
        .into_iter()
        .map(|area_contest| {
            (
                area_contest.contest.id.clone(),
                area_contest.contest.clone(),
            )
        })
        .collect();
    tally_sheet_rows
        .iter()
        .map(|tally_sheet| -> Result<TallySheet> {
            let Some(content) = tally_sheet.content.clone() else {
                return Err(
                    anyhow!("Invalid tally sheet {:?}, content missing", tally_sheet).into(),
                );
            };

            if tally_sheet.area_id != content.area_id {
                return Err(
                    anyhow!("Invalid tally sheet {:?}, area not consistent", tally_sheet).into(),
                );
            }
            if tally_sheet.contest_id != content.contest_id {
                return Err(anyhow!(
                    "Invalid tally sheet {:?}, contest not consistent",
                    tally_sheet
                )
                .into());
            }
            let Some(contest) = contests_map.get(&tally_sheet.contest_id) else {
                return Err(
                    anyhow!("Invalid tally sheet {:?}, can't find contest", tally_sheet).into(),
                );
            };
            validate_tally_sheet(tally_sheet, &contest)?;

            Ok(tally_sheet.clone())
        })
        .collect::<Result<Vec<TallySheet>>>()
}

#[instrument(skip_all, err)]
async fn map_plaintext_data(
    auth_headers: AuthHeaders,
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
    ceremony_status: TallyCeremonyStatus,
    keys_ceremony: &KeysCeremony,
    tally_session_data: get_last_tally_session_execution::ResponseData,
) -> Result<
    Option<(
        Vec<AreaContestDataType>,
        i64,
        bool,
        TallyCeremonyStatus,
        Option<Vec<i64>>,
        Vec<ElectionCastVotes>,
        Vec<TallySheet>,
        ElectionEvent,
        TallySession,
    )>,
> {
    // fetch election_event
    let Ok(election_event) =
        get_election_event_by_id(hasura_transaction, &tenant_id, &election_event_id).await
    else {
        event!(
            Level::INFO,
            "Election Event not found {}",
            election_event_id.clone()
        );

        return Ok(None);
    };
    let tally_session_hasura = get_tally_session_by_id(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &tally_session_id,
    )
    .await?;

    // get name of bulletin board
    let (bulletin_board, _) = get_keys_ceremony_board(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        keys_ceremony,
    )
    .await?;

    let tally_session = &tally_session_data.sequent_backend_tally_session[0];
    let tally_session_created_at_timestamp_secs =
        get_tally_session_created_at_timestamp_secs(tally_session)? as u64;

    let Some(execution_status) = get_execution_status(tally_session.execution_status.clone())
    else {
        event!(
            Level::INFO,
            "Election Event {} Tally execution status not found",
            election_event_id.clone()
        );
        return Ok(None);
    };

    let keys_ceremonies = get_keys_ceremonies(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await?
    .data
    .with_context(|| "error listing existing keys ceremonies")?
    .sequent_backend_keys_ceremony;

    if keys_ceremonies.is_empty() {
        event!(
            Level::INFO,
            "Election Event {} has no keys ceremony",
            election_event_id.clone()
        );
        return Ok(None);
    }

    let threshold = keys_ceremonies[0].threshold as usize;
    let mut available_trustees: Vec<String> = ceremony_status
        .trustees
        .into_iter()
        .filter(|trustee| TallyTrusteeStatus::KEY_RESTORED == trustee.status)
        .map(|trustee| trustee.name.clone())
        .collect();
    let mut rng = StdRng::from_entropy();
    available_trustees.shuffle(&mut rng);

    let trustee_names: Vec<String> = available_trustees.into_iter().take(threshold).collect();

    if trustee_names.len() < threshold {
        event!(
            Level::INFO,
            "Election Event {} has {} connected trustees but threshold is {}",
            election_event_id.clone(),
            trustee_names.len(),
            threshold
        );
        return Ok(None);
    }
    event!(
        Level::INFO,
        "Election Event {}. Selected trustees {:#?}",
        election_event_id.clone(),
        trustee_names
    );

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
    let board_client = protocol_manager::get_b3_pgsql_client().await?;
    let board_messages = board_client.get_messages(&bulletin_board, -1).await?;
    event!(Level::INFO, "Num board_messages {}", board_messages.len());

    // convert board messages into messages
    let messages: Vec<Message> = protocol_manager::convert_board_messages(&board_messages)?;
    print_messages(&messages, &bulletin_board)?;

    let new_ballots_messages = upsert_ballots_messages(
        &auth_headers,
        hasura_transaction,
        keycloak_transaction,
        &tenant_id,
        &election_event_id,
        &bulletin_board,
        trustee_names,
        &messages,
        &tally_session_data.sequent_backend_tally_session_contest,
        &tally_session_hasura,
    )
    .await?;

    if !new_ballots_messages.is_empty() {
        event!(
            Level::INFO,
            "Ballots messages inserted: {} skipping iteration",
            new_ballots_messages.len()
        );
        return Ok(None);
    }

    // find a new board message
    let next_new_board_message_opt = board_messages
        .iter()
        .find(|board_message| board_message.id > last_message_id);

    let newest_message_id = board_messages
        .last()
        .map(|board_message| board_message.id)
        .unwrap_or(-1);

    let Some(next_new_board_message) = next_new_board_message_opt else {
        event!(Level::INFO, "Board has no new messages",);
        return Ok(None);
    };

    // find the timestamp of the new board message.
    // We do this because once we convert into a Message, we lose the link to the board message id
    let mut next_timestamp = Message::strand_deserialize(&next_new_board_message.message)?
        .statement
        .get_timestamp();
    next_timestamp = std::cmp::max(tally_session_created_at_timestamp_secs, next_timestamp);

    // get the batch ids that are linked to this tally session
    let batch_ids = tally_session_data
        .sequent_backend_tally_session_contest
        .iter()
        .map(|tsc| tsc.session_id)
        .collect::<Vec<_>>();
    event!(Level::INFO, "Num batch_ids {}", batch_ids.len());

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
    let mut new_logs = generate_logs(&messages, next_timestamp, &batch_ids)?;

    new_status.elections_status = new_tally_progress;

    {
        let mut logs = new_status.logs.clone();
        logs.append(&mut new_logs);
        new_status.logs = sort_logs(&logs);
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
    let session_ids: Vec<i64> = relevant_plaintexts
        .iter()
        .map(|message| message.statement.get_batch_number() as i64)
        .collect();
    // we have all plaintexts
    let is_execution_completed = relevant_plaintexts.len() == batch_ids.len();

    let areas = get_event_areas(hasura_transaction, &tenant_id, &election_event_id).await?;

    let tally_sheet_rows =
        get_published_tally_sheets_by_event(hasura_transaction, &tenant_id, &election_event_id)
            .await?;

    let contest_encryption_policy = tally_session_hasura
        .configuration
        .clone()
        .unwrap_or_default()
        .get_contest_encryption_policy();
    let plaintexts_data: Vec<AreaContestDataType> = process_plaintexts(
        auth_headers.clone(),
        hasura_transaction,
        keycloak_transaction,
        relevant_plaintexts,
        ballot_styles,
        tally_session_data,
        &areas,
        &tenant_id,
        &election_event_id,
        contest_encryption_policy,
    )
    .await?;
    event!(Level::INFO, "Num plaintexts_data {}", plaintexts_data.len());
    let tally_sheets = clean_tally_sheets(&tally_sheet_rows, &plaintexts_data)?;

    let cast_votes_count = count_cast_votes_election_with_census(
        auth_headers.clone(),
        hasura_transaction,
        keycloak_transaction,
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
        tally_sheets,
        election_event,
        tally_session_hasura,
    )))
}

#[instrument(skip(hasura_transaction), err)]
async fn create_results_event(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<String> {
    let results_event = &insert_results_event(hasura_transaction, tenant_id, election_event_id)
        .await
        .with_context(|| "can't find results_event")?;

    Ok(results_event.id.clone())
}

async fn build_reports_template_data(
    tally_type_enum: TallyType,
    tenant_id: String,
    election_event_id: String,
    election_id: &str,
    hasura_transaction: &Transaction<'_>,
) -> Result<(Option<String>, String, Option<PrintToPdfOptionsLocal>)> {
    let (report_content_template, pdf_options): (Option<String>, Option<PrintToPdfOptionsLocal>) =
        match tally_type_enum {
            TallyType::INITIALIZATION_REPORT => {
                let renderer = InitializationTemplate::new(ReportOrigins {
                    tenant_id: tenant_id.clone(),
                    election_event_id: election_event_id.clone(),
                    election_id: Some(election_id.to_string()),
                    template_alias: None,
                    voter_id: None,
                    report_origin: ReportOriginatedFrom::ExportFunction,
                    executer_username: None, //TODO: fix?
                    tally_session_id: None,
                });
                let template_data_opt: Option<SendTemplateBody> = renderer
                    .get_custom_user_template_data(hasura_transaction)
                    .await
                    .map_err(|e| {
                        anyhow!("Error getting initialization report custom user template: {e:?}")
                    })?;

                match template_data_opt {
                    Some(template) => (template.document, template.pdf_options),
                    None => {
                        let default_doc: String = renderer.get_default_user_template()
                        .await
                        .map_err(|err| {
                            anyhow!("Error getting initialization report default user template: {err:?}")
                        })?;

                        let pdf_options: Option<PrintToPdfOptionsLocal> =
                            if let Ok(default_extra_config) =
                                renderer.get_default_extra_config().await
                            {
                                Some(default_extra_config.pdf_options)
                            } else {
                                None
                            };
                        (Some(default_doc), pdf_options)
                    }
                }
            }
            _ => {
                let renderer = ElectoralResults::new(ReportOrigins {
                    tenant_id: tenant_id.clone(),
                    election_event_id: election_event_id.clone(),
                    election_id: None,
                    template_alias: None,
                    voter_id: None,
                    report_origin: ReportOriginatedFrom::ExportFunction,
                    executer_username: None, //TODO: fix?
                    tally_session_id: None,
                });
                let template_data_opt: Option<SendTemplateBody> = renderer
                    .get_custom_user_template_data(hasura_transaction)
                    .await
                    .map_err(|e| {
                        anyhow!("Error getting electoral results  custom user template: {e:?}")
                    })?;

                match template_data_opt {
                    Some(template) => (template.document, template.pdf_options),
                    None => {
                        let default_doc: String = renderer.get_default_user_template()
                    .await
                    .map_err(|err| {
                        anyhow!("Error getting electoral results  default user template: {err:?}")
                    })?;
                        let pdf_options: Option<PrintToPdfOptionsLocal> =
                            if let Ok(default_extra_config) =
                                renderer.get_default_extra_config().await
                            {
                                Some(default_extra_config.pdf_options)
                            } else {
                                None
                            };
                        (Some(default_doc), pdf_options)
                    }
                }
            }
        };

    let report_system_template = match tally_type_enum {
        TallyType::INITIALIZATION_REPORT => {
            get_public_asset_template(PUBLIC_ASSETS_INITIALIZATION_TEMPLATE_SYSTEM).await?
        }
        _ => get_public_asset_template(PUBLIC_ASSETS_ELECTORAL_RESULTS_TEMPLATE_SYSTEM).await?,
    };
    Ok((report_content_template, report_system_template, pdf_options))
}

#[instrument(err, skip(auth_headers, hasura_transaction, keycloak_transaction))]
pub async fn execute_tally_session_wrapped(
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
    auth_headers: AuthHeaders,
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tally_type: Option<String>,
    election_ids: Option<Vec<String>>,
) -> Result<()> {
    let Some((tally_session_execution, tally_session, tally_session_data)) =
        find_last_tally_session_execution(
            auth_headers.clone(),
            tenant_id.clone(),
            election_event_id.clone(),
            tally_session_id.clone(),
            election_ids.clone().unwrap_or(vec![]),
        )
        .await?
    else {
        event!(Level::INFO, "Can't find last execution status, skipping");
        return Ok(());
    };

    let keys_ceremony = get_keys_ceremony_by_id(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &tally_session.keys_ceremony_id,
    )
    .await?;

    let tally_type_enum = tally_type
        .map(|val: String| TallyType::try_from(val.as_str()).unwrap_or_default())
        .unwrap_or_default();

    let election_ids_default = election_ids.clone().unwrap_or_default();
    let election_id = election_ids_default.get(0).map_or("", |v| v.as_str());

    // Check the report type and create renderer according the report type
    let (report_content_template, report_system_template, pdf_options) =
        build_reports_template_data(
            tally_type_enum.clone(),
            tenant_id.clone(),
            election_event_id.clone(),
            election_id,
            hasura_transaction,
        )
        .await?;

    let status = get_tally_ceremony_status(tally_session_execution.status.clone())?;

    // map plaintexts to contests
    let plaintexts_data_opt = map_plaintext_data(
        auth_headers.clone(),
        hasura_transaction,
        keycloak_transaction,
        tenant_id.clone(),
        election_event_id.clone(),
        tally_session_id.clone(),
        status,
        &keys_ceremony,
        tally_session_data,
    )
    .await?;

    let Some((
        plaintexts_data,
        newest_message_id,
        is_execution_completed,
        mut new_status,
        session_ids,
        cast_votes_count,
        tally_sheets,
        election_event,
        tally_session,
    )) = plaintexts_data_opt
    else {
        event!(Level::INFO, "map_plaintext_data is None, skipping");
        return Ok(());
    };

    event!(Level::INFO, "Num plaintexts_data {}", plaintexts_data.len());

    // base temp folder
    let base_tempdir = tempdir()?;

    let areas: Vec<Area> =
        get_event_areas(hasura_transaction, &tenant_id, &election_event_id).await?;

    let status = if !plaintexts_data.is_empty() {
        Some(
            run_velvet_tally(
                base_tempdir.path().to_path_buf(),
                &plaintexts_data,
                &cast_votes_count,
                &tally_sheets,
                report_content_template,
                report_system_template,
                pdf_options,
                &areas,
                hasura_transaction,
                &election_event,
                &tally_session,
                tally_type_enum.clone(),
            )
            .await?,
        )
    } else {
        None
    };

    let default_language = election_event.get_default_language();

    let results_event_id = populate_results_tables(
        hasura_transaction,
        &base_tempdir.path().to_path_buf(),
        status,
        &tenant_id,
        &election_event_id,
        session_ids.clone(),
        tally_session_execution.clone(),
        &areas,
        &default_language,
        tally_type_enum.clone(),
    )
    .await?;
    // map_plaintext_data also calls this but at this point the credentials
    // could be expired
    let auth_headers = keycloak::get_client_credentials().await?;

    let session_ids_i32: Option<Vec<i32>> = session_ids
        .clone()
        .map(|values| values.clone().into_iter().map(|int| int as i32).collect());

    new_status.logs =
        append_tally_finished(&new_status.logs, &election_ids.clone().unwrap_or(vec![]));

    // insert tally_session_execution
    insert_tally_session_execution(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        newest_message_id as i32,
        &tally_session_id,
        Some(new_status),
        results_event_id,
        session_ids_i32,
    )
    .await?;

    if is_execution_completed {
        // update tally session to flag it as completed
        set_tally_session_completed(
            auth_headers.clone(),
            hasura_transaction,
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
        let new_event_status = current_status.clone();
        let new_status_js = serde_json::to_value(new_event_status)?;
        update_election_event_status(
            auth_headers.clone(),
            tenant_id.clone(),
            election_event_id.clone(),
            new_status_js,
        )
        .await?;
        if tally_type_enum == TallyType::INITIALIZATION_REPORT {
            for election_id in election_ids_default {
                set_election_initialization_report_generated(
                    hasura_transaction,
                    &tenant_id,
                    &election_event_id,
                    &election_id,
                    &true,
                )
                .await?;
            }
        }
    }

    Ok(())
}

#[instrument(err)]
pub async fn transactions_wrapper(
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
    tally_type: Option<String>,
    election_ids: Option<Vec<String>>,
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

    let res = execute_tally_session_wrapped(
        tenant_id.clone(),
        election_event_id.clone(),
        tally_session_id.clone(),
        auth_headers.clone(),
        &hasura_transaction,
        &keycloak_transaction,
        tally_type.clone(),
        election_ids.clone(),
    )
    .await;

    match res {
        Ok(res) => {
            hasura_transaction
                .commit()
                .await
                .with_context(|| "error comitting transaction")?;
            Ok(res)
        }
        Err(err) => {
            tracing::error!("Error in transactions_wrapper: {:?}", err);
            let hasura_rollback = hasura_transaction.rollback().await;
            let keycloak_rollback = keycloak_transaction.rollback().await;
            handle_tally_session_error(
                &err.to_string(),
                &tenant_id,
                &election_event_id,
                &tally_session_id,
            )
            .await?;
            hasura_rollback?;
            keycloak_rollback?;
            Err(err)
        }
    }
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 1200000, max_retries = 0, expires = 15)]
pub async fn execute_tally_session(
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
    tally_type: Option<String>,
    election_ids: Option<Vec<String>>,
) -> Result<()> {
    let _permit = acquire_semaphore().await?;
    let Ok(lock) = PgLock::acquire(
        format!(
            "execute_tally_session-{}-{}-{}",
            tenant_id, election_event_id, tally_session_id
        ),
        Uuid::new_v4().to_string(),
        ISO8601::now() + Duration::seconds(120),
    )
    .await
    else {
        info!(
            "Skipping: tally in progress for event {} and session id {}",
            election_event_id, tally_session_id
        );
        return Ok(());
    };
    let mut interval = tokio::time::interval(ChronoDuration::from_secs(30));
    let mut current_task = tokio::spawn(transactions_wrapper(
        tenant_id.clone(),
        election_event_id.clone(),
        tally_session_id.clone(),
        tally_type.clone(),
        election_ids.clone(),
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
