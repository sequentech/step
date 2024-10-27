// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_election_data, generate_voters_turnout,
    get_election_contests_area_results_and_total_ballot_counted,
};
use super::template_renderer::*;
use crate::postgres::election::get_election_by_id;
use crate::postgres::reports::ReportType;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::temp_path::*;
use crate::services::users::count_keycloak_enabled_users_by_area_id;
use crate::{postgres::election_event::get_election_event_by_id, services::s3::get_minio_url};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::{Client as DbClient, Transaction};
use rocket::http::Status;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::scheduled_event::generate_voting_period_dates;
use sequent_core::types::templates::EmailConfig;
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{info, instrument};

/// Struct for Transition Report Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub date_printed: String,
    pub election_date: String,
    pub election_title: String,
    pub voting_period_start: String,
    pub voting_period_end: String,
    pub geographical_region: String,
    pub post: String,
    pub area_id: String,
    pub voting_center: String,
    pub precinct_code: String,
    pub registered_voters: i64,
    pub ballots_counted: i64,
    pub voters_turnout: i64,
    pub sboc_code: String,
    pub sboc_transmitted: String,
    pub sboc_date_transmitted: String,
    pub sboc_received: String,
    pub sboc_date_received: String,
    pub central_server_code: String,
    pub citizens_arm_1_code: String,
    pub citizens_arm_1_transmitted: String,
    pub citizens_arm_1_date_transmitted: String,
    pub citizens_arm_1_received: String,
    pub citizens_arm_1_date_received: String,
    pub citizens_arm_2_code: String,
    pub citizens_arm_2_transmitted: String,
    pub citizens_arm_2_date_transmitted: String,
    pub citizens_arm_2_received: String,
    pub citizens_arm_2_date_received: String,
    pub dominant_majority_party_code: String,
    pub dominant_majority_party_transmitted: String,
    pub dominant_majority_party_date_transmitted: String,
    pub dominant_majority_party_received: String,
    pub dominant_majority_party_date_received: String,
    pub dominant_minority_party_code: String,
    pub dominant_minority_party_transmitted: String,
    pub dominant_minority_party_date_transmitted: String,
    pub dominant_minority_party_received: String,
    pub dominant_minority_party_date_received: String,
    pub media_code: String,
    pub media_transmitted: String,
    pub media_date_transmitted: String,
    pub media_received: String,
    pub media_server_date_received: String,
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
    pub qr_codes: Vec<String>,
}

/// Struct for System Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
}

#[derive(Debug)]
pub struct TransmissionReport {
    tenant_id: String,
    election_event_id: String,
}

#[async_trait]
impl TemplateRenderer for TransmissionReport {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type() -> ReportType {
        ReportType::TRANSITIONS
    }

    fn get_tenant_id(&self) -> String {
        self.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.election_event_id.clone()
    }

    fn base_name() -> String {
        "transitions_report".to_string()
    }

    fn prefix(&self) -> String {
        format!("transitions_report_{}", self.election_event_id)
    }

    fn get_email_config() -> EmailConfig {
        EmailConfig {
            subject: "Sequent Online Voting - Transitions".to_string(),
            plaintext_body: "".to_string(),
            html_body: None,
        }
    }

