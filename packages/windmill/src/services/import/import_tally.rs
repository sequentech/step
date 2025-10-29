// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::{
    postgres::{
        results_area_contest::insert_many_results_area_contests,
        results_area_contest_candidate::insert_many_results_area_contest_candidates,
        results_contest::insert_many_results_contests,
        results_contest_candidate::insert_many_results_contest_candidates,
        results_election::insert_many_results_elections,
        results_election_area::insert_many_results_elections_areas,
        results_event::insert_many_results_events, tally_session::insert_many_tally_sessions,
        tally_session_contest::insert_many_tally_session_contests,
        tally_session_execution::insert_many_tally_session_executions,
    },
    types::documents::ETallyDocuments,
};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Local};
use csv::StringRecord;
use deadpool_postgres::Transaction;
use ordered_float::NotNan;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::{
    services::date::ISO8601,
    types::{
        hasura::core::{TallySession, TallySessionContest, TallySessionExecution},
        results::{
            ResultDocuments, ResultsAreaContest, ResultsAreaContestCandidate, ResultsContest,
            ResultsContestCandidate, ResultsElection, ResultsElectionArea, ResultsEvent,
        },
    },
};
use serde_json::Value;
use std::{collections::HashMap, fs::File};
use tempfile::NamedTempFile;
use tracing::{info, instrument};
use uuid::Uuid;

#[instrument(err, skip_all)]
async fn process_uuids(
    ids: Option<&str>,
    replacement_map: HashMap<String, String>,
) -> Result<Option<Vec<String>>> {
    match ids {
        None => Ok(None),
        Some(ids) => {
            let parsed: Vec<String> = deserialize_str::<Vec<String>>(ids)
                .map_err(|e| anyhow!("Failed to parse UUID array as JSON: {:?}", e))?;

            let new_ids: Vec<String> = parsed
                .into_iter()
                .map(|id| {
                    replacement_map
                        .get(&id)
                        .cloned()
                        .ok_or_else(|| anyhow!("Can't find id: {id} in replacement map"))
                })
                .collect::<Result<_>>()?;

            Ok(Some(new_ids))
        }
    }
}

#[instrument(skip_all)]
fn remap_result_documents(
    original: Option<ResultDocuments>,
    replacement_map: &HashMap<String, String>,
) -> Option<ResultDocuments> {
    original.map(|doc| ResultDocuments {
        json: doc
            .json
            .as_ref()
            .map(|id| replacement_map.get(id).cloned())
            .unwrap_or(None),
        pdf: doc
            .pdf
            .as_ref()
            .map(|id| replacement_map.get(id).cloned())
            .unwrap_or(None),
        html: doc
            .html
            .as_ref()
            .map(|id| replacement_map.get(id).cloned())
            .unwrap_or(None),
        tar_gz: doc
            .tar_gz
            .as_ref()
            .map(|id| replacement_map.get(id).cloned())
            .unwrap_or(None),
        tar_gz_original: doc
            .tar_gz_original
            .as_ref()
            .map(|id| replacement_map.get(id).cloned())
            .unwrap_or(None),
        vote_receipts_pdf: doc
            .vote_receipts_pdf
            .as_ref()
            .map(|id| replacement_map.get(id).cloned())
            .unwrap_or(None),
    })
}

#[instrument(err, skip_all)]
pub async fn get_replaced_id(
    record: &StringRecord,
    index: i32,
    replacement_map: &HashMap<String, String>,
) -> Result<String> {
    let id: String = record
        .get(index as usize)
        .ok_or_else(|| anyhow!("Missing column {index}"))
        .and_then(|s| deserialize_str(s).map_err(|e| anyhow!("Invalid JSON: {:?}", e)))?;
    let new_id = replacement_map
        .get(&id)
        .ok_or(anyhow!("Can't find id:{id} in replacement map"))?
        .clone();

    Ok(new_id)
}

#[instrument(err, skip_all)]
pub async fn get_opt_i64_item(record: &StringRecord, index: usize) -> Result<Option<i64>> {
    let item = record
        .get(index)
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "null")
        .map(|s| s.parse::<i64>())
        .transpose()
        .map_err(|err| anyhow!("Error parsing as i64 at column {index}: {:?}", err))?;
    Ok(item)
}

#[instrument(err, skip_all)]
pub async fn get_opt_json_value_item(record: &StringRecord, index: usize) -> Result<Option<Value>> {
    let item = record
        .get(index)
        .filter(|s| !s.is_empty())
        .map(deserialize_str)
        .transpose()
        .map_err(|err| anyhow!("Error process json column {index} {:?}", err))?;
    Ok(item)
}

