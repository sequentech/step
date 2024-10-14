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
use tracing::{info, instrument};

/// Struct to represent each OV (Overseas Voter) user
// #[derive(Serialize, Deserialize, Debug, Clone)]
// pub struct OVUserData {
//     pub no: i32, // Table index
//     pub first_name: String,
//     pub last_name: String,
//     pub middle_name: Option<String>,
//     pub suffix: Option<String>,
//     pub id: String,
//     pub status: String,             // Voted/Not Voted/Not Enrolled
//     pub date_voted: Option<String>, // Date when voted (Philippines time)
//     pub time_voted: Option<String>, // Time when voted (Philippines time)
// }

/// Struct for User Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    // pub election_start_date: String,
    // pub election_title: String,
    // pub geograpic_region: String,
    // pub area: String,
    // pub country: String,
    // pub voting_center: String,
    // pub total_voted: u32,
    // pub total_not_voted: u32,
    // pub total_not_enrolled: u32,
    // pub total_eb_with_privilege: u32,
    // pub total_ov: u32,
    // pub ov_users_who_voted: Vec<OVUserData>,
    // pub chairperson_name: String,
    // pub poll_clerk_name: String,
    // pub third_member_name: String,
}

/// Struct for System Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Voter {
    pub number: u32,
    pub last_name: String,
    pub first_name: String,
    pub middle_name: String,
    pub suffix: String,
    pub id: String,
    pub status: String,
    pub date_voted: String,
    pub time_voted: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub report_hash: String,
    pub version: String,
    pub system_hash: String,
    pub file_logo: String,
    pub file_qrcode_lib: String,
    pub printing_code: String,
    pub date_printed: String,
    pub time_printed: String,
    pub election_date: String,
    pub election_title: String,
    pub voting_period: String,
    pub post: String,
    pub country: String,
    pub voters: Vec<Voter>,
    pub voted: u32,
    pub not_voted: u32,
    pub not_pre_enrolled: u32,
    pub voting_privilege_voted: u32,
    pub total: u32,
    pub ovcs_version: String,
    pub qr_code: String,
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

    // Prepare system data
    async fn prepare_system_data(
        &self,
        _rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        // Placeholder system data, adjust based on your actual environment
        // Mock OV user data
        let voters = vec![
            Voter {
                number: 1,
                last_name: "Anderson".to_string(),
                first_name: "Paul".to_string(),
                middle_name: "M.".to_string(),
                suffix: "".to_string(),
                id: "NYC12345".to_string(),
                status: "Voted".to_string(),
                date_voted: "May 2, 2024".to_string(),
                time_voted: "08:45".to_string(),
            },
            Voter {
                number: 2,
                last_name: "Garcia".to_string(),
                first_name: "Maria".to_string(),
                middle_name: "L.".to_string(),
                suffix: "".to_string(),
                id: "NYC12346".to_string(),
                status: "Voted".to_string(),
                date_voted: "May 3, 2024".to_string(),
                time_voted: "09:30".to_string(),
            },
            Voter {
                number: 3,
                last_name: "Johnson".to_string(),
                first_name: "Michael".to_string(),
                middle_name: "T.".to_string(),
                suffix: "".to_string(),
                id: "NYC12347".to_string(),
                status: "Voted".to_string(),
                date_voted: "May 4, 2024".to_string(),
                time_voted: "10:15".to_string(),
            },
            Voter {
                number: 4,
                last_name: "Lee".to_string(),
                first_name: "Sophie".to_string(),
                middle_name: "K.".to_string(),
                suffix: "".to_string(),
                id: "NYC12348".to_string(),
                status: "Voted".to_string(),
                date_voted: "May 4, 2024".to_string(),
                time_voted: "11:00".to_string(),
            }
        ];
    
        // Calculate statistics
        let total_voted = voters.iter().filter(|ov| ov.status == "Voted").count() as u32;
        let total_not_voted = voters
            .iter()
            .filter(|ov| ov.status == "Not Voted")
            .count() as u32;
        let not_pre_enrolled = voters
            .iter()
            .filter(|ov| ov.status == "Not Enrolled")
            .count() as u32;
        let voting_privilege_voted = 2; // Mocking this value
        let total = voters.len() as u32;
    
        // Mock UserData
        let temp_val: &str = "test";

        Ok(SystemData {
            election_date: "2024-10-01".to_string(),
            election_title: "National Elections 2024".to_string(),
            post: "Southeast Asia".to_string(),
            country: "Philippines".to_string(),
            voting_period: temp_val.to_string(),
            voted: 0,
            not_voted: 0,
            voters,
            not_pre_enrolled,
            voting_privilege_voted,
            total,
            report_hash: "abc123".to_string(),
            version: "1.0".to_string(),
            system_hash: "def456".to_string(),
            file_logo: "logo.png".to_string(),
            file_qrcode_lib: "qrcode.png".to_string(),
            date_printed: "2024-10-09 14:00:00".to_string(),
            time_printed: "2024-10-09 14:00:00".to_string(),
            printing_code: "PRT789".to_string(),
            ovcs_version: String::new(),
            qr_code: String::new(),
        })
    }
}

#[instrument]
pub async fn generate_ov_users_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    mode: GenerateReportMode,
) -> Result<()> {
    let template = OVUserTemplate {
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
    };
    template
        .execute_report(document_id, tenant_id, election_event_id, false, None, mode)
        .await
}