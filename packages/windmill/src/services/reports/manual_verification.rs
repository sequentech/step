// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::template_renderer::*;
use crate::postgres::reports::{Report, ReportType};
use crate::services::temp_path::*;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use sequent_core::services::pdf;
use sequent_core::services::s3::get_minio_url;
use sequent_core::util::temp_path::*;
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{info, instrument};

/// Struct returned by the API call for manual verification URL
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManualVerificationOutput {
    pub link: String,
}

/// Struct for User Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub manual_verification_url: String,
    pub qrcode: String,
    pub logo: String,
}

/// Struct for System Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_logo: String,
    pub file_qrcode_lib: String,
}

/// Implementation of TemplateRenderer for Manual Verification
#[derive(Debug)]
pub struct ManualVerificationTemplate {
    ids: ReportOrigins,
}

impl ManualVerificationTemplate {
    pub fn new(ids: ReportOrigins) -> Self {
        ManualVerificationTemplate { ids }
    }
}

#[async_trait]
impl TemplateRenderer for ManualVerificationTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type(&self) -> ReportType {
        ReportType::MANUAL_VERIFICATION
    }
    fn get_tenant_id(&self) -> String {
        self.ids.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.ids.election_event_id.clone()
    }

    fn get_initial_template_alias(&self) -> Option<String> {
        self.ids.template_alias.clone()
    }

    fn get_report_origin(&self) -> ReportOriginatedFrom {
        self.ids.report_origin
    }

    fn get_voter_id(&self) -> Option<String> {
        self.ids.voter_id.clone()
    }

    fn base_name(&self) -> String {
        "manual_verification".to_string()
    }

    fn prefix(&self) -> String {
        format!(
            "manual_verification_{}",
            self.ids.voter_id.clone().unwrap_or_default()
        )
    }

    #[instrument(err, skip_all)]
    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        let manual_verification_url = get_manual_verification_url(
            &self.ids.tenant_id,
            &self.ids.election_event_id,
            &self.ids.voter_id.clone().unwrap_or_default(),
        )
        .await
        .with_context(|| "Error getting manual verification URL")?;

        Ok(UserData {
            manual_verification_url,
            qrcode: QR_CODE_TEMPLATE.to_string(),
            logo: LOGO_TEMPLATE.to_string(),
        })
    }

    #[instrument(err, skip_all)]
    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        let public_asset_path = get_public_assets_path_env_var()?;
        let minio_endpoint_base =
            get_minio_url().with_context(|| "Error getting minio endpoint")?;
        if pdf::doc_renderer_backend() == pdf::DocRendererBackend::InPlace {
            Ok(SystemData {
                rendered_user_template,
                file_logo: format!(
                    "{}/{}/{}",
                    minio_endpoint_base, public_asset_path, PUBLIC_ASSETS_LOGO_IMG
                ),
                file_qrcode_lib: format!(
                    "{}/{}/{}",
                    minio_endpoint_base, public_asset_path, PUBLIC_ASSETS_QRCODE_LIB
                ),
            })
        } else {
            // If we are rendering with a lambda, the QRCode lib is
            // already included in the lambda container image.
            Ok(SystemData {
                rendered_user_template,
                file_logo: format!(
                    "{}/{}/{}",
                    minio_endpoint_base, public_asset_path, PUBLIC_ASSETS_LOGO_IMG
                ),
                file_qrcode_lib: "/assets/qrcode.min.js".to_string(),
            })
        }
    }
}

/// Function to get the manual verification URL
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

    // Redirect to login
    let login_url = format!("{base_url}/tenant/{tenant_id}/event/{election_event_id}/login");

    let generate_token_url = format!(
        "{keycloak_url}/realms/tenant-{tenant_id}-event-{election_event_id}/manual-verification/generate-link?userId={voter_id}&redirectUri={login_url}"
    );

    let client = reqwest::Client::new();

    info!("Requesting HTTP GET {:?}", generate_token_url);
    let response = client.get(generate_token_url).send().await?;

    if response.status() != reqwest::StatusCode::OK {
        return Err(anyhow!("Error during generate_token_url"));
    }
    let response_body: ManualVerificationOutput = response.json().await?;

    Ok(response_body.link)
}
