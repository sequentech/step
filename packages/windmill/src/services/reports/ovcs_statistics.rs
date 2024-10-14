// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::template_renderer::*;
use crate::postgres::reports::ReportType;
use crate::services::database::get_hasura_pool;
use crate::services::s3::get_minio_url;
use crate::services::temp_path::*;
use anyhow::{Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Client as DbClient;
use sequent_core::types::templates::EmailConfig;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

// Struct to hold user data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub election_start_date: String,
    pub election_title: String,
    pub geograpic_region: String,
    pub area: String,
    pub country: String,
    pub voting_center: String,
    pub num_of_rovs: u32,
    pub none_enrolled_rov: u32,
    pub enrolled_ov: u32,
    pub enrolled_did_not_vote: u32,
    pub enrolled_voted: u32,
    pub voted: u32,
    pub num_password_resets: u32,
    pub remarks: String,
    pub chairperson_name: String,
    pub poll_clerk_name: String,
    pub third_member_name: String,
}

// Struct to hold system data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub report_hash: String,
    pub ovcs_version: String,
    pub system_hash: String,
    pub file_logo: String,
    pub file_qrcode_lib: String,
    pub date_time_printed: String,
    pub printing_code: String,
}

#[derive(Debug)]
pub struct OVCSStatisticsTemplate {
    tenant_id: String,
    election_event_id: String,
}

#[async_trait]
impl TemplateRenderer for OVCSStatisticsTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type() -> ReportType {
        ReportType::OVCS_STATISTICS
    }

    fn base_name() -> String {
        "ovcs_statistics".to_string()
    }

    fn prefix(&self) -> String {
        format!("ovcs_statistics_{}", self.election_event_id)
    }

    fn get_tenant_id(&self) -> String {
        self.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.election_event_id.clone()
    }

    fn get_email_config() -> EmailConfig {
        EmailConfig {
            subject: "Sequent Online Voting - OVCS Statistics".to_string(),
            plaintext_body: "".to_string(),
            html_body: None,
        }
    }

    // TODO: replace mock data with actual data
    async fn prepare_user_data(&self) -> Result<Option<Self::UserData>> {
        // Fetch the Hasura database client from the pool
        let mut hasura_db_client: DbClient = get_hasura_pool()
            .await
            .get()
            .await
            .with_context(|| "Error getting hasura db pool")?;

        let hasura_transaction = hasura_db_client
            .transaction()
            .await
            .with_context(|| "Error starting hasura transaction")?;

        // Fetch the relevant statistics from the database (dummy values for now)
        let temp_val: &str = "test";
        let user_data = UserData {
            election_start_date: temp_val.to_string(),
            election_title: temp_val.to_string(),
            geograpic_region: "Asia".to_string(), // Replace with actual data
            area: "Region 1".to_string(),         // Replace with actual data
            country: "Philippines".to_string(),   // Replace with actual data
            voting_center: "Manila".to_string(),  // Replace with actual data
            num_of_rovs: 1000,
            none_enrolled_rov: 200,
            enrolled_ov: 800,
            enrolled_did_not_vote: 300,
            enrolled_voted: 500,
            voted: 400,
            num_password_resets: 50,
            remarks: "Smooth voting process".to_string(),
            chairperson_name: temp_val.to_string(),
            third_member_name: temp_val.to_string(),
            poll_clerk_name: temp_val.to_string(),
        };

        Ok(Some(user_data))
    }

    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        let public_asset_path = get_public_assets_path_env_var()?;
        let minio_endpoint_base =
            get_minio_url().with_context(|| "Error getting minio endpoint")?;

        Ok(SystemData {
            report_hash: String::new(),
            ovcs_version: String::new(),
            system_hash: String::new(),
            file_logo: String::new(),
            file_qrcode_lib: String::new(),
            date_time_printed: String::new(),
            printing_code: String::new(),
        })
    }
}