#[instrument(err, skip_all)]
pub async fn get_opt_f64_item(record: &StringRecord, index: usize) -> Result<Option<NotNan<f64>>> {
    let item = record
        .get(index)
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "null")
        .map(|s| {
            let value = s
                .parse::<f64>()
                .map_err(|e| anyhow!("Error parsing as f64 at column {index}: {:?}", e))?;
            NotNan::new(value).map_err(|e| anyhow!("Value is NaN (not allowed in NotNan): {:?}", e))
        })
        .transpose()?;
    Ok(item)
}

#[instrument(err, skip_all)]
pub async fn get_string_or_null_item(
    record: &StringRecord,
    index: usize,
) -> Result<Option<String>> {
    let item = record
        .get(index)
        .map(str::trim)
        .map(|s| {
            if s == "null" {
                Ok(None)
            } else {
                deserialize_str::<String>(s).map(Some)
            }
        })
        .transpose()
        .map_err(|err| anyhow!("Error at column {index}: {:?}", err))?
        .flatten();
    Ok(item)
}

#[instrument(err, skip_all)]
pub async fn get_opt_date(record: &StringRecord, index: usize) -> Result<Option<DateTime<Local>>> {
    let item = record
        .get(index)
        .map(|s| {
            let s = s.trim_matches('"');
            ISO8601::to_date(s).ok()
        })
        .flatten();
    Ok(item)
}

#[instrument(err, skip_all)]
async fn process_event_results_file(
    hasura_transaction: &Transaction<'_>,
    temp_file: &NamedTempFile,
    tenant_id: &str,
    election_event_id: &str,
    replacement_map: HashMap<String, String>,
) -> Result<()> {
    let file = File::open(temp_file)?;
    let mut rdr = csv::Reader::from_reader(file);

    let mut results_events: Vec<ResultsEvent> = Vec::new();

    for result in rdr.records() {
        let record = result.map_err(|e| anyhow!("Error reading CSV record: {e:?}"))?;

        let results_event_id = get_replaced_id(&record, 0, &replacement_map).await?;
        let name = get_string_or_null_item(&record, 3).await?;

        let created_at = get_opt_date(&record, 4).await?;
        let last_updated_at = get_opt_date(&record, 5).await?;

        let annotations = get_opt_json_value_item(&record, 6).await?;
        let labels = get_opt_json_value_item(&record, 7).await?;

        let documents = record
            .get(8)
            .map(str::trim)
            .filter(|s| *s != "null" && *s != "\"null\"")
            .map(|s| deserialize_str(s))
            .transpose()
            .map_err(|err| anyhow!("Error at process documents: {:?}", err))?;
        let documents_with_new_ids = remap_result_documents(documents, &replacement_map);

        let results_event = ResultsEvent {
            id: results_event_id,
            tenant_id: tenant_id.to_string(),
            election_event_id: election_event_id.to_string(),
            name,
            labels,
            annotations,
            created_at,
            last_updated_at,
            documents: documents_with_new_ids,
        };
        results_events.push(results_event);
    }

    let _ = insert_many_results_events(hasura_transaction, results_events)
        .await
        .map_err(|err| anyhow!("Error at insert_many_results_events {:?}", err))?;

    Ok(())
}

#[instrument(err, skip_all)]
async fn process_results_election_file(
    hasura_transaction: &Transaction<'_>,
    temp_file: &NamedTempFile,
    tenant_id: &str,
    election_event_id: &str,
    replacement_map: HashMap<String, String>,
) -> Result<()> {
    let file = File::open(temp_file)?;
    let mut rdr = csv::Reader::from_reader(file);

    let mut results_elections: Vec<ResultsElection> = Vec::new();

    for result in rdr.records() {
        let record = result.map_err(|e| anyhow!("Error reading CSV record: {e:?}"))?;

        let election_id = get_replaced_id(&record, 3, &replacement_map).await?;
        let results_event_id = get_replaced_id(&record, 4, &replacement_map).await?;

        let name: Option<String> = get_string_or_null_item(&record, 5).await?;

        let elegible_census = get_opt_i64_item(&record, 6).await?;
        let total_voters = get_opt_i64_item(&record, 7).await?;

        let created_at = get_opt_date(&record, 8).await?;
        let last_updated_at = get_opt_date(&record, 9).await?;

        let labels = get_opt_json_value_item(&record, 10).await?;
        let annotations = get_opt_json_value_item(&record, 11).await?;

        let total_voters_percent = get_opt_f64_item(&record, 12).await?;

        let documents = record
            .get(13)
            .map(str::trim)
            .filter(|s| *s != "null" && *s != "\"null\"")
            .map(|s| deserialize_str(s))
            .transpose()
            .map_err(|err| anyhow!("Error at process documents: {:?}", err))?;
        let documents_with_new_ids = remap_result_documents(documents, &replacement_map);

        let results_election = ResultsElection {
            id: Uuid::new_v4().to_string(),
            tenant_id: tenant_id.to_string(),
            election_event_id: election_event_id.to_string(),
            election_id,
            results_event_id,
            name,
            elegible_census,
            total_voters,
            labels,
            annotations,
            created_at,
            last_updated_at,
            total_voters_percent,
            documents: documents_with_new_ids,
        };

        results_elections.push(results_election);
    }
    let _ = insert_many_results_elections(hasura_transaction, results_elections)
        .await
        .map_err(|err| anyhow!("Error at insert_many_results_elections {:?}", err))?;

    Ok(())
}

