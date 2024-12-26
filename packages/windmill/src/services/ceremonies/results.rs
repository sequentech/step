// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::result_documents::save_result_documents;
use crate::hasura::results_area_contest::insert_results_area_contest;
use crate::hasura::results_area_contest_candidate::insert_results_area_contest_candidate;
use crate::hasura::results_contest::insert_results_contest;
use crate::hasura::results_contest_candidate::insert_results_contest_candidate;
use crate::hasura::results_election::insert_results_election;
use crate::hasura::results_event::insert_results_event;
use crate::hasura::tally_session_execution::get_last_tally_session_execution::GetLastTallySessionExecutionSequentBackendTallySessionExecution;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::services::connection;
use sequent_core::services::keycloak;
use sequent_core::types::ceremonies::TallyType;
use sequent_core::types::hasura::core::Area;
use serde_json::json;
use std::cmp;
use std::path::PathBuf;
use tracing::{event, instrument, Level};
use velvet::cli::state::State;
use velvet::pipes::generate_reports::ElectionReportDataComputed;

#[instrument(skip_all)]
pub async fn save_results(
    results: Vec<ElectionReportDataComputed>,
    tenant_id: &str,
    election_event_id: &str,
    results_event_id: &str,
) -> Result<()> {
    let mut idx: usize = 0;
    let mut auth_headers = keycloak::get_client_credentials().await?;
    for election in &results {
        let total_voters_percent: f64 =
            (election.total_votes as f64) / (cmp::max(election.census, 1) as f64);
        idx += 1;
        if idx % 200 == 0 {
            auth_headers = keycloak::get_client_credentials().await?;
        }
        insert_results_election(
            &auth_headers,
            tenant_id,
            election_event_id,
            results_event_id,
            &election.election_id,
            &None,                                       // name
            &Some(election.census as i64),               // census
            &Some(election.total_votes as i64),          // total_voters,
            &Some(total_voters_percent.clamp(0.0, 1.0)), // total_votes_percent,
        )
        .await?;

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
                idx += 1;
                if idx % 200 == 0 {
                    auth_headers = keycloak::get_client_credentials().await?;
                }
                insert_results_area_contest(
                    &auth_headers,
                    tenant_id,
                    election_event_id,
                    &election.election_id,
                    &contest.contest.id,
                    &area.id,
                    results_event_id,
                    Some(contest.contest_result.census as i64),
                    Some(contest.contest_result.total_votes as i64),
                    Some(total_votes_percent.clamp(0.0, 1.0)),
                    Some(contest.contest_result.auditable_votes as i64),
                    Some(auditable_votes_percent.clamp(0.0, 1.0)),
                    Some(contest.contest_result.total_valid_votes as i64),
                    Some(total_valid_votes_percent.clamp(0.0, 1.0)),
                    Some(contest.contest_result.total_invalid_votes as i64),
                    Some(total_invalid_votes_percent.clamp(0.0, 1.0)),
                    Some(contest.contest_result.invalid_votes.explicit as i64),
                    Some(explicit_invalid_votes_percent.clamp(0.0, 1.0)),
                    Some(contest.contest_result.invalid_votes.implicit as i64),
                    Some(implicit_invalid_votes_percent.clamp(0.0, 1.0)),
                    Some(contest.contest_result.total_blank_votes as i64),
                    Some(total_blank_votes_percent.clamp(0.0, 1.0)),
                    Some(annotations),
                )
                .await?;

                let votes_base: f64 = cmp::max(
                    contest.contest_result.total_votes
                        - contest.contest_result.total_invalid_votes
                        - contest.contest_result.total_blank_votes,
                    1,
                ) as f64;

                for candidate in &contest.candidate_result {
                    let cast_votes_percent: f64 = (candidate.total_count as f64) / votes_base;
                    idx += 1;
                    if idx % 200 == 0 {
                        auth_headers = keycloak::get_client_credentials().await?;
                    }
                    insert_results_area_contest_candidate(
                        &auth_headers,
                        tenant_id,
                        election_event_id,
                        &election.election_id,
                        &contest.contest.id,
                        &area.id,
                        &candidate.candidate.id,
                        results_event_id,
                        Some(candidate.total_count as i64),
                        Some(cast_votes_percent.clamp(0.0, 1.0)),
                        candidate.winning_position.map(|val| val as i64),
                        None, // points
                    )
                    .await?;
                }
            } else {
                idx += 1;
                if idx % 200 == 0 {
                    auth_headers = keycloak::get_client_credentials().await?;
                }
                insert_results_contest(
                    &auth_headers,
                    tenant_id,
                    election_event_id,
                    &election.election_id,
                    &contest.contest.id,
                    results_event_id,
                    Some(contest.contest_result.census as i64),
                    Some(contest.contest_result.total_votes as i64),
                    Some(total_votes_percent.clamp(0.0, 1.0)),
                    Some(contest.contest_result.auditable_votes as i64),
                    Some(auditable_votes_percent.clamp(0.0, 1.0)),
                    Some(contest.contest_result.total_valid_votes as i64),
                    Some(total_valid_votes_percent.clamp(0.0, 1.0)),
                    Some(contest.contest_result.total_invalid_votes as i64),
                    Some(total_invalid_votes_percent.clamp(0.0, 1.0)),
                    Some(contest.contest_result.invalid_votes.explicit as i64),
                    Some(explicit_invalid_votes_percent.clamp(0.0, 1.0)),
                    Some(contest.contest_result.invalid_votes.implicit as i64),
                    Some(implicit_invalid_votes_percent.clamp(0.0, 1.0)),
                    Some(contest.contest_result.total_blank_votes as i64),
                    Some(total_blank_votes_percent.clamp(0.0, 1.0)),
                    contest.contest.voting_type.clone(),
                    contest.contest.counting_algorithm.clone(),
                    contest.contest.name.clone(),
                    Some(annotations),
                )
                .await?;

                let votes_base: f64 = cmp::max(
                    contest.contest_result.total_votes
                        - contest.contest_result.total_invalid_votes
                        - contest.contest_result.total_blank_votes,
                    1,
                ) as f64;

                for candidate in &contest.candidate_result {
                    let cast_votes_percent: f64 = (candidate.total_count as f64) / votes_base;
                    idx += 1;
                    if idx % 200 == 0 {
                        auth_headers = keycloak::get_client_credentials().await?;
                    }
                    insert_results_contest_candidate(
                        &auth_headers,
                        tenant_id,
                        election_event_id,
                        &election.election_id,
                        &contest.contest.id,
                        &candidate.candidate.id,
                        results_event_id,
                        Some(candidate.total_count as i64),
                        Some(cast_votes_percent.clamp(0.0, 1.0)),
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

#[instrument(skip_all)]
pub async fn generate_results_id_if_necessary(
    auth_headers: &connection::AuthHeaders,
    tenant_id: &str,
    election_event_id: &str,
    session_ids_opt: Option<Vec<i64>>,
    previous_execution: GetLastTallySessionExecutionSequentBackendTallySessionExecution,
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
    let results_event_id =
        create_results_event(&auth_headers, &tenant_id, &election_event_id).await?;
    Ok(Some(results_event_id))
}

#[instrument(skip_all)]
pub async fn populate_results_tables(
    hasura_transaction: &Transaction<'_>,
    base_tally_path: &PathBuf,
    state_opt: Option<State>,
    tenant_id: &str,
    election_event_id: &str,
    session_ids: Option<Vec<i64>>,
    previous_execution: GetLastTallySessionExecutionSequentBackendTallySessionExecution,
    areas: &Vec<Area>,
    default_language: &str,
    tally_type_enum: Option<TallyType>,
) -> Result<Option<String>> {
    let mut auth_headers = keycloak::get_client_credentials().await?;
    let results_event_id_opt = generate_results_id_if_necessary(
        &auth_headers,
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
            )
            .await?;
        }
        Ok(results_event_id_opt)
    } else {
        Ok(previous_execution.results_event_id)
    }
}
