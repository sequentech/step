// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use csv::StringRecord;
use deadpool_postgres::Transaction;
use sequent_core::types::scheduled_event::{generate_manage_date_task_name, ScheduledEvent, CronConfig, EventProcessors, ManageElectionDatePayload};
use std::collections::HashMap;
use std::fs::File;
use tempfile::NamedTempFile;
use tracing::{info, instrument};
use uuid::Uuid;
use serde_json::Value as JsonValue;
use crate::postgres::scheduled_event::insert_new_scheduled_event;
use sequent_core::serialization::deserialize_with_path::deserialize_value;

#[instrument(err)]
pub async fn import_scheduled_events(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    temp_file: NamedTempFile,
    replacement_map: HashMap<String, String>,
) -> Result<()> {
    let file = File::open(temp_file)?;
    let mut rdr = csv::Reader::from_reader(file);

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
    println!("------------ record: {:?}", record);
    let old_election_id = record.get(10)
        .map(|v| serde_json::from_str::<JsonValue>(v))
        .transpose()?
        .and_then(|json| json.get("election_id").and_then(|id| id.as_str().map(String::from)));

    println!("------------ old_election_id: {:?}", old_election_id);
    let election_id = if old_election_id.is_some() {
        Some(
            replacement_map
                .get(&old_election_id.unwrap().to_string())
                .ok_or(anyhow!("Can't find election id in replacement map"))?
                .clone(),
        )
    } else {
        None
    };

    println!("------------ election_id: {:?}", election_id);
    let id = Uuid::new_v4().to_string();
    let created_at = Some(Utc::now());
    let stopped_at = record.get(4).map(|s| s.parse::<DateTime<Utc>>().ok()).flatten();
    let archived_at = record.get(5).map(|s| s.parse::<DateTime<Utc>>().ok()).flatten();
    let labels = record.get(6).map(|s| serde_json::from_str::<JsonValue>(s).ok()).flatten();
    println!("------------ labels: {:?}", labels);
    let annotations = record.get(7).map(|s| serde_json::from_str::<JsonValue>(s).ok()).flatten();
    println!("------------ annotations: {:?}", annotations);
    let event_processor: Option<EventProcessors> = record.get(8)
        .map(|val| serde_json::from_str::<EventProcessors>(val))
        .transpose()?; // This propagates any deserialization errors properly
    println!("------------ event_processor: {:?}", event_processor);
    let cron_config: Option<CronConfig> = record
    .get(9)
    .map(|val| serde_json::from_str(val))
    .transpose().context("Error deserializing cron_config")?;
println!("------------ cron_config: {:?}", cron_config);
    let event_payload = ManageElectionDatePayload {
        election_id: election_id.clone(),
    };
    let event_payload = Some(serde_json::to_value(event_payload)?);
    let task_id = match &event_processor {
        Some(event_processor) => Some(generate_manage_date_task_name(
            tenant_id,
            election_event_id,
            election_id.as_deref(),
            event_processor, // Use the reference here
        )),
        None => None,
    };

    let scheduled_event = ScheduledEvent {
        id,
        election_event_id: Some(election_event_id.clone().to_string()),
        tenant_id: Some(tenant_id.to_string()),
        created_at,
        stopped_at,
        archived_at,
        labels,
        annotations,
        event_processor,
        cron_config,
        event_payload,
        task_id,
    };

    println!("------------ scheduled_event: {:?}", scheduled_event);

    insert_new_scheduled_event(hasura_transaction, scheduled_event.clone()).await?;

    Ok(())
}
