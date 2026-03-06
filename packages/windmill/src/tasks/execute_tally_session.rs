// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::area::get_event_areas;
use crate::postgres::contest::export_contests;
use crate::postgres::election::set_election_initialization_report_generated;
use crate::postgres::election_event::{get_election_event_by_id, update_election_event_status};
use crate::postgres::keys_ceremony::{get_keys_ceremonies, get_keys_ceremony_by_id};
use crate::postgres::reports::get_template_alias_for_report;
use crate::postgres::reports::ReportType;
use crate::postgres::results_event::insert_results_event;
use crate::postgres::tally_session::get_tally_session_by_id;
use crate::postgres::tally_session::{
    append_tally_session_tie_break_annotation, update_tally_session_annotation,
    update_tally_session_status,
};
use crate::postgres::tally_session_contest::update_tally_session_contests_annotations;
use crate::postgres::tally_session_execution::insert_tally_session_execution;
use crate::postgres::tally_session_resolution::{
    create_tally_session_resolution, get_pending_resolutions, get_resolution_by_tally_session,
    ResolutionStatus, ResolutionType, TallySessionResolution,
};
use crate::postgres::tally_sheet::get_published_tally_sheets_by_event;
use crate::postgres::template::get_template_by_alias;
use crate::services::cast_votes::{count_cast_votes_election, ElectionCastVotes};
use crate::services::celery_app::get_celery_app;
use crate::services::ceremonies::insert_ballots::{
    get_elections_end_dates, insert_ballots_messages,
};
use crate::services::ceremonies::keys_ceremony::get_keys_ceremony_board;
use crate::services::ceremonies::results::populate_results_tables;
use crate::services::ceremonies::serialize_logs::{
    append_tally_finished, append_tally_resumed_after_resolution, append_tally_updated,
    generate_logs, print_messages, sort_logs,
};
use crate::services::ceremonies::tally_ceremony::find_last_tally_session_execution_and_all_related_data;
use crate::services::ceremonies::tally_ceremony::{
    get_tally_ceremony_status, set_tally_session_completed,
};
use crate::services::ceremonies::tally_progress::generate_tally_progress;
use crate::services::ceremonies::tally_session_error::handle_tally_session_error;
use crate::services::ceremonies::velvet_tally::run_velvet_tally;
use crate::services::ceremonies::velvet_tally::AreaContestDataType;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::election::get_election_event_elections;
use crate::services::election_event_status::get_election_event_status;
use crate::services::electoral_log::ElectoralLog;
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
use crate::tasks::electoral_log::{
    enqueue_electoral_log_event, LogEventBody, LogEventInput, LogMessageType,
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
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::serialization::deserialize_with_path::*;
use sequent_core::services::area_tree::TreeNode;
use sequent_core::services::area_tree::TreeNodeArea;
use sequent_core::services::date::ISO8601;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::ceremonies::TallyExecutionStatus;
use sequent_core::types::ceremonies::TallyTrusteeStatus;
use sequent_core::types::ceremonies::TallyType;
use sequent_core::types::ceremonies::{CeremoniesPolicy, TallyCeremonyStatus};
use sequent_core::types::hasura::core::Area;
use sequent_core::types::hasura::core::BallotStyle as BallotStyleHasura;
use sequent_core::types::hasura::core::ElectionEvent;
use sequent_core::types::hasura::core::KeysCeremony;
use sequent_core::types::hasura::core::TallySession;
use sequent_core::types::hasura::core::TallySessionContest;
use sequent_core::types::hasura::core::TallySessionContestAnnotations;
use sequent_core::types::hasura::core::TallySessionExecution;
use sequent_core::types::hasura::core::TallySheet;
use sequent_core::types::templates::PrintToPdfOptionsLocal;
use sequent_core::types::templates::ReportExtraConfig;
use sequent_core::types::templates::SendTemplateBody;
use serde_json;
use std::collections::HashMap;
use std::collections::HashSet;
use std::str::FromStr;
use strand::{backend::ristretto::RistrettoCtx, context::Ctx, serialization::StrandDeserialize};
use tempfile::tempdir;
use tokio::time::Duration as ChronoDuration;
use tracing::{event, info, instrument, warn, Level};
use uuid::Uuid;

struct TieResolutionMetadata {
    // Vec of (results_contest_id, contest_id, tie_metadata)
    pending: Vec<(String, String, serde_json::Value)>,
}

/// Groups resolved IRV tie-break rows into a per-contest map keyed by the
/// actual contest UUID. Each entry is an array of
/// `{round_number, resolved_by_candidate_id}` objects so that multi-round
/// resumes can supply the correct decision for every past round.
pub(crate) fn build_tie_resolutions_map(
    resolutions: &[TallySessionResolution],
) -> HashMap<String, Vec<serde_json::Value>> {
    let mut map: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
    for r in resolutions
        .iter()
        .filter(|r| r.resolution_type == ResolutionType::IrvTieBreak)
        .filter(|r| r.resolution.is_some())
    {
        let Some(actual_contest_id) = r.contest_id.as_deref() else {
            continue;
        };
        let Some(candidate_id) = r
            .resolution
            .as_ref()
            .and_then(|v| v.get("resolved_by_candidate_id"))
        else {
            continue;
        };
        let round_number = r
            .resolution_data
            .get("round_number")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        map.entry(actual_contest_id.to_string())
            .or_default()
            .push(serde_json::json!({
                "round_number": round_number,
                "resolved_by_candidate_id": candidate_id,
            }));
    }
    map
}

/// Extracts a `pending_tie_resolution` value from a results_contest annotations
/// blob, if present. The annotations JSON is expected to have the shape:
/// `{"process_results": {"pending_tie_resolution": {...}}}`.
pub(crate) fn parse_pending_tie_from_annotations(
    annotations: &serde_json::Value,
) -> Option<serde_json::Value> {
    annotations
        .get("process_results")?
        .as_object()?
        .get("pending_tie_resolution")
        .cloned()
}

/// Returns true if `existing` already contains a pending IRV tie-break resolution
/// for the given `contest_id` and the same `round_number` as in `tie_metadata`.
///
/// Using `(contest_id, round_number)` as the key — rather than `contest_id` alone —
/// allows area-level `ProcessBallotsAll` runs to produce independent resolutions for
/// different rounds of the same contest without silently dropping any of them.
pub(crate) fn pending_resolution_exists(
    existing: &[TallySessionResolution],
    contest_id: &str,
    tie_metadata: &serde_json::Value,
) -> bool {
    let tie_round = tie_metadata.get("round_number");
    existing.iter().any(|r| {
        r.contest_id.as_deref() == Some(contest_id)
            && r.resolution_type == ResolutionType::IrvTieBreak
            && r.resolution_data.get("round_number") == tie_round
    })
}

/// Checks if the results contain pending tie resolution metadata and returns them.
/// Resolved ties are tracked in tally_session_resolution by harvest — not created here.
async fn check_for_tie_resolutions(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    results_event_id: &str,
) -> AnyhowResult<TieResolutionMetadata> {
    let query = r#"
        SELECT id, contest_id, annotations
        FROM sequent_backend.results_contest
        WHERE tenant_id = $1
          AND election_event_id = $2
          AND results_event_id = $3
          AND annotations IS NOT NULL
          AND annotations->'process_results'->>'pending_tie_resolution' IS NOT NULL
    "#;

    let rows = hasura_transaction
        .query(
            query,
            &[
                &Uuid::parse_str(tenant_id).context("Failed to parse tenant_id")?,
                &Uuid::parse_str(election_event_id).context("Failed to parse election_event_id")?,
                &Uuid::parse_str(results_event_id).context("Failed to parse results_event_id")?,
            ],
        )
        .await?;

    let mut pending = Vec::new();

    for row in rows {
        // results_contest.id is what tally_session_resolution.contest_id FK references
        let results_contest_id: Uuid = row.get(0);
        let results_contest_id_str = results_contest_id.to_string();
        // results_contest.contest_id is the actual contest identifier
        let contest_id: Uuid = row.get(1);
        let contest_id_str = contest_id.to_string();
        let annotations: serde_json::Value = row.get(2);

        if let Some(tie_metadata) = parse_pending_tie_from_annotations(&annotations) {
            pending.push((results_contest_id_str, contest_id_str, tie_metadata));
        }
    }

    Ok(TieResolutionMetadata { pending })
}

#[instrument(skip_all, err)]
fn get_ballot_styles(ballot_styles: &Vec<BallotStyleHasura>) -> Result<Vec<BallotStyle>> {
    // get ballot styles, from where we'll get the Contest(s)
    ballot_styles
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
    tally_session_contest: &Vec<TallySessionContest>,
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
    for session_election in tally_session_contest.clone() {
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

        // Extract plaintexts once per session/batch
        let batch_num: i64 = session_election.session_id as i64;

        // We wrap this in an Option. We will 'take' it for the first valid contest we find.
        let mut pending_plaintexts: Option<Vec<<RistrettoCtx as Ctx>::P>> = relevant_plaintexts
            .iter()
            .find(|plaintexts_message| {
                batch_num == plaintexts_message.statement.get_batch_number() as i64
            })
            .and_then(|plaintexts_message| {
                plaintexts_message.artifact.clone().and_then(|artifact| {
                    Plaintexts::<RistrettoCtx>::strand_deserialize(&artifact)
                        .ok()
                        .map(|plaintexts| plaintexts.0 .0)
                })
            });

        if pending_plaintexts.is_none() {
            event!(
                Level::INFO,
                "Expected: Plaintexts not found yet for session contest = {}, batch number = {}",
                session_election.id,
                batch_num
            );
            // Skips the whole batch if there are no plaintexts.
            continue;
        }

        // Loop over contests
        for contest_id in contest_ids.iter() {
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

            // Assign plaintexts to the first VALID contest
            // .take() returns the value inside the Option and replaces it with None.
            // This ensures plaintexts are added exactly once, to the first contest that survives the checks above.
            let plaintexts = if let Some(plaintexts) = pending_plaintexts.take() {
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

            let (eligible_voters, auditable_votes) = if let Some(annotations) =
                session_election.annotations.clone()
            {
                let annotations: TallySessionContestAnnotations = deserialize_value(annotations)?;

                (
                    annotations.elegible_voters,
                    annotations.ballots_without_voter,
                )
            } else {
                (0u64, 0u64)
            };

            almost_vec.push(AreaContestDataType {
                plaintexts,
                last_tally_session_execution: session_election.clone(),
                contest: contest.clone(),
                ballot_style: ballot_style.clone(),
                eligible_voters,
                auditable_votes,
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
    tally_session_contest: &Vec<TallySessionContest>,
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
        &tally_session_contest.len()
    );

    let almost_vec: Vec<AreaContestDataType> = tally_session_contest.clone()
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

            let batch_num: i64 = session_contest.session_id as i64;
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

            let (eligible_voters, auditable_votes) =
            if let Some(annotations) = session_contest.annotations.clone() {
                let annotations: TallySessionContestAnnotations =
                    deserialize_value(annotations).ok()?;

                (
                    annotations.elegible_voters,
                    annotations.ballots_without_voter,
                )
            } else {
                (0u64, 0u64)
            };

            Some(AreaContestDataType {
                plaintexts,
                last_tally_session_execution: session_contest.clone(),
                contest: contest.clone(),
                ballot_style: ballot_style.clone(),
                eligible_voters,
                auditable_votes,
                area: area.clone(),
            })
        })
        .collect();

    Ok(almost_vec)
}

#[instrument(skip_all, err)]
async fn process_plaintexts(
    hasura_transaction: &Transaction<'_>,
    relevant_plaintexts: Vec<&Message>,
    ballot_styles: Vec<BallotStyle>,
    tally_session_contest: Vec<TallySessionContest>,
    areas: &Vec<Area>,
    tenant_id: &str,
    election_event_id: &str,
    contest_encryption_policy: ContestEncryptionPolicy,
) -> Result<Vec<AreaContestDataType>> {
    event!(
        Level::WARN,
        "Num sequent_backend_tally_session_contest = {}",
        &tally_session_contest.len()
    );
    let almost_vec = match contest_encryption_policy {
        ContestEncryptionPolicy::MULTIPLE_CONTESTS => {
            generate_area_contests_mc(
                hasura_transaction,
                &relevant_plaintexts,
                &ballot_styles,
                &tally_session_contest,
                areas,
                tenant_id,
                election_event_id,
            )
            .await?
        }
        ContestEncryptionPolicy::SINGLE_CONTEST => generate_area_contests(
            &relevant_plaintexts,
            &ballot_styles,
            &tally_session_contest,
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

    Ok(filtered_area_contests)
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
    tally_session_area_contest: &[TallySessionContest],
) -> Result<Vec<ElectionCastVotes>> {
    let mut cast_votes_map: HashMap<String, ElectionCastVotes> = HashMap::new();

    // (election_id, (area_ids))
    let mut election_areas_map: HashMap<String, HashSet<String>> = HashMap::new();
    for area_contest in tally_session_area_contest {
        let annotations: serde_json::Value = area_contest
            .annotations
            .clone()
            .ok_or(anyhow!("Missing annotations in tally session contest."))?;

        let annotations: TallySessionContestAnnotations = deserialize_value(annotations)?;

        let entry = cast_votes_map
            .entry(area_contest.election_id.clone())
            .or_insert_with(|| ElectionCastVotes {
                election_id: area_contest.election_id.clone(),
                census: 0,
                cast_votes: 0,
            });

        let areas_set = election_areas_map
            .entry(area_contest.election_id.clone())
            .or_insert_with(|| HashSet::new());

        if areas_set.contains(&area_contest.area_id) {
            continue;
        }

        entry.census += annotations.elegible_voters as i64;
        entry.cast_votes += annotations.casted_ballots as i64;

        areas_set.insert(area_contest.area_id.clone());
    }

    Ok(cast_votes_map.into_values().collect())
}

#[instrument(skip_all, err)]
pub async fn upsert_ballots_messages(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    board_name: &str,
    trustee_names: Vec<String>,
    messages: &Vec<Message>,
    tally_session_contests: &Vec<TallySessionContest>,
    tally_session_hasura: &TallySession,
) -> Result<Vec<TallySessionContest>> {
    let contest_encryption_policy = tally_session_hasura
        .configuration
        .clone()
        .unwrap_or_default()
        .get_contest_encryption_policy();
    let delegated_voting_policy = tally_session_hasura
        .configuration
        .clone()
        .unwrap_or_default()
        .get_delegated_voting_policy();
    let expected_batch_ids: Vec<i64> = tally_session_contests
        .clone()
        .into_iter()
        .map(|tally_session_contest| tally_session_contest.session_id.clone() as i64)
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
    let missing_ballots_batches: Vec<TallySessionContest> = tally_session_contests
        .clone()
        .into_iter()
        .filter(|tally_session_contest| {
            !existing_ballots_batches.contains(&(tally_session_contest.session_id as i64))
        })
        .collect();

    event!(
        Level::INFO,
        "missing_ballots_batches num: {}",
        missing_ballots_batches.len()
    );

    let tally_session_contests_updated = if missing_ballots_batches.len() > 0 {
        insert_ballots_messages(
            hasura_transaction,
            keycloak_transaction,
            tenant_id,
            election_event_id,
            board_name,
            trustee_names,
            missing_ballots_batches.clone(),
            contest_encryption_policy,
            delegated_voting_policy,
        )
        .await?
    } else {
        vec![]
    };

    Ok(tally_session_contests_updated)
}

fn get_tally_session_created_at_timestamp_secs(tally_session: &TallySession) -> Result<i64> {
    let Some(created_at) = &tally_session.created_at.clone() else {
        return Err(Error::String(format!(
            "Missing created_at for tally_session"
        )));
    };
    Ok(created_at.timestamp())
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
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
    ceremony_status: TallyCeremonyStatus,
    keys_ceremony: &KeysCeremony,
    tally_session: TallySession,
    tally_session_execution: TallySessionExecution,
    tally_session_contest: Vec<TallySessionContest>,
    ballot_styles: Vec<BallotStyleHasura>,
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

    // get name of bulletin board
    let (bulletin_board, _) = get_keys_ceremony_board(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        keys_ceremony,
    )
    .await?;

    // let tally_session = &tally_session_data.sequent_backend_tally_session[0];
    let tally_session_created_at_timestamp_secs =
        get_tally_session_created_at_timestamp_secs(&tally_session)? as u64;

    let Some(execution_status) = get_execution_status(tally_session.execution_status.clone())
    else {
        event!(
            Level::INFO,
            "Election Event {} Tally execution status not found",
            election_event_id.clone()
        );
        return Ok(None);
    };

    let keys_ceremonies = get_keys_ceremonies(hasura_transaction, &tenant_id, &election_event_id)
        .await
        .with_context(|| "error listing existing keys ceremonies")?;

    if keys_ceremonies.is_empty() {
        event!(
            Level::INFO,
            "Election Event {} has no keys ceremony",
            election_event_id.clone()
        );
        return Ok(None);
    }

    let keys_ceremony_policy = keys_ceremony.policy();

    let threshold = keys_ceremonies[0].threshold as usize;
    let mut available_trustees: Vec<String> = match keys_ceremony_policy {
        CeremoniesPolicy::MANUAL_CEREMONIES => ceremony_status
            .trustees
            .into_iter()
            .filter(|trustee| TallyTrusteeStatus::KEY_RESTORED == trustee.status)
            .map(|trustee| trustee.name.clone())
            .collect(),
        CeremoniesPolicy::AUTOMATED_CEREMONIES => ceremony_status
            .trustees
            .into_iter()
            .map(|trustee| trustee.name.clone())
            .collect(),
    };

    let mut rng = StdRng::from_os_rng();
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

    let last_message_id: i64 = tally_session_execution.current_message_id as i64;

    // get board messages
    let board_client = protocol_manager::get_b3_pgsql_client().await?;
    let board_messages = board_client.get_messages(&bulletin_board, -1).await?;
    event!(Level::INFO, "Num board_messages {}", board_messages.len());

    // convert board messages into messages
    let messages: Vec<Message> = protocol_manager::convert_board_messages(&board_messages)?;
    print_messages(&messages, &bulletin_board)?;

    let new_ballots_messages = upsert_ballots_messages(
        hasura_transaction,
        keycloak_transaction,
        &tenant_id,
        &election_event_id,
        &bulletin_board,
        trustee_names,
        &messages,
        &tally_session_contest,
        &tally_session,
    )
    .await?;

    if !new_ballots_messages.is_empty() {
        update_tally_session_contests_annotations(hasura_transaction, &new_ballots_messages)
            .await?;

        event!(
            Level::INFO,
            "Ballots messages inserted: {} skipping iteration",
            new_ballots_messages.len()
        );
        return Ok(None);
    }

    // Determine whether this is a tie-break re-run by checking if any resolved
    // resolutions exist for this session.
    let tie_break_rerun = {
        let all_resolutions = get_resolution_by_tally_session(
            hasura_transaction,
            &tenant_id,
            &election_event_id,
            &tally_session_id,
        )
        .await
        .unwrap_or_default();
        !build_tie_resolutions_map(&all_resolutions).is_empty()
    };

    let newest_message_id = board_messages
        .last()
        .map(|board_message| board_message.id)
        .unwrap_or(-1);

    // In a tie-break re-run the tally replays from the last processed message;
    // normally we require a new (unprocessed) message to proceed.
    let board_message_to_process = match board_messages.iter().find(|m| m.id > last_message_id) {
        Some(msg) => msg,
        None if tie_break_rerun => {
            event!(
                Level::INFO,
                "Replaying last board message for tie-break re-run"
            );
            board_messages
                .last()
                .ok_or_else(|| anyhow::anyhow!("No board messages found for tie-break re-run"))?
        }
        None => {
            event!(Level::INFO, "No new board messages — skipping");
            return Ok(None);
        }
    };

    // Extract the timestamp before deserializing — once converted to Message the board message id is lost.
    let mut next_timestamp = Message::strand_deserialize(&board_message_to_process.message)?
        .statement
        .get_timestamp();
    next_timestamp = std::cmp::max(tally_session_created_at_timestamp_secs, next_timestamp);

    // get the batch ids that are linked to this tally session
    let batch_ids = tally_session_contest
        .iter()
        .map(|tsc| tsc.session_id as i64)
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

    let initial_status = tally_session_execution.status.clone();

    let mut new_status = get_tally_ceremony_status(initial_status)?;

    let new_tally_progress = generate_tally_progress(
        tally_session.clone(),
        tally_session_contest.clone(),
        &messages,
    )
    .await?;
    let mut new_logs = generate_logs(&messages, next_timestamp, &batch_ids)?;

    new_status.elections_status = new_tally_progress;

    {
        let mut logs = new_status.logs.clone();
        logs.append(&mut new_logs);
        new_status.logs = sort_logs(&logs);
    }

    if tie_break_rerun {
        new_status.logs = append_tally_resumed_after_resolution(&new_status.logs);
        let log_body = serde_json::json!({
            "event_type": "tally_resumed_after_resolution",
            "tally_session_id": tally_session_id,
        });
        let log_input = LogEventInput {
            election_event_id: election_event_id.clone(),
            message_type: LogMessageType::Internal,
            user_id: None,
            username: None,
            tenant_id: tenant_id.clone(),
            body: LogEventBody::Plain(
                serde_json::to_string(&log_body).unwrap_or_else(|_| "{}".to_string()),
            ),
        };
        let celery_app = get_celery_app().await;
        if let Err(e) = celery_app
            .send_task(enqueue_electoral_log_event::new(log_input))
            .await
        {
            event!(
                Level::WARN,
                "Failed to enqueue tally_resumed_after_resolution log: {}",
                e
            );
        }
    }

    // get ballot styles, from where we'll get the Contest(s)
    let ballot_styles: Vec<BallotStyle> = get_ballot_styles(&ballot_styles)?;
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

    let contest_encryption_policy = tally_session
        .configuration
        .clone()
        .unwrap_or_default()
        .get_contest_encryption_policy();
    let plaintexts_data: Vec<AreaContestDataType> = process_plaintexts(
        hasura_transaction,
        relevant_plaintexts,
        ballot_styles,
        tally_session_contest.clone(),
        &areas,
        &tenant_id,
        &election_event_id,
        contest_encryption_policy,
    )
    .await?;
    event!(Level::INFO, "Num plaintexts_data {}", plaintexts_data.len());
    let tally_sheets = clean_tally_sheets(&tally_sheet_rows, &plaintexts_data)?;

    let cast_votes_count = count_cast_votes_election_with_census(&tally_session_contest).await?;
    Ok(Some((
        plaintexts_data,
        newest_message_id,
        is_execution_completed,
        new_status,
        Some(session_ids),
        cast_votes_count,
        tally_sheets,
        election_event,
        tally_session,
    )))
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

#[instrument(err, skip(hasura_transaction, keycloak_transaction))]
pub async fn execute_tally_session_wrapped(
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tally_type: Option<String>,
    election_ids: Option<Vec<String>>,
) -> Result<()> {
    let Some((tally_session_execution, tally_session, tally_session_contests, ballot_styles)) =
        find_last_tally_session_execution_and_all_related_data(
            hasura_transaction,
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

    // Fetch resolved tie-break resolutions to pass into the velvet tally and
    // to determine has_resolved_tie_break for populate_results_tables.
    let tie_resolutions: HashMap<String, Vec<serde_json::Value>> = {
        let all_resolutions = get_resolution_by_tally_session(
            hasura_transaction,
            &tenant_id,
            &election_event_id,
            &tally_session_id,
        )
        .await
        .unwrap_or_default();
        build_tie_resolutions_map(&all_resolutions)
    };
    let has_resolved_tie_break = !tie_resolutions.is_empty();

    // map plaintexts to contests
    let plaintexts_data_opt = map_plaintext_data(
        hasura_transaction,
        keycloak_transaction,
        tenant_id.clone(),
        election_event_id.clone(),
        tally_session_id.clone(),
        status,
        &keys_ceremony,
        tally_session.clone(),
        tally_session_execution.clone(),
        tally_session_contests.clone(),
        ballot_styles.clone(),
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
        match run_velvet_tally(
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
            tie_resolutions,
        )
        .await
        {
            Ok(state) => Some(state),
            Err(err) => {
                // Propagate error - ties are no longer errors, they're detected via metadata
                return Err(err.into());
            }
        }
    } else {
        None
    };

    let default_language = election_event.get_default_language();

    let (results_event_id, tally_session_execution_documents) = populate_results_tables(
        hasura_transaction,
        &base_tempdir.path().to_path_buf(),
        status,
        &tenant_id,
        &election_event_id,
        &tally_session_id,
        session_ids.clone(),
        tally_session_execution.clone(),
        &areas,
        &default_language,
        tally_type_enum.clone(),
        plaintexts_data.is_empty(), // &tally_session,
        has_resolved_tie_break,
    )
    .await?;

    // Check if results contain tie resolution metadata (pending or resolved)
    if let Some(ref results_event_id_str) = results_event_id {
        let tie_resolutions = check_for_tie_resolutions(
            hasura_transaction,
            &tenant_id,
            &election_event_id,
            results_event_id_str,
        )
        .await?;

        // Handle pending tie resolutions (external procedure requiring input).
        // On a re-run after partial resolution the new results_event_id ensures
        // check_for_tie_resolutions only returns ties from the freshly-computed
        // results, so old annotations are not accidentally re-processed.
        if !tie_resolutions.pending.is_empty() {
            info!(
                "Detected {} pending tie resolution(s) in results - creating resolution records",
                tie_resolutions.pending.len()
            );

            // Get all existing pending resolutions for this tally session
            let existing_pending_resolutions = get_pending_resolutions(
                hasura_transaction,
                &tenant_id,
                &election_event_id,
                &tally_session_id,
            )
            .await?;

            for (results_contest_id, contest_id, tie_metadata) in &tie_resolutions.pending {
                let resolution_exists = pending_resolution_exists(
                    &existing_pending_resolutions,
                    contest_id,
                    tie_metadata,
                );

                if !resolution_exists {
                    let resolution_id = create_tally_session_resolution(
                        hasura_transaction,
                        &tenant_id,
                        &election_event_id,
                        &tally_session_id,
                        results_contest_id,
                        contest_id,
                        results_event_id_str,
                        ResolutionType::IrvTieBreak,
                        tie_metadata.clone(),
                    )
                    .await?;
                    info!(
                        "Created pending resolution {} for IRV tie-break in contest {}",
                        resolution_id, contest_id
                    );
                    let log_body = serde_json::json!({
                        "event_type": "tally_tie_detected",
                        "tally_session_id": tally_session_id,
                        "contest_id": contest_id,
                        "resolution_id": resolution_id,
                        "tied_candidate_ids": tie_metadata.get("tied_candidate_ids"),
                        "round_number": tie_metadata.get("round_number"),
                    });
                    let log_input = LogEventInput {
                        election_event_id: election_event_id.clone(),
                        message_type: LogMessageType::Internal,
                        user_id: None,
                        username: None,
                        tenant_id: tenant_id.clone(),
                        body: LogEventBody::Plain(
                            serde_json::to_string(&log_body)
                                .unwrap_or_else(|_| "{}".to_string()),
                        ),
                    };
                    let celery_app = get_celery_app().await;
                    if let Err(e) = celery_app
                        .send_task(enqueue_electoral_log_event::new(log_input))
                        .await
                    {
                        event!(
                            Level::WARN,
                            "Failed to enqueue tally_tie_detected log: {}",
                            e
                        );
                    }
                }
            }

            // Insert execution record so frontend can load partial results
            let session_ids_i32: Option<Vec<i32>> = session_ids
                .clone()
                .map(|values| values.into_iter().map(|int| int as i32).collect());
            new_status.logs =
                append_tally_updated(&new_status.logs, &election_ids.clone().unwrap_or(vec![]));
            insert_tally_session_execution(
                hasura_transaction,
                &tenant_id,
                &election_event_id,
                newest_message_id as i32,
                &tally_session_id,
                Some(new_status),
                results_event_id,
                session_ids_i32,
                tally_session_execution_documents,
            )
            .await?;

            // Update status to AWAITING_INPUT
            update_tally_session_status(
                hasura_transaction,
                &tenant_id,
                &election_event_id,
                &tally_session_id,
                TallyExecutionStatus::AWAITING_INPUT,
                false,
            )
            .await?;

            info!(
                "Tally paused - awaiting administrator tie-break decisions for {} contest(s)",
                tie_resolutions.pending.len()
            );
            let pending_contest_ids: Vec<&str> = tie_resolutions
                .pending
                .iter()
                .map(|(_, contest_id, _)| contest_id.as_str())
                .collect();
            let log_body = serde_json::json!({
                "event_type": "tally_paused_awaiting_input",
                "tally_session_id": tally_session_id,
                "pending_contest_ids": pending_contest_ids,
                "pending_count": tie_resolutions.pending.len(),
            });
            let log_input = LogEventInput {
                election_event_id: election_event_id.clone(),
                message_type: LogMessageType::Internal,
                user_id: None,
                username: None,
                tenant_id: tenant_id.clone(),
                body: LogEventBody::Plain(
                    serde_json::to_string(&log_body).unwrap_or_else(|_| "{}".to_string()),
                ),
            };
            let celery_app = get_celery_app().await;
            if let Err(e) = celery_app
                .send_task(enqueue_electoral_log_event::new(log_input))
                .await
            {
                event!(
                    Level::WARN,
                    "Failed to enqueue tally_paused_awaiting_input log: {}",
                    e
                );
            }
            return Ok(());
        }
    }

    // map_plaintext_data also calls this but at this point the credentials
    // could be expired

    let session_ids_i32: Option<Vec<i32>> = session_ids
        .clone()
        .map(|values| values.clone().into_iter().map(|int| int as i32).collect());

    new_status.logs = if is_execution_completed {
        append_tally_finished(&new_status.logs, &election_ids.clone().unwrap_or(vec![]))
    } else {
        append_tally_updated(&new_status.logs, &election_ids.clone().unwrap_or(vec![]))
    };

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
        tally_session_execution_documents,
    )
    .await?;

    if is_execution_completed {
        // update tally session to flag it as completed
        set_tally_session_completed(
            hasura_transaction,
            tenant_id.clone(),
            election_event_id.clone(),
            tally_session_id.clone(),
        )
        .await?;
        // get the election event
        let election_event =
            get_election_event_by_id(hasura_transaction, &tenant_id, &election_event_id).await?;
        let current_status = get_election_event_status(election_event.status)
            .ok_or(anyhow!("Empty election status"))?;
        let new_event_status = current_status.clone();
        let new_status_js = serde_json::to_value(new_event_status)?;
        update_election_event_status(
            hasura_transaction,
            &tenant_id,
            &election_event_id,
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

#[cfg(test)]
mod tests {

    use crate::tasks::execute_tally_session::count_cast_votes_election_with_census;
    use anyhow::anyhow;
    use anyhow::Result;
    use sequent_core::types::hasura::core::TallySessionContest;

    #[tokio::test]
    async fn test_count_cast_votes_election_with_census() -> Result<()> {
        let tally_session_contest: Vec<TallySessionContest> = vec![
            TallySessionContest {
                id: "da77960c-2982-4ee1-ae31-d78d9fccec5a".into(),
                tenant_id: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5".into(),
                election_event_id: "53c8a9ee-4cce-477b-a3a1-afc193f6a503".into(),
                area_id: "b673d615-d363-47d2-aae9-10334f987ce2".into(),
                contest_id: Some("6b3bff7c-e272-4f34-bbaf-2f27a1fad2d0".into()),
                session_id: 11,
                created_at: None,      // not used for the test
                last_updated_at: None, // not used for the test
                labels: None,
                annotations: Some(serde_json::json!({
                    "casted_ballots": 1,
                    "elegible_voters": 1,
                    "ballots_without_voter": 0
                })),
                tally_session_id: "93d39ddd-f868-4873-86fe-08948ae9e23f".into(),
                election_id: "7c261aae-5918-439a-a298-f4e89d30b5e9".into(),
            },
            TallySessionContest {
                id: "3ae7714d-76d5-4302-98d3-92d77e7162b8".into(),
                tenant_id: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5".into(),
                election_event_id: "53c8a9ee-4cce-477b-a3a1-afc193f6a503".into(),
                area_id: "7903cfd5-b782-4584-abff-0674b8507d9b".into(),
                contest_id: Some("b8bcd18f-7b1f-4c85-bcee-2ab0bf75e022".into()),
                session_id: 27,
                created_at: None,      // not used for the test
                last_updated_at: None, // not used for the test
                labels: None,
                annotations: Some(serde_json::json!({
                    "casted_ballots": 2,
                    "elegible_voters": 2,
                    "ballots_without_voter": 0
                })),
                tally_session_id: "93d39ddd-f868-4873-86fe-08948ae9e23f".into(),
                election_id: "f19bca5f-2104-43ba-a255-87870c9875c8".into(),
            },
            TallySessionContest {
                id: "338485c9-9aad-48c0-bf7c-9ed42da927a0".into(),
                tenant_id: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5".into(),
                election_event_id: "53c8a9ee-4cce-477b-a3a1-afc193f6a503".into(),
                area_id: "7903cfd5-b782-4584-abff-0674b8507d9b".into(),
                contest_id: Some("2d22e285-d9bf-45d0-a42f-86adbae3035f".into()),
                session_id: 33,
                created_at: None,      // not used for the test
                last_updated_at: None, // not used for the test
                labels: None,
                annotations: Some(serde_json::json!({
                    "casted_ballots": 2,
                    "elegible_voters": 2,
                    "ballots_without_voter": 0
                })),
                tally_session_id: "93d39ddd-f868-4873-86fe-08948ae9e23f".into(),
                election_id: "f19bca5f-2104-43ba-a255-87870c9875c8".into(),
            },
            TallySessionContest {
                id: "fa96870c-fc44-42ef-8a3c-b7d31a5a22cb".into(),
                tenant_id: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5".into(),
                election_event_id: "53c8a9ee-4cce-477b-a3a1-afc193f6a503".into(),
                area_id: "b673d615-d363-47d2-aae9-10334f987ce2".into(),
                contest_id: Some("b8bcd18f-7b1f-4c85-bcee-2ab0bf75e022".into()),
                session_id: 42,
                created_at: None,      // not used for the test
                last_updated_at: None, // not used for the test
                labels: None,
                annotations: Some(serde_json::json!({
                    "casted_ballots": 1,
                    "elegible_voters": 1,
                    "ballots_without_voter": 0
                })),
                tally_session_id: "93d39ddd-f868-4873-86fe-08948ae9e23f".into(),
                election_id: "f19bca5f-2104-43ba-a255-87870c9875c8".into(),
            },
            TallySessionContest {
                id: "1605576f-c35e-4af0-8a2b-f98e4295ac17".into(),
                tenant_id: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5".into(),
                election_event_id: "53c8a9ee-4cce-477b-a3a1-afc193f6a503".into(),
                area_id: "b673d615-d363-47d2-aae9-10334f987ce2".into(),
                contest_id: Some("438089fa-2d40-487b-be71-824ba5376212".into()),
                session_id: 47,
                created_at: None,      // not used for the test
                last_updated_at: None, // not used for the test
                labels: None,
                annotations: Some(serde_json::json!({
                    "casted_ballots": 1,
                    "elegible_voters": 1,
                    "ballots_without_voter": 0
                })),
                tally_session_id: "93d39ddd-f868-4873-86fe-08948ae9e23f".into(),
                election_id: "7c261aae-5918-439a-a298-f4e89d30b5e9".into(),
            },
            TallySessionContest {
                id: "527fa899-290c-460d-a76b-326babec03cd".into(),
                tenant_id: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5".into(),
                election_event_id: "53c8a9ee-4cce-477b-a3a1-afc193f6a503".into(),
                area_id: "7903cfd5-b782-4584-abff-0674b8507d9b".into(),
                contest_id: Some("6b3bff7c-e272-4f34-bbaf-2f27a1fad2d0".into()),
                session_id: 48,
                created_at: None,      // not used for the test
                last_updated_at: None, // not used for the test
                labels: None,
                annotations: Some(serde_json::json!({
                    "casted_ballots": 2,
                    "elegible_voters": 2,
                    "ballots_without_voter": 0
                })),
                tally_session_id: "93d39ddd-f868-4873-86fe-08948ae9e23f".into(),
                election_id: "7c261aae-5918-439a-a298-f4e89d30b5e9".into(),
            },
            TallySessionContest {
                id: "547fac16-5af7-4084-ad39-b9b72ea83613".into(),
                tenant_id: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5".into(),
                election_event_id: "53c8a9ee-4cce-477b-a3a1-afc193f6a503".into(),
                area_id: "b673d615-d363-47d2-aae9-10334f987ce2".into(),
                contest_id: Some("2d22e285-d9bf-45d0-a42f-86adbae3035f".into()),
                session_id: 49,
                created_at: None,      // not used for the test
                last_updated_at: None, // not used for the test
                labels: None,
                annotations: Some(serde_json::json!({
                    "casted_ballots": 1,
                    "elegible_voters": 1,
                    "ballots_without_voter": 0
                })),
                tally_session_id: "93d39ddd-f868-4873-86fe-08948ae9e23f".into(),
                election_id: "f19bca5f-2104-43ba-a255-87870c9875c8".into(),
            },
        ];

        let cast_votes_count =
            count_cast_votes_election_with_census(&tally_session_contest).await?;
        let election_ee1e1 = cast_votes_count
            .get(0)
            .ok_or(anyhow!("Election1 not found"))?;
        let election_ee1e2 = cast_votes_count
            .get(1)
            .ok_or(anyhow!("Election2 not found"))?;

        assert_eq!(election_ee1e1.census, 3);
        assert_eq!(election_ee1e1.cast_votes, 3);
        assert_eq!(election_ee1e2.census, 3);
        assert_eq!(election_ee1e2.cast_votes, 3);

        Ok(())
    }

    // -------------------------------------------------------------------------
    // Tests for build_tie_resolutions_map
    // -------------------------------------------------------------------------

    use crate::postgres::tally_session_resolution::{
        ResolutionStatus, ResolutionType, TallySessionResolution,
    };
    use crate::tasks::execute_tally_session::{
        build_tie_resolutions_map, parse_pending_tie_from_annotations, pending_resolution_exists,
    };

    fn make_resolution(
        contest_id: &str,
        round_number: u64,
        candidate_id: &str,
        resolution_type: ResolutionType,
    ) -> TallySessionResolution {
        TallySessionResolution {
            id: uuid::Uuid::new_v4().to_string(),
            tenant_id: "tenant-1".to_string(),
            election_event_id: "event-1".to_string(),
            tally_session_id: "session-1".to_string(),
            results_contest_id: None,
            contest_id: Some(contest_id.to_string()),
            results_event_id: None,
            created_at: None,
            last_updated_at: None,
            resolution_type,
            status: ResolutionStatus::Resolved,
            resolution_data: serde_json::json!({"round_number": round_number}),
            resolution: Some(serde_json::json!({"resolved_by_candidate_id": candidate_id})),
            resolved_by_user: None,
            resolved_at: None,
            labels: None,
            annotations: None,
        }
    }

    /// Multiple resolved rows for the same contest_id (different rounds) must
    /// all be preserved under the single contest key, not overwritten.
    #[test]
    fn test_build_tie_resolutions_map_groups_by_contest() {
        let rows = vec![
            make_resolution("contest-1", 1, "candidate-a", ResolutionType::IrvTieBreak),
            make_resolution("contest-1", 3, "candidate-c", ResolutionType::IrvTieBreak),
            make_resolution("contest-2", 2, "candidate-b", ResolutionType::IrvTieBreak),
        ];
        let map = build_tie_resolutions_map(&rows);

        assert_eq!(map.len(), 2, "Two distinct contest IDs expected");

        let contest1 = map.get("contest-1").unwrap();
        assert_eq!(
            contest1.len(),
            2,
            "Both round-1 and round-3 entries must be present"
        );
        let rounds: Vec<u64> = contest1
            .iter()
            .filter_map(|v| v.get("round_number").and_then(|n| n.as_u64()))
            .collect();
        assert!(rounds.contains(&1));
        assert!(rounds.contains(&3));

        let contest2 = map.get("contest-2").unwrap();
        assert_eq!(contest2.len(), 1);
        assert_eq!(
            contest2[0]
                .get("resolved_by_candidate_id")
                .and_then(|v| v.as_str()),
            Some("candidate-b")
        );
    }

    /// Rows whose resolution_type is not IrvTieBreak must be ignored.
    #[test]
    fn test_build_tie_resolutions_map_ignores_non_irv_rows() {
        let rows = vec![
            make_resolution("contest-1", 1, "candidate-a", ResolutionType::IrvTieBreak),
            make_resolution("contest-1", 2, "candidate-b", ResolutionType::ManualRecount),
        ];
        let map = build_tie_resolutions_map(&rows);
        let contest1 = map.get("contest-1").unwrap();
        assert_eq!(contest1.len(), 1, "ManualRecount row must be excluded");
    }

    /// Rows with a missing contest_id or no resolution must be skipped.
    #[test]
    fn test_build_tie_resolutions_map_skips_incomplete_rows() {
        let mut no_contest_id =
            make_resolution("contest-1", 1, "candidate-a", ResolutionType::IrvTieBreak);
        no_contest_id.contest_id = None;

        let mut no_resolution =
            make_resolution("contest-1", 2, "candidate-a", ResolutionType::IrvTieBreak);
        no_resolution.resolution = None;

        let map = build_tie_resolutions_map(&[no_contest_id, no_resolution]);
        assert!(map.is_empty(), "Incomplete rows must produce an empty map");
    }

    /// An empty slice must produce an empty map (no panics).
    #[test]
    fn test_build_tie_resolutions_map_empty_input() {
        let map = build_tie_resolutions_map(&[]);
        assert!(map.is_empty());
    }

    /// When resolution_data carries no "round_number" key the entry must be
    /// stored with round_number = 0 rather than being discarded.
    #[test]
    fn test_build_tie_resolutions_map_defaults_round_to_zero_when_missing() {
        let mut row = make_resolution("contest-1", 99, "candidate-a", ResolutionType::IrvTieBreak);
        // Overwrite resolution_data with an object that lacks round_number
        row.resolution_data = serde_json::json!({"other_field": "value"});

        let map = build_tie_resolutions_map(&[row]);
        let entries = map.get("contest-1").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].get("round_number").and_then(|v| v.as_u64()),
            Some(0),
            "Missing round_number must default to 0"
        );
    }

    /// A resolved row whose resolution object does not contain
    /// "resolved_by_candidate_id" must be silently skipped.
    #[test]
    fn test_build_tie_resolutions_map_skips_row_missing_candidate_id() {
        let mut row = make_resolution("contest-1", 1, "candidate-a", ResolutionType::IrvTieBreak);
        // resolution exists but contains no resolved_by_candidate_id key
        row.resolution = Some(serde_json::json!({"some_other_key": "value"}));

        let map = build_tie_resolutions_map(&[row]);
        assert!(
            map.is_empty(),
            "Row missing resolved_by_candidate_id must be excluded"
        );
    }

    // -------------------------------------------------------------------------
    // Tests for parse_pending_tie_from_annotations
    // -------------------------------------------------------------------------

    /// Well-formed annotations must return the pending_tie_resolution object.
    #[test]
    fn test_parse_pending_tie_from_annotations_extracts_pending() {
        let tie_metadata = serde_json::json!({
            "round_number": 2,
            "tied_candidate_ids": ["candidate-a", "candidate-c"],
        });
        let annotations = serde_json::json!({
            "process_results": {
                "pending_tie_resolution": tie_metadata,
            }
        });

        let result = parse_pending_tie_from_annotations(&annotations);
        assert!(result.is_some(), "Should extract pending_tie_resolution");
        let extracted = result.unwrap();
        assert_eq!(extracted["round_number"], 2);
        assert_eq!(
            extracted["tied_candidate_ids"],
            serde_json::json!(["candidate-a", "candidate-c"])
        );
    }

    /// Annotations without pending_tie_resolution must return None.
    #[test]
    fn test_parse_pending_tie_from_annotations_returns_none_when_absent() {
        let annotations = serde_json::json!({"process_results": {"some_other_key": 1}});
        assert!(parse_pending_tie_from_annotations(&annotations).is_none());

        let no_process_results = serde_json::json!({"other": "data"});
        assert!(parse_pending_tie_from_annotations(&no_process_results).is_none());
    }

    /// When process_results is not a JSON object (e.g. a string or array) the
    /// as_object() guard must prevent a panic and return None.
    #[test]
    fn test_parse_pending_tie_when_process_results_is_not_an_object() {
        let string_value = serde_json::json!({"process_results": "unexpected string"});
        assert!(parse_pending_tie_from_annotations(&string_value).is_none());

        let array_value = serde_json::json!({"process_results": [1, 2, 3]});
        assert!(parse_pending_tie_from_annotations(&array_value).is_none());
    }

    /// pending_tie_resolution: null is stored as JSON null; the function must
    /// return Some(Value::Null) because the key exists.
    #[test]
    fn test_parse_pending_tie_returns_null_value_when_key_is_null() {
        let annotations = serde_json::json!({
            "process_results": {"pending_tie_resolution": null}
        });
        let result = parse_pending_tie_from_annotations(&annotations);
        assert!(
            result.is_some(),
            "Key present with null value must return Some"
        );
        assert!(result.unwrap().is_null());
    }

    // -------------------------------------------------------------------------
    // Tests for pending_resolution_exists
    // These cover the (contest_id, round_number) uniqueness key used when
    // ProcessBallotsAll is set at area level and the same contest can tie in
    // different rounds across areas within a single tally run.
    // -------------------------------------------------------------------------

    fn make_pending_resolution(contest_id: &str, round_number: u64) -> TallySessionResolution {
        TallySessionResolution {
            id: uuid::Uuid::new_v4().to_string(),
            tenant_id: "tenant-1".to_string(),
            election_event_id: "event-1".to_string(),
            tally_session_id: "session-1".to_string(),
            results_contest_id: None,
            contest_id: Some(contest_id.to_string()),
            results_event_id: None,
            created_at: None,
            last_updated_at: None,
            resolution_type: ResolutionType::IrvTieBreak,
            status: ResolutionStatus::Pending,
            resolution_data: serde_json::json!({"round_number": round_number}),
            resolution: None,
            resolved_by_user: None,
            resolved_at: None,
            labels: None,
            annotations: None,
        }
    }

    /// Empty list must never report an existing resolution (default SkipCandidateResults path).
    #[test]
    fn test_pending_resolution_exists_false_when_list_empty() {
        let tie_metadata = serde_json::json!({"round_number": 2});
        assert!(!pending_resolution_exists(&[], "contest-x", &tie_metadata));
    }

    /// Same contest and same round — the existing pending record must be found.
    #[test]
    fn test_pending_resolution_exists_true_for_same_contest_and_round() {
        let existing = vec![make_pending_resolution("contest-x", 2)];
        let tie_metadata = serde_json::json!({"round_number": 2});
        assert!(pending_resolution_exists(
            &existing,
            "contest-x",
            &tie_metadata
        ));
    }

    /// Same contest but different round — area-level ProcessBallotsAll can produce
    /// independent ties per round; they must be treated as distinct and both created.
    #[test]
    fn test_pending_resolution_exists_false_for_same_contest_different_round() {
        let existing = vec![make_pending_resolution("contest-x", 2)];
        let tie_metadata = serde_json::json!({"round_number": 3});
        assert!(!pending_resolution_exists(
            &existing,
            "contest-x",
            &tie_metadata
        ));
    }

    /// Different contest, same round — must not collide.
    #[test]
    fn test_pending_resolution_exists_false_for_different_contest() {
        let existing = vec![make_pending_resolution("contest-x", 2)];
        let tie_metadata = serde_json::json!({"round_number": 2});
        assert!(!pending_resolution_exists(
            &existing,
            "contest-y",
            &tie_metadata
        ));
    }

    /// A pending record with a non-IrvTieBreak type must not match even if contest
    /// and round match.
    #[test]
    fn test_pending_resolution_exists_ignores_non_irv_type() {
        let mut r = make_pending_resolution("contest-x", 2);
        r.resolution_type = ResolutionType::ManualRecount;
        let tie_metadata = serde_json::json!({"round_number": 2});
        assert!(!pending_resolution_exists(&[r], "contest-x", &tie_metadata));
    }

    /// Two areas produce ties for the same contest in different rounds
    /// (ProcessBallotsAll at area level). Both rounds must be independently
    /// detectable — neither should suppress the other.
    #[test]
    fn test_pending_resolution_exists_area_level_different_rounds_are_independent() {
        let round2 = make_pending_resolution("contest-x", 2);
        let round3 = make_pending_resolution("contest-x", 3);
        let existing = vec![round2, round3];

        // Querying for round 2 finds it.
        let meta_r2 = serde_json::json!({"round_number": 2});
        assert!(pending_resolution_exists(&existing, "contest-x", &meta_r2));

        // Querying for round 3 finds it.
        let meta_r3 = serde_json::json!({"round_number": 3});
        assert!(pending_resolution_exists(&existing, "contest-x", &meta_r3));

        // Round 4 has no record yet.
        let meta_r4 = serde_json::json!({"round_number": 4});
        assert!(!pending_resolution_exists(&existing, "contest-x", &meta_r4));
    }
}
