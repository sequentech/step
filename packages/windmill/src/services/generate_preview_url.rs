// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::{
    postgres::{document::get_document, preview::insert_preview},
    services::{
        documents::{get_document_as_temp_file, upload_and_return_document},
        providers::transactions_provider::provide_hasura_transaction,
    },
    tasks::prepare_publication_preview::PublicationPreview,
};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::{
    serialization::deserialize_with_path::deserialize_value, temp_path::get_file_size,
    types::hasura::core::BallotStyle,
};
use std::{fs::File, io::SeekFrom};
use tempfile::NamedTempFile;
use tracing::instrument;

#[instrument(err)]
pub fn construct_preview_url(
    tenant_id: &str,
    document_id: &str,
    area_id: &str,
    ballot_style_id: &str,
) -> Result<String> {
    let voting_portal_url = std::env::var("VOTING_PORTAL_URL")
        .map_err(|err| anyhow!("AWS_RVOTING_PORTAL_URLEGION env var missing: {err}"))?;

    let url = format!(
        "{}/preview/{}/{}/{}/{}",
        voting_portal_url, tenant_id, document_id, area_id, ballot_style_id
    );
    Ok(url)
}

#[instrument(err)]
pub async fn get_documnet_data(preview_file_path: &str) -> Result<(String, String)> {
    let file = File::open(preview_file_path)
        .map_err(|e| anyhow::anyhow!("Failed to open preview file: {}", e))?;
    let parsed: PublicationPreview = serde_json::from_reader(file)
        .map_err(|e| anyhow!("Error reading uploaded preview file: {}", e))?;

    let ballot_styles: Vec<BallotStyle> =
        deserialize_value::<Vec<BallotStyle>>(parsed.ballot_styles.clone())?;
    let area_ballot_style = ballot_styles
        .get(0)
        .ok_or_else(|| anyhow!("Error reading ballot stlyes"))?;

    let area_id = area_ballot_style
        .clone()
        .area_id
        .ok_or_else(|| anyhow!("area_id not found in uploaded documnet"))?;
    let ballot_publication_id = area_ballot_style.ballot_publication_id.clone();

    Ok((ballot_publication_id, area_id))
}

#[instrument(err)]
pub async fn generate_preview_url(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    document_id: &str,
    executer_name: &str,
) -> Result<String> {
    let uploaded_preview_document = get_document(hasura_transaction, tenant_id, None, document_id)
        .await?
        .ok_or_else(|| anyhow!("Ballot style document not found"))?;

    let preview_temp_file: NamedTempFile =
        get_document_as_temp_file(tenant_id, &uploaded_preview_document).await?;

    let preview_file_path = preview_temp_file
        .into_temp_path()
        .to_string_lossy()
        .to_string();

    let file_size =
        get_file_size(preview_file_path.as_str()).with_context(|| "Error obtaining file size")?;

    let (ballot_publication_id, area_id) = get_documnet_data(&preview_file_path).await?;
    let doc_name = format!("{ballot_publication_id}.json");

    let document = upload_and_return_document(
        hasura_transaction,
        &preview_file_path,
        file_size,
        "application/json",
        &tenant_id,
        None,
        &doc_name,
        None,
        true,
    )
    .await
    .map_err(|err| anyhow!("Error uploading document: {err:?}"))?;

    let preview_url = construct_preview_url(&tenant_id, &document.id, "", "")?;

    insert_preview(
        hasura_transaction,
        tenant_id,
        &document.id,
        preview_url.clone(),
        executer_name,
    )
    .await
    .map_err(|err| anyhow!("Error insert preview: {err:?}"))?;

    Ok(preview_url)
}
