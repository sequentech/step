// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::tenant::update_tenant;
use crate::services::google_meet::{store_service_account_key, take_service_account_key};
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
use std::sync::LazyLock;
use tempfile::NamedTempFile;
use tracing::{info, instrument};
use uuid::Uuid;

pub static HEADER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9._-]+$").expect("Failed to compile header regex"));

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

/// Parse one tenant-config CSV row into a [`Tenant`], extracting the Google
/// service-account key out of `settings`.
///
/// Columns mirror the exporter header order in `export_tenant.rs`:
/// `0 id, 1 slug, 2 created_at, 3 updated_at, 4 labels, 5 annotations,`
/// `6 is_active, 7 voting_channels, 8 settings, 9 test`. In particular `test`
/// is column 9 — distinct from `settings` at column 8.
pub(crate) fn parse_tenant_record(
    old_tenant_id: &str,
    record: &StringRecord,
) -> Result<(Tenant, Option<JsonValue>)> {
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
    let mut settings = record
        .get(8)
        .and_then(|s| deserialize_str::<JsonValue>(s).ok());
    let service_account_key = settings.as_mut().and_then(take_service_account_key);
    // `test` is column 9 (distinct from `settings` at 8). A tenant export made
    // before the `test` column existed has only nine columns; treat an absent or
    // non-integer value as None so such exports still import (backwards compatible).
    let test = record
        .get(9)
        .and_then(|val| deserialize_str::<i32>(val).ok());

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

    Ok((tenant, service_account_key))
}

#[instrument(err, skip_all)]
pub async fn process_record(
    hasura_transaction: &Transaction<'_>,
    old_tenant_id: &str,
    record: &StringRecord,
) -> Result<()> {
    info!("record: {:?}", record);

    let (tenant, service_account_key) = parse_tenant_record(old_tenant_id, record)?;

    update_tenant(hasura_transaction, tenant, old_tenant_id)
        .await
        .map_err(|e| anyhow!("Error upserting tenant into the database: {e:?}"))?;

    if let Some(service_account_key) = service_account_key {
        store_service_account_key(hasura_transaction, old_tenant_id, service_account_key)
            .await
            .map_err(|e| anyhow!("Error storing the Google service account key: {e}"))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Columns mirror the exporter header order in `export_tenant.rs`:
    // 0 id, 1 slug, 2 created_at, 3 updated_at, 4 labels, 5 annotations,
    // 6 is_active, 7 voting_channels, 8 settings, 9 test.
    //
    // Regression: `test` must be read from column 9, not column 8 (`settings`).
    // The old code read `.get(8)` for both, so `test` parsed the settings JSON
    // object as an i32 and silently collapsed to `None`.
    #[test]
    fn parses_exporter_row_keeping_settings_and_test_distinct() {
        let record = StringRecord::from(vec![
            "ignored-source-id",
            "my-slug",
            "",
            "",
            "",
            "",
            "true",
            "",
            r#"{"theme":"dark"}"#,
            "7",
        ]);

        let (tenant, _key) = parse_tenant_record("tenant-id", &record).expect("record parses");

        assert_eq!(tenant.settings, Some(json!({ "theme": "dark" })));
        assert_eq!(tenant.test, Some(7));
        assert_eq!(tenant.slug, "my-slug");
        assert!(tenant.is_active);
    }

    // The exporter writes a serde JSON value per column; `test: None` serializes as
    // `null`, which must parse back to None (not an error).
    #[test]
    fn a_null_test_column_parses_as_none() {
        let record = StringRecord::from(vec![
            "id",
            "slug",
            "",
            "",
            "",
            "",
            "false",
            "",
            r#"{"a":1}"#,
            "null",
        ]);
        let (tenant, _key) = parse_tenant_record("t", &record).expect("record parses");
        assert_eq!(tenant.test, None);
        assert_eq!(tenant.settings, Some(json!({ "a": 1 })));
    }

    // A tenant export made before the `test` column existed has only nine columns;
    // it must still import, with `test` defaulting to None (backwards compatible).
    #[test]
    fn a_pre_test_column_nine_field_export_still_imports() {
        let record =
            StringRecord::from(vec!["id", "slug", "", "", "", "", "true", "", r#"{"a":1}"#]);
        let (tenant, _key) = parse_tenant_record("t", &record).expect("legacy record parses");
        assert_eq!(tenant.test, None);
        assert_eq!(tenant.settings, Some(json!({ "a": 1 })));
        assert!(tenant.is_active);
    }
}
