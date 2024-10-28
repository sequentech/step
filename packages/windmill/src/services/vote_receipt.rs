// SPDX-FileCopyrightText: 2024 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::providers::transactions_provider::provide_hasura_transaction;
use super::s3;
use crate::postgres::reports::{get_template_id_for_report, ReportType};
use crate::postgres::template;
use crate::postgres::{self, election};
use crate::services::database::get_hasura_pool;
use crate::services::{documents::upload_and_return_document, temp_path::*};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::services::keycloak;
use sequent_core::services::{pdf, reports};
use sequent_core::types::date_time::{DateFormat, TimeZone};
use sequent_core::util::date_time::generate_timestamp;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tracing::instrument;
use uuid::Uuid;

enum TemplateType {
    Root,
    Content,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Receipt {
    pub allowed: bool,
    pub template: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReceiptsRoot {
    #[serde(rename = "SMS")]
    pub sms: Option<Receipt>,
    #[serde(rename = "EMAIL")]
    pub email: Option<Receipt>,
    #[serde(rename = "DOCUMENT")]
    pub document: Option<Receipt>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TemplateValue {
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
    let template_id = get_template_id_for_report(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &ReportType::BALLOT_RECEIPT,
        Some(election_id),
    )
    .await?
    .with_context(|| "Error getting vote receipt template id")?;

    let Some(template) =
        template::get_template_by_id(hasura_transaction, tenant_id, &template_id).await?
    else {
        return Ok(None);
    };

    let template_value: TemplateValue =
        deserialize_value(template.template).with_context(|| "Error parsing the template")?;

    Ok(Some(template_value.document))
}

async fn get_public_asset_vote_receipt_template(tpl_type: TemplateType) -> Result<String> {
    let public_assets_path = get_public_assets_path_env_var()?;

    let file_vote_receipt_template = match tpl_type {
        TemplateType::Root => "",    // PUBLIC_ASSETS_VOTE_RECEIPT_TEMPLATE,
        TemplateType::Content => "", //PUBLIC_ASSETS_VOTE_RECEIPT_TEMPLATE_CONTENT,
    };

    let minio_endpoint_base = s3::get_minio_url()?;
    let vote_receipt_template = format!(
        "{}/{}/{}",
        minio_endpoint_base, public_assets_path, file_vote_receipt_template
    );

    let client = reqwest::Client::new();
    let response = client.get(vote_receipt_template).send().await?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(anyhow!("File not found: {}", file_vote_receipt_template));
    }
    if !response.status().is_success() {
        return Err(anyhow!(
            "Unexpected response status: {:?}",
            response.status()
        ));
    }

    let template_hbs: String = response.text().await?;

    Ok(template_hbs)
}

async fn verify_ballot_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    area_id: &str,
    voter_id: &str,
    ballot_id_to_verify: &str,
) -> Result<()> {
    let cast_votes = postgres::cast_vote::get_cast_votes(
        hasura_transaction,
        &Uuid::parse_str(tenant_id)?,
        &Uuid::parse_str(election_event_id)?,
        &Uuid::parse_str(election_id)?,
        voter_id,
    )
    .await?;

    if cast_votes.iter().any(|cv| {
        cv.ballot_id
            .as_deref()
            .map_or(false, |id| id == ballot_id_to_verify)
            && cv.area_id.as_deref().map_or(false, |id| id == area_id)
    }) {
        Ok(())
    } else {
        Err(anyhow!("BallotID not found in cast votes for {voter_id}"))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VoteReceiptData {
    pub ballot_id: String,
    pub ballot_tracker_url: String,
    pub qrcode: String,
    pub logo: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VoteReceiptDataTemplate {
    pub template: Option<String>,
    pub title: String,
    pub file_logo: String,
    pub file_qrcode_lib: String,
    pub ballot_tracker_url: String,
    pub timestamp: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VoteReceiptRoot {
    pub data: VoteReceiptData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VoteReceiptRootTemplate {
    pub data: VoteReceiptDataTemplate,
}

trait ToMap {
    fn to_map(&self) -> Result<Map<String, Value>>;
}

impl ToMap for VoteReceiptRoot {
    fn to_map(&self) -> Result<Map<String, Value>> {
        let Value::Object(map) = serde_json::to_value(self.clone())? else {
            return Err(anyhow!("Can't convert VoteReceiptRoot to Map"));
        };

        Ok(map)
    }
}

impl ToMap for VoteReceiptRootTemplate {
    fn to_map(&self) -> Result<Map<String, Value>> {
        let Value::Object(map) = serde_json::to_value(self.clone())? else {
            return Err(anyhow!("Can't convert VoteReceiptRootTemplate to Map"));
        };

        Ok(map)
    }
}

#[instrument(err)]
pub async fn create_vote_receipt_task(
    element_id: String,
    ballot_id: String,
    ballot_tracker_url: String,
    tenant_id: String,
    election_event_id: String,
    election_id: String,
    area_id: String,
    voter_id: String,
    time_zone: Option<TimeZone>,
    date_format: Option<DateFormat>,
) -> Result<()> {
    provide_hasura_transaction(|hasura_transaction| {
        let element_id = element_id.clone();
        let tenant_id = tenant_id.clone();
        let election_event_id = election_event_id.clone();
        let election_id = election_id.clone();
        let area_id = area_id.clone();
        let voter_id = voter_id.clone();
        let ballot_id = ballot_id.clone();
        let ballot_tracker_url = ballot_tracker_url.clone();
        let time_zone = time_zone.clone();
        let date_format = date_format.clone();
        Box::pin(async move {
            // Your async code here
            create_vote_receipt(
                hasura_transaction,
                &element_id,
                &tenant_id,
                &election_event_id,
                &election_id,
                &area_id,
                &voter_id,
                &ballot_id,
                &ballot_tracker_url,
                time_zone,
                date_format,
            )
            .await
        })
    })
    .await?;

    Ok(())
}

#[instrument(skip(hasura_transaction), err)]
pub async fn create_vote_receipt(
    hasura_transaction: &Transaction<'_>,
    element_id: &str, // document_id actually
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    // The rest can be wrapped in an option
    // which will be None in the PREVIEW mode:
    area_id: &str,                   //Check
    voter_id: &str,                  //Check
    ballot_id: &str,                 //Check
    ballot_tracker_url: &str,        //Check
    time_zone: Option<TimeZone>,     //Check
    date_format: Option<DateFormat>, //Check
) -> Result<()> {
    verify_ballot_id(
        hasura_transaction,
        tenant_id,
        election_event_id,
        election_id,
        area_id,
        voter_id,
        ballot_id,
    )
    .await?;

    let public_assets_path = get_public_assets_path_env_var()?;

    let template_opt = get_template(
        hasura_transaction,
        tenant_id,
        election_event_id,
        election_id,
    )
    .await?;

    let template_hbs = get_public_asset_vote_receipt_template(TemplateType::Root).await?;

    let template = if template_opt.is_some() {
        template_opt.unwrap()
    } else {
        get_public_asset_vote_receipt_template(TemplateType::Content).await?
    };

    let minio_endpoint_base = s3::get_minio_url()?;

    let mut data = VoteReceiptData {
        ballot_id: ballot_id.to_string(),
        ballot_tracker_url: ballot_tracker_url.to_string(),
        qrcode: QR_CODE_TEMPLATE.to_string(),
        logo: LOGO_TEMPLATE.to_string(),
    };
    let map = VoteReceiptRoot { data: data.clone() }.to_map()?;

    let template = reports::render_template_text(&template, map)?;

    let map = VoteReceiptRootTemplate {
        data: VoteReceiptDataTemplate {
            template: Some(template),
            file_logo: format!(
                "{}/{}/{}",
                minio_endpoint_base, public_assets_path, PUBLIC_ASSETS_LOGO_IMG
            ),
            file_qrcode_lib: format!(
                "{}/{}/{}",
                minio_endpoint_base, public_assets_path, PUBLIC_ASSETS_QRCODE_LIB
            ),
            title: "".to_string(),
            ballot_tracker_url: ballot_tracker_url.to_string(),
            timestamp: generate_timestamp(time_zone, date_format, None),
        },
    }
    .to_map()?;

    let render = reports::render_template_text(&template_hbs, map)?;

    // Gen pdf
    let bytes_pdf = pdf::html_to_pdf(render, None).map_err(|err| anyhow!("{}", err))?;

    let (_temp_path, temp_path_string, file_size) =
        write_into_named_temp_file(&bytes_pdf, "vote-receipt-", ".pdf")
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