#[instrument(err, skip_all)]
async fn process_tally_session_file(
    hasura_transaction: &Transaction<'_>,
    temp_file: &NamedTempFile,
    tenant_id: &str,
    election_event_id: &str,
    replacement_map: HashMap<String, String>,
) -> Result<()> {
    let file = File::open(temp_file)?;
    let mut rdr = csv::Reader::from_reader(file);

    let mut tally_sessions: Vec<TallySession> = Vec::new();

    for result in rdr.records() {
        let record = result.map_err(|e| anyhow!("Error reading CSV record: {e:?}"))?;
        let tally_session = process_tally_session_record(
            tenant_id,
            election_event_id,
            &record,
            replacement_map.clone(),
        )
        .await
        .with_context(|| "Error proccess tally_session record")?;
        tally_sessions.push(tally_session);
    }
    let _ = insert_many_tally_sessions(hasura_transaction, tally_sessions)
        .await
        .map_err(|err| anyhow!("Error at insert_many_tally_sessions {:?}", err))?;
    Ok(())
}

#[instrument(err, skip_all)]
pub async fn process_tally_session_record(
    tenant_id: &str,
    election_event_id: &str,
    record: &StringRecord,
    replacement_map: HashMap<String, String>,
) -> Result<TallySession> {
    let tally_session_id = get_replaced_id(record, 0, &replacement_map).await?;

    let created_at = get_opt_date(&record, 3).await?;
    let last_updated_at = get_opt_date(&record, 4).await?;

    let labels = get_opt_json_value_item(&record, 5).await?;
    let annotations = get_opt_json_value_item(&record, 6).await?;
    let election_ids = process_uuids(record.get(7), replacement_map.clone()).await?;
    let area_ids = process_uuids(record.get(8), replacement_map.clone()).await?;

    let is_execution_completed = record
        .get(9)
        .unwrap_or("false")
        .parse::<bool>()
        .map_err(|err| anyhow!("Error at process is_execution_completed {:?}", err))?;

    let keys_ceremony_id = get_replaced_id(record, 10, &replacement_map).await?;

    let execution_status = get_string_or_null_item(record, 11).await?;

    let threshold = record
        .get(12)
        .unwrap_or("0")
        .parse::<i64>()
        .map_err(|err| anyhow!("Error at process threshold {:?}", err))?;

    let configuration = record
        .get(13)
        .filter(|s| !s.is_empty())
        .map(deserialize_str)
        .transpose()
        .map_err(|err| anyhow!("Error at process configuration {:?}", err))?;

    let tally_type = get_string_or_null_item(record, 14).await?;

    let permission_label = record
        .get(15)
        .filter(|s| !s.is_empty())
        .map(deserialize_str)
        .transpose()
        .map_err(|err| anyhow!("Error at process permission_label {:?}", err))?;

    let tally_session = TallySession {
        id: tally_session_id,
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        created_at,
        last_updated_at,
        labels,
        annotations,
        election_ids,
        area_ids,
        is_execution_completed,
        keys_ceremony_id,
        execution_status,
        threshold,
        configuration,
        tally_type,
        permission_label,
    };

    Ok(tally_session)
}