    /// Prepare user data by fetching the relevant details
    async fn prepare_user_data(
        &self,
        hasura_transaction: Option<&Transaction<'_>>,
        keycloak_transaction: Option<&Transaction<'_>>,
    ) -> Result<Self::UserData> {
        let realm_name: String =
            get_event_realm(self.tenant_id.as_str(), self.election_event_id.as_str());
        // Fetch election event data
        let election_event = if let Some(transaction) = hasura_transaction {
            get_election_event_by_id(&transaction, &self.tenant_id, &self.election_event_id)
                .await
                .with_context(|| "Error obtaining election event")?
        } else {
            return Err(anyhow::anyhow!("Transaction is missing"));
        };

        // Fetch election event data
        let start_election_event = if let Some(transaction) = hasura_transaction {
            find_scheduled_event_by_election_event_id(
                &transaction,
                &self.get_tenant_id(),
                &self.get_election_event_id(),
            )
            .await
            .map_err(|e| {
                anyhow::anyhow!("Error getting scheduled event by election event_id: {}", e)
            })?
        } else {
            return Err(anyhow::anyhow!("Transaction is missing"));
        };

        // Fetch election's voting periods
        let voting_period_dates = generate_voting_period_dates(
            start_election_event,
            &self.get_tenant_id(),
            &self.get_election_event_id(),
            Some(&self.get_election_id().unwrap()),
        )?;

        // extract start date from voting period
        let voting_period_start_date = voting_period_dates.start_date.unwrap_or_default();
        // extract end date from voting period
        let voting_period_end_date = voting_period_dates.end_date.unwrap_or_default();

        // get election instace
        let election = if let Some(transaction) = hasura_transaction {
            match get_election_by_id(
                &transaction, // Use the unwrapped transaction reference
                &self.get_tenant_id(),
                &self.get_election_event_id(),
                &self.get_election_id().unwrap(),
            )
            .await
            .with_context(|| "Error getting election by id")?
            {
                Some(election) => election,
                None => return Err(anyhow::anyhow!("Election not found")),
            }
        } else {
            return Err(anyhow::anyhow!("Transaction is missing"));
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

        // fetch total of registerd voters
        let registered_voters = if let Some(transaction) = keycloak_transaction {
            count_keycloak_enabled_users_by_area_id(
                &transaction, // Pass the actual reference to the transaction
                &realm_name,
                &election_general_data.area_id,
            )
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Error fetching count_keycloak_enabled_users_by_area_id '{}': {}",
                    &election_general_data.area_id,
                    e
                )
            })?
        } else {
            return Err(anyhow::anyhow!("Keycloak Transaction is missing"));
        };

        let (ballots_counted, results_area_contests, contests) = if let Some(transaction) =
            hasura_transaction
        {
            get_election_contests_area_results_and_total_ballot_counted(
                &transaction,
                &self.get_tenant_id(),
                &self.get_election_event_id(),
                &self.get_election_id().unwrap(),
            )
            .await
            .map_err(|e| anyhow::anyhow!("Error getting election contests area results: {}", e))?
        } else {
            return Err(anyhow::anyhow!("Transaction is missing"));
        };

        // Calculate voter turnout
        let voters_turnout = generate_voters_turnout(&ballots_counted, &registered_voters)
            .await
            .map_err(|e| anyhow::anyhow!(format!("Error in generating voters turnout {:?}", e)))?;

        // Fetch necessary data (dummy placeholders for now)
        let chairperson_name = "John Doe".to_string();
        let poll_clerk_name = "Jane Smith".to_string();
        let third_member_name = "Alice Johnson".to_string();
        let chairperson_digital_signature = "DigitalSignatureABC".to_string();
        let poll_clerk_digital_signature = "DigitalSignatureDEF".to_string();
        let third_member_digital_signature = "DigitalSignatureGHI".to_string();
        let server_code = "123456".to_string();
        let report_hash = "dummy_report_hash".to_string();
        let ovcs_version = "1.0".to_string();
        let software_version = "1.0".to_string();
        let system_hash = "dummy_system_hash".to_string();

        Ok(UserData {
            date_printed: "2024-10-09T14:30:00-04:00".to_string(),
            election_date: "2024-05-10T14:30:00-04:00".to_string(),
            election_title: election_event.name.clone(),
            voting_period_start: voting_period_start_date,
            voting_period_end: voting_period_end_date,
            geographical_region: election_general_data.geographical_region,
            post: election_general_data.post,
            area_id: election_general_data.area_id,
            voting_center: election_general_data.voting_center,
            precinct_code: election_general_data.precinct_code,
            registered_voters,
            ballots_counted,
            voters_turnout,
            central_server_code: server_code,
            sboc_code: "SB123".to_string(),
            sboc_transmitted: "Transmitted".to_string(),
            sboc_date_transmitted: "2024-05-10T00:00:00".to_string(),
            sboc_received: "Received".to_string(),
            sboc_date_received: "2024-05-11T00:00:00".to_string(),
            citizens_arm_1_code: "CA1-789".to_string(),
            citizens_arm_1_transmitted: "Transmitted".to_string(),
            citizens_arm_1_date_transmitted: "2024-05-10T00:00:00".to_string(),
            citizens_arm_1_received: "Received".to_string(),
            citizens_arm_1_date_received: "2024-05-11T00:00:00".to_string(),
            citizens_arm_2_code: "CA2-012".to_string(),
            citizens_arm_2_transmitted: "Transmitted".to_string(),
            citizens_arm_2_date_transmitted: "2024-05-10T00:00:00".to_string(),
            citizens_arm_2_received: "Received".to_string(),
            citizens_arm_2_date_received: "2024-05-11T00:00:00".to_string(),
            dominant_majority_party_code: "DM-345".to_string(),
            dominant_majority_party_transmitted: "Transmitted".to_string(),
            dominant_majority_party_date_transmitted: "2024-05-10T00:00:00".to_string(),
            dominant_majority_party_received: "Received".to_string(),
            dominant_majority_party_date_received: "2024-05-11T00:00:00".to_string(),
            dominant_minority_party_code: "DN-678".to_string(),
            dominant_minority_party_transmitted: "Transmitted".to_string(),
            dominant_minority_party_date_transmitted: "2024-05-10T00:00:00".to_string(),
            dominant_minority_party_received: "Received".to_string(),
            dominant_minority_party_date_received: "2024-05-11T00:00:00".to_string(),
            media_code: "MS-901".to_string(),
            media_transmitted: "Transmitted".to_string(),
            media_date_transmitted: "2024-05-10T00:00:00".to_string(),
            media_received: "Received".to_string(),
            media_server_date_received: "2024-05-11T00:00:00".to_string(),
            chairperson_name,
            chairperson_digital_signature,
            poll_clerk_name,
            poll_clerk_digital_signature,
            third_member_name,
            third_member_digital_signature,
            report_hash,
            ovcs_version,
            system_hash,
            software_version,
            qr_codes: vec![
                "String 1".to_string(),
                "String 2".to_string(),
                "String 3".to_string(),
                "String 4".to_string(),
            ],
        })
    }

    #[instrument]
    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        Ok(SystemData {
            rendered_user_template,
        })
    }
}

#[instrument]
pub async fn generate_transmission_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    mode: GenerateReportMode,
    hasura_transaction: Option<&Transaction<'_>>,
    keycloak_transaction: Option<&Transaction<'_>>,
) -> Result<()> {
    let template = TransmissionReport {
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
            hasura_transaction,
            keycloak_transaction,
        )
        .await
}
