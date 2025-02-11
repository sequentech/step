// SPDX-FileCopyrightText: 2024 Sequent Tech <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::tenant::update_tenant;
use anyhow::{anyhow, Context, Result};
use csv::StringRecord;
use deadpool_postgres::Transaction;
use regex::Regex;
use sequent_core::services::date::ISO8601;
use sequent_core::{
    serialization::deserialize_with_path::deserialize_str, types::hasura::core::Tenant,
};
use serde_json::Value as JsonValue;
use std::fs::File;
use tempfile::NamedTempFile;
use tracing::{info, instrument};
use uuid::Uuid;

lazy_static! {
    pub static ref HEADER_RE: Regex = Regex::new(r"^[a-zA-Z0-9._-]+$").unwrap();
}

#[instrument(err, skip_all)]
pub async fn upsert_tenant(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    temp_file: NamedTempFile,
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
        // Only one tenant should be in the file
        let record = result.map_err(|e| anyhow!("Error reading CSV record: {e:?}"))?;

        process_record(hasura_transaction, tenant_id, &record)
            .await
            .map_err(|e| anyhow!("Error processing record: {e:?}"))?;
    }

    Ok(())
}

#[instrument(err, skip_all)]
pub async fn process_record(
    hasura_transaction: &Transaction<'_>,
    old_tenant_id: &str,
    record: &StringRecord,
) -> Result<()> {
    info!("record: {:?}", record);

    let slug = record
        .get(1)
        .ok_or_else(|| anyhow!("Missing Report Type"))?
        .to_string();
    let created_at = record
        .get(2)
        .and_then(|s| ISO8601::to_date(s.trim_matches('"')).ok());
    let updated_at = record
        .get(3)
        .and_then(|s| ISO8601::to_date(s.trim_matches('"')).ok());
    let labels = record
        .get(4)
        .and_then(|s| deserialize_str::<JsonValue>(s).ok());
    let annotations = record
        .get(5)
        .and_then(|s| deserialize_str::<JsonValue>(s).ok());
    let is_active: bool = record
        .get(6)
        .map(|val| deserialize_str::<bool>(val).ok())
        .flatten()
        .ok_or_else(|| anyhow!("Error deserializing is_active"))?;
    let voting_channels = record
        .get(7)
        .and_then(|s| deserialize_str::<JsonValue>(s).ok());
    let settings = record
        .get(8)
        .and_then(|s| deserialize_str::<JsonValue>(s).ok());
    let test = record
        .get(8)
        .map(|val| deserialize_str::<i32>(val).ok())
        .ok_or_else(|| anyhow!("Error deserializing test"))?;

    let tenant = Tenant {
        id: old_tenant_id.to_string(),
        slug,
        created_at,
        updated_at,
        labels,
        annotations,
        is_active,
        voting_channels,
        settings,
        test,
    };

    update_tenant(hasura_transaction, tenant, old_tenant_id)
        .await
        .map_err(|e| anyhow!("Error upserting tenant into the database: {e:?}"))?;

    Ok(())
}
