// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::template::get_templates_by_tenant_id;
use crate::services::database::get_hasura_pool;
use crate::services::{
    documents::upload_and_return_document_postgres, temp_path::write_into_named_temp_file,
};
use anyhow::{anyhow, Result};
use csv::Writer;
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::serialization::deserialize_with_path;
use sequent_core::types::hasura::core::Template;
use sequent_core::{services::keycloak::get_event_realm, types::hasura::core::Document};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tracing::{event, info, instrument, Level};

#[instrument(err, skip(transaction))]
pub async fn read_export_data(
    transaction: &Transaction<'_>,
    election_event_id: &str,
    tenant_id: &str,
    ballot_design: &str,
) -> Result<String> {
    let ballot_design_json: Value = deserialize_with_path::deserialize_str(ballot_design)?;
    let mut csv_data = vec![];

    fn flatten_json(prefix: String, value: &Value, csv_data: &mut Vec<(String, String)>) {
        match value {
            Value::Object(map) => {
                for (k, v) in map {
                    flatten_json(format!("{}.{}", prefix, k), v, csv_data);
                }
            }
            Value::Array(arr) => {
                for (i, v) in arr.iter().enumerate() {
                    flatten_json(format!("{}[{}]", prefix, i), v, csv_data);
                }
            }
            _ => {
                csv_data.push((prefix, value.to_string()));
            }
        }
    }
    flatten_json("".to_string(), &ballot_design_json, &mut csv_data);
    let mut writer = Writer::from_writer(vec![]);
    writer.write_record(&["key", "value"])?;

    for (key, value) in csv_data {
        writer.write_record(&[key.trim_start_matches('.'), &value])?;
    }

    writer.flush()?;

    let csv_data = String::from_utf8(writer.into_inner()?)
        .map_err(|e| anyhow!("Error converting CSV data to String: {e:?}"))?;

    Ok(csv_data)
}

#[instrument(err, skip(transaction, data))]
pub async fn write_export_document(
    transaction: &Transaction<'_>,
    data: Vec<u8>,
    document_id: &str,
    election_event_id: &str,
    tenant_id: &str,
) -> Result<Document> {
    let name = format!("export-{}.csv", document_id);

    let (_temp_path, temp_path_string, file_size) =
        write_into_named_temp_file(&data, &name, ".csv")
            .map_err(|e| anyhow!("Error writing into named temp file: {e:?}"))?;

    upload_and_return_document_postgres(
        transaction,
        &temp_path_string,
        file_size,
        "text/csv",
        tenant_id,
        None,
        &name,
        Some(document_id.to_string()),
        false,
    )
    .await
}

#[instrument(err)]
pub async fn process_export_json_to_csv(
    tenant_id: &str,
    election_event_id: &str,
    document_id: &str,
    ballot_design: &str,
) -> Result<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| anyhow!("Error getting hasura db pool: {err}"))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|err| anyhow!("Error starting hasura transaction: {err}"))?;

    let csv_data = read_export_data(
        &hasura_transaction,
        election_event_id,
        tenant_id,
        ballot_design,
    )
    .await?;

    write_export_document(
        &hasura_transaction,
        csv_data.into(),
        document_id,
        election_event_id,
        tenant_id,
    )
    .await?;
    info!(
        "CSV data exported successfully for document_id: {}",
        document_id
    );

    hasura_transaction
        .commit()
        .await
        .map_err(|e| anyhow!("Commit failed: {e}"))?;

    Ok(())
}
