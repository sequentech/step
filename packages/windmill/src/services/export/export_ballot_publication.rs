// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::{
    documents::upload_and_return_document_postgres, temp_path::write_into_named_temp_file,
};
use crate::postgres::ballot_style::get_ballot_styles_by_ballot_publication_by_id;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use anyhow::{anyhow, Result};
use csv::Writer;
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::serialization::deserialize_with_path;
use crate::postgres::ballot_publication::get_ballot_publication;
use sequent_core::types::hasura::core::Document;
use sequent_core::types::hasura::core::{BallotPublication, Template};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tempfile::TempPath;
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
    to_upload: bool,
) -> Result<TempPath> {
    let document_name = format!("export-{}.csv", document_id);

    let (_temp_path, temp_path_string, file_size) =
        write_into_named_temp_file(&data, &document_name, ".csv")
            .map_err(|e| anyhow!("Error writing into named temp file: {e:?}"))?;

    if to_upload {
        upload_and_return_document_postgres(
            transaction,
            &temp_path_string,
            file_size,
            "text/csv",
            tenant_id,
            None,
            &document_name,
            Some(document_id.to_string()),
            false,
        )
        .await
        .map_err(|e| anyhow!("Error uploading and returning document to postgres: {e:?}"))?;
    }

    Ok(_temp_path)
}

#[instrument(err)]
pub async fn process_export_ballot_publication(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    document_id: &str,
    ballot_publication: &BallotPublication,
) -> Result<()> {
    let ballot_styles = match get_ballot_styles_by_ballot_publication_by_id(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &ballot_publication.id,
    )
    .await
    {
        Ok(ballot_styles) => ballot_styles,
        Err(err) => {
            return Err(anyhow!("Error obtaining ballot styles: {err:?}"));
        }
    };

    let ballot_emls = match ballot_styles
        .into_iter()
        .filter_map(|val| val.ballot_eml.as_ref().map(|eml| Ok(deserialize_str(eml)?)))
        .collect::<Result<Vec<Value>>>()
    {
        Ok(ballot_emls) => ballot_emls,
        Err(err) => {
            return Err(anyhow!("Error deserializing ballot emls: {err:?}"));
        }
    };

    let ballot_design = json!({
        "ballot_publication_id": &ballot_publication.id,
        "ballot_styles": ballot_emls,
    })
    .to_string();

    let csv_data = read_export_data(
        &hasura_transaction,
        election_event_id,
        tenant_id,
        &ballot_design,
    )
    .await.map_err(|e| anyhow!("Error reading export data: {e:?}"))?;

    write_export_document(
        &hasura_transaction,
        csv_data.into(),
        document_id,
        election_event_id,
        tenant_id,
        true,
    )
    .await?;
    info!(
        "CSV data exported successfully for document_id: {}",
        document_id
    );

    Ok(())
}

#[instrument(err)]
pub async fn export_ballot_publications(
    hasura_transaction: &Transaction<'_>,
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<()>{
    let ballot_publications_data = get_ballot_publication(
        &hasura_transaction,
        tenant_id,
        election_event_id,
    )
    .await
    .map_err(|e| anyhow!("Error reading ballot publications data: {e:?}"))?;

    for ballot_publication in ballot_publications_data {
        process_export_ballot_publication(
            &hasura_transaction,
            &tenant_id,
            &election_event_id,
            &document_id,
            &ballot_publication,
        )
        .await.map_err(|e| anyhow!("Error processing export ballot publication: {e:?}"))?;
    };

    // TODO: needs to return temp path
    Ok(())
}

#[instrument(err, skip(hasura_transaction))]
pub async fn write_export_document_csv(
    data: Vec<BallotPublication>,
    hasura_transaction: &Transaction<'_>,
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<(TempPath)> {
    let headers = if let Some(example_event) = data.get(0) {
        serde_json::to_value(example_event)?
            .as_object()
            .ok_or_else(|| {
                anyhow!("Failed to convert Ballots Publication to JSON object for headers")
            })?
            .keys()
            .cloned()
            .collect::<Vec<String>>()
    } else {
        vec![
            "id".to_string(),
            "tenant_id".to_string(),
            "election_event_id".to_string(),
            "labels".to_string(),
            "annotations".to_string(),
            "created_at".to_string(),
            "deleted_at".to_string(),
            "created_by_user".to_string(),
            "is_generated".to_string(),
            "election_ids".to_string(),
            "published_at".to_string(),
            "election_id".to_string(),
        ]
    };

    let name = format!("ballot-publications-{}", election_event_id);

    let mut writer = Writer::from_writer(vec![]);
    writer.write_record(&headers)?;

    for ballot_publication in data.clone() {
        let values: Vec<String> = serde_json::to_value(ballot_publication)?
            .as_object()
            .ok_or_else(|| anyhow!("Failed to convert ballot publication to JSON object"))?
            .values()
            .map(|value| value.to_string())
            .collect();

        writer.write_record(&values)?;
    }

    let data_bytes = writer
        .into_inner()
        .map_err(|e| anyhow!("Error converting writer into inner: {e:?}"))?;

    let temp_path = write_export_document(
        &hasura_transaction,
        data_bytes,
        document_id,
        election_event_id,
        tenant_id,
        false,
    )
    .await
    .map_err(|e| anyhow!("Error writing export document: {e:?}"))?;

    Ok(temp_path)
}
