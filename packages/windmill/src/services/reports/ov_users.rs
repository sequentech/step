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

/// Struct to represent each OV (Overseas Voter) user
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OVUserData {
    pub no: i32, // Table index
    pub first_name: String,
    pub last_name: String,
    pub middle_name: Option<String>,
    pub suffix: Option<String>,
    pub id: String,
    pub status: String,             // Voted/Not Voted/Not Enrolled
    pub date_voted: Option<String>, // Date when voted (Philippines time)
    pub time_voted: Option<String>, // Time when voted (Philippines time)
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
    pub total_not_enrolled: u32,
    pub total_eb_with_privilege: u32,
    pub total_ov: u32,
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

#[derive(Debug)]
pub struct OVUserTemplate {
    tenant_id: String,
    election_event_id: String,
}

#[async_trait]
impl TemplateRenderer for OVUserTemplate {
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
        "ov_users_information".to_string()
    }

    fn prefix(&self) -> String {
        format!("ov_users_information_{}", self.tenant_id)
    }

    fn get_email_config() -> EmailConfig {
        EmailConfig {
            subject: "Sequent Online Voting - OV Users".to_string(),
            plaintext_body: "".to_string(),
            html_body: None,
        }
    }

    // Fetch user data
    async fn prepare_user_data(&self) -> Result<Option<Self::UserData>> {
        // Mock OV user data
        let ov_users = vec![
            OVUserData {
                no: 1,
                first_name: "John".to_string(),
                last_name: "Doe".to_string(),
                middle_name: Some("M".to_string()),
                suffix: None,
                id: "OV123".to_string(),
                status: "Voted".to_string(),
                date_voted: Some("2024-10-08".to_string()),
                time_voted: Some("10:00:00".to_string()),
            },
            OVUserData {
                no: 2,
                first_name: "Jane".to_string(),
                last_name: "Smith".to_string(),
                middle_name: None,
                suffix: Some("Jr".to_string()),
                id: "OV124".to_string(),
                status: "Not Voted".to_string(),
                date_voted: None,
                time_voted: None,
            },
            OVUserData {
                no: 3,
                first_name: "Alice".to_string(),
                last_name: "Johnson".to_string(),
                middle_name: None,
                suffix: None,
                id: "OV125".to_string(),
                status: "Not Enrolled".to_string(),
                date_voted: None,
                time_voted: None,
            },
        ];

        // Calculate statistics
        let total_voted = ov_users.iter().filter(|ov| ov.status == "Voted").count() as u32;
        let total_not_voted = ov_users
            .iter()
            .filter(|ov| ov.status == "Not Voted")
            .count() as u32;
        let total_not_enrolled = ov_users
            .iter()
            .filter(|ov| ov.status == "Not Enrolled")
            .count() as u32;
        let total_eb_with_privilege = 2; // Mocking this value
        let total_ov = ov_users.len() as u32;

        // Mock UserData
        let temp_val: &str = "test";
        let user_data = UserData {
            election_start_date: "2024-10-01".to_string(),
            election_title: "National Elections 2024".to_string(),
            geograpic_region: "Asia".to_string(),
            area: "Southeast Asia".to_string(),
            country: "Philippines".to_string(),
            voting_center: "Manila".to_string(),
            total_voted,
            total_not_voted,
            total_not_enrolled,
            total_eb_with_privilege,
            total_ov,
            ov_users_who_voted: ov_users
                .into_iter()
                .filter(|ov| ov.status == "Voted")
                .collect(),
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
