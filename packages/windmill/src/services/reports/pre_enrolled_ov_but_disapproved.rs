use super::template_renderer::*;
use crate::services::database::get_hasura_pool;
use crate::services::s3::get_minio_url;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use crate::services::temp_path::*;
use deadpool_postgres::Client as DbClient;
use serde::{Deserialize, Serialize};
use rocket::http::Status;
use tracing::{info, instrument};
use sequent_core::types::templates::EmailConfig;

/// Struct for User Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DisapprovedOVData {
    pub no: u32,
    pub first_name: String,
    pub last_name: String,
    pub middle_name: Option<String>,
    pub suffix: Option<String>,
    pub date_disapproved: String,
    pub disapproved_by: String, // OFOV, SBEI, SYSTEM
    pub reason: String,
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
    pub pre_enrolled_users: Vec<DisapprovedOVData>
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
pub struct PreEnrolledDisapprovedTemplate {
    tenant_id: String,
    election_event_id: String,
}

#[async_trait]
impl TemplateRenderer for PreEnrolledDisapprovedTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type() -> ReportType {
        ReportType::PRE_ENROLLED_OV_BUT_DISAPPROVED
    }

    fn get_tenant_id(&self) -> String {
        self.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.election_event_id.clone()
    }

    fn base_name() -> String {
        "pre_enrolled_ov_disapproved".to_string()
    }

    fn prefix(&self) -> String {
        format!("pre_enrolled_disapproved_{}", self.election_event_id)
    }

    fn get_email_config() -> EmailConfig {
        EmailConfig {
            subject: "Sequent Online Voting - Pre Enrolled OV But Disapproved".to_string(),
            plaintext_body: "".to_string(),
            html_body: None,
        }
    }

    async fn prepare_user_data(&self) -> Result<Self::UserData> {
        // Mock data for pre_enrolled_users
        let mock_users = vec![
            DisapprovedOVData {
                no: 1,
                first_name: "Juan".to_string(),
                last_name: "Dela Cruz".to_string(),
                middle_name: Some("Santos".to_string()),
                suffix: None,
                date_disapproved: "2024-10-01T12:34:56".to_string(),
                disapproved_by: "OFOV".to_string(),
                reason: "Incomplete documents".to_string(),
            },
            DisapprovedOVData {
                no: 2,
                first_name: "Maria".to_string(),
                last_name: "Santiago".to_string(),
                middle_name: None,
                suffix: Some("Jr.".to_string()),
                date_disapproved: "2024-10-02T08:23:45".to_string(),
                disapproved_by: "SBEI".to_string(),
                reason: "Not eligible".to_string(),
            },
        ];

        Ok(UserData {
            election_start_date: "2024-09-30".to_string(),
            election_title: "2024 National Elections".to_string(),
            geograpic_region: "Luzon".to_string(),
            area: "Metro Manila".to_string(),
            country: "Philippines".to_string(),
            voting_center: "Voting Center 1".to_string(),
            pre_enrolled_users: mock_users,
        })
    }

    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        let public_asset_path = get_public_assets_path_env_var()?;
        let minio_endpoint_base = get_minio_url().with_context(|| "Error getting minio endpoint")?;

        Ok(SystemData {
            report_hash: String::new(),
            version: "1.0".to_string(),
            system_hash: String::new(),
            file_logo: String::new(),
            file_qrcode_lib: String::new(),
            date_time_printed: "2024-10-10T10:00:00".to_string(),
            printing_code: "ABC123".to_string(),
        })
    }
}