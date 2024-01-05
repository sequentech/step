// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura::results_area_contest::insert_results_area_contest;
use crate::hasura::results_area_contest_candidate::insert_results_area_contest_candidate;
use crate::hasura::results_contest::insert_results_contest;
use crate::hasura::results_contest_candidate::insert_results_contest_candidate;
use crate::hasura::results_election::insert_results_election;
use crate::hasura::results_event::insert_results_event;
use crate::hasura::tally_session_execution::get_last_tally_session_execution::GetLastTallySessionExecutionSequentBackendTallySessionExecution;
use anyhow::{anyhow, Context, Result};
use sequent_core::services::connection;
use sequent_core::services::keycloak;
use sequent_core::types::ceremonies::*;
use std::cmp;
use std::path::PathBuf;
use tracing::{event, instrument, Level};
use velvet::cli::state::State;
use velvet::pipes::generate_reports::ElectionReportDataComputed;

#[instrument(skip_all)]
pub async fn save_results(
    auth_headers: connection::AuthHeaders,
    results: Vec<ElectionReportDataComputed>,
    tenant_id: &str,
    election_event_id: &str,
    results_event_id: &str,
) -> Result<()> {
    for election in &results {
        let total_valid_votes_percent: f64 =
            (election.census as f64) / (cmp::max(election.total_votes, 1) as f64);
        insert_results_election(
            &auth_headers,
            tenant_id,
            election_event_id,
            results_event_id,
            &election.election_id,
            &None,                                            // name
            &Some(election.census as i64),                    // census
            &Some(election.total_votes as i64),               // total_valid_votes,
            &Some(total_valid_votes_percent.clamp(0.0, 1.0)), // total_valid_votes_percent,
        )
        .await?;

        for contest in &election.reports {
            if let Some(area_id) = &contest.area_id {
                let total_valid_votes_percent: f64 = (contest.contest_result.total_votes as f64)
                    / (cmp::max(contest.contest_result.census, 1) as f64);
                let total_votes = cmp::max(contest.contest_result.total_votes, 1) as f64;
                let total_invalid_votes_percent: f64 =
                    (contest.contest_result.total_invalid_votes as f64) / total_votes;
                let explicit_invalid_votes_percent: f64 =
                    (contest.contest_result.invalid_votes.explicit as f64) / total_votes;
                let implicit_invalid_votes_percent: f64 =
                    (contest.contest_result.invalid_votes.implicit as f64) / total_votes;
                let total_blank_votes_percent: f64 =
                    (contest.contest_result.total_blank_votes as f64) / total_votes;
                insert_results_area_contest(
                    &auth_headers,
                    tenant_id,
                    election_event_id,
                    &election.election_id,
                    &contest.contest.id,
                    area_id,
                    results_event_id,
                    Some(contest.contest_result.census as i64),
                    Some(contest.contest_result.total_votes as i64),
                    Some(total_valid_votes_percent.clamp(0.0, 1.0)),
                    Some(contest.contest_result.total_invalid_votes as i64),
                    Some(total_invalid_votes_percent.clamp(0.0, 1.0)),
                    Some(contest.contest_result.invalid_votes.explicit as i64),
                    Some(explicit_invalid_votes_percent.clamp(0.0, 1.0)),
                    Some(contest.contest_result.invalid_votes.implicit as i64),
                    Some(implicit_invalid_votes_percent.clamp(0.0, 1.0)),
                    Some(contest.contest_result.total_blank_votes as i64),
                    Some(total_blank_votes_percent.clamp(0.0, 1.0)),
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
                        Some(cast_votes_percent.clamp(0.0, 1.0)),
                        candidate.winning_position.map(|val| val as i64),
                        None, // points
                    )
                    .await?;
                }
            } else {
                let census = cmp::max(contest.contest_result.census, 1) as f64;
                let total_votes_percent: f64 = (contest.contest_result.total_votes as f64) / census;
                let total_invalid_votes_percent: f64 = (contest.contest_result.total_invalid_votes as f64) / census;
                let explicit_invalid_votes_percent: f64 =
                    (contest.contest_result.invalid_votes.explicit as f64) / census;
                let implicit_invalid_votes_percent: f64 =
                    (contest.contest_result.invalid_votes.implicit as f64) / census;
                let blank_votes_percent: f64 =
                    (contest.contest_result.total_blank_votes as f64) / census;

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
                    Some(contest.contest_result.total_invalid_votes as i64),
                    Some(total_invalid_votes_percent.clamp(0.0, 1.0)),
                    Some(contest.contest_result.invalid_votes.explicit as i64),
                    Some(explicit_invalid_votes_percent.clamp(0.0, 1.0)),
                    Some(contest.contest_result.invalid_votes.implicit as i64),
                    Some(implicit_invalid_votes_percent.clamp(0.0, 1.0)),
                    Some(contest.contest_result.total_blank_votes as i64),
                    Some(blank_votes_percent.clamp(0.0, 1.0)),
                    contest.contest.voting_type.clone(),
                    contest.contest.counting_algorithm.clone(),
                    contest.contest.name.clone(),
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
) -> Result<Option<String>> {
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
    base_tally_path: PathBuf,
    state: State,
    tenant_id: &str,
    election_event_id: &str,
    session_ids: Option<Vec<i64>>,
    previous_execution: GetLastTallySessionExecutionSequentBackendTallySessionExecution,
) -> Result<Option<String>> {
    // get credentials
    // map_plaintext_data also calls this but at this point the credentials
    // could be expired
    let auth_headers = keycloak::get_client_credentials().await?;

    let results_event_id_opt = generate_results_id_if_necessary(
        &auth_headers,
        tenant_id,
        election_event_id,
        session_ids,
        previous_execution.clone(),
    )
    .await?;

    if let Some(results_event_id) = results_event_id_opt.clone() {
        if let Ok(results) = state.get_results() {
            save_results(
                auth_headers.clone(),
                results,
                tenant_id,
                election_event_id,
                &results_event_id,
            )
            .await?;
        }
        Ok(results_event_id_opt)
    } else {
        Ok(previous_execution.results_event_id)
    }
}
