// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    services::{
        database::get_hasura_pool, documents::upload_and_return_document,
        temp_path::write_into_named_temp_file,
    },
    types::error::Result,
};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use sequent_core::services::keycloak;
use sequent_core::services::{pdf, reports};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tracing::instrument;

use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use tokio_postgres::row::Row;
use uuid::Uuid;

async fn get_template(election_id: &str) -> Result<Option<String>> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| anyhow!("{}", err))?;

    let mut hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|err| anyhow!("{}", err))?;

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

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 60000)]
pub async fn create_vote_receipt(
    element_id: String,
    ballot_id: String,
    ballot_tracker_url: String,
    tenant_id: String,
    election_event_id: String,
    election_id: String,
) -> Result<()> {
    const QR_CODE_TEMPLATE: &'static str = "<div id=\"qrcode\"></div>";

    let auth_headers = keycloak::get_client_credentials().await?;

    let mut map = Map::new();

    let template = get_template(&election_id).await?;

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
        tenant_id,
        election_event_id,
        format!("vote-receipt-{ballot_id}.pdf"),
        Some(element_id),
        true,
    )
    .await?;

    Ok(())
}
