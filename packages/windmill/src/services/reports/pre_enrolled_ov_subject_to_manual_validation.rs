// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{extract_election_data, get_date_and_time};
use super::template_renderer::*;
use crate::postgres::election::get_election_by_id;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::database::get_hasura_pool;
use crate::{postgres::reports::ReportType, services::database::get_keycloak_pool};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::{Client as DbClient, Transaction};
use rocket::http::Status;
use sequent_core::types::scheduled_event::generate_voting_period_dates;
use sequent_core::types::templates::EmailConfig;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

/// Struct for User Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub date_printed: String,
    pub report_hash: String,
    pub system_hash: String,
    pub election_title: String,
    pub election_date: String,
    pub voting_period_start: String,
    pub voting_period_end: String,
    pub post: String,
    pub area_id: String,
    pub voters: Vec<Voter>,
    pub ovcs_version: String,
    pub qr_code: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Voter {
    pub number: u32,
    pub last_name: String,
    pub first_name: String,
    pub middle_name: String,
    pub suffix: String,
    pub id: String,
    pub reason: String,
    pub date_pre_enrolled: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_qrcode_lib: String,
}

/// Struct for PreEnrolledUsersRenderer
#[derive(Debug)]
pub struct PreEnrolledManualUsersTemplate {
    tenant_id: String,
    election_event_id: String,
    election_id: String,
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

    fn get_election_id(&self) -> Option<String> {
        Some(self.election_id.clone())
    }

    fn base_name() -> String {
        "pre_enrolled_ov_subject_to_manual_validation".to_string()
    }

    fn prefix(&self) -> String {
        format!("pre_enrolled_ov_{}", self.election_event_id)
    }

    fn get_email_config() -> EmailConfig {
        EmailConfig {
            subject: "Sequent Online Voting - Pre Enrolled OV Subject To Manual Validation"
                .to_string(),
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
        let datetime_printed: String = get_date_and_time();
        let report_hash = "dummy_report_hash".to_string();
        let ovcs_version = "1.0".to_string();
        let system_hash = "a9b8c7d6".to_string();
        let qr_code = "code1".to_string();
        let voters = vec![
            Voter {
                number: 1,
                last_name: "Taylor".to_string(),
                first_name: "Alice".to_string(),
                middle_name: "M.".to_string(),
                suffix: "".to_string(),
                id: "LA123456".to_string(),
                reason: "ID verification needed".to_string(),
                date_pre_enrolled: "2024-04-12T00:00:00".to_string(),
            },
            Voter {
                number: 2,
                last_name: "Lee".to_string(),
                first_name: "Brian".to_string(),
                middle_name: "N.".to_string(),
                suffix: "".to_string(),
                id: "LA123457".to_string(),
                reason: "Address mismatch".to_string(),
                date_pre_enrolled: "2024-04-13T00:00:00".to_string(),
            },
            Voter {
                number: 3,
                last_name: "Walker".to_string(),
                first_name: "Chris".to_string(),
                middle_name: "O.".to_string(),
                suffix: "".to_string(),
                id: "LA123458".to_string(),
                reason: "Incomplete documents".to_string(),
                date_pre_enrolled: "2024-04-14T00:00:00".to_string(),
            },
            Voter {
                number: 4,
                last_name: "Martinez".to_string(),
                first_name: "David".to_string(),
                middle_name: "P.".to_string(),
                suffix: "".to_string(),
                id: "LA123459".to_string(),
                reason: "Pending background check".to_string(),
                date_pre_enrolled: "2024-04-15T00:00:00".to_string(),
            },
            Voter {
                number: 5,
                last_name: "Lopez".to_string(),
                first_name: "Emily".to_string(),
                middle_name: "Q.".to_string(),
                suffix: "".to_string(),
                id: "LA123460".to_string(),
                reason: "Double registration".to_string(),
                date_pre_enrolled: "2024-04-16T00:00:00".to_string(),
            },
        ];

        Ok(UserData {
            date_printed: datetime_printed,
            election_date: election_date.to_string(),
            election_title: election.name.clone(),
            voting_period_start: voting_period_start_date,
            voting_period_end: voting_period_end_date,
            post: election_general_data.post,
            area_id: election_general_data.area_id,
            voters,
            report_hash,
            system_hash,
            ovcs_version,
            qr_code,
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
            file_qrcode_lib: file_qrcode_lib.to_string(),
        })
    }
}

#[instrument(err, skip(hasura_transaction, keycloak_transaction))]
pub async fn generate_pre_enrolled_ov_subject_to_manual_validation_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    mode: GenerateReportMode,
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
) -> Result<()> {
    let template = PreEnrolledManualUsersTemplate {
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
