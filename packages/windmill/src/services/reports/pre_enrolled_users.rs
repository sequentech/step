// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::template_renderer::*;
use crate::postgres::reports::ReportType;
use crate::services::database::get_hasura_pool;
use crate::services::s3::get_minio_url;
use anyhow::{anyhow, Context, Ok, Result};
use async_trait::async_trait;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use sequent_core::types::templates::EmailConfig;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

/// Struct for Pre-Enrolled User Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PreEnrolledUserData {
    pub no: u32,
    pub first_name: String,
    pub last_name: String,
    pub middle_name: Option<String>,
    pub suffix: Option<String>,
    pub id: String,
    pub status: String,            // Either "voted" or "not voted"
    pub date_pre_enrolled: String, // Assuming this is a string, format: YYYY-MM-DD
    pub time_pre_enrolled: String, // Assuming this is a string, format: HH:MM:SS
    pub approved_by: String,       // OFOV/SBEI/SYSTEM
}

/// Struct for OV Count Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub election_start_date: String,
    pub election_title: String,
    pub geograpic_region: String,
    pub area: String,
    pub country: String,
    pub voting_center: String,
    pub number_of_ovs_voted: u32,
    pub number_of_ovs_not_voted: u32,
    pub number_of_ovs_total: u32,
    pub number_of_ovs_approved_by: String, // OFOV/SBEI/SYSTEM
    pub pre_enrolled_users: Vec<PreEnrolledUserData>,
    pub chairperson_name: String,
    pub poll_clerk_name: String,
    pub third_member_name: String,
    pub report_hash: String,
    pub ovcs_version: String,
    pub system_hash: String,
    pub file_logo: String,
    pub file_qrcode_lib: String,
    pub date_time_printed: String,
    pub printing_code: String,
}

// Struct to hold system data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_qrcode_lib: String,
}

/// Implement the `TemplateRenderer` trait for PreEnrolledUserTemplate
#[derive(Debug)]
pub struct PreEnrolledUserTemplate {
    tenant_id: String,
    election_event_id: String,
    pre_enrolled_user_id: String,
}

#[async_trait]
impl TemplateRenderer for PreEnrolledUserTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type() -> ReportType {
        ReportType::PRE_ENROLLED_USERS
    }

    fn get_tenant_id(&self) -> String {
        self.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.election_event_id.clone()
    }

    fn base_name() -> String {
        "pre_enrolled_users".to_string()
    }

    fn prefix(&self) -> String {
        format!("pre_enrolled_user_{}", self.pre_enrolled_user_id)
    }

    fn get_email_config() -> EmailConfig {
        EmailConfig {
            subject: "Sequent Online Voting - Pre Enrolled Users".to_string(),
            plaintext_body: "".to_string(),
            html_body: None,
        }
    }

    // TODO: replace mock data with actual data
    // Fetch and prepare user data
    async fn prepare_user_data(&self) -> Result<Self::UserData> {
        // Mock data for pre_enrolled_users
        let pre_enrolled_users = vec![
            PreEnrolledUserData {
                no: 1,
                first_name: "John".to_string(),
                last_name: "Doe".to_string(),
                middle_name: Some("A".to_string()),
                suffix: None,
                id: "123456".to_string(),
                status: "voted".to_string(),
                date_pre_enrolled: "2024-01-01".to_string(),
                time_pre_enrolled: "08:00:00".to_string(),
                approved_by: "OFOV".to_string(),
            },
            PreEnrolledUserData {
                no: 2,
                first_name: "Jane".to_string(),
                last_name: "Smith".to_string(),
                middle_name: None,
                suffix: Some("Jr".to_string()),
                id: "7891011".to_string(),
                status: "not voted".to_string(),
                date_pre_enrolled: "2024-01-02".to_string(),
                time_pre_enrolled: "09:00:00".to_string(),
                approved_by: "SBEI".to_string(),
            },
            PreEnrolledUserData {
                no: 3,
                first_name: "Michael".to_string(),
                last_name: "Johnson".to_string(),
                middle_name: Some("B".to_string()),
                suffix: None,
                id: "987654".to_string(),
                status: "voted".to_string(),
                date_pre_enrolled: "2024-01-03".to_string(),
                time_pre_enrolled: "10:30:00".to_string(),
                approved_by: "SYSTEM".to_string(),
            },
        ];

        // Calculate the number of OVs who voted, didn't vote, and the total
        let number_of_ovs_voted = pre_enrolled_users
            .iter()
            .filter(|u| u.status == "voted")
            .count() as u32;
        let number_of_ovs_not_voted = pre_enrolled_users
            .iter()
            .filter(|u| u.status == "not voted")
            .count() as u32;
        let number_of_ovs_total = pre_enrolled_users.len() as u32;

        // Assuming "OFOV" approval is common, modify logic to fit your use case
        let number_of_ovs_approved_by = "OFOV".to_string();
        let temp_val: &str = "test";

        Ok(UserData {
            election_start_date: "2024-01-01".to_string(),
            election_title: "National Election 2024".to_string(),
            geograpic_region: "Region 1".to_string(),
            area: "Metro Area".to_string(),
            country: "Philippines".to_string(),
            voting_center: "Voting Center 1".to_string(),
            number_of_ovs_voted,
            number_of_ovs_not_voted,
            number_of_ovs_total,
            number_of_ovs_approved_by,
            pre_enrolled_users,
            chairperson_name: temp_val.to_string(),
            poll_clerk_name: temp_val.to_string(),
            third_member_name: temp_val.to_string(),
            report_hash: String::new(),
            ovcs_version: String::new(),
            system_hash: String::new(),
            file_logo: String::new(),
            file_qrcode_lib: String::new(),
            date_time_printed: String::new(),
            printing_code: String::new(),
        })
    }

    /// Prepare system metadata for the report
    #[instrument]
    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        let file_qrcode_lib: &str = "test";
        Ok(SystemData {
            rendered_user_template,
            file_qrcode_lib: file_qrcode_lib.to_string()
        })
    }
}
