// SPDX-FileCopyrightText: 2024 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::{communication_template, election};
use crate::services::{
    documents::upload_and_return_document, temp_path::write_into_named_temp_file,
};
use anyhow::{anyhow, Context, Result};
use sequent_core::services::keycloak;
use sequent_core::services::{pdf, reports};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tracing::instrument;

use deadpool_postgres::Transaction;
use tokio_postgres::row::Row;
use uuid::Uuid;

const QR_CODE_TEMPLATE: &'static str = "<div id=\"qrcode\"></div>";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Receipt {
    pub allowed: bool,
    pub template: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReceiptsRoot {
    pub SMS: Option<Receipt>,
    pub EMAIL: Option<Receipt>,
    pub DOCUMENT: Option<Receipt>,
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_template(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
) -> Result<Option<String>> {
    let Some(election) = election::get_election_by_id(
        hasura_transaction,
        tenant_id,
        election_event_id,
        election_id,
    )
    .await?
    else {
        return Ok(None);
    };

    let Some(receipts_json) = election.receipts else {
        return Ok(None);
    };

    let receipts: ReceiptsRoot = serde_json::from_value(receipts_json)?;
    let Some(template_id) = receipts.DOCUMENT.map(|document| document.template) else {
        return Ok(None);
    };

    let Some(communication_template) = communication_template::get_communication_template_by_id(
        hasura_transaction,
        tenant_id,
        &template_id,
    )
    .await?
    else {
        return Ok(None);
    };
    let id = template_id.as_str();
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VoteReceiptData {
    pub ballot_id: String,
    pub ballot_tracker_url: String,
    pub qrcode: String,
    pub template: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VoteReceiptRoot {
    pub data: VoteReceiptData,
}

impl VoteReceiptRoot {
    pub fn to_map(self) -> Result<Map<String, Value>> {
        let Value::Object(map) = serde_json::to_value(self.clone())? else {
            return Err(anyhow!("Can't convert VoteReceiptRoot to Map"));
        };
        Ok(map)
    }
}

#[instrument(skip(hasura_transaction), err)]
pub async fn create_vote_receipt(
    hasura_transaction: &Transaction<'_>,
    element_id: &str,
    ballot_id: &str,
    ballot_tracker_url: &str,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
) -> Result<()> {
    let template_opt = get_template(
        hasura_transaction,
        tenant_id,
        election_event_id,
        election_id,
    )
    .await?;

    let mut data = VoteReceiptData {
        ballot_id: ballot_id.to_string(),
        ballot_tracker_url: ballot_tracker_url.to_string(),
        qrcode: QR_CODE_TEMPLATE.to_string(),
        template: None,
    };
    let sub_map = VoteReceiptRoot { data: data.clone() }.to_map()?;

    let template = template_opt
        .map(|template| reports::render_template_text(&template, sub_map))
        .transpose()?;

    data.template = template;

    let map = VoteReceiptRoot { data: data.clone() }.to_map()?;
    let custom_html_template = include_str!("../resources/vote_receipt_custom.hbs");
    let render = reports::render_template_text(custom_html_template, map)?;

    let bytes_pdf = pdf::html_to_pdf(render).map_err(|err| anyhow!("{}", err))?;

    let (_temp_path, temp_path_string, file_size) =
        write_into_named_temp_file(&bytes_pdf, "vote-receipt-", ".html")
            .with_context(|| "Error writing to file")?;

    let auth_headers = keycloak::get_client_credentials().await?;
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