#[instrument(err, skip_all)]
async fn process_tally_session_contest_file(
    hasura_transaction: &Transaction<'_>,
    temp_file: &NamedTempFile,
    tenant_id: &str,
    election_event_id: &str,
    replacement_map: HashMap<String, String>,
) -> Result<()> {
    let file = File::open(temp_file)?;
    let mut rdr = csv::Reader::from_reader(file);

    let mut tally_session_contests: Vec<TallySessionContest> = Vec::new();

    for result in rdr.records() {
        let record = result.map_err(|e| anyhow!("Error reading CSV record: {e:?}"))?;

        let area_id = get_replaced_id(&record, 3, &replacement_map).await?;
        let contest_id: Option<String> = get_string_or_null_item(&record, 4).await?;

        let new_contest_id = match contest_id {
            Some(contest_id) => Some(
                replacement_map
                    .get(&contest_id)
                    .ok_or_else(|| {
                        anyhow!("Can't find contest_id={contest_id:?} in replacement map")
                    })?
                    .clone(),
            ),
            None => None,
        };

        let session_id = record
            .get(5)
            .unwrap_or("0")
            .parse::<i32>()
            .map_err(|err| anyhow!("Error at process session_id {:?}", err))?;

        let created_at = get_opt_date(&record, 6).await?;
        let last_updated_at = get_opt_date(&record, 7).await?;

        let labels = get_opt_json_value_item(&record, 8).await?;
        let annotations = get_opt_json_value_item(&record, 9).await?;
        let tally_session_id = get_replaced_id(&record, 10, &replacement_map).await?;

        let election_id = get_replaced_id(&record, 11, &replacement_map).await?;

        let tally_session_contest = TallySessionContest {
            id: Uuid::new_v4().to_string(),
            tenant_id: tenant_id.to_string(),
            election_event_id: election_event_id.to_string(),
            area_id,
            contest_id: new_contest_id,
            session_id,
            created_at,
            last_updated_at,
            labels,
            annotations,
            tally_session_id,
            election_id,
        };

        tally_session_contests.push(tally_session_contest);
    }

    let _ = insert_many_tally_session_contests(hasura_transaction, tally_session_contests)
        .await
        .map_err(|err| anyhow!("Error at insert_many_tally_session_contests {:?}", err))?;

    Ok(())
}

#[instrument(err, skip_all)]
async fn process_tally_session_execution_file(
    hasura_transaction: &Transaction<'_>,
    temp_file: &NamedTempFile,
    tenant_id: &str,
    election_event_id: &str,
    replacement_map: HashMap<String, String>,
) -> Result<()> {
    let file = File::open(temp_file)?;
    let mut rdr = csv::Reader::from_reader(file);

    let mut tally_session_executions: Vec<TallySessionExecution> = Vec::new();

    for result in rdr.records() {
        let record = result.map_err(|e| anyhow!("Error reading CSV record: {e:?}"))?;
        let created_at = get_opt_date(&record, 3).await?;
        let last_updated_at = get_opt_date(&record, 4).await?;

        let labels = get_opt_json_value_item(&record, 5).await?;
        let annotations = get_opt_json_value_item(&record, 6).await?;

        let current_message_id = record
            .get(7)
            .unwrap_or("0")
            .parse::<i32>()
            .map_err(|err| anyhow!("Error at process current_message_id {:?}", err))?;

        let tally_session_id: String = get_replaced_id(&record, 8, &replacement_map).await?;

        let session_ids = record
            .get(9)
            .map(str::trim)
            .filter(|s| *s != "null" && *s != "\"null\"")
            .map(|s| deserialize_str::<Vec<i32>>(s))
            .transpose()
            .map_err(|err| anyhow!("Error parsing session_ids: {:?}", err))?;

        let status = get_opt_json_value_item(&record, 10).await?;

        let results_event_id: Option<String> = get_string_or_null_item(&record, 11).await?;

        let new_results_event_id = match results_event_id {
            Some(results_event_id) => Some(
                replacement_map
                    .get(&results_event_id)
                    .ok_or_else(|| {
                        anyhow!(
                            "Can't find results_event_id={results_event_id:?} in replacement map"
                        )
                    })?
                    .clone(),
            ),
            None => None,
        };

        let tally_session_execution = TallySessionExecution {
            id: Uuid::new_v4().to_string(),
            tenant_id: tenant_id.to_string(),
            election_event_id: election_event_id.to_string(),
            created_at,
            last_updated_at,
            labels,
            annotations,
            current_message_id,
            tally_session_id,
            session_ids,
            status,
            results_event_id: new_results_event_id,
            documents: None,
        };

        tally_session_executions.push(tally_session_execution);
    }

    let _ = insert_many_tally_session_executions(hasura_transaction, tally_session_executions)
        .await
        .map_err(|err| anyhow!("Error at insert_many_tally_session_executions {:?}", err))?;

    Ok(())
}

