// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::area::{self, get_areas, get_areas_by_ids, get_event_areas};
use crate::postgres::area_contest::{export_area_contests, get_area_contests_by_area_contest_ids};
use crate::postgres::contest::{export_contests, get_contest_by_election_ids};
use crate::postgres::document;
use crate::postgres::election::{get_elections, get_elections_by_ids};
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::results_area_contest::insert_results_area_contests;
use crate::postgres::results_area_contest_candidate::insert_results_area_contest_candidates;
use crate::postgres::results_contest::insert_results_contests;
use crate::postgres::results_contest_candidate::insert_results_contest_candidates;
use crate::postgres::results_election::insert_results_elections;
use crate::postgres::results_event::insert_results_event;
use crate::services::ceremonies::result_documents::save_result_documents;
use crate::services::documents::upload_and_return_document;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use rusqlite::Connection;
use rusqlite::Transaction as SqliteTransaction;
use sequent_core::sqlite::results_event::find_results_event_sqlite;
use sequent_core::types::ceremonies::{TallySessionDocuments, TallyType};
use sequent_core::types::hasura::core::TallySessionExecution;
use sequent_core::types::hasura::core::{Area, TallySession};
use sequent_core::types::results::{
    ResultsAreaContest, ResultsAreaContestCandidate, ResultsContest, ResultsContestCandidate,
    ResultsElection, EXTENDED_METRICS, PROCESS_RESULTS,
};
use sequent_core::util::temp_path::get_file_size;
use serde_json::{json, Map, Value};
use std::cmp;
use std::path::PathBuf;
use tempfile::{NamedTempFile, TempPath};
use tracing::info;
use tracing::{event, instrument, Level};
use uuid::Uuid;
use velvet::cli::state::State;
use velvet::pipes::generate_db::DATABASE_FILENAME;
use velvet::pipes::generate_reports::ElectionReportDataComputed;
use velvet::pipes::pipe_name::PipeNameOutputDir;

/// Converts `u64` counts to `f64` for percentage math (same rounding as `as f64`).
#[allow(clippy::cast_precision_loss)]
#[inline]
const fn u64_to_f64(n: u64) -> f64 {
    n as f64
}

/// Converts `usize` ranks to `i64` for persistence (same wrapping as `as i64` on overflow).
#[allow(clippy::cast_possible_wrap)]
#[inline]
const fn usize_to_i64(value: usize) -> i64 {
    value as i64
}

