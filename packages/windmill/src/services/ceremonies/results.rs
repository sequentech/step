// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
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
use sequent_core::types::results::*;
use sequent_core::util::temp_path::get_file_size;
use serde_json::json;
use std::cmp;
use std::path::PathBuf;
use tempfile::{NamedTempFile, TempPath};
use tracing::info;
use tracing::{event, instrument, Level};
use uuid::Uuid;
use velvet::cli::state::State;
use velvet::pipes::generate_reports::ElectionReportDataComputed;

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
            (election.total_votes as f64) / (cmp::max(election.census, 1) as f64);
        results_elections.push(ResultsElection {
            id: Uuid::new_v4().into(),
            tenant_id: tenant_id.into(),
            election_event_id: election_event_id.into(),
            election_id: election.election_id.clone(),
            results_event_id: results_event_id.into(),
            name: None,
            elegible_census: Some(election.census as i64),
            total_voters: Some(election.total_votes as i64),
            created_at: None,
            last_updated_at: None,
            labels: None,
            annotations: None,
            total_voters_percent: Some(total_voters_percent.clamp(0.0, 1.0).try_into()?),
            documents: None,
        });

        for contest in &election.reports {
            let total_votes_percent: f64 = contest.contest_result.percentage_total_votes / 100.0;
            let auditable_votes_percent: f64 =
                contest.contest_result.percentage_auditable_votes / 100.0;
            let total_valid_votes_percent: f64 =
                contest.contest_result.percentage_total_valid_votes / 100.0;
            let total_invalid_votes_percent: f64 =
                contest.contest_result.percentage_total_invalid_votes / 100.0;
            let explicit_invalid_votes_percent: f64 =
                contest.contest_result.percentage_invalid_votes_explicit / 100.0;
            let implicit_invalid_votes_percent: f64 =
                contest.contest_result.percentage_invalid_votes_implicit / 100.0;
            let total_blank_votes_percent: f64 =
                contest.contest_result.percentage_total_blank_votes / 100.0;

            let extended_metrics_value = serde_json::to_value(
                contest
                    .contest_result
                    .extended_metrics
                    .clone()
                    .unwrap_or_default(),
            )
            .expect("Failed to convert to JSON");
            let mut annotations = json!({});
            annotations["extended_metrics"] = extended_metrics_value;

            if let Some(area) = &contest.area {
                results_area_contests.push(ResultsAreaContest {
                    id: Uuid::new_v4().into(),
                    tenant_id: tenant_id.into(),
                    election_event_id: election_event_id.into(),
                    election_id: election.election_id.clone(),
                    contest_id: contest.contest.id.clone(),
                    area_id: area.id.clone(),
                    results_event_id: results_event_id.into(),
                    elegible_census: Some(contest.contest_result.census as i64),
                    total_votes: Some(contest.contest_result.total_votes as i64),
                    total_votes_percent: Some(total_votes_percent.clamp(0.0, 1.0).try_into()?),
                    total_auditable_votes: Some(contest.contest_result.auditable_votes as i64),
                    total_auditable_votes_percent: Some(
                        auditable_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                    total_valid_votes: Some(contest.contest_result.total_valid_votes as i64),
                    total_valid_votes_percent: Some(
                        total_valid_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                    total_invalid_votes: Some(contest.contest_result.total_invalid_votes as i64),
                    total_invalid_votes_percent: Some(
                        total_invalid_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                    explicit_invalid_votes: Some(
                        contest.contest_result.invalid_votes.explicit as i64,
                    ),
                    explicit_invalid_votes_percent: Some(
                        explicit_invalid_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                    implicit_invalid_votes: Some(
                        contest.contest_result.invalid_votes.implicit as i64,
                    ),
                    implicit_invalid_votes_percent: Some(
                        implicit_invalid_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                    blank_votes: Some(contest.contest_result.total_blank_votes as i64),
                    blank_votes_percent: Some(
                        total_blank_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                    created_at: None,
                    last_updated_at: None,
                    labels: None,
                    annotations: Some(annotations),
                    documents: None,
                });

                let votes_base: f64 = cmp::max(
                    contest.contest_result.total_votes
                        - contest.contest_result.total_invalid_votes
                        - contest.contest_result.total_blank_votes,
                    1,
                ) as f64;

                for candidate in &contest.candidate_result {
                    let cast_votes_percent: f64 = (candidate.total_count as f64) / votes_base;
                    results_area_contest_candidates.push(ResultsAreaContestCandidate {
                        id: Uuid::new_v4().into(),
                        tenant_id: tenant_id.into(),
                        election_event_id: election_event_id.into(),
                        election_id: election.election_id.clone(),
                        contest_id: contest.contest.id.clone(),
                        candidate_id: candidate.candidate.id.clone(),
                        results_event_id: results_event_id.into(),
                        area_id: area.id.clone(),
                        cast_votes: Some(candidate.total_count as i64),
                        cast_votes_percent: Some(cast_votes_percent.clamp(0.0, 1.0).try_into()?),
                        winning_position: candidate.winning_position.map(|val| val as i64),
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
                    contest_id: contest.contest.id.clone(),
                    results_event_id: results_event_id.into(),
                    elegible_census: Some(contest.contest_result.census as i64),
                    total_valid_votes: Some(contest.contest_result.total_valid_votes as i64),
                    explicit_invalid_votes: Some(
                        contest.contest_result.invalid_votes.explicit as i64,
                    ),
                    implicit_invalid_votes: Some(
                        contest.contest_result.invalid_votes.implicit as i64,
                    ),
                    blank_votes: Some(contest.contest_result.total_blank_votes as i64),
                    voting_type: contest.contest.voting_type.clone(),
                    counting_algorithm: contest.contest.counting_algorithm.clone(),
                    name: contest.contest.name.clone(),
                    created_at: None,
                    last_updated_at: None,
                    labels: None,
                    annotations: Some(annotations),
                    total_invalid_votes: Some(contest.contest_result.total_invalid_votes as i64),
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
                    total_votes: Some(contest.contest_result.total_votes as i64),
                    total_votes_percent: Some(total_votes_percent.clamp(0.0, 1.0).try_into()?),
                    documents: None,
                    total_auditable_votes: Some(contest.contest_result.auditable_votes as i64),
                    total_auditable_votes_percent: Some(
                        auditable_votes_percent.clamp(0.0, 1.0).try_into()?,
                    ),
                });

                let votes_base: f64 = cmp::max(
                    contest.contest_result.total_votes
                        - contest.contest_result.total_invalid_votes
                        - contest.contest_result.total_blank_votes,
                    1,
                ) as f64;

                for candidate in &contest.candidate_result {
                    let cast_votes_percent: f64 = (candidate.total_count as f64) / votes_base;
                    results_contest_candidates.push(ResultsContestCandidate {
                        id: Uuid::new_v4().into(),
                        tenant_id: tenant_id.into(),
                        election_event_id: election_event_id.into(),
                        election_id: election.election_id.clone(),
                        contest_id: contest.contest.id.clone(),
                        candidate_id: candidate.candidate.id.clone(),
                        results_event_id: results_event_id.into(),
                        cast_votes: Some(candidate.total_count as i64),
                        winning_position: candidate.winning_position.map(|val| val as i64),
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
        tenant_id.into(),
        election_event_id.into(),
        results_event_id.into(),
        results_contests.clone(),
    )
    .await?;

    insert_results_area_contests(
        hasura_transaction,
        tenant_id.into(),
        election_event_id.into(),
        results_event_id.into(),
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

#[instrument(skip_all)]
pub async fn generate_results_id_if_necessary(
    hasura_transaction: &Transaction<'_>,
    sqlite_transaction: &SqliteTransaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    session_ids_opt: Option<Vec<i64>>,
    previous_execution: TallySessionExecution,
    state_opt: &Option<State>,
) -> Result<Option<String>> {
    if state_opt.is_none() {
        return Ok(None);
    }
    let previous_session_ids = previous_execution.session_ids.unwrap_or(vec![]);
    let session_ids = session_ids_opt.unwrap_or(vec![]);

    if !(session_ids.len() > previous_session_ids.len()) {
        return Ok(None);
    }

    let results_event = find_results_event_sqlite(sqlite_transaction, tenant_id, election_event_id)
        .context("Failed to find results event table")?;

    insert_results_event(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &results_event.id,
    )
    .await?;
    Ok(Some(results_event.id))
}

#[instrument(skip_all)]
pub async fn process_results_tables(
    hasura_transaction: &Transaction<'_>,
    base_tally_path: &PathBuf,
    state_opt: Option<State>,
    tenant_id: &str,
    election_event_id: &str,
    session_ids: Option<Vec<i64>>,
    previous_execution: TallySessionExecution,
    areas: &Vec<Area>,
    default_language: &str,
    tally_type_enum: TallyType,
    sqlite_transaction: &SqliteTransaction<'_>,
) -> Result<Option<String>> {
    let results_event_id_opt = generate_results_id_if_necessary(
        hasura_transaction,
        sqlite_transaction,
        tenant_id,
        election_event_id,
        session_ids,
        previous_execution.clone(),
        &state_opt,
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
                sqlite_transaction,
            )
            .await?;
        }
        Ok(results_event_id_opt)
    } else {
        Ok(previous_execution.results_event_id)
    }
}

#[instrument(skip_all)]
pub async fn populate_results_tables(
    hasura_transaction: &Transaction<'_>,
    base_tally_path: &PathBuf,
    state_opt: Option<State>,
    tenant_id: &str,
    election_event_id: &str,
    session_ids: Option<Vec<i64>>,
    previous_execution: TallySessionExecution,
    areas: &Vec<Area>,
    default_language: &str,
    tally_type_enum: TallyType,
) -> Result<(Option<String>, Option<TallySessionDocuments>)> {
    let velvet_output_dir = base_tally_path.join("output");
    let base_database_path = velvet_output_dir.join("velvet-generate-database");
    let database_path = base_database_path.join("results.db");

    let document_id = Uuid::new_v4().to_string();

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
                    session_ids,
                    previous_execution,
                    areas,
                    default_language,
                    tally_type_enum,
                    &sqlite_transaction,
                )
                .await
            })?;
            sqlite_transaction.commit()?;
            Ok(process_result)
        })?;

    if let Some(ref results_event_id) = results_event_id_opt {
        let file_name = format!("results-{}.db", results_event_id);
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
            Some(document_id.to_string()),
            false,
        )
        .await?;

        let documents = TallySessionDocuments {
            sqlite: Some(document_id.to_string()),
        };

        Ok((results_event_id_opt, Some(documents)))
    } else {
        Ok((results_event_id_opt, None))
    }
}