#[instrument(err, skip_all)]
async fn process_results_election_area_file(
    hasura_transaction: &Transaction<'_>,
    temp_file: &NamedTempFile,
    tenant_id: &str,
    election_event_id: &str,
    replacement_map: HashMap<String, String>,
) -> Result<()> {
    let file = File::open(temp_file)?;
    let mut rdr = csv::Reader::from_reader(file);

    let mut results_elections_areas: Vec<ResultsElectionArea> = Vec::new();

    for result in rdr.records() {
        let record = result.map_err(|e| anyhow!("Error reading CSV record: {e:?}"))?;

        let election_id = get_replaced_id(&record, 3, &replacement_map).await?;
        let area_id = get_replaced_id(&record, 4, &replacement_map).await?;
        let results_event_id: String = get_replaced_id(&record, 5, &replacement_map).await?;

        let created_at = get_opt_date(&record, 6).await?;
        let last_updated_at = get_opt_date(&record, 7).await?;

        let documents = record
            .get(8)
            .map(str::trim)
            .filter(|s| *s != "null" && *s != "\"null\"")
            .map(|s| deserialize_str(s))
            .transpose()
            .map_err(|err| anyhow!("Error at process documents: {:?}", err))?;
        let documents_with_new_ids = remap_result_documents(documents, &replacement_map);

        let name: Option<String> = get_string_or_null_item(&record, 9).await?;

        let results_election_area = ResultsElectionArea {
            id: Uuid::new_v4().to_string(),
            tenant_id: tenant_id.to_string(),
            election_event_id: election_event_id.to_string(),
            election_id,
            results_event_id,
            area_id,
            created_at,
            last_updated_at,
            documents: documents_with_new_ids,
            name,
        };

        results_elections_areas.push(results_election_area);
    }
    let _ = insert_many_results_elections_areas(hasura_transaction, results_elections_areas)
        .await
        .map_err(|err| anyhow!("Error at insert_many_results_elections_area {:?}", err))?;

    Ok(())
}

#[instrument(err, skip_all)]
async fn process_results_contest_file(
    hasura_transaction: &Transaction<'_>,
    temp_file: &NamedTempFile,
    tenant_id: &str,
    election_event_id: &str,
    replacement_map: HashMap<String, String>,
) -> Result<()> {
    let file = File::open(temp_file)?;
    let mut rdr = csv::Reader::from_reader(file);

    let mut results_contests: Vec<ResultsContest> = Vec::new();

    for result in rdr.records() {
        let record = result.map_err(|e| anyhow!("Error reading CSV record: {e:?}"))?;
        let results_contest = process_results_contest_record(
            tenant_id,
            election_event_id,
            &record,
            replacement_map.clone(),
        )
        .await
        .with_context(|| "Error proccess results_contest record")?;
        results_contests.push(results_contest);
    }

    let _ = insert_many_results_contests(hasura_transaction, results_contests)
        .await
        .map_err(|err| anyhow!("Error at insert_many_results_contests {:?}", err))?;

    Ok(())
}

#[instrument(err, skip_all)]
async fn process_results_contest_candidate_file(
    hasura_transaction: &Transaction<'_>,
    temp_file: &NamedTempFile,
    tenant_id: &str,
    election_event_id: &str,
    replacement_map: HashMap<String, String>,
) -> Result<()> {
    let file = File::open(temp_file)?;

    let mut rdr = csv::Reader::from_reader(file);

    let mut results_contests_candidates: Vec<ResultsContestCandidate> = Vec::new();

    for result in rdr.records() {
        let record = result.map_err(|e| anyhow!("Error reading CSV record: {e:?}"))?;
        let election_id = get_replaced_id(&record, 3, &replacement_map).await?;
        let contest_id = get_replaced_id(&record, 4, &replacement_map).await?;
        let candidate_id = get_replaced_id(&record, 5, &replacement_map).await?;
        let results_event_id = get_replaced_id(&record, 6, &replacement_map).await?;
        let cast_votes = get_opt_i64_item(&record, 7).await?;
        let winning_position = get_opt_i64_item(&record, 8).await?;
        let points = get_opt_i64_item(&record, 9).await?;
        let created_at = get_opt_date(&record, 10).await?;
        let last_updated_at = get_opt_date(&record, 11).await?;
        let labels = get_opt_json_value_item(&record, 12).await?;
        let annotations = get_opt_json_value_item(&record, 13).await?;
        let cast_votes_percent = get_opt_f64_item(&record, 14).await?;
        let documents = record
            .get(15)
            .map(str::trim)
            .filter(|s| *s != "null" && *s != "\"null\"")
            .map(|s| deserialize_str(s))
            .transpose()
            .map_err(|err| anyhow!("Error at process documents: {:?}", err))?;
        let documents_with_new_ids = remap_result_documents(documents, &replacement_map);

        let results_contest_candidate = ResultsContestCandidate {
            id: Uuid::new_v4().to_string(),
            tenant_id: tenant_id.to_string(),
            election_event_id: election_event_id.to_string(),
            election_id,
            contest_id,
            candidate_id,
            results_event_id,
            cast_votes,
            winning_position,
            points,
            created_at,
            last_updated_at,
            labels,
            annotations,
            cast_votes_percent,
            documents: documents_with_new_ids,
        };
        results_contests_candidates.push(results_contest_candidate);
    }

    let _ = insert_many_results_contest_candidates(hasura_transaction, results_contests_candidates)
        .await
        .map_err(|err| anyhow!("Error at insert_many_results_contest_candidates {:?}", err))?;

    Ok(())
}

