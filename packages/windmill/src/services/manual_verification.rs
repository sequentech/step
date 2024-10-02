// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2024 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::s3::get_minio_url;
use crate::postgres::{election_event, template};
use crate::services::database::get_hasura_pool;
use crate::services::{documents::upload_and_return_document, temp_path::*};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use sequent_core::services::keycloak;
use sequent_core::services::{pdf, reports};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::env;
use tracing::{info, instrument, warn};

/// Struct returned by the API call
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManualVerificationOutput {
    pub link: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemTemplateData {
    pub rendered_user_template: String,
    pub manual_verification_url: String,
    pub file_logo: String,
    pub file_qrcode_lib: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserTemplateData {
    pub manual_verification_url: String,
    pub qrcode: String,
    pub logo: String,
}

trait ToMap {
    fn to_map(&self) -> Result<Map<String, Value>>;
}

impl ToMap for SystemTemplateData {
    fn to_map(&self) -> Result<Map<String, Value>> {
        let Value::Object(map) = serde_json::to_value(self.clone())? else {
            return Err(anyhow!("Can't convert SystemTemplateData to Map"));
        };
        Ok(map)
    }
}

impl ToMap for UserTemplateData {
    fn to_map(&self) -> Result<Map<String, Value>> {
        let Value::Object(map) = serde_json::to_value(self.clone())? else {
            return Err(anyhow!("Can't convert UserTemplateData to Map"));
        };
        Ok(map)
    }
}

#[derive(Debug)]
enum TemplateType {
    System,
    User,
}

#[instrument(err)]
async fn get_public_asset_manual_verification_template(tpl_type: TemplateType) -> Result<String> {
    let public_asset_path = get_public_assets_path_env_var()?;
    let file_manual_verification_template = match tpl_type {
        TemplateType::System => PUBLIC_ASSETS_MANUAL_VERIFICATION_SYSTEM_TEMPLATE,
        TemplateType::User => PUBLIC_ASSETS_MANUAL_VERIFICATION_USER_TEMPLATE,
    };

    let minio_endpoint_base = get_minio_url().with_context(|| "Error getting minio endpoint")?;

    let manual_verification_template = format!(
        "{}/{}/{}",
        minio_endpoint_base, public_asset_path, file_manual_verification_template
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&manual_verification_template)
        .send()
        .await
        .with_context(|| "Error getting/send request for manual verification template")?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(anyhow!("File not found: {}", manual_verification_template));
    }
    if !response.status().is_success() {
        return Err(anyhow!(
            "Unexpected response status: {:?}",
            response.status()
        ));
    }

    let template_hbs: String = response
        .text()
        .await
        .with_context(|| "Error reading the manual verification template response")?;

    Ok(template_hbs)
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

    info!("Requesting HTTP GET {:?}", generate_token_url);
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
pub async fn get_custom_user_template(
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Option<String>> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .with_context(|| "Error getting hasura db pool")?;

    let transaction = hasura_db_client
        .transaction()
        .await
        .with_context(|| "Error starting hasura transaction")?;

    let election_event =
        election_event::get_election_event_by_id(&transaction, tenant_id, election_event_id)
            .await
            .with_context(|| "Error to get the election event by id")?;

    let presentation = match election_event.presentation {
        Some(val) => val,
        None => return Err(anyhow!("Election event has no presentation")),
    };

    let active_template_ids = match presentation.get("active_template_ids") {
        Some(val) => val,
        None => {
            warn!("No active_template_ids in presentation");
            return Ok(None);
        }
    };
    info!("active_template_ids: {active_template_ids}");

    let usr_verfication_tpl_id = match active_template_ids
        .get("manual_verification")
        .and_then(Value::as_str)
    {
        Some(id) if !id.is_empty() => id.to_string(),
        _ => {
            info!("manual_verification id not found or empty");
            return Ok(None);
        }
    };
    info!("usr_verfication_tpl_id: {usr_verfication_tpl_id}");

    // Get the template by ID and return its value:
    let template_data_opt =
        template::get_template_by_id(&transaction, tenant_id, &usr_verfication_tpl_id)
            .await
            .with_context(|| "Error to get template by id")?;

    let tpl_document: Option<&str> = match &template_data_opt {
        Some(template_data) => template_data
            .template
            .get("document")
            .and_then(Value::as_str),
        None => {
            warn!("No manual verification template was found by id, perhaps it was deleted");
            return Ok(None);
        }
    };

    match tpl_document {
        Some(document) if !document.is_empty() => Ok(Some(document.to_string())),
        _ => Ok(None),
    }
}

#[instrument(err)]
pub async fn get_manual_verification_pdf(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    voter_id: &str,
) -> Result<()> {
    let public_asset_path = get_public_assets_path_env_var()?;
    let manual_verification_url =
        get_manual_verification_url(tenant_id, election_event_id, voter_id)
            .await
            .with_context(|| "Error getting manual verification url")?;

    let minio_endpoint_base = get_minio_url().with_context(|| "Error getting minio endpoint")?;

    let user_template_data = UserTemplateData {
        manual_verification_url: manual_verification_url.to_string(),
        qrcode: QR_CODE_TEMPLATE.to_string(),
        logo: LOGO_TEMPLATE.to_string(),
    }
    .to_map()?;

    let custom_user_template: Option<String> =
        get_custom_user_template(tenant_id, election_event_id)
            .await
            .with_context(|| "Error getting custom user template")?;

    let user_template = match custom_user_template {
        Some(template) => {
            info!("Found a custom user template for manual verification!");
            template
        }
        None => {
            info!("Setting default user template for manual verification!");
            get_public_asset_manual_verification_template(TemplateType::User)
                .await
                .map_err(|e| anyhow!("Error getting default user template: {e}"))?
        }
    };
    info!("user template: {user_template:?}");

    let rendered_user_template = reports::render_template_text(&user_template, user_template_data)
        .with_context(|| "Error rendering user template")?;
    info!("rendered user template: {rendered_user_template:?}");

    let system_template_data = SystemTemplateData {
        rendered_user_template,
        manual_verification_url: manual_verification_url.to_string(),
        file_logo: format!(
            "{}/{}/{}",
            minio_endpoint_base, public_asset_path, PUBLIC_ASSETS_LOGO_IMG
        ),
        file_qrcode_lib: format!(
            "{}/{}/{}",
            minio_endpoint_base, public_asset_path, PUBLIC_ASSETS_QRCODE_LIB
        ),
    }
    .to_map()
    .with_context(|| "Error converting to map")?;

    let system_template = get_public_asset_manual_verification_template(TemplateType::System)
        .await
        .with_context(|| "Error getting default system template")?;
    info!("system template: {system_template:?}");
    let rendered_system_template =
        reports::render_template_text(&system_template, system_template_data)
            .with_context(|| "Error rendering template")?;
    info!("rendered system template: {rendered_system_template:?}");

    // Gen pdf
    let bytes_pdf = pdf::html_to_pdf(rendered_system_template)
        .map_err(|err| anyhow!("error rendering manual verification pdf: {}", err))?;
    let (_temp_path, temp_path_string, file_size) =
        write_into_named_temp_file(&bytes_pdf, "manual-verification-", ".pdf")
            .with_context(|| "Error writing to file")?;

    let auth_headers = keycloak::get_client_credentials()
        .await
        .with_context(|| "Error getting client credentials")?;
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
    .await
    .with_context(|| "Error uploading document")?;

    Ok(())
}
