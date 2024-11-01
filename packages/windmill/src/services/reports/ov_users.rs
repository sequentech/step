// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{extract_election_data, get_app_hash, get_app_version};
use super::template_renderer::*;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::postgres::{election::get_election_by_id, reports::ReportType};
use crate::services::database::get_hasura_pool;
use anyhow::{Context, Result};
use async_trait::async_trait;
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::scheduled_event::generate_voting_period_dates;
use sequent_core::types::templates::EmailConfig;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

/// Struct for User Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub report_hash: String,
    pub system_hash: String,
    pub date_printed: String,
    pub election_date: String,
    pub election_title: String,
    pub voting_period_start: String,
    pub voting_period_end: String,
    pub post: String,
    pub area_id: String,
    pub voters: Vec<Voter>,
    pub voted: u32,
    pub not_voted: u32,
    pub not_pre_enrolled: u32,
    pub voting_privilege_voted: u32,
    pub total: u32,
    pub ovcs_version: String,
    pub qr_code: String,
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
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_qrcode_lib: String,
}

#[derive(Debug)]
pub struct OVUserTemplate {
    tenant_id: String,
    election_event_id: String,
    election_id: String,
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

    fn get_election_id(&self) -> Option<String> {
        Some(self.election_id.clone())
    }

    fn base_name() -> String {
        "ov_users".to_string()
    }

    fn prefix(&self) -> String {
        format!("ov_users_{}", self.tenant_id)
    }

    fn get_email_config() -> EmailConfig {
        EmailConfig {
            subject: "Sequent Online Voting - OV Users".to_string(),
            plaintext_body: "".to_string(),
            html_body: None,
        }
    }

    #[instrument(err, skip(self, hasura_transaction, keycloak_transaction))]
    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        let election = match get_election_by_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
            &self.election_id,
        )
        .await
        .with_context(|| "Error getting election by id")?
        {
            Some(election) => election,
            None => return Err(anyhow::anyhow!("Election not found")),
        };

        // get election instace's general data (post, area, etc...)
        let election_general_data = match extract_election_data(&election).await {
            Ok(data) => data, // Extracting the ElectionData struct out of Ok
            Err(err) => {
                return Err(anyhow::anyhow!(format!(
                    "Error fetching election data: {}",
                    err
                )));
            }
        };

        // Fetch election event data
        let start_election_event = find_scheduled_event_by_election_event_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
        )
        .await
        .map_err(|e| {
            anyhow::anyhow!("Error getting scheduled event by election event_id: {}", e)
        })?;

        // Fetch election's voting periods
        let voting_period_dates = generate_voting_period_dates(
            start_election_event,
            &self.tenant_id,
            &self.election_event_id,
            Some(&self.election_id),
        )?;

        // extract start date from voting period
        let voting_period_start_date = voting_period_dates.start_date.unwrap_or_default();
        // extract end date from voting period
        let voting_period_end_date = voting_period_dates.end_date.unwrap_or_default();
        let election_date: &String = &voting_period_start_date;

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
                date_voted: "2024-05-03T09:30:00-04:00".to_string(),
            },
            Voter {
                number: 2,
                last_name: "Garcia".to_string(),
                first_name: "Maria".to_string(),
                middle_name: "L.".to_string(),
                suffix: "".to_string(),
                id: "NYC12346".to_string(),
                status: "Voted".to_string(),
                date_voted: "2024-05-03T09:30:00-04:00".to_string(),
            },
            Voter {
                number: 3,
                last_name: "Johnson".to_string(),
                first_name: "Michael".to_string(),
                middle_name: "T.".to_string(),
                suffix: "".to_string(),
                id: "NYC12347".to_string(),
                status: "Voted".to_string(),
                date_voted: "2024-05-03T09:30:00-04:00".to_string(),
            },
            Voter {
                number: 4,
                last_name: "Lee".to_string(),
                first_name: "Sophie".to_string(),
                middle_name: "K.".to_string(),
                suffix: "".to_string(),
                id: "NYC12348".to_string(),
                status: "Voted".to_string(),
                date_voted: "2024-05-03T09:30:00-04:00".to_string(),
            },
        ];

        // Calculate statistics
        let total_voted = voters.iter().filter(|ov| ov.status == "Voted").count() as u32;
        let total_not_voted = voters.iter().filter(|ov| ov.status == "Not Voted").count() as u32;
        let not_pre_enrolled = voters
            .iter()
            .filter(|ov| ov.status == "Not Enrolled")
            .count() as u32;
        let voting_privilege_voted = 2; // Mocking this value
        let total = voters.len() as u32;

        let ovcs_version = get_app_version();
        let system_hash = get_app_hash();

        // partial Mock UserData
        Ok(UserData {
            election_date: election_date.to_string(),
            election_title: election.name.clone(),
            post: election_general_data.post,
            area_id: election_general_data.area_id,
            voting_period_start: voting_period_start_date,
            voting_period_end: voting_period_end_date,
            voted: total_voted,
            not_voted: total_not_voted,
            voters,
            not_pre_enrolled,
            voting_privilege_voted,
            total,
            report_hash: "-".to_string(),
            system_hash,
            date_printed: "2024-10-09 14:00:00".to_string(),
            ovcs_version,
            qr_code: String::new(),
        })
    }

    // Prepare system data
    #[instrument(err, skip(self))]
    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        let temp_val: &str = "test";
        Ok(SystemData {
            rendered_user_template,
            file_qrcode_lib: temp_val.to_string(),
        })
    }
}

#[instrument(err, skip(hasura_transaction, keycloak_transaction))]
pub async fn generate_ov_users_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    mode: GenerateReportMode,
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
) -> Result<()> {
    let template = OVUserTemplate {
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        election_id: election_id.to_string(),
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
            hasura_transaction,
            keycloak_transaction,
        )
        .await
}
