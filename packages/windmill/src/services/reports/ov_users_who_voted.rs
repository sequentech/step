// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::template_renderer::*;
use crate::postgres::reports::ReportType;
use crate::services::database::get_hasura_pool;
use anyhow::{Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Client as DbClient;
use sequent_core::types::templates::EmailConfig;
use serde::{Deserialize, Serialize};
use tracing::info;

/// Struct for OV User Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OVUserData {
    pub no: u32,
    pub first_name: String,
    pub last_name: String,
    pub middle_name: Option<String>,
    pub suffix: Option<String>,
    pub id: String,
    pub date_voted: String,
}

/// Struct for User Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub election_start_date: String,
    pub election_title: String,
    pub geograpic_region: String,
    pub area: String,
    pub country: String,
    pub voting_center: String,
    pub total_voted: u32,
    pub total_not_voted: u32,
    pub total_eb_with_privileges: u32,
    pub total_ov_users: u32,
    pub ov_users_who_voted: Vec<OVUserData>,
    pub chairperson_name: String,
    pub poll_clerk_name: String,
    pub third_member_name: String,
}

/// Struct for System Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub report_hash: String,
    pub version: String,
    pub system_hash: String,
    pub file_logo: String,
    pub file_qrcode_lib: String,
    pub date_time_printed: String,
    pub printing_code: String,
}

/// Struct for OVUsersWhoVotedTemplate
#[derive(Debug)]
pub struct OVUsersWhoVotedTemplate {
    tenant_id: String,
    election_event_id: String,
}

#[async_trait]
impl TemplateRenderer for OVUsersWhoVotedTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type() -> ReportType {
        ReportType::OV_USERS
    }

    fn get_tenant_id(&self) -> String {
        self.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.election_event_id.clone()
    }

    fn base_name() -> String {
        "ov_users_who_voted".to_string()
    }

    fn prefix(&self) -> String {
        format!("ov_users_who_voted_{}", self.election_event_id)
    }

    fn get_email_config() -> EmailConfig {
        EmailConfig {
            subject: "Sequent Online Voting - OV Users Who Voted".to_string(),
            plaintext_body: "".to_string(),
            html_body: None,
        }
    }

    // Prepare user data with statistics and mock data
    async fn prepare_user_data(&self) -> Result<Option<Self::UserData>> {
        // Fetch the Hasura database client from the pool
        let mut hasura_db_client: DbClient = get_hasura_pool()
            .await
            .get()
            .await
            .with_context(|| "Error getting hasura db pool")?;

        // Mock OVUsers data for now, can replace with actual database fetching later
        let mock_ov_users_who_voted = vec![
            OVUserData {
                no: 1,
                first_name: "Juan".to_string(),
                last_name: "Dela Cruz".to_string(),
                middle_name: Some("Garcia".to_string()),
                suffix: None,
                id: "OV12345".to_string(),
                date_voted: "2024-05-09 10:00:00".to_string(),
            },
            OVUserData {
                no: 2,
                first_name: "Maria".to_string(),
                last_name: "Santos".to_string(),
                middle_name: Some("Reyes".to_string()),
                suffix: Some("Jr.".to_string()),
                id: "OV67890".to_string(),
                date_voted: "2024-05-09 11:00:00".to_string(),
            },
        ];

        let temp_val: &str = "test";
        let user_data = UserData {
            election_start_date: "2024-05-01".to_string(), // Placeholder value, replace with real data
            election_title: "2024 National Elections".to_string(), // Placeholder value
            geograpic_region: "Asia Pacific".to_string(),  // Placeholder value
            area: "Metro Manila".to_string(),              // Placeholder value
            country: "Philippines".to_string(),            // Placeholder value
            voting_center: "Manila Voting Center".to_string(), // Placeholder value
            total_voted: 0,
            total_not_voted: 0,
            total_eb_with_privileges: 0,
            total_ov_users: 0,
            ov_users_who_voted: mock_ov_users_who_voted, // Using mock data for now
            chairperson_name: temp_val.to_string(),
            poll_clerk_name: temp_val.to_string(),
            third_member_name: temp_val.to_string(),
        };

        Ok(Some(user_data))
    }

    // Prepare system data
    async fn prepare_system_data(
        &self,
        _rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        // Placeholder system data, adjust based on your actual environment
        Ok(SystemData {
            report_hash: "abc123".to_string(),
            version: "1.0".to_string(),
            system_hash: "def456".to_string(),
            file_logo: "logo.png".to_string(),
            file_qrcode_lib: "qrcode.png".to_string(),
            date_time_printed: "2024-10-09 14:00:00".to_string(),
            printing_code: "PRT789".to_string(),
        })
    }
}