#[instrument(err, skip_all)]
pub async fn process_results_contest_record(
    tenant_id: &str,
    election_event_id: &str,
    record: &StringRecord,
    replacement_map: HashMap<String, String>,
) -> Result<ResultsContest> {
    let election_id: String = get_replaced_id(record, 3, &replacement_map).await?;
    let contest_id: String = get_replaced_id(record, 4, &replacement_map).await?;
    let results_event_id: String = get_replaced_id(record, 5, &replacement_map).await?;

    let elegible_census = get_opt_i64_item(record, 6).await?;

    let total_valid_votes = get_opt_i64_item(record, 7).await?;

    let explicit_invalid_votes = get_opt_i64_item(record, 8).await?;

    let implicit_invalid_votes = get_opt_i64_item(record, 9).await?;

    let blank_votes = get_opt_i64_item(record, 10).await?;

    let voting_type: Option<String> = get_string_or_null_item(&record, 11).await?;
    let counting_algorithm: Option<String> = get_string_or_null_item(&record, 12).await?;
    let name: Option<String> = get_string_or_null_item(&record, 13).await?;

    let created_at = get_opt_date(&record, 14).await?;
    let last_updated_at = get_opt_date(&record, 15).await?;

    let labels = get_opt_json_value_item(record, 16).await?;

    let annotations = get_opt_json_value_item(record, 17).await?;

    let total_invalid_votes = get_opt_i64_item(record, 18).await?;

    let total_invalid_votes_percent = get_opt_f64_item(record, 19).await?;
    let total_valid_votes_percent = get_opt_f64_item(record, 20).await?;

    let explicit_invalid_votes_percent = get_opt_f64_item(record, 21).await?;
    let implicit_invalid_votes_percent = get_opt_f64_item(record, 22).await?;
    let blank_votes_percent = get_opt_f64_item(record, 23).await?;

    let total_votes = get_opt_i64_item(record, 24).await?;
    let total_votes_percent = get_opt_f64_item(record, 25).await?;

    let documents = record
        .get(26)
        .map(str::trim)
        .filter(|s| *s != "null" && *s != "\"null\"")
        .map(|s| deserialize_str(s))
        .transpose()
        .map_err(|err| anyhow!("Error at process documents: {:?}", err))?;
    let documents_with_new_ids = remap_result_documents(documents, &replacement_map);

    let total_auditable_votes = get_opt_i64_item(record, 27).await?;
    let total_auditable_votes_percent = get_opt_f64_item(record, 28).await?;

    let results_contest = ResultsContest {
        id: Uuid::new_v4().to_string(),
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        election_id,
        contest_id,
        results_event_id,
        elegible_census,
        total_valid_votes,
        explicit_invalid_votes,
        implicit_invalid_votes,
        blank_votes,
        voting_type,
        counting_algorithm,
        name,
        created_at,
        last_updated_at,
        labels,
        annotations,
        total_invalid_votes,
        total_invalid_votes_percent,
        total_valid_votes_percent,
        explicit_invalid_votes_percent,
        implicit_invalid_votes_percent,
        blank_votes_percent,
        total_votes,
        total_votes_percent,
        documents: documents_with_new_ids,
        total_auditable_votes,
        total_auditable_votes_percent,
    };
    Ok(results_contest)
}

