// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2024 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::env;

use super::s3;
use crate::postgres::{self, communication_template, election};
use crate::services::database::get_hasura_pool;
use crate::services::{
    documents::upload_and_return_document, temp_path::write_into_named_temp_file,
};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::services::keycloak;
use sequent_core::services::{pdf, reports};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tracing::{event, instrument, Level};
use uuid::Uuid;

const QR_CODE_TEMPLATE: &'static str = "<div id=\"qrcode\"></div>";
const LOGO_TEMPLATE: &'static str = "<div class=\"logo\"></div>";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManualVerificationOutput {
    pub link: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManualVerificationData {
    pub manual_verification_url: String,
    pub qrcode: String,
    pub logo: String,
    pub file_logo: String,
    pub file_qrcode_lib: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManualVerificationRoot {
    pub data: ManualVerificationData,
}

trait ToMap {
    fn to_map(&self) -> Result<Map<String, Value>>;
}

impl ToMap for ManualVerificationRoot {
    fn to_map(&self) -> Result<Map<String, Value>> {
        let Value::Object(map) = serde_json::to_value(self.clone())? else {
            return Err(anyhow!("Can't convert ManualVerificationRoot to Map"));
        };
        Ok(map)
    }
}

impl ToMap for ManualVerificationData {
    fn to_map(&self) -> Result<Map<String, Value>> {
        let Value::Object(map) = serde_json::to_value(self.clone())? else {
            return Err(anyhow!("Can't convert ManualVerificationData to Map"));
        };
        Ok(map)
    }
}

enum TemplateType {
    Root,
    Content,
}

async fn get_public_asset_manual_verification_template(tpl_type: TemplateType) -> Result<String> {
    let public_asset_path = env::var("PUBLIC_ASSETS_PATH")?;

    let file_manual_verification_template = match tpl_type {
        TemplateType::Root => env::var("PUBLIC_ASSETS_MANUAL_VERIFICATION_TEMPLATE")?,
        TemplateType::Content => env::var("PUBLIC_ASSETS_MANUAL_VERIFICATION_TEMPLATE_CONTENT")?,
    };

    let minio_endpoint_base = s3::get_minio_url()?;
    let manual_verification_template = format!(
        "{}/{}/{}",
        minio_endpoint_base, public_asset_path, file_manual_verification_template
    );

    let client = reqwest::Client::new();
    let response = client.get(manual_verification_template).send().await?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(anyhow!(
            "File not found: {}",
            file_manual_verification_template
        ));
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

fn get_minio_url() -> Result<String> {
    let minio_private_uri =
        env::var("AWS_S3_PRIVATE_URI").map_err(|err| anyhow!("AWS_S3_PRIVATE_URI must be set"))?;
    let bucket = s3::get_public_bucket()?;

    Ok(format!("{}/{}", minio_private_uri, bucket))
}

#[instrument(err)]
async fn get_manual_verification_url(
    tenant_id: &str,
    election_event_id: &str,
    voter_id: &str,
) -> Result<String> {
    let keycloak_url =
        env::var("KEYCLOAK_URL").map_err(|_| anyhow!("KEYCLOAK_URL env var missing"))?;
    let base_url = std::env::var("VOTING_PORTAL_URL")
        .map_err(|_| anyhow!("VOTING_PORTAL_URL env var missing"))?;

    // redirect to login
    let login_url = format!("{base_url}/tenant/{tenant_id}/event/{election_event_id}/login");

    let generate_token_url = format!(
    "{keycloak_url}/realms/tenant-{tenant_id}-event-{election_event_id}/manual-verification/generate-link?userId={voter_id}&redirectUri={login_url}"
  );

    let client = reqwest::Client::new();

    event!(Level::INFO, "Requesting HTTP GET {:?}", generate_token_url);
    let response = client.get(generate_token_url).send().await?;

    let unwrapped_response = if response.status() != reqwest::StatusCode::OK {
        return Err(anyhow!("Error during generate_token_url"));
    } else {
        response
    };
    let response_body: ManualVerificationOutput = unwrapped_response.json().await?;

    Ok(response_body.link)
}

#[instrument(err)]
pub async fn get_manual_verification_pdf(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    voter_id: &str,
) -> Result<()> {
    let public_asset_path = env::var("PUBLIC_ASSETS_PATH")?;
    let file_logo = env::var("PUBLIC_ASSETS_LOGO_IMG")?;
    let file_qrcode_lib = env::var("PUBLIC_ASSETS_QRCODE_LIB")?;
    let manual_verification_url =
        get_manual_verification_url(tenant_id, election_event_id, voter_id).await?;

    let minio_endpoint_base = get_minio_url()?;

    let data = ManualVerificationData {
        manual_verification_url: manual_verification_url.to_string(),
        qrcode: QR_CODE_TEMPLATE.to_string(),
        logo: LOGO_TEMPLATE.to_string(),
        file_logo: format!(
            "{}/{}/{}",
            minio_endpoint_base, public_asset_path, file_logo
        ),
        file_qrcode_lib: format!(
            "{}/{}/{}",
            minio_endpoint_base, public_asset_path, file_qrcode_lib
        ),
    };

    // TODO: make it configurable per election event, like vote_receipt.rs
    let template = get_public_asset_manual_verification_template(TemplateType::Root).await?;

    let map = ManualVerificationRoot { data: data.clone() }.to_map()?;
    let render = reports::render_template_text(&template, map)?;

    // Gen pdf
    let bytes_pdf = pdf::html_to_pdf(render)
        .map_err(|err| anyhow!("error rendering manual verification pdf: {}", err))?;
    let (_temp_path, temp_path_string, file_size) =
        write_into_named_temp_file(&bytes_pdf, "manual-verification-", ".pdf")
            .with_context(|| "Error writing to file")?;

    let auth_headers = keycloak::get_client_credentials().await?;
    let _document = upload_and_return_document(
        temp_path_string,
        file_size,
        "application/pdf".to_string(),
        auth_headers.clone(),
        tenant_id.to_string(),
        election_event_id.to_string(),
        format!("manual-verification-{voter_id}.pdf"),
        Some(document_id.to_string()),
        true,
    )
    .await?;

    Ok(())
}
