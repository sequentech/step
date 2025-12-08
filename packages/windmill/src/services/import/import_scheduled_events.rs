// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::scheduled_event::insert_new_scheduled_event;
use anyhow::{anyhow, Context, Result};
use csv::StringRecord;
use deadpool_postgres::Transaction;
use regex::Regex;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::services::date::ISO8601;
use sequent_core::types::scheduled_event::{
    generate_manage_date_task_name, CronConfig, EventProcessors, ManageElectionDatePayload,
    ScheduledEvent,
};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::fs::File;
use tempfile::NamedTempFile;
use tracing::{info, instrument};
use uuid::Uuid;

lazy_static! {
    pub static ref HEADER_RE: Regex = Regex::new(r"^[a-zA-Z0-9._-]+$").unwrap();
}

#[instrument(err, skip(replacement_map))]
pub async fn import_scheduled_events(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    temp_file: NamedTempFile,
    replacement_map: HashMap<String, String>,
) -> Result<()> {
    let file = File::open(temp_file)?;
    let separator = b',';

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(separator)
        .from_reader(file);

    let headers = rdr
        .headers()
        .map(|headers| headers.clone())
        .map_err(|err| anyhow!("Error reading CSV headers: {err:?}"))?;

    for header in headers.iter() {
        if !HEADER_RE.is_match(header) {
            return Err(anyhow!("Invalid header name: {header:?}"));
        }
    }

    for result in rdr.records() {
        let record = result.map_err(|e| anyhow!("Error reading CSV record: {e:?}"))?;

        process_record(
            hasura_transaction,
            tenant_id,
            election_event_id,
            &record,
            replacement_map.clone(),
        )
        .await
        .with_context(|| "Error inserting scheduled_events into the database")?;
    }

    Ok(())
}

#[instrument(err, skip_all)]
pub async fn process_record(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    record: &StringRecord,
    replacement_map: HashMap<String, String>,
) -> Result<()> {
    info!("record: {:?}", record);
    let old_election_id = record
        .get(10)
        .map(|v| deserialize_str::<JsonValue>(v))
        .transpose()?
        .and_then(|json| {
            json.get("election_id")
                .and_then(|id| id.as_str().map(String::from))
        });

    let election_id = if let Some(old_election_id) = old_election_id {
        Some(
            replacement_map
                .get(&old_election_id)
                .ok_or(anyhow!("Can't find election id in replacement map"))?
                .clone(),
        )
    } else {
        None
    };

    let id = Uuid::new_v4().to_string();
    let created_at = record
        .get(3)
        .map(|s| {
            let s = s.trim_matches('"');
            ISO8601::to_date_utc(s).ok()
        })
        .flatten();
    let labels = record
        .get(6)
        .map(|s| deserialize_str::<JsonValue>(s).ok())
        .flatten();
    let annotations = record
        .get(7)
        .map(|s| deserialize_str::<JsonValue>(s).ok())
        .flatten();
    let event_processor: Option<EventProcessors> = record
        .get(8)
        .map(|val| deserialize_str::<EventProcessors>(val))
        .transpose()
        .context("Error deserializing event_processor")?;
    let cron_config: Option<CronConfig> = record
        .get(9)
        .map(|val| deserialize_str(val))
        .transpose()
        .context("Error deserializing cron_config")?;
    let event_payload = ManageElectionDatePayload {
        election_id: election_id.clone(),
    };
    let event_payload = Some(serde_json::to_value(event_payload)?);
    let task_id = match &event_processor {
        Some(event_processor) => Some(generate_manage_date_task_name(
            tenant_id,
            election_event_id,
            election_id.as_deref(),
            event_processor,
        )),
        None => None,
    };

    let scheduled_event = ScheduledEvent {
        id,
        election_event_id: Some(election_event_id.to_string()),
        tenant_id: Some(tenant_id.to_string()),
        created_at,
        stopped_at: None,
        archived_at: None,
        labels,
        annotations,
        event_processor,
        cron_config,
        event_payload,
        task_id,
    };

    insert_new_scheduled_event(hasura_transaction, scheduled_event.clone())
        .await
        .map_err(|e| anyhow!("Error inserting scheduled_event into the database: {e:?}"))?;

    Ok(())
}
