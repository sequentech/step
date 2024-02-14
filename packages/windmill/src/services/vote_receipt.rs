// SPDX-FileCopyrightText: 2024 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::{
    database::get_hasura_pool, documents::upload_and_return_document,
    temp_path::write_into_named_temp_file,
};
use anyhow::{anyhow, Context, Result};
use celery::error::TaskError;
use sequent_core::services::keycloak;
use sequent_core::services::{pdf, reports};
use serde_json::{json, Map};
use tracing::instrument;

use deadpool_postgres::{Client as DbClient, Transaction};
use tokio_postgres::row::Row;
use uuid::Uuid;

const QR_CODE_TEMPLATE: &'static str = "<div id=\"qrcode\"></div>";

pub async fn get_template(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_id: &str,
) -> Result<Option<String>> {
    let query = hasura_transaction
        .prepare(
            r#"
            SELECT receipts FROM sequent_backend.election WHERE id = $1;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &query,
            &[&Uuid::parse_str(election_id).map_err(|err| anyhow!("{}", err))?],
        )
        .await?;

    let results: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|row| -> Result<serde_json::Value> {
            Ok(row
                .try_get::<_, serde_json::Value>("receipts")
                .map_err(|err| anyhow!("Error getting the receipts of a row: {}", err))?)
        })
        .collect::<Result<Vec<serde_json::Value>>>()
        .map_err(|err| anyhow!("Error getting the receipts: {}", err))?;

    let template_id = results[0]
        .get("DOCUMENT")
        .and_then(|doc| doc.get("template"));

    if let Some(id) = template_id {
        if let Some(id) = id.as_str() {
            let query = hasura_transaction
                .prepare(
                    r#"
                    SELECT template FROM sequent_backend.communication_template WHERE id = $1;
                "#,
                )
                .await?;

            let rows: Vec<Row> = hasura_transaction
                .query(
                    &query,
                    &[&Uuid::parse_str(id).map_err(|err| anyhow!("{}", err))?],
                )
                .await?;

            let results: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|row| -> Result<serde_json::Value> {
                    Ok(row
                        .try_get::<_, serde_json::Value>("template")
                        .map_err(|err| anyhow!("Error getting the template of a row: {}", err))?)
                })
                .collect::<Result<Vec<serde_json::Value>>>()
                .map_err(|err| anyhow!("Error getting the template: {}", err))?;

            let template = results[0].get("document");

            return Ok(template
                .and_then(|t| t.as_str())
                .and_then(|s| Some(s.to_string())));
        }
    }

    Ok(None)
}

pub async fn create_vote_receipt(
    transaction: &Transaction<'_>,
    element_id: &str,
    ballot_id: &str,
    ballot_tracker_url: &str,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
) -> Result<()> {
    let auth_headers = keycloak::get_client_credentials().await?;

    let mut map = Map::new();

    let template = get_template(transaction, tenant_id, election_id).await?;

    let render = match template {
        Some(template) => {
            let mut sub_map = Map::new();
            sub_map.insert(
                "data".to_string(),
                json!({
                    "ballot_id": ballot_id.clone(),
                    "ballot_tracker_url": ballot_tracker_url,
                    "qrcode": QR_CODE_TEMPLATE
                }),
            );

            map.insert(
                "data".to_string(),
                json!({
                    "ballot_id": ballot_id.clone(),
                    "ballot_tracker_url": ballot_tracker_url,
                    "qrcode": QR_CODE_TEMPLATE,
                    "template": Some(
                        reports::render_template_text(&template, sub_map)
                            .map_err(|err| anyhow!("{}", err))?,
                    ),
                }),
            );

            let custom_html_template = include_str!("../resources/vote_receipt_custom.hbs");

            reports::render_template_text(custom_html_template, map)
                .map_err(|err| anyhow!("{}", err))?
        }
        None => {
            map.insert(
                "data".to_string(),
                json!({
                    "ballot_id": ballot_id.clone(),
                    "ballot_tracker_url": ballot_tracker_url,
                    "qrcode": QR_CODE_TEMPLATE,
                }),
            );

            let default_html_template = include_str!("../resources/vote_receipt.hbs");

            reports::render_template_text(default_html_template, map)
                .map_err(|err| anyhow!("{}", err))?
        }
    };

    let bytes_pdf = pdf::html_to_pdf(render).map_err(|err| anyhow!("{}", err))?;

    let (_temp_path, temp_path_string, file_size) =
        write_into_named_temp_file(&bytes_pdf, "vote-receipt-", ".html")
            .with_context(|| "Error writing to file")?;

    let _document = upload_and_return_document(
        temp_path_string,
        file_size,
        "application/pdf".to_string(),
        auth_headers.clone(),
        tenant_id.to_string(),
        election_event_id.to_string(),
        format!("vote-receipt-{ballot_id}.pdf"),
        Some(element_id.to_string()),
        true,
    )
    .await?;

    Ok(())
}
