// SPDX-FileCopyrightText: 2024 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::env;

use super::s3;
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

const QR_CODE_TEMPLATE: &'static str = "<div id=\"qrcode\"></div>";
const LOGO_TEMPLATE: &'static str = "<div class=\"logo\"></div>";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Receipt {
    pub allowed: bool,
    pub template: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReceiptsRoot {
    pub sms: Option<Receipt>,
    pub email: Option<Receipt>,
    pub document: Option<Receipt>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommunicationTemplateValue {
    pub sms: String,
    pub name: String,
    pub alias: String,
    pub email: EmailTemplate,
    pub document: String,
    pub schedule_now: Option<bool>,
    pub schedule_date: Option<String>,
    pub audience_selection: Option<String>,
    pub audience_voter_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct EmailTemplate {
    subject: String,
    html_body: String,
    plaintext_body: String,
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
    let Some(template_id) = receipts.document.and_then(|document| document.template) else {
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

    let communication_template_value: CommunicationTemplateValue =
        serde_json::from_value(communication_template.template)?;

    Ok(Some(communication_template_value.document))
}

async fn get_public_asset_vote_receipt_template() -> Result<String> {
    let public_asset_path = env::var("PUBLIC_ASSETS_PATH")?;
    let file_vote_receipt_template = env::var("PUBLIC_ASSETS_VOTE_RECEIPT_TEMPLATE")?;

    let minio_private_uri =
        env::var("AWS_S3_PRIVATE_URI").map_err(|err| anyhow!("AWS_S3_PRIVATE_URI must be set"))?;
    let bucket = s3::get_public_bucket()?;

    let minio_endpoint_base = format!("{}/{}", minio_private_uri, bucket);
    let vote_receipt_template = format!(
        "{}/{}/{}",
        minio_endpoint_base, public_asset_path, file_vote_receipt_template
    );

    let client = reqwest::Client::new();
    let response = client.get(vote_receipt_template).send().await?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(anyhow!("File not found: {}", file_vote_receipt_template));
    } else if !response.status().is_success() {
        return Err(anyhow!(
            "Unexpected response status: {:?}",
            response.status()
        ));
    }

    let template_hbs: String = response.text().await?;

    Ok(template_hbs)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VoteReceiptData {
    pub ballot_id: String,
    pub ballot_tracker_url: String,
    pub qrcode: String,
    pub logo: String,
    pub title: String,
    pub file_logo: String,
    pub file_qrcode_lib: String,
    pub template: Option<String>, // TODO: remove this
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
    let file_logo = env::var("PUBLIC_ASSETS_LOGO_IMG")?;
    let file_qrcode_lib = env::var("PUBLIC_ASSETS_QRCODE_LIB")?;
    let vote_receipt_title = env::var("VOTE_RECEIPT_TEMPLATE_TITLE")?;

    let template_opt = get_template(
        hasura_transaction,
        tenant_id,
        election_event_id,
        election_id,
    )
    .await?;

    let template_hbs = get_public_asset_vote_receipt_template().await?;

    let template = template_opt.unwrap_or(
        r#"
            <div>
            {{{data.logo}}}
            </div>
            <div>
            <h2>Your vote has been cast</h2>
            <p>
                The confirmation code bellow verifies that your ballot has been cast
                successfully. You can use this code to verify that your ballot has
                been counted.
            </p>
            <p>
                Your Ballot ID: <span class="id-content">{{data.ballot_id}}</span>
            </p>
            </div>

            <div>
            <h3>Verify that your ballot has been cast</h3>
            <p>
                You can verify your ballot has been cast correctly at any moment using
                the following QR code:
            </p>
            {{{data.qrcode}}}
            </div>
        "#
        .to_string(),
    );

    let mut data = VoteReceiptData {
        ballot_id: ballot_id.to_string(),
        ballot_tracker_url: ballot_tracker_url.to_string(),
        qrcode: QR_CODE_TEMPLATE.to_string(),
        logo: LOGO_TEMPLATE.to_string(),
        file_logo: file_logo.to_string(),
        file_qrcode_lib: file_qrcode_lib.to_string(),
        title: vote_receipt_title.to_string(),
        template: None, // TODO: remove this
    };
    let map = VoteReceiptRoot { data: data.clone() }.to_map()?;

    let template = reports::render_template_text(&template, map)?;

    data.template = Some(template.clone());

    let map = VoteReceiptRoot { data: data.clone() }.to_map()?;

    let render = reports::render_template_text(&template_hbs, map)?;

    dbg!(&render);

    // let custom_html_template = if data.template.is_some() {
    //     // include_str!("../resources/vote_receipt_custom.hbs")
    // } else {
    //     // include_str!("../resources/vote_receipt.hbs")
    // };
    //
    // let custom_html_template = "";
    //
    // let render = reports::render_template_text(custom_html_template, map)?;

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
