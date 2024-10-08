// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::template_renderer::*;
use crate::services::database::get_hasura_pool;
use crate::{postgres::election_event::get_election_event_by_id, services::s3::get_minio_url};
use crate::{postgres::scheduled_event::find_scheduled_event_by_election_event_id_and_event_processor};
use crate::services::temp_path::*;
use anyhow::{anyhow, Context, Ok, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{info, instrument};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;


/// Struct for User Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub election_start_date: String,
    pub election_title: String,
    pub geograpic_region: String,
    pub area: String,
    pub country: String,
    pub voating_center: String,
    pub num_of_registered_voters: u32,
}

/// Struct for System Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub report_hash: String,
    pub ovsc_version: String,
    pub system_hash: String,
    pub file_logo: String,
    pub file_qrcode_lib: String,
    pub date_time_printed: String,
    pub printing_code: String,
}

#[derive(Debug)]
pub struct OVCSInformaitionTemplate {
    tenant_id: String,
    election_event_id: String,
    voter_id: String,
}


#[async_trait]
impl TemplateRenderer for OVCSInformaitionTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_tenant_id(&self) -> String {
        self.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.election_event_id.clone()
    }

    fn base_name() -> String {
        "ovcs_information".to_string()
    }

    fn prefix(&self) -> String {
        format!("ovcs_information_{}", self.voter_id)
    }

    async fn prepare_user_data(&self) -> Result<Self::UserData> {
        // Fetch the Hasura database client from the pool
        let hasura_db_client = get_hasura_pool()
            .await
            .get()
            .await
            .map_err(|e| (Status::InternalServerError, format!("Failed to get DB client: {:?}", e)))?;

        // Start a transaction
        let hasura_transaction = hasura_db_client
            .transaction()
            .await
            .map_err(|e| (Status::InternalServerError, format!("Failed to start transaction: {:?}", e)))?;

        // Fetch election event data
        let election_event = get_election_event_by_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id
        )
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

        // Fetch election event data
        let start_election_event = find_scheduled_event_by_election_event_id_and_event_processor(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
            "START_ELECTION"
        )
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

        let mut election_start_date: String;
        if let Some(cron_config) = start_election_event.get(0).and_then(|event| event.cron_config.clone()) {
            // Now cron_config is a CronConfig, not an Option
            if let Some(scheduled_date) = cron_config.scheduled_date {
                election_start_date = scheduled_date;
            } 
            // else {
            //     return Err((Status::InternalServerError, "Scheduled date not found".to_string()));
            // }
        } 
        // else {
        //     return Err((Status::InternalServerError, "Election start event or cron config not found".to_string()));
        // }

        let temp_val = "test";
        Ok(UserData{
            election_start_date: election_start_date,
            election_title: election_event.name.clone(),
            geograpic_region: temp_val.to_string(),
            area: temp_val.to_string(),
            country: temp_val.to_string(),
            voating_center: temp_val.to_string(),
            num_of_registered_voters: 0,
        })
    }



    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        let public_asset_path = get_public_assets_path_env_var()?;
        let minio_endpoint_base =
            get_minio_url().with_context(|| "Error getting minio endpoint")?;

        let manual_verification_url =
            get_manual_verification_url(&self.tenant_id, &self.election_event_id, &self.voter_id)
                .await
                .with_context(|| "Error getting manual verification URL")?;

        Ok(SystemData {
            rendered_user_template,
            manual_verification_url,
            file_logo: format!(
                "{}/{}/{}",
                minio_endpoint_base, public_asset_path, PUBLIC_ASSETS_LOGO_IMG
            ),
            file_qrcode_lib: format!(
                "{}/{}/{}",
                minio_endpoint_base, public_asset_path, PUBLIC_ASSETS_QRCODE_LIB
            ),
        })
    }
}

/// Function to generate the manual verification report using the TemplateRenderer
#[instrument(err)]
pub async fn generate_manual_verification_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    voter_id: &str,
) -> Result<()> {
    let template = ManualVerificationTemplate {
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        voter_id: voter_id.to_string(),
    };
    template
        .execute_report(document_id, tenant_id, election_event_id)
        .await
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
