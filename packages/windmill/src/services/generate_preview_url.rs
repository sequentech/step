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
    serialization::deserialize_with_path::deserialize_value, types::hasura::core::BallotStyle,
    util::temp_path::write_into_named_temp_file,
};
use serde_json::Value;
use std::fs::File;
use tempfile::NamedTempFile;
use tracing::instrument;
use uuid::Uuid;

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

/// Reads an uploaded preview payload, returning it alongside the identifiers
/// the preview URL is built from. The payload is sanitized here because it is
/// re-uploaded to the public bucket: a preview generated before the event
/// annotations were sanitized would otherwise publish the Datafix credentials
/// it still carries.
#[instrument(err)]
pub fn read_sanitized_preview(
    preview_file_path: &str,
) -> Result<(PublicationPreview, String, String)> {
    let file = File::open(preview_file_path)
        .map_err(|e| anyhow::anyhow!("Failed to open preview file: {}", e))?;
    let mut parsed: PublicationPreview = serde_json::from_reader(file)
        .map_err(|e| anyhow!("Error reading uploaded preview file: {}", e))?;
    parsed.remove_datafix_annotations();

    let ballot_styles = parsed
        .ballot_styles
        .as_array()
        .ok_or_else(|| anyhow!("ballot_styles is not an array"))?;

    let area_ballot_style = ballot_styles
        .get(0)
        .ok_or_else(|| anyhow!("ballot_styles array is empty"))?;

    let area_id = area_ballot_style
        .get("area_id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("area_id not found in ballot_styles[0]"))?
        .to_string();

    let ballot_style_id = area_ballot_style
        .get("id")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    Ok((parsed, ballot_style_id, area_id))
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

    let temp_path = preview_temp_file.into_temp_path();
    let temp_path_string = temp_path.to_string_lossy().to_string();

    let (preview, ballot_style_id, area_id) = read_sanitized_preview(&temp_path_string)?;
    let doc_name = format!("{ballot_style_id}.json");

    let preview_data: Vec<u8> =
        serde_json::to_vec(&preview).with_context(|| "Error serializing publication preview")?;
    let (_sanitized_temp_path, sanitized_path_string, file_size) = write_into_named_temp_file(
        &preview_data,
        &format!("preview-{ballot_style_id}-"),
        ".json",
    )
    .with_context(|| "Error writing sanitized preview to file")?;

    let document = upload_and_return_document(
        hasura_transaction,
        &sanitized_path_string,
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

    let preview_url = construct_preview_url(&tenant_id, &document.id, &area_id, &ballot_style_id)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::external::utils::DATAFIX_VOTERVIEW_REQ_KEY;
    use serde_json::json;
    use std::io::Write;

    fn preview_file(payload: Value) -> NamedTempFile {
        let mut file = NamedTempFile::new().expect("temp file");
        file.write_all(payload.to_string().as_bytes())
            .expect("write payload");
        file
    }

    #[test]
    fn reading_an_uploaded_preview_strips_datafix_annotations() {
        let file = preview_file(json!({
            "ballot_styles": [{"id": "style-id", "area_id": "area-id"}],
            "election_event": {
                "id": "event",
                "annotations": {
                    DATAFIX_VOTERVIEW_REQ_KEY: r#"{"url":"https://example.invalid","usr":"user","psw":"secret"}"#,
                    "miru:election-event-id": "miru-event",
                },
            },
            "elections": [],
            "support_materials": [],
            "documents": [],
        }));

        let (preview, ballot_style_id, area_id) =
            read_sanitized_preview(&file.path().to_string_lossy()).expect("preview read");

        assert_eq!(ballot_style_id, "style-id");
        assert_eq!(area_id, "area-id");
        let sanitized = serde_json::to_value(&preview).expect("preview serialization");
        assert_eq!(
            sanitized["election_event"]["annotations"],
            json!({"miru:election-event-id": "miru-event"})
        );
    }

    #[test]
    fn reading_a_preview_without_ballot_styles_fails() {
        let file = preview_file(json!({
            "ballot_styles": [],
            "election_event": {"id": "event"},
            "elections": [],
            "support_materials": [],
            "documents": [],
        }));

        assert!(read_sanitized_preview(&file.path().to_string_lossy()).is_err());
    }
}
