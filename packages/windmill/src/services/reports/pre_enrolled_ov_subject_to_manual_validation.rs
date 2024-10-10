use super::template_renderer::*;
use crate::services::database::get_hasura_pool;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Client as DbClient;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use rocket::http::Status;
use sequent_core::types::templates::EmailConfig;

/// Struct for User Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PreEnrolledUserData {
    pub no: u32,
    pub first_name: String,
    pub last_name: String,
    pub middle_name: Option<String>,
    pub suffix: Option<String>,
    pub id: String,
    pub reason: Option<String>,
    pub date_pre_enrolled: String, // Philippines time, formatted as string
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
    pub pre_enrolled_users: Vec<PreEnrolledUserData>
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

/// Struct for PreEnrolledUsersRenderer
#[derive(Debug)]
pub struct PreEnrolledManualUsersTemplate {
    tenant_id: String,
    election_event_id: String,
}

#[async_trait]
impl TemplateRenderer for PreEnrolledManualUsersTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type() -> ReportType {
        ReportType::PRE_ENROLLED_OV_SUBJECT_TO_MANUAL_VALIDATION
    }

    fn get_tenant_id(&self) -> String {
        self.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.election_event_id.clone()
    }

    fn base_name() -> String {
        "pre_enrolled_ov_subject_to_manual_validation".to_string()
    }

    fn prefix(&self) -> String {
        format!("pre_enrolled_ov_{}", self.election_event_id)
    }

    fn get_email_config() -> EmailConfig {
        EmailConfig {
            subject: "Sequent Online Voting - Pre Enrolled OV Subject To Manual Validation".to_string(),
            plaintext_body: "".to_string(),
            html_body: None,
        }
    }

    /// Fetches pre-enrolled users and prepares user data
    #[instrument]
    async fn prepare_user_data(&self) -> Result<Self::UserData> {
        // Mock pre-enrolled users
        let pre_enrolled_users = vec![
            PreEnrolledUserData {
                no: 1,
                first_name: "Juan".to_string(),
                last_name: "Dela Cruz".to_string(),
                middle_name: Some("Santos".to_string()),
                suffix: None,
                id: "ID001".to_string(),
                reason: Some("Manual validation required for ID mismatch".to_string()),
                date_pre_enrolled: "2024-10-09 14:30:00".to_string(), // Example Philippines time
            },
            PreEnrolledUserData {
                no: 2,
                first_name: "Maria".to_string(),
                last_name: "Clara".to_string(),
                middle_name: None,
                suffix: Some("Jr.".to_string()),
                id: "ID002".to_string(),
                reason: Some("Pending document review".to_string()),
                date_pre_enrolled: "2024-10-09 15:00:00".to_string(),
            },
            PreEnrolledUserData {
                no: 3,
                first_name: "Jose".to_string(),
                last_name: "Rizal".to_string(),
                middle_name: Some("Protacio".to_string()),
                suffix: Some("III".to_string()),
                id: "ID003".to_string(),
                reason: Some("Incomplete application".to_string()),
                date_pre_enrolled: "2024-10-09 16:45:00".to_string(),
            },
        ];

        Ok(UserData {
            election_start_date: "2024-11-01".to_string(),
            election_title: "National Elections".to_string(),
            geograpic_region: "Luzon".to_string(),
            area: "Metro Manila".to_string(),
            country: "Philippines".to_string(),
            voting_center: "Quezon City Voting Center".to_string(),
            pre_enrolled_users,
        })
    }

    /// Prepares system data
    #[instrument]
    async fn prepare_system_data(
        &self,
        _rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        // Mock system-related data
        Ok(SystemData {
            report_hash: "mock_report_hash_123".to_string(),
            ovcs_version: "v1.0".to_string(),
            system_hash: "mock_system_hash_456".to_string(),
            file_logo: "path/to/mock_logo.png".to_string(),
            file_qrcode_lib: "path/to/mock_qrcode.png".to_string(),
            date_time_printed: chrono::Utc::now().to_rfc3339(), // Example current timestamp
            printing_code: "mock_printing_code_789".to_string(),
        })
    }
}