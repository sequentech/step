use super::report_variables::{
    extract_election_data, get_app_hash, get_app_version, get_date_and_time,
};
// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::template_renderer::*;
use crate::postgres::election::get_election_by_id;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::database::get_hasura_pool;
use crate::{postgres::reports::ReportType, services::database::get_keycloak_pool};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::scheduled_event::generate_voting_period_dates;
use sequent_core::types::templates::EmailConfig;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

/// Struct for User Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
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
    pub report_hash: String,
    pub software_version: String,
    pub ovcs_version: String,
    pub system_hash: String,
    pub qr_code: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_qrcode_lib: String,
}

/// Struct for each voter
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Voter {
    pub number: u32,
    pub last_name: String,
    pub first_name: String,
    pub middle_name: String,
    pub suffix: String,
    pub id: String,
    pub date_voted: String,
}

/// Struct for OVUsersWhoVotedTemplate
#[derive(Debug)]
pub struct OVUsersWhoVotedTemplate {
    tenant_id: String,
    election_event_id: String,
    pub election_id: Option<String>,
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

    fn get_election_id(&self) -> Option<String> {
        self.election_id.clone()
    }

    fn base_name() -> String {
        "ov_users_who_voted".to_string()
    }

    fn prefix(&self) -> String {
        format!(
            "ov_users_who_voted_{}_{}_{}",
            self.tenant_id,
            self.election_event_id,
            self.election_id.clone().unwrap_or_default()
        )
    }

    fn get_email_config() -> EmailConfig {
        EmailConfig {
            subject: "Sequent Online Voting - OV Users Who Voted".to_string(),
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
        let Some(election_id) = &self.election_id else {
            return Err(anyhow!("Empty election_id"));
        };

        let election = match get_election_by_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
            &election_id,
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
            Some(&election_id),
        )?;

        // extract start date from voting period
        let voting_period_start_date = voting_period_dates.start_date.unwrap_or_default();
        // extract end date from voting period
        let voting_period_end_date = voting_period_dates.end_date.unwrap_or_default();

        let election_date: &String = &voting_period_start_date;

        // Mock OVUsers data for now, can replace with actual database fetching later
        let voters = vec![
            Voter {
                number: 1,
                first_name: "Juan".to_string(),
                last_name: "Dela Cruz".to_string(),
                middle_name: "Garcia".to_string(),
                suffix: "".to_string(),
                id: "OV12345".to_string(),
                date_voted: "2024-05-09T14:30:00-04:00".to_string(),
            },
            Voter {
                number: 2,
                first_name: "Maria".to_string(),
                last_name: "Santos".to_string(),
                middle_name: "Reyes".to_string(),
                suffix: "Jr.".to_string(),
                id: "OV67890".to_string(),
                date_voted: "2024-05-09T14:30:00-04:00".to_string(),
            },
        ];
        let datetime_printed: String = get_date_and_time();

        let ovcs_version = get_app_version();
        let system_hash = get_app_hash();
        let software_version = ovcs_version.clone();

        Ok(UserData {
            election_date: election_date.to_string(),
            election_title: election.name.clone(),
            post: election_general_data.post,
            area_id: election_general_data.area_id,
            voting_period_start: voting_period_start_date,
            voting_period_end: voting_period_end_date,
            voted: 0,
            not_voted: 0,
            not_pre_enrolled: 0,
            voters,
            voting_privilege_voted: 0,
            total: 0,
            report_hash: "-".to_string(),
            ovcs_version,
            system_hash,
            date_printed: datetime_printed,
            software_version,
            qr_code: "code1".to_string(),
        })
    }

    // Prepare system data
    #[instrument]
    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        let file_qrcode_lib: &str = "test";
        Ok(SystemData {
            rendered_user_template,
            file_qrcode_lib: file_qrcode_lib.to_string(),
        })
    }
}

#[instrument(err, skip(hasura_transaction, keycloak_transaction))]
pub async fn generate_ov_users_who_voted_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<&str>,
    mode: GenerateReportMode,
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    is_scheduled_task: bool,
    email_recipients: Option<String>,
) -> Result<()> {
    let template = OVUsersWhoVotedTemplate {
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        election_id: election_id.map(|s| s.to_string()),
    };
    template
        .execute_report(
            document_id,
            tenant_id,
            election_event_id,
            is_scheduled_task,
            email_recipients,
            None,
            mode,
            hasura_transaction,
            keycloak_transaction,
        )
        .await
}
