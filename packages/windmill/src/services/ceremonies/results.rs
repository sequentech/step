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
use crate::services::ceremonies::tally_ceremony::get_tally_ceremony_status;
use crate::services::ceremonies::velvet_tally::AreaContestDataType;
use anyhow::{anyhow, Context, Result};
use sequent_core::services::connection;
use sequent_core::services::keycloak;
use sequent_core::types::ceremonies::*;
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
    let auth_headers = keycloak::get_client_credentials().await?;
    for election in &results {
        insert_results_election(
            &auth_headers,
            tenant_id,
            election_event_id,
            results_event_id,
            &election.election_id,
            &None, // name
            &None, // census
            &None, // total_valid_votes,
            &None, // total_valid_votes_percent,
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
                    Some(contest.contest_result.census as i64),
                    Some(contest.contest_result.total_votes as i64),
                    None, // totalValidVotesPercent
                    None, // totalInvalidVotes
                    None, // totalInvalidVotesPercent
                    Some(contest.contest_result.invalid_votes.explicit as i64),
                    None, // explicitInvalidVotesPercent
                    Some(contest.contest_result.invalid_votes.implicit as i64),
                    None, // implicitInvalidVotesPercent
                    Some(contest.contest_result.total_blank_votes as i64),
                    None, // blankVotesPercent
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
                        None, // cast_votes_percent
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
                    Some(contest.contest_result.census as i64),
                    Some(contest.contest_result.total_votes as i64),
                    // missing total valid votes
                    Some(contest.contest_result.invalid_votes.explicit as i64),
                    Some(contest.contest_result.invalid_votes.implicit as i64),
                    Some(contest.contest_result.total_blank_votes as i64),
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

pub async fn populate_results_tables(
    base_tally_path: PathBuf,
    state: State,
    tenant_id: &str,
    election_event_id: &str,
    session_ids: Option<Vec<i64>>,
    previous_execution: GetLastTallySessionExecutionSequentBackendTallySessionExecution,
) -> Result<Option<String>> {
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
            save_results(results, tenant_id, election_event_id, &results_event_id).await?;
        }
        Ok(results_event_id_opt)
    } else {
        Ok(previous_execution.results_event_id)
    }
}