#[instrument(err, skip_all)]
async fn process_results_area_contest_file(
    hasura_transaction: &Transaction<'_>,
    temp_file: &NamedTempFile,
    tenant_id: &str,
    election_event_id: &str,
    replacement_map: HashMap<String, String>,
) -> Result<()> {
    let file = File::open(temp_file)?;

    let mut rdr = csv::Reader::from_reader(file);

    let mut results_area_contests: Vec<ResultsAreaContest> = Vec::new();

    for result in rdr.records() {
        let record = result.map_err(|e| anyhow!("Error reading CSV record: {e:?}"))?;
        let election_id = get_replaced_id(&record, 3, &replacement_map).await?;
        let contest_id = get_replaced_id(&record, 4, &replacement_map).await?;
        let area_id = get_replaced_id(&record, 5, &replacement_map).await?;
        let results_event_id = get_replaced_id(&record, 6, &replacement_map).await?;
        let elegible_census = get_opt_i64_item(&record, 7).await?;
        let total_valid_votes = get_opt_i64_item(&record, 8).await?;
        let explicit_invalid_votes = get_opt_i64_item(&record, 9).await?;
        let implicit_invalid_votes = get_opt_i64_item(&record, 10).await?;
        let blank_votes = get_opt_i64_item(&record, 11).await?;
        let created_at = get_opt_date(&record, 12).await?;
        let last_updated_at = get_opt_date(&record, 13).await?;
        let labels = get_opt_json_value_item(&record, 14).await?;
        let annotations = get_opt_json_value_item(&record, 15).await?;
        let total_valid_votes_percent = get_opt_f64_item(&record, 16).await?;
        let total_invalid_votes = get_opt_i64_item(&record, 17).await?;
        let total_invalid_votes_percent = get_opt_f64_item(&record, 18).await?;
        let explicit_invalid_votes_percent = get_opt_f64_item(&record, 19).await?;
        let blank_votes_percent = get_opt_f64_item(&record, 20).await?;
        let implicit_invalid_votes_percent = get_opt_f64_item(&record, 21).await?;
        let total_votes = get_opt_i64_item(&record, 22).await?;
        let total_votes_percent = get_opt_f64_item(&record, 23).await?;

        let documents = record
            .get(24)
            .map(str::trim)
            .filter(|s| *s != "null" && *s != "\"null\"")
            .map(|s| deserialize_str(s))
            .transpose()
            .map_err(|err| anyhow!("Error at process documents: {:?}", err))?;
        let documents_with_new_ids = remap_result_documents(documents, &replacement_map);

        let total_auditable_votes = get_opt_i64_item(&record, 25).await?;
        let total_auditable_votes_percent = get_opt_f64_item(&record, 26).await?;

        let results_area_contest = ResultsAreaContest {
            id: Uuid::new_v4().to_string(),
            tenant_id: tenant_id.to_string(),
            election_event_id: election_event_id.to_string(),
            election_id,
            contest_id,
            area_id,
            results_event_id,
            elegible_census,
            total_valid_votes,
            explicit_invalid_votes,
            implicit_invalid_votes,
            blank_votes,
            created_at,
            last_updated_at,
            labels,
            annotations,
            total_valid_votes_percent,
            total_invalid_votes,
            total_invalid_votes_percent,
            explicit_invalid_votes_percent,
            blank_votes_percent,
            implicit_invalid_votes_percent,
            total_votes,
            total_votes_percent,
            documents: documents_with_new_ids,
            total_auditable_votes,
            total_auditable_votes_percent,
        };
        results_area_contests.push(results_area_contest);
    }
    let _ = insert_many_results_area_contests(hasura_transaction, results_area_contests)
        .await
        .map_err(|err| anyhow!("Error at insert_many_results_area_contests {:?}", err))?;

    Ok(())
}

#[instrument(err, skip_all)]
async fn process_results_area_contest_candidate_file(
    hasura_transaction: &Transaction<'_>,
    temp_file: &NamedTempFile,
    tenant_id: &str,
    election_event_id: &str,
    replacement_map: HashMap<String, String>,
) -> Result<()> {
    let file = File::open(temp_file)?;

    let mut rdr = csv::Reader::from_reader(file);

    let mut results_area_contests_candidates: Vec<ResultsAreaContestCandidate> = Vec::new();

    for result in rdr.records() {
        let record = result.map_err(|e| anyhow!("Error reading CSV record: {e:?}"))?;
        let election_id = get_replaced_id(&record, 3, &replacement_map).await?;
        let contest_id = get_replaced_id(&record, 4, &replacement_map).await?;
        let area_id = get_replaced_id(&record, 5, &replacement_map).await?;
        let candidate_id = get_replaced_id(&record, 6, &replacement_map).await?;
        let results_event_id = get_replaced_id(&record, 7, &replacement_map).await?;
        let cast_votes = get_opt_i64_item(&record, 8).await?;
        let winning_position = get_opt_i64_item(&record, 9).await?;
        let points = get_opt_i64_item(&record, 10).await?;
        let created_at = get_opt_date(&record, 11).await?;
        let last_updated_at = get_opt_date(&record, 12).await?;
        let labels = get_opt_json_value_item(&record, 13).await?;
        let annotations = get_opt_json_value_item(&record, 14).await?;
        let cast_votes_percent = get_opt_f64_item(&record, 15).await?;
        let documents = record
            .get(16)
            .map(str::trim)
            .filter(|s| *s != "null" && *s != "\"null\"")
            .map(|s| deserialize_str(s))
            .transpose()
            .map_err(|err| anyhow!("Error at process documents: {:?}", err))?;
        let documents_with_new_ids = remap_result_documents(documents, &replacement_map);

        let results_area_contest_candidate = ResultsAreaContestCandidate {
            id: Uuid::new_v4().to_string(),
            tenant_id: tenant_id.to_string(),
            election_event_id: election_event_id.to_string(),
            election_id,
            contest_id,
            area_id,
            candidate_id,
            results_event_id,
            cast_votes,
            winning_position,
            points,
            created_at,
            last_updated_at,
            labels,
            annotations,
            cast_votes_percent,
            documents: documents_with_new_ids,
        };
        results_area_contests_candidates.push(results_area_contest_candidate);
    }

    let _ = insert_many_results_area_contest_candidates(
        hasura_transaction,
        results_area_contests_candidates,
    )
    .await
    .map_err(|err| {
        anyhow!(
            "Error at insert_many_results_area_contest_candidates {:?}",
            err
        )
    })?;

    Ok(())
}

#[instrument(err, skip_all)]
pub async fn process_tally_file(
    hasura_transaction: &Transaction<'_>,
    temp_file: &NamedTempFile,
    file_name: String,
    tenant_id: &str,
    election_event_id: &str,
    replacement_map: HashMap<String, String>,
) -> Result<()> {
    if file_name == ETallyDocuments::TALLY_SESSION.to_file_name().to_string() {
        process_tally_session_file(
            hasura_transaction,
            temp_file,
            tenant_id,
            election_event_id,
            replacement_map.clone(),
        )
        .await?;
    } else if file_name
        == ETallyDocuments::TALLY_SESSION_CONTEST
            .to_file_name()
            .to_string()
    {
        process_tally_session_contest_file(
            hasura_transaction,
            temp_file,
            tenant_id,
            election_event_id,
            replacement_map.clone(),
        )
        .await?;
    } else if file_name == ETallyDocuments::RESULTS_EVENT.to_file_name().to_string() {
        process_event_results_file(
            hasura_transaction,
            temp_file,
            tenant_id,
            election_event_id,
            replacement_map.clone(),
        )
        .await?;
    } else if file_name
        == ETallyDocuments::TALLY_SESSION_EXECUTION
            .to_file_name()
            .to_string()
    {
        process_tally_session_execution_file(
            hasura_transaction,
            temp_file,
            tenant_id,
            election_event_id,
            replacement_map.clone(),
        )
        .await?;
    } else if file_name == ETallyDocuments::RESULTS_ELECTION.to_file_name().to_string() {
        process_results_election_file(
            hasura_transaction,
            temp_file,
            tenant_id,
            election_event_id,
            replacement_map.clone(),
        )
        .await?;
    } else if file_name
        == ETallyDocuments::RESULTS_ELECTION_AREA
            .to_file_name()
            .to_string()
    {
        process_results_election_area_file(
            hasura_transaction,
            temp_file,
            tenant_id,
            election_event_id,
            replacement_map.clone(),
        )
        .await?;
    } else if file_name == ETallyDocuments::RESULTS_CONTEST.to_file_name().to_string() {
        process_results_contest_file(
            hasura_transaction,
            temp_file,
            tenant_id,
            election_event_id,
            replacement_map.clone(),
        )
        .await?;
    } else if file_name
        == ETallyDocuments::RESULTS_CONTEST_CANDIDATE
            .to_file_name()
            .to_string()
    {
        process_results_contest_candidate_file(
            hasura_transaction,
            temp_file,
            tenant_id,
            election_event_id,
            replacement_map.clone(),
        )
        .await?;
    } else if file_name
        == ETallyDocuments::RESULTS_AREA_CONTEST
            .to_file_name()
            .to_string()
    {
        process_results_area_contest_file(
            hasura_transaction,
            temp_file,
            tenant_id,
            election_event_id,
            replacement_map.clone(),
        )
        .await?;
    } else if file_name
        == ETallyDocuments::RESULTS_AREA_CONTEST_CANDIDATE
            .to_file_name()
            .to_string()
    {
        process_results_area_contest_candidate_file(
            hasura_transaction,
            temp_file,
            tenant_id,
            election_event_id,
            replacement_map.clone(),
        )
        .await?;
    }

    Ok(())
}
