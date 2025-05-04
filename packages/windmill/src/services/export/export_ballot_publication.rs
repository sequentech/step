// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::ballot_publication::get_ballot_publication;
use crate::types::documents::EDocuments;
use crate::postgres::ballot_style::{
    export_event_ballot_styles, get_ballot_styles_by_ballot_publication_by_id,
};
use crate::services::documents::upload_and_return_document;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use futures::{pin_mut, StreamExt};
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::types::hasura::core::BallotPublication;
use sequent_core::util::temp_path::{generate_temp_file, write_into_named_temp_file};
use serde_json::{json, Value};
use std::path::PathBuf;
use tempfile::TempPath;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::{info, instrument};

#[instrument(err, skip(transaction, data))]
pub async fn write_export_document(
    transaction: &Transaction<'_>,
    data: Vec<u8>,
    document_id: &str,
    election_event_id: &str,
    tenant_id: &str,
    to_upload: bool,
) -> Result<TempPath> {
    let document_name = format!("export-{}.json", document_id);

    let (temp_path, temp_path_string, file_size) =
        write_into_named_temp_file(&data, &document_name, ".json")
            .map_err(|e| anyhow!("Error writing into named temp file: {e:?}"))?;

    if to_upload {
        upload_and_return_document(
            transaction,
            &temp_path_string,
            file_size,
            "text/json",
            tenant_id,
            None,
            &document_name,
            Some(document_id.to_string()),
            false,
        )
        .await
        .map_err(|e| anyhow!("Error uploading and returning document to postgres: {e:?}"))?;
    }

    Ok(temp_path)
}

#[instrument(err)]
pub async fn process_export_ballot_publication(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    document_id: &str,
    ballot_publications: &Vec<BallotPublication>,
    to_upload: bool,
) -> Result<TempPath> {
    let mut ballot_designs = vec![];
    let event_styles =
        export_event_ballot_styles(&hasura_transaction, &tenant_id, &election_event_id)
            .await
            .with_context(|| "Error obtaining ballot styles")?;
    for ballot_publication in ballot_publications {
        let ballot_publication_id = ballot_publication.id.clone();
        let ballot_styles = event_styles
            .iter()
            .filter(|style| style.ballot_publication_id == ballot_publication_id)
            .collect::<Vec<_>>()
            .clone();

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
        });

        ballot_designs.push(ballot_design);
    }

    // Serialize the array into JSON string
    let data = serde_json::to_vec_pretty(&ballot_designs)
        .map_err(|e| anyhow!("Error serializing ballot designs to JSON: {e:?}"))?;

    // Write the JSON data to a file
    let temp_path = write_export_document(
        &hasura_transaction,
        data,
        document_id,
        election_event_id,
        tenant_id,
        to_upload,
    )
    .await?;
    info!(
        "JSON data exported successfully for document_id: {}",
        document_id
    );

    Ok(temp_path)
}

#[instrument(err)]
pub async fn export_ballot_publications_csv(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<(String, TempPath)> {
    let ballot_publications =
        get_ballot_publication(&hasura_transaction, tenant_id, election_event_id)
            .await
            .map_err(|e| anyhow!("Error reading ballot publications data: {e:?}"))?;

    let file_name = EDocuments::PUBLICATIONS.to_file_name().to_string();

    let mut writer = csv::WriterBuilder::new().delimiter(b',').from_writer(
        generate_temp_file(&file_name, ".csv").with_context(|| "Error creating temporary file")?,
    );

    writer.write_record(&[
        "id".to_string(),
        "tenant_id".to_string(),
        "election_event_id".to_string(),
        "labels".to_string(),
        "annotations".to_string(),
        "created_at".to_string(),
        "deleted_at".to_string(),
        "created_by_user_id".to_string(),
        "is_generated".to_string(),
        "election_ids".to_string(),
        "published_at".to_string(),
        "election_id".to_string(),
    ])?;

    for ballot_publication in ballot_publications {
        let values: Vec<String> = serde_json::to_value(ballot_publication)?
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

#[instrument(err)]
pub async fn export_ballot_styles_csv(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<(String, TempPath)> {
    let file_name = EDocuments::BALLOT_STYLE.to_file_name().to_string();

    let mut named_file = generate_temp_file(&file_name, ".csv")
        .with_context(|| "Error creating temporary file for CSV export")?;
    let temp_path: TempPath = named_file.into_temp_path();
    let path_buf: PathBuf = temp_path.to_path_buf();

    let mut file = File::create(&path_buf)
        .await
        .with_context(|| "Error opening temp file for async write")?;

    let copy_query = format!(
        r#"COPY (
            SELECT
                id,
                tenant_id,
                election_id,
                area_id,
                created_at,
                last_updated_at,
                labels::text       AS labels,
                annotations::text  AS annotations,
                ballot_eml,
                ballot_signature,
                status,
                election_event_id,
                deleted_at,
                ballot_publication_id
            FROM sequent_backend.ballot_style
            WHERE tenant_id = '{}'
              AND election_event_id = '{}'
              AND deleted_at IS NULL
        ) TO STDOUT WITH CSV HEADER"#,
        tenant_id, election_event_id,
    );

    let mut stream = hasura_transaction
        .copy_out(&copy_query)
        .await
        .with_context(|| "Failed to initiate COPY OUT for ballot_styles")?;
    pin_mut!(stream);

    while let Some(chunk) = stream.next().await {
        let data = chunk.with_context(|| "Error reading COPY OUT stream")?;
        file.write_all(&data)
            .await
            .with_context(|| "Error writing CSV data to temp file")?;
    }

    file.flush()
        .await
        .with_context(|| "Error flushing temporary CSV file")?;

    Ok((file_name, temp_path))
}

#[instrument(err)]
pub async fn export_publications(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<(String, TempPath)>> {
    let ballot_publications =
        export_ballot_publications_csv(hasura_transaction, tenant_id, election_event_id)
            .await
            .map_err(|e| anyhow!("Error export ballot publications: {e:?}"))?;
    let ballot_styles = export_ballot_styles_csv(hasura_transaction, tenant_id, election_event_id)
        .await
        .map_err(|e| anyhow!("Error export ballot styles: {e:?}"))?;

    Ok(vec![ballot_publications, ballot_styles])
}
