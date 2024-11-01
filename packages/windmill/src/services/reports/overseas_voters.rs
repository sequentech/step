// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_election_data, get_app_hash, get_app_version, get_date_and_time,
};
use super::template_renderer::*;
use crate::postgres::election::get_election_by_id;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::reports::ReportType;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::temp_path::*;
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::offset::TimeZone;
use chrono::{DateTime, Utc};
use deadpool_postgres::{Client as DbClient, Transaction};
use rocket::http::Status;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::scheduled_event::generate_voting_period_dates;
use sequent_core::types::templates::EmailConfig;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub date_printed: String,
    pub election_date: String,
    pub election_title: String,
    pub voting_period_start: String,
    pub voting_period_end: String,
    pub post: String,
    pub area_id: String,
    pub voters: Vec<Voter>,       // Voter list field
    pub ov_voted: u32,            // Number of overseas voters who voted
    pub ov_not_voted: u32,        // Number of overseas voters who did not vote
    pub ov_not_pre_enrolled: u32, // Number of overseas voters not pre-enrolled
    pub eb_voted: u32,            // Election board voted count
    pub ov_total: u32,            // Total overseas voters
    pub precinct_code: String,
    pub chairperson_name: String,
    pub chairperson_digital_signature: String,
    pub poll_clerk_name: String,
    pub poll_clerk_digital_signature: String,
    pub third_member_name: String,
    pub third_member_digital_signature: String,
    pub report_hash: String,
    pub software_version: String,
    pub ovcs_version: String,
    pub system_hash: String,
    pub qr_code: String, // Single QR code field
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Voter {
    pub number: u32,
    pub last_name: String,
    pub first_name: String,
    pub middle_name: String,
    pub suffix: String,
    pub status: String,
    pub date_voted: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_qrcode_lib: String,
}

/// Main struct for generating Overseas Voters Report
#[derive(Debug)]
pub struct OverseasVotersReport {
    tenant_id: String,
    election_event_id: String,
    election_id: String,
}

#[async_trait]
impl TemplateRenderer for OverseasVotersReport {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type() -> ReportType {
        ReportType::OVERSEAS_VOTERS
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
        "overseas_voters".to_string()
    }

    fn prefix(&self) -> String {
        format!("overseas_voters_{}", self.election_event_id)
    }

    fn get_email_config() -> EmailConfig {
        EmailConfig {
            subject: "Sequent Online Voting - Overseas Voters".to_string(),
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
        let datetime_printed: String = get_date_and_time();
        let election_date: &String = &voting_period_start_date;

        let voters = vec![
            Voter {
                number: 1,
                last_name: "Smith".to_string(),
                first_name: "John".to_string(),
                middle_name: "A.".to_string(),
                suffix: "Jr.".to_string(),
                status: "voted".to_string(),
                date_voted: "2024-05-05T10:30:00-04:00".to_string(),
            },
            Voter {
                number: 2,
                last_name: "Doe".to_string(),
                first_name: "Jane".to_string(),
                middle_name: "B.".to_string(),
                suffix: "".to_string(),
                status: "not voted".to_string(),
                date_voted: "2024-05-05T10:45:00-04:00".to_string(),
            },
            Voter {
                number: 3,
                last_name: "Johnson".to_string(),
                first_name: "Michael".to_string(),
                middle_name: "C.".to_string(),
                suffix: "".to_string(),
                status: "voted".to_string(),
                date_voted: "2024-05-06T09:45:00-04:00".to_string(),
            },
            Voter {
                number: 4,
                last_name: "Garcia".to_string(),
                first_name: "Maria".to_string(),
                middle_name: "D.".to_string(),
                suffix: "".to_string(),
                status: "not_pre_enrolled".to_string(),
                date_voted: "2024-05-07T12:45:00-04:00".to_string(),
            },
            Voter {
                number: 5,
                last_name: "Brown".to_string(),
                first_name: "James".to_string(),
                middle_name: "E.".to_string(),
                suffix: "III".to_string(),
                status: "voted".to_string(),
                date_voted: "2024-05-07T12:15:00-04:00".to_string(),
            },
        ];

        // Fetch necessary data (dummy placeholders for now)
        let chairperson_name = "John Doe".to_string();
        let poll_clerk_name = "Jane Smith".to_string();
        let third_member_name = "Alice Johnson".to_string();
        let chairperson_digital_signature = "DigitalSignatureABC".to_string();
        let poll_clerk_digital_signature = "DigitalSignatureDEF".to_string();
        let third_member_digital_signature = "DigitalSignatureGHI".to_string();
        let ovcs_version = get_app_version();
        let system_hash = get_app_hash();
        let software_version = ovcs_version.clone();
        let report_hash = "-".to_string();
        let qr_code = "code1".to_string();

        Ok(UserData {
            date_printed: datetime_printed,
            election_date: election_date.to_string(),
            election_title: election.name.clone(),
            voting_period_start: voting_period_start_date,
            voting_period_end: voting_period_end_date,
            post: election_general_data.post,
            area_id: election_general_data.area_id,
            voters: voters,
            precinct_code: election_general_data.precinct_code,
            ov_voted: 0,
            ov_not_voted: 0,
            ov_not_pre_enrolled: 0,
            eb_voted: 0,
            ov_total: 0,
            chairperson_name,
            chairperson_digital_signature,
            poll_clerk_name,
            poll_clerk_digital_signature,
            third_member_name,
            third_member_digital_signature,
            report_hash,
            software_version,
            ovcs_version,
            system_hash,
            qr_code,
        })
    }

    /// Prepare system metadata for the report
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

/// Generate Overseas Voters Report
#[instrument(err, skip(hasura_transaction, keycloak_transaction))]
pub async fn generate_overseas_voters_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    mode: GenerateReportMode,
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
) -> Result<()> {
    let template = OverseasVotersReport {
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
