// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::{
    postgres::{
        results_area_contest::get_event_results_area_contest,
        results_area_contest_candidate::get_event_results_area_contest_candidates,
        results_contest::get_event_results_contest,
        results_contest_candidate::get_event_results_contest_candidates,
        results_election::get_event_results_election,
        results_election_area::get_event_results_election_area,
        results_event::get_results_event_by_event_id,
        tally_session::get_tally_sessions_by_election_event_id,
        tally_session_contest::get_event_tally_session_contest,
        tally_session_execution::get_event_tally_session_executions,
    },
    types::documents::ETallyDocuments,
};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;

use futures::future::join_all;
use sequent_core::{
    types::{
        hasura::core::{TallySession, TallySessionContest, TallySessionExecution},
        results::{
            ResultsAreaContest, ResultsAreaContestCandidate, ResultsContest,
            ResultsContestCandidate, ResultsElection, ResultsElectionArea, ResultsEvent,
        },
    },
    util::temp_path::generate_temp_file,
};
use std::{future::Future, pin::Pin};
use tempfile::TempPath;
use tracing::{info, instrument};

type ExportResult = Result<(String, TempPath), anyhow::Error>;

#[instrument(err, skip(hasura_transaction))]
pub async fn export_tally_session(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<(String, TempPath)> {
    let event_tally_sessions: Vec<TallySession> = get_tally_sessions_by_election_event_id(
        hasura_transaction,
        tenant_id,
        election_event_id,
        false,
    )
    .await
    .map_err(|e| anyhow!("Error in get_tally_sessions_by_election_event_id: {e:?}"))?;

    let file_name = ETallyDocuments::TALLY_SESSION.to_file_name().to_string();

    let mut writer = csv::WriterBuilder::new().delimiter(b',').from_writer(
        generate_temp_file(&file_name, ".csv").with_context(|| "Error creating temporary file")?,
    );

    writer.write_record(&[
        "id".to_string(),
        "tenant_id".to_string(),
        "election_event_id".to_string(),
        "created_at".to_string(),
        "last_updated_at".to_string(),
        "labels".to_string(),
        "annotations".to_string(),
        "election_ids".to_string(),
        "area_ids".to_string(),
        "is_execution_completed".to_string(),
        "keys_ceremony_id".to_string(),
        "execution_status".to_string(),
        "threshold".to_string(),
        "configuration".to_string(),
        "tally_type".to_string(),
        "permission_label".to_string(),
    ])?;

    for tally_session in event_tally_sessions {
        let values: Vec<String> = serde_json::to_value(tally_session)?
            .as_object()
            .ok_or_else(|| anyhow!("Failed to convert tally_session to JSON object"))?
            .values()
            .map(|value| value.to_string())
            .collect();

        writer.write_record(&values);
    }

    writer
        .flush()
        .with_context(|| "Error flushing CSV writer")?;

    let temp_path = writer
        .into_inner()
        .with_context(|| "Error getting inner writer")?
        .into_temp_path();

    Ok((file_name, temp_path))
}

#[instrument(err, skip(hasura_transaction))]
pub async fn export_tally_session_execution(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<(String, TempPath)> {
    let event_tally_sessions_executions: Vec<TallySessionExecution> =
        get_event_tally_session_executions(hasura_transaction, tenant_id, election_event_id)
            .await
            .map_err(|e| anyhow!("Error in get_event_tally_session_executions: {e:?}"))?;

    let file_name = ETallyDocuments::TALLY_SESSION_EXECUTION
        .to_file_name()
        .to_string();

    let mut writer = csv::WriterBuilder::new().delimiter(b',').from_writer(
        generate_temp_file(&file_name, ".csv").with_context(|| "Error creating temporary file")?,
    );

    writer.write_record(&[
        "id".to_string(),
        "tenant_id".to_string(),
        "election_event_id".to_string(),
        "created_at".to_string(),
        "last_updated_at".to_string(),
        "labels".to_string(),
        "annotations".to_string(),
        "current_message_id".to_string(),
        "tally_session_id".to_string(),
        "session_ids".to_string(),
        "status".to_string(),
        "results_event_id".to_string(),
    ])?;

    for tally_session_execution in event_tally_sessions_executions {
        let values: Vec<String> = serde_json::to_value(tally_session_execution)?
            .as_object()
            .ok_or_else(|| anyhow!("Failed to convert tally_session_execution to JSON object"))?
            .values()
            .map(|value| value.to_string())
            .collect();

        writer.write_record(&values);
    }

    writer
        .flush()
        .with_context(|| "Error flushing CSV writer")?;

    let temp_path = writer
        .into_inner()
        .with_context(|| "Error getting inner writer")?
        .into_temp_path();

    Ok((file_name, temp_path))
}

#[instrument(err, skip(hasura_transaction))]
pub async fn export_tally_session_contest(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<(String, TempPath)> {
    let event_tally_sessions_contests: Vec<TallySessionContest> =
        get_event_tally_session_contest(hasura_transaction, tenant_id, election_event_id)
            .await
            .map_err(|e| anyhow!("Error in get_event_tally_session_contest: {e:?}"))?;

    let file_name = ETallyDocuments::TALLY_SESSION_CONTEST
        .to_file_name()
        .to_string();

    let mut writer = csv::WriterBuilder::new().delimiter(b',').from_writer(
        generate_temp_file(&file_name, ".csv").with_context(|| "Error creating temporary file")?,
    );

    writer.write_record(&[
        "id".to_string(),
        "tenant_id".to_string(),
        "election_event_id".to_string(),
        "area_id".to_string(),
        "contest_id".to_string(),
        "session_id".to_string(),
        "created_at".to_string(),
        "last_updated_at".to_string(),
        "labels".to_string(),
        "annotations".to_string(),
        "tally_session_id".to_string(),
        "election_id".to_string(),
    ])?;

    for tally_session_contest in event_tally_sessions_contests {
        let values: Vec<String> = serde_json::to_value(tally_session_contest)?
            .as_object()
            .ok_or_else(|| anyhow!("Failed to convert tally_session_contest to JSON object"))?
            .values()
            .map(|value| value.to_string())
            .collect();

        writer.write_record(&values);
    }

    writer
        .flush()
        .with_context(|| "Error flushing CSV writer")?;

    let temp_path = writer
        .into_inner()
        .with_context(|| "Error getting inner writer")?
        .into_temp_path();

    Ok((file_name, temp_path))
}

#[instrument(err, skip(hasura_transaction))]
pub async fn export_results_event(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<(String, TempPath)> {
    let results_events: Vec<ResultsEvent> =
        get_results_event_by_event_id(hasura_transaction, tenant_id, election_event_id)
            .await
            .map_err(|e| anyhow!("Error in get_results_event_by_event_id: {e:?}"))?;

    let file_name = ETallyDocuments::RESULTS_EVENT.to_file_name().to_string();

    let mut writer = csv::WriterBuilder::new().delimiter(b',').from_writer(
        generate_temp_file(&file_name, ".csv").with_context(|| "Error creating temporary file")?,
    );

    writer.write_record(&[
        "id".to_string(),
        "tenant_id".to_string(),
        "election_event_id".to_string(),
        "name".to_string(),
        "created_at".to_string(),
        "last_updated_at".to_string(),
        "annotations".to_string(),
        "labels".to_string(),
        "documents".to_string(),
    ])?;

    for results_event in results_events {
        let values: Vec<String> = serde_json::to_value(results_event)?
            .as_object()
            .ok_or_else(|| anyhow!("Failed to convert results_event to JSON object"))?
            .values()
            .map(|value| value.to_string())
            .collect();

        writer.write_record(&values);
    }

    writer
        .flush()
        .with_context(|| "Error flushing CSV writer")?;

    let temp_path = writer
        .into_inner()
        .with_context(|| "Error getting inner writer")?
        .into_temp_path();

    Ok((file_name, temp_path))
}

#[instrument(err, skip(hasura_transaction))]
pub async fn export_results_election_area(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<(String, TempPath)> {
    let results_election_areas: Vec<ResultsElectionArea> =
        get_event_results_election_area(hasura_transaction, tenant_id, election_event_id)
            .await
            .map_err(|e| anyhow!("Error in get_event_results_election_area: {e:?}"))?;

    let file_name = ETallyDocuments::RESULTS_ELECTION_AREA
        .to_file_name()
        .to_string();

    let mut writer = csv::WriterBuilder::new().delimiter(b',').from_writer(
        generate_temp_file(&file_name, ".csv").with_context(|| "Error creating temporary file")?,
    );

    writer.write_record(&[
        "id".to_string(),
        "tenant_id".to_string(),
        "election_event_id".to_string(),
        "election_id".to_string(),
        "area_id".to_string(),
        "results_event_id".to_string(),
        "created_at".to_string(),
        "last_updated_at".to_string(),
        "documents".to_string(),
        "name".to_string(),
    ])?;

    for results_election_area in results_election_areas {
        let values: Vec<String> = serde_json::to_value(results_election_area)?
            .as_object()
            .ok_or_else(|| anyhow!("Failed to convert results_election_area to JSON object"))?
            .values()
            .map(|value| value.to_string())
            .collect();

        writer.write_record(&values);
    }

    writer
        .flush()
        .with_context(|| "Error flushing CSV writer")?;

    let temp_path = writer
        .into_inner()
        .with_context(|| "Error getting inner writer")?
        .into_temp_path();

    Ok((file_name, temp_path))
}

#[instrument(err, skip(hasura_transaction))]
pub async fn export_results_election(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<(String, TempPath)> {
    let results_elections: Vec<ResultsElection> =
        get_event_results_election(hasura_transaction, tenant_id, election_event_id)
            .await
            .map_err(|e| anyhow!("Error in get_event_results_election: {e:?}"))?;

    let file_name = ETallyDocuments::RESULTS_ELECTION.to_file_name().to_string();

    let mut writer = csv::WriterBuilder::new().delimiter(b',').from_writer(
        generate_temp_file(&file_name, ".csv").with_context(|| "Error creating temporary file")?,
    );

    writer.write_record(&[
        "id".to_string(),
        "tenant_id".to_string(),
        "election_event_id".to_string(),
        "election_id".to_string(),
        "results_event_id".to_string(),
        "name".to_string(),
        "elegible_census".to_string(),
        "total_voters".to_string(),
        "created_at".to_string(),
        "last_updated_at".to_string(),
        "labels".to_string(),
        "annotations".to_string(),
        "total_voters_percent".to_string(),
        "documents".to_string(),
    ])?;

    for results_election in results_elections {
        let values: Vec<String> = serde_json::to_value(results_election)?
            .as_object()
            .ok_or_else(|| anyhow!("Failed to convert results_election to JSON object"))?
            .values()
            .map(|value| value.to_string())
            .collect();

        writer.write_record(&values);
    }

    writer
        .flush()
        .with_context(|| "Error flushing CSV writer")?;

    let temp_path = writer
        .into_inner()
        .with_context(|| "Error getting inner writer")?
        .into_temp_path();

    Ok((file_name, temp_path))
}

#[instrument(err, skip(hasura_transaction))]
pub async fn export_results_contest(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<(String, TempPath)> {
    let results_contests: Vec<ResultsContest> =
        get_event_results_contest(hasura_transaction, tenant_id, election_event_id)
            .await
            .map_err(|e| anyhow!("Error in get_event_results_contest: {e:?}"))?;

    let file_name = ETallyDocuments::RESULTS_CONTEST.to_file_name().to_string();

    let mut writer = csv::WriterBuilder::new().delimiter(b',').from_writer(
        generate_temp_file(&file_name, ".csv").with_context(|| "Error creating temporary file")?,
    );

    writer.write_record(&[
        "id".to_string(),
        "tenant_id".to_string(),
        "election_event_id".to_string(),
        "election_id".to_string(),
        "contest_id".to_string(),
        "results_event_id".to_string(),
        "elegible_census".to_string(),
        "total_valid_votes".to_string(),
        "explicit_invalid_votes".to_string(),
        "implicit_invalid_votes".to_string(),
        "blank_votes".to_string(),
        "voting_type".to_string(),
        "counting_algorithm".to_string(),
        "name".to_string(),
        "created_at".to_string(),
        "last_updated_at".to_string(),
        "labels".to_string(),
        "annotations".to_string(),
        "total_invalid_votes".to_string(),
        "total_invalid_votes_percent".to_string(),
        "total_valid_votes_percent".to_string(),
        "explicit_invalid_votes_percent".to_string(),
        "implicit_invalid_votes_percent".to_string(),
        "blank_votes_percent".to_string(),
        "total_votes".to_string(),
        "total_votes_percent".to_string(),
        "documents".to_string(),
        "total_auditable_votes".to_string(),
        "total_auditable_votes_percent".to_string(),
    ])?;

    for results_contest in results_contests {
        let values: Vec<String> = serde_json::to_value(results_contest)?
            .as_object()
            .ok_or_else(|| anyhow!("Failed to convert results_contest to JSON object"))?
            .values()
            .map(|value| value.to_string())
            .collect();

        writer.write_record(&values);
    }

    writer
        .flush()
        .with_context(|| "Error flushing CSV writer")?;

    let temp_path = writer
        .into_inner()
        .with_context(|| "Error getting inner writer")?
        .into_temp_path();

    Ok((file_name, temp_path))
}

#[instrument(err, skip(hasura_transaction))]
pub async fn export_results_contest_candidate(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<(String, TempPath)> {
    let results_contests_candidates: Vec<ResultsContestCandidate> =
        get_event_results_contest_candidates(hasura_transaction, tenant_id, election_event_id)
            .await
            .map_err(|e| anyhow!("Error in get_event_results_contest_candidates: {e:?}"))?;

    let file_name = ETallyDocuments::RESULTS_CONTEST_CANDIDATE
        .to_file_name()
        .to_string();

    let mut writer = csv::WriterBuilder::new().delimiter(b',').from_writer(
        generate_temp_file(&file_name, ".csv").with_context(|| "Error creating temporary file")?,
    );

    writer.write_record(&[
        "id".to_string(),
        "tenant_id".to_string(),
        "election_event_id".to_string(),
        "election_id".to_string(),
        "contest_id".to_string(),
        "candidate_id".to_string(),
        "results_event_id".to_string(),
        "cast_votes".to_string(),
        "winning_position".to_string(),
        "points".to_string(),
        "created_at".to_string(),
        "last_updated_at".to_string(),
        "labels".to_string(),
        "annotations".to_string(),
        "cast_votes_percent".to_string(),
        "documents".to_string(),
    ])?;

    for results_contests_candidate in results_contests_candidates {
        let values: Vec<String> = serde_json::to_value(results_contests_candidate)?
            .as_object()
            .ok_or_else(|| anyhow!("Failed to convert results_contests_candidate to JSON object"))?
            .values()
            .map(|value| value.to_string())
            .collect();

        writer.write_record(&values);
    }

    writer
        .flush()
        .with_context(|| "Error flushing CSV writer")?;

    let temp_path = writer
        .into_inner()
        .with_context(|| "Error getting inner writer")?
        .into_temp_path();

    Ok((file_name, temp_path))
}

#[instrument(err, skip(hasura_transaction))]
pub async fn export_results_area_contest(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<(String, TempPath)> {
    let results_area_contests: Vec<ResultsAreaContest> =
        get_event_results_area_contest(hasura_transaction, tenant_id, election_event_id)
            .await
            .map_err(|e| anyhow!("Error in get_event_results_area_contest: {e:?}"))?;

    let file_name = ETallyDocuments::RESULTS_AREA_CONTEST
        .to_file_name()
        .to_string();

    let mut writer = csv::WriterBuilder::new().delimiter(b',').from_writer(
        generate_temp_file(&file_name, ".csv").with_context(|| "Error creating temporary file")?,
    );

    writer.write_record(&[
        "id".to_string(),
        "tenant_id".to_string(),
        "election_event_id".to_string(),
        "election_id".to_string(),
        "contest_id".to_string(),
        "area_id".to_string(),
        "results_event_id".to_string(),
        "elegible_census".to_string(),
        "total_valid_votes".to_string(),
        "explicit_invalid_votes".to_string(),
        "implicit_invalid_votes".to_string(),
        "blank_votes".to_string(),
        "created_at".to_string(),
        "last_updated_at".to_string(),
        "labels".to_string(),
        "annotations".to_string(),
        "total_valid_votes_percent".to_string(),
        "total_invalid_votes".to_string(),
        "total_invalid_votes_percent".to_string(),
        "explicit_invalid_votes_percent".to_string(),
        "blank_votes_percent".to_string(),
        "implicit_invalid_votes_percent".to_string(),
        "total_votes".to_string(),
        "total_votes_percent".to_string(),
        "documents".to_string(),
        "total_auditable_votes".to_string(),
        "total_auditable_votes_percent".to_string(),
    ])?;

    for results_area_contest in results_area_contests {
        let values: Vec<String> = serde_json::to_value(results_area_contest)?
            .as_object()
            .ok_or_else(|| anyhow!("Failed to convert results_area_contest to JSON object"))?
            .values()
            .map(|value| value.to_string())
            .collect();

        writer.write_record(&values);
    }

    writer
        .flush()
        .with_context(|| "Error flushing CSV writer")?;

    let temp_path = writer
        .into_inner()
        .with_context(|| "Error getting inner writer")?
        .into_temp_path();

    Ok((file_name, temp_path))
}

#[instrument(err, skip(hasura_transaction))]
pub async fn export_results_area_contest_candidate(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<(String, TempPath)> {
    let results_area_contests_candidates: Vec<ResultsAreaContestCandidate> =
        get_event_results_area_contest_candidates(hasura_transaction, tenant_id, election_event_id)
            .await
            .map_err(|e| anyhow!("Error in get_event_results_area_contest_candidates: {e:?}"))?;

    let file_name = ETallyDocuments::RESULTS_AREA_CONTEST_CANDIDATE
        .to_file_name()
        .to_string();

    let mut writer = csv::WriterBuilder::new().delimiter(b',').from_writer(
        generate_temp_file(&file_name, ".csv").with_context(|| "Error creating temporary file")?,
    );

    writer.write_record(&[
        "id".to_string(),
        "tenant_id".to_string(),
        "election_event_id".to_string(),
        "election_id".to_string(),
        "contest_id".to_string(),
        "area_id".to_string(),
        "candidate_id".to_string(),
        "results_event_id".to_string(),
        "cast_votes".to_string(),
        "winning_position".to_string(),
        "points".to_string(),
        "created_at".to_string(),
        "last_updated_at".to_string(),
        "labels".to_string(),
        "annotations".to_string(),
        "cast_votes_percent".to_string(),
        "documents".to_string(),
    ])?;

    for results_area_contests_candidate in results_area_contests_candidates {
        let values: Vec<String> = serde_json::to_value(results_area_contests_candidate)?
            .as_object()
            .ok_or_else(|| {
                anyhow!("Failed to convert results_area_contests_candidate to JSON object")
            })?
            .values()
            .map(|value| value.to_string())
            .collect();

        writer.write_record(&values);
    }

    writer
        .flush()
        .with_context(|| "Error flushing CSV writer")?;

    let temp_path = writer
        .into_inner()
        .with_context(|| "Error getting inner writer")?
        .into_temp_path();

    Ok((file_name, temp_path))
}

fn get_export_tasks<'a>(
    hasura_transaction: &'a Transaction<'a>,
    tenant_id: &'a str,
    election_event_id: &'a str,
) -> Vec<Pin<Box<dyn Future<Output = ExportResult> + Send + 'a>>> {
    vec![
        Box::pin(export_tally_session(
            hasura_transaction,
            tenant_id,
            election_event_id,
        )),
        Box::pin(export_tally_session_execution(
            hasura_transaction,
            tenant_id,
            election_event_id,
        )),
        Box::pin(export_tally_session_contest(
            hasura_transaction,
            tenant_id,
            election_event_id,
        )),
        Box::pin(export_results_event(
            hasura_transaction,
            tenant_id,
            election_event_id,
        )),
        Box::pin(export_results_election(
            hasura_transaction,
            tenant_id,
            election_event_id,
        )),
        Box::pin(export_results_election_area(
            hasura_transaction,
            tenant_id,
            election_event_id,
        )),
        Box::pin(export_results_contest(
            hasura_transaction,
            tenant_id,
            election_event_id,
        )),
        Box::pin(export_results_area_contest(
            hasura_transaction,
            tenant_id,
            election_event_id,
        )),
        Box::pin(export_results_contest_candidate(
            hasura_transaction,
            tenant_id,
            election_event_id,
        )),
        Box::pin(export_results_area_contest_candidate(
            hasura_transaction,
            tenant_id,
            election_event_id,
        )),
    ]
}

#[instrument(err)]
pub async fn read_tally_data(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<(String, TempPath)>> {
    let tasks = get_export_tasks(&hasura_transaction, tenant_id, election_event_id);

    let results = join_all(tasks).await;

    for result in &results {
        if let Err(e) = result {
            return Err(anyhow::anyhow!("Export tally failed: {:?}", e));
        }
    }

    let exports = results.into_iter().map(|res| res.expect("")).collect();

    Ok(exports)
}
