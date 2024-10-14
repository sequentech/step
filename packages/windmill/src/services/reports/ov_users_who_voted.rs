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

/// Struct for User Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
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
    pub voting_privilege_voted: u32,
    pub total: u32,
    pub report_hash: String,
    pub software_version: String,
    pub ovcs_version: String,
    pub system_hash: String,
    pub qr_code: String,
}

/// Struct for each voter
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Voter {
    pub number: u32,         // Voter number
    pub last_name: String,   // Last name
    pub first_name: String,  // First name
    pub middle_name: String, // Middle name
    pub suffix: String,      // Suffix (if any)
    pub id: String,          // Voter ID
    pub date_voted: String,  // Date the voter voted
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
    // Prepare system data
    async fn prepare_system_data(
        &self,
        _rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        // Placeholder system data, adjust based on your actual environment
        // Fetch the Hasura database client from the pool
        let mut hasura_db_client: DbClient = get_hasura_pool()
            .await
            .get()
            .await
            .with_context(|| "Error getting hasura db pool")?;

        // Mock OVUsers data for now, can replace with actual database fetching later
        let voters = vec![
            Voter {
                number: 1,
                first_name: "Juan".to_string(),
                last_name: "Dela Cruz".to_string(),
                middle_name: "Garcia".to_string(),
                suffix: "".to_string(),
                id: "OV12345".to_string(),
                date_voted: "2024-05-09".to_string(),
            },
            Voter {
                number: 2,
                first_name: "Maria".to_string(),
                last_name: "Santos".to_string(),
                middle_name: "Reyes".to_string(),
                suffix: "Jr.".to_string(),
                id: "OV67890".to_string(),
                date_voted: "2024-05-09".to_string(),
            },
        ];

        let temp_val: &str = "test";

        Ok(SystemData {
            election_date: "2024-05-01".to_string(),
            election_title: "2024 National Elections".to_string(),
            post: "Metro Manila".to_string(),
            country: "Philippines".to_string(),
            voting_period: temp_val.to_string(),
            voted: 0,
            not_voted: 0,
            voters,
            voting_privilege_voted: 0,
            total: 0,
            report_hash: "abc123".to_string(),
            ovcs_version: "1.0".to_string(),
            system_hash: "def456".to_string(),
            date_printed: "2024-10-09 14:00:00".to_string(),
            time_printed: "2024-10-09 14:00:00".to_string(),
            software_version: String::new(),
            qr_code: "code1".to_string(),
        })
    }
}

#[instrument]
pub async fn generate_ov_users_who_voted_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    mode: GenerateReportMode,
) -> Result<()> {
    let template = OVUsersWhoVotedTemplate {
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
    };
    template
        .execute_report(
            document_id,
            tenant_id,
            election_event_id,
            false,
            None,
            None,
            mode,
        )
        .await
}
