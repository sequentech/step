use crate::{postgres::tally_session::insert_tally_session_obj, types::documents::ETallyDocuments};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Local};
use csv::StringRecord;
use deadpool_postgres::Transaction;
use sequent_core::{services::date::ISO8601, types::hasura::core::TallySession};
use std::{collections::HashMap, fs::File};
use tempfile::NamedTempFile;
use tracing::{info, instrument};

#[instrument(err, skip_all)]
async fn process_uuids(
    ids: Option<&str>,
    replacement_map: HashMap<String, String>,
) -> Result<Option<Vec<String>>> {
    match ids {
        None => return Ok(None),
        Some(ids) => {
            let trimmed = ids.trim_matches(|c| c == '[' || c == ']');
            let new_ids: Vec<String> = trimmed
                .split(',')
                .map(|s| s.trim()) // Remove any whitespace
                .map(|uuid| {
                    replacement_map
                        .get(uuid)
                        .cloned()
                        .unwrap_or_else(|| uuid.to_string()) // Keep original if not found
                })
                .collect();
            Ok(Some(new_ids))
        }
    }
}
#[instrument(err, skip_all)]
async fn process_tally_event_results_file(
    hasura_transaction: &Transaction<'_>,
    temp_file: &NamedTempFile,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<()> {
    let file = File::open(temp_file)?;

    let mut rdr = csv::Reader::from_reader(file);

    // let mut reports: Vec<_> = Vec::new();
    println!("rdr:: {:?}", &rdr);

    for result in rdr.records() {
        let record = result.map_err(|e| anyhow!("Error reading CSV record: {e:?}"))?;
        println!("record:: {:?}", &record);
    }
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

    for result in rdr.records() {
        let record = result.map_err(|e| anyhow!("Error reading CSV record: {e:?}"))?;
        process_tally_session_record(
            hasura_transaction,
            tenant_id,
            election_event_id,
            &record,
            replacement_map.clone(),
        )
        .await
        .with_context(|| "Error inserting tally_session into the database")?;
    }
    Ok(())
}

#[instrument(err, skip_all)]
pub async fn process_tally_session_record(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    record: &StringRecord,
    replacement_map: HashMap<String, String>,
) -> Result<()> {
    info!("record: {:?}", record);

    let election_ids = process_uuids(record.get(7), replacement_map.clone()).await?;
    let area_ids = process_uuids(record.get(8), replacement_map.clone()).await?;

    let tally_session_id = record
        .get(0)
        .ok_or_else(|| anyhow!("Missing column 0 (tally_session_id) in CSV record"))?
        .to_string();
    let new_tally_session_id = replacement_map
        .get(&tally_session_id)
        .cloned()
        .unwrap_or_else(|| tally_session_id.clone());

    let keys_ceremony_id = record
        .get(10)
        .ok_or_else(|| anyhow!("Missing column 10 (keys_ceremony_id) in CSV record"))?
        .to_string();
    let new_keys_ceremony_id = replacement_map
        .get(&keys_ceremony_id)
        .cloned()
        .unwrap_or_else(|| keys_ceremony_id.clone());

    let created_at = record
        .get(3)
        .filter(|s| !s.is_empty())
        .map(|s| DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&Local)))
        .transpose()?;

    let last_updated_at = record
        .get(4)
        .filter(|s| !s.is_empty())
        .map(|s| DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&Local)))
        .transpose()?;

    let labels = record
        .get(5)
        .filter(|s| !s.is_empty())
        .map(serde_json::from_str)
        .transpose()?;

    let annotations = record
        .get(6)
        .filter(|s| !s.is_empty())
        .map(serde_json::from_str)
        .transpose()?;

    let configuration = record
        .get(13)
        .filter(|s| !s.is_empty())
        .map(serde_json::from_str)
        .transpose()?;

    // Booleans and integers
    let is_execution_completed = record.get(9).unwrap_or("false").parse::<bool>()?;

    let threshold = record.get(12).unwrap_or("0").parse::<i64>()?;

    // permission_label: try parse as JSON array, fallback to None
    let permission_label = record
        .get(15)
        .filter(|s| !s.is_empty())
        .map(serde_json::from_str)
        .transpose()?;

    let execution_status = record.get(11).map(|s| s.to_string());
    let tally_type = record.get(14).map(|s| s.to_string());

    let tally_session = TallySession {
        id: new_tally_session_id,
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        created_at,
        last_updated_at,
        labels,
        annotations,
        election_ids,
        area_ids,
        is_execution_completed,
        keys_ceremony_id: new_keys_ceremony_id,
        execution_status,
        threshold,
        configuration,
        tally_type,
        permission_label,
    };

    let tally_session_id = insert_tally_session_obj(hasura_transaction, tally_session).await?;

    Ok(())
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

    for result in rdr.records() {
        let record = result.map_err(|e| anyhow!("Error reading CSV record: {e:?}"))?;
        println!("record:: {:?}", &record);
    }
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
    }
    if file_name
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
    }
    if file_name == ETallyDocuments::RESULTS_EVENT.to_file_name().to_string() {
        process_tally_event_results_file(
            hasura_transaction,
            temp_file,
            election_event_id,
            tenant_id,
        )
        .await?;
    }

    Ok(())
}