/// Inserts contest, area-contest, election, and candidate result rows for `results_event_id` from
/// Velvet’s computed [`ElectionReportDataComputed`] vector.
///
/// # Panics
///
/// Panics if serializing extended contest metrics to JSON fails (`expect` on `serde_json::to_value`).
///
/// # Errors
///
/// Percent-to-fraction conversions that fail `try_into`, or any Postgres insert error from the
/// `insert_results_*` helpers.
#[allow(clippy::too_many_lines)]
#[instrument(skip_all)]
pub async fn save_results(
    hasura_transaction: &Transaction<'_>,
    results: Vec<ElectionReportDataComputed>,
    tenant_id: &str,
    election_event_id: &str,
    results_event_id: &str,
) -> Result<()> {
    let mut results_contests: Vec<ResultsContest> = Vec::new();
    let mut results_area_contests: Vec<ResultsAreaContest> = Vec::new();
    let mut results_elections: Vec<ResultsElection> = Vec::new();
    let mut results_contest_candidates: Vec<ResultsContestCandidate> = Vec::new();
    let mut results_area_contest_candidates: Vec<ResultsAreaContestCandidate> = Vec::new();
    for election in &results {
        let total_voters_percent: f64 =
            u64_to_f64(election.total_votes) / u64_to_f64(cmp::max(election.census, 1));
        results_elections.push(ResultsElection {
            id: Uuid::new_v4().into(),
            tenant_id: tenant_id.into(),
            election_event_id: election_event_id.into(),
            election_id: election.election_id.clone(),
            results_event_id: results_event_id.into(),
            name: None,
            elegible_census: Some(election.census.cast_signed()),
            total_voters: Some(election.total_votes.cast_signed()),
            created_at: None,
            last_updated_at: None,
            labels: None,
            annotations: None,
            total_voters_percent: Some(total_voters_percent.clamp(0.0, 1.0).try_into()?),
            documents: None,
        });

        for contest in &election.reports {
            let Some(contest_result) = contest.contest_result.clone() else {
                continue;
            };
            let Some(current_contest) = contest.contest.clone() else {
                continue;
            };

            let contest_total_votes_percent: f64 = contest_result.percentage_total_votes / 100.0;
            let auditable_votes_percent: f64 = contest_result.percentage_auditable_votes / 100.0;
            let total_valid_votes_percent: f64 =
                contest_result.percentage_total_valid_votes / 100.0;
            let total_invalid_votes_percent: f64 =
                contest_result.percentage_total_invalid_votes / 100.0;
            let explicit_invalid_votes_percent: f64 =
                contest_result.percentage_invalid_votes_explicit / 100.0;
            let implicit_invalid_votes_percent: f64 =
                contest_result.percentage_invalid_votes_implicit / 100.0;
            let total_blank_votes_percent: f64 =
                contest_result.percentage_total_blank_votes / 100.0;

            let contest_result_ext_metrics = contest_result.extended_metrics.unwrap_or_default();
            let extended_metrics_value = serde_json::to_value(contest_result_ext_metrics)
                .expect("Failed to convert to JSON");
            let votes_base: f64 = u64_to_f64(cmp::max(contest_result_ext_metrics.total_weight, 1));
            let mut annotation_map = Map::new();
            annotation_map.insert(EXTENDED_METRICS.to_string(), extended_metrics_value);
            if let Some(process_results) = contest_result.process_results.clone() {
                annotation_map.insert(PROCESS_RESULTS.to_string(), process_results);
            }
            let annotations = Value::Object(annotation_map);

            if let Some(area) = &contest.area {
                results_area_contests.push(ResultsAreaContest {
                    id: Uuid::new_v4().into(),
                    tenant_id: tenant_id.into(),
                    election_event_id: election_event_id.into(),
                    election_id: election.election_id.clone(),
                    contest_id: current_contest.id.clone(),
                    area_id: area.id.clone(),
                    results_event_id: results_event_id.into(),
                    elegible_census: Some(contest_result.census.cast_signed()),
                    total_votes: Some(contest_result.total_votes.cast_signed()),
                    total_votes_percent: Some(
                        contest_total_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                    total_auditable_votes: Some(contest_result.auditable_votes.cast_signed()),
                    total_auditable_votes_percent: Some(
                        auditable_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                    total_valid_votes: Some(contest_result.total_valid_votes.cast_signed()),
                    total_valid_votes_percent: Some(
                        total_valid_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                    total_invalid_votes: Some(contest_result.total_invalid_votes.cast_signed()),
                    total_invalid_votes_percent: Some(
                        total_invalid_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                    explicit_invalid_votes: Some(
                        contest_result.invalid_votes.explicit.cast_signed(),
                    ),
                    explicit_invalid_votes_percent: Some(
                        explicit_invalid_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                    implicit_invalid_votes: Some(
                        contest_result.invalid_votes.implicit.cast_signed(),
                    ),
                    implicit_invalid_votes_percent: Some(
                        implicit_invalid_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                    blank_votes: Some(contest_result.total_blank_votes.cast_signed()),
                    blank_votes_percent: Some(
                        total_blank_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                    created_at: None,
                    last_updated_at: None,
                    labels: None,
                    annotations: Some(annotations),
                    documents: None,
                });

                for candidate in &contest.candidate_result {
                    let cast_votes_percent: f64 = u64_to_f64(candidate.total_count) / votes_base;
                    results_area_contest_candidates.push(ResultsAreaContestCandidate {
                        id: Uuid::new_v4().into(),
                        tenant_id: tenant_id.into(),
                        election_event_id: election_event_id.into(),
                        election_id: election.election_id.clone(),
                        contest_id: current_contest.id.clone(),
                        candidate_id: candidate.candidate.id.clone(),
                        results_event_id: results_event_id.into(),
                        area_id: area.id.clone(),
                        cast_votes: Some(candidate.total_count.cast_signed()),
                        cast_votes_percent: Some(cast_votes_percent.clamp(0.0, 1.0).try_into()?),
                        winning_position: candidate.winning_position.map(usize_to_i64),
                        points: None,
                        created_at: None,
                        last_updated_at: None,
                        labels: None,
                        annotations: None,
                        documents: None,
                    });
                }
            } else {
                results_contests.push(ResultsContest {
                    id: Uuid::new_v4().into(),
                    tenant_id: tenant_id.into(),
                    election_event_id: election_event_id.into(),
                    election_id: election.election_id.clone(),
                    contest_id: current_contest.id.clone(),
                    results_event_id: results_event_id.into(),
                    elegible_census: Some(contest_result.census.cast_signed()),
                    total_valid_votes: Some(contest_result.total_valid_votes.cast_signed()),
                    explicit_invalid_votes: Some(
                        contest_result.invalid_votes.explicit.cast_signed(),
                    ),
                    implicit_invalid_votes: Some(
                        contest_result.invalid_votes.implicit.cast_signed(),
                    ),
                    blank_votes: Some(contest_result.total_blank_votes.cast_signed()),
                    voting_type: current_contest.voting_type.clone(),
                    counting_algorithm: current_contest
                        .counting_algorithm
                        .map(|val| val.to_string()),
                    name: current_contest.name.clone(),
                    created_at: None,
                    last_updated_at: None,
                    labels: None,
                    annotations: Some(annotations),
                    total_invalid_votes: Some(contest_result.total_invalid_votes.cast_signed()),
                    total_invalid_votes_percent: Some(
                        total_invalid_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                    total_valid_votes_percent: Some(
                        total_valid_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                    explicit_invalid_votes_percent: Some(
                        explicit_invalid_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                    implicit_invalid_votes_percent: Some(
                        implicit_invalid_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                    blank_votes_percent: Some(
                        total_blank_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                    total_votes: Some(contest_result.total_votes.cast_signed()),
                    total_votes_percent: Some(
                        contest_total_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                    documents: None,
                    total_auditable_votes: Some(contest_result.auditable_votes.cast_signed()),
                    total_auditable_votes_percent: Some(
                        auditable_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                });

                for candidate in &contest.candidate_result {
                    let cast_votes_percent: f64 = u64_to_f64(candidate.total_count) / votes_base;
                    results_contest_candidates.push(ResultsContestCandidate {
                        id: Uuid::new_v4().into(),
                        tenant_id: tenant_id.into(),
                        election_event_id: election_event_id.into(),
                        election_id: election.election_id.clone(),
                        contest_id: current_contest.id.clone(),
                        candidate_id: candidate.candidate.id.clone(),
                        results_event_id: results_event_id.into(),
                        cast_votes: Some(candidate.total_count.cast_signed()),
                        winning_position: candidate.winning_position.map(usize_to_i64),
                        points: None,
                        created_at: None,
                        last_updated_at: None,
                        labels: None,
                        annotations: None,
                        cast_votes_percent: Some(cast_votes_percent.clamp(0.0, 1.0).try_into()?),
                        documents: None,
                    });
                }
            }
        }
    }
    insert_results_contests(
        hasura_transaction,
        tenant_id,
        election_event_id,
        results_event_id,
        results_contests.clone(),
    )
    .await?;

    insert_results_area_contests(
        hasura_transaction,
        tenant_id,
        election_event_id,
        results_event_id,
        results_area_contests.clone(),
    )
    .await?;

    insert_results_elections(
        hasura_transaction,
        tenant_id,
        election_event_id,
        results_event_id,
        results_elections.clone(),
    )
    .await?;

    insert_results_contest_candidates(
        hasura_transaction,
        tenant_id,
        election_event_id,
        results_event_id,
        results_contest_candidates.clone(),
    )
    .await?;

    insert_results_area_contest_candidates(
        hasura_transaction,
        tenant_id,
        election_event_id,
        results_event_id,
        results_area_contest_candidates.clone(),
    )
    .await?;

    Ok(())
}

/// When `force_new_id` is set or the tally gained new session batches, inserts a new `results_event`
/// row (sourced from `SQLite` when a transaction is supplied) so later writes target a fresh id.
///
/// # Errors
///
/// `SQLite` lookup failures, missing results-event metadata, or Postgres insert failures.
#[allow(clippy::future_not_send)]
#[instrument(skip_all)]
pub async fn generate_results_id_if_necessary(
    hasura_transaction: &Transaction<'_>,
    sqlite_transaction_opt: Option<&SqliteTransaction<'_>>,
    tenant_id: &str,
    election_event_id: &str,
    session_ids_opt: Option<Vec<i64>>,
    previous_execution: TallySessionExecution,
    state_opt: &Option<State>,
    force_new_id: bool,
) -> Result<Option<String>> {
    if state_opt.is_none() {
        return Ok(None);
    }
    let previous_session_ids = previous_execution.session_ids.unwrap_or(vec![]);
    let session_ids = session_ids_opt.unwrap_or(vec![]);

    if !force_new_id && (session_ids.len() <= previous_session_ids.len()) {
        return Ok(None);
    }

    if let Some(sqlite_transaction) = sqlite_transaction_opt {
        let results_event =
            find_results_event_sqlite(sqlite_transaction, tenant_id, election_event_id)
                .context("Failed to find results event table")?;

        insert_results_event(
            hasura_transaction,
            tenant_id,
            election_event_id,
            &results_event.id,
        )
        .await?;
        Ok(Some(results_event.id))
    } else {
        Ok(None)
    }
}

/// Persists aggregates and report documents for the optional new `results_event_id`;
/// otherwise returns the previous execution’s event id.
///
/// # Errors
/// Should never return an error.
#[allow(clippy::future_not_send, clippy::large_futures)]
#[instrument(skip_all)]
pub async fn process_results_tables(
    hasura_transaction: &Transaction<'_>,
    base_tally_path: &PathBuf,
    state_opt: Option<State>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
    session_ids: Option<Vec<i64>>,
    previous_execution: TallySessionExecution,
    areas: &Vec<Area>,
    default_language: &str,
    tally_type_enum: TallyType,
    sqlite_transaction_opt: Option<&SqliteTransaction<'_>>,
    force_new_id: bool,
) -> Result<Option<String>> {
    let results_event_id_opt = generate_results_id_if_necessary(
        hasura_transaction,
        sqlite_transaction_opt,
        tenant_id,
        election_event_id,
        session_ids,
        previous_execution.clone(),
        &state_opt,
        force_new_id,
    )
    .await?;

    if let (Some(results_event_id), Some(state)) = (results_event_id_opt.clone(), state_opt) {
        if let Ok(results) = state.get_results(false) {
            save_results(
                hasura_transaction,
                results.clone(),
                tenant_id,
                election_event_id,
                &results_event_id,
            )
            .await?;
            save_result_documents(
                hasura_transaction,
                results.clone(),
                tenant_id,
                election_event_id,
                &results_event_id,
                base_tally_path,
                areas,
                default_language,
                tally_type_enum,
                sqlite_transaction_opt,
            )
            .await?;
        }

        Ok(results_event_id_opt)
    } else {
        Ok(previous_execution.results_event_id)
    }
}

/// Updates the `SQLite` results database tables and uploads the artifact to object storage,
/// and returns the active `results_event_id` and document handles.
///
/// # Errors
///
/// `SQLite` open/transaction failures, async errors propagated through `block_in_place`, document
/// upload failures, or missing filesystem paths when preparing uploads.
#[allow(clippy::large_futures)]
#[instrument(skip(hasura_transaction, state_opt, previous_execution, areas))]
pub async fn populate_results_tables(
    hasura_transaction: &Transaction<'_>,
    base_tally_path: &PathBuf,
    state_opt: Option<State>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
    session_ids: Option<Vec<i64>>,
    previous_execution: TallySessionExecution,
    areas: &Vec<Area>,
    default_language: &str,
    tally_type_enum: TallyType,
    is_empty: bool,
    force_new_id: bool,
) -> Result<(Option<String>, Option<TallySessionDocuments>)> {
    let velvet_output_dir = base_tally_path.join("output");
    let base_database_path = velvet_output_dir.join(PipeNameOutputDir::GenerateDatabase.as_ref());
    let database_path = base_database_path.join(DATABASE_FILENAME);
    let document_id = Uuid::new_v4().to_string();

    let results_event_id_opt = if is_empty {
        let results_event_id_opt =
            tokio::task::block_in_place(|| -> anyhow::Result<Option<String>> {
                let process_result = tokio::runtime::Handle::current().block_on(async {
                    process_results_tables(
                        hasura_transaction,
                        base_tally_path,
                        state_opt,
                        tenant_id,
                        election_event_id,
                        tally_session_id,
                        session_ids,
                        previous_execution,
                        areas,
                        default_language,
                        tally_type_enum,
                        None,
                        force_new_id,
                    )
                    .await
                })?;
                Ok(process_result)
            })?;
        results_event_id_opt
    } else {
        let results_event_id_opt =
            tokio::task::block_in_place(|| -> anyhow::Result<Option<String>> {
                let mut sqlite_connection = Connection::open(&database_path)?;
                let sqlite_transaction = sqlite_connection.transaction()?;

                let process_result = tokio::runtime::Handle::current().block_on(async {
                    process_results_tables(
                        hasura_transaction,
                        base_tally_path,
                        state_opt,
                        tenant_id,
                        election_event_id,
                        tally_session_id,
                        session_ids,
                        previous_execution,
                        areas,
                        default_language,
                        tally_type_enum,
                        Some(&sqlite_transaction),
                        force_new_id,
                    )
                    .await
                })?;
                sqlite_transaction.commit()?;
                Ok(process_result)
            })?;
        results_event_id_opt
    };

    if let Some(ref results_event_id) = results_event_id_opt {
        let file_name = format!("results-{results_event_id}.db");
        let file_path = database_path.to_str().ok_or(anyhow!("Empty upload path"))?;
        let file_size = get_file_size(file_path)?;

        let _document = upload_and_return_document(
            hasura_transaction,
            file_path,
            file_size,
            "application/vnd.sqlite3",
            tenant_id,
            Some(election_event_id.to_string()),
            &file_name,
            Some(document_id.clone()),
            false,
        )
        .await?;

        let documents = TallySessionDocuments {
            sqlite: Some(document_id.clone()),
            xlsx: None,
        };

        Ok((results_event_id_opt, Some(documents)))
    } else {
        Ok((results_event_id_opt, None))
    }
}
