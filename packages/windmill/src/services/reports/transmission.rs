// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::template_renderer::*;
use crate::postgres::reports::ReportType;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id_and_event_processor;
use crate::services::database::get_hasura_pool;
use crate::services::temp_path::*;
use crate::{postgres::election_event::get_election_event_by_id, services::s3::get_minio_url};
use anyhow::{anyhow, Context, Ok, Result};
use async_trait::async_trait;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
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
    pub voting_period: String,
    pub geographical_region: String,
    pub post: String,
    pub country: String,
    pub voting_center: String,
    pub precinct_code: String,
    pub registered_voters: i64,
    pub ballots_counted: i64,
    pub voters_turnout: String,
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
    pub qr_codes: Vec<String>
}

// pub struct UserData {
//     pub registered_voters: u32,
//     pub ballots_counted: u32,
//     pub voter_turnout: f32,
//     pub server_code: String,
//     pub transmitted: bool,
//     pub transmitted_datetime: Option<String>,
//     pub received: bool,
//     pub received_datetime: Option<String>,
//     pub election_date: String,
//     pub election_title: String,
//     pub voting_period: String,
//     pub precinct_code: String,
//     pub geographical_region: String,
//     pub post: String,
//     pub country: String,
//     pub voting_center: String,
//     pub chairperson_name: String,
//     pub poll_clerk_name: String,
//     pub third_member_name: String,
//     pub report_hash: String,
//     pub ovsc_version: String,
//     pub system_hash: String,
//     pub file_logo: String,
//     pub file_qrcode_lib: String,
//     pub date_printed: String,
//     pub printing_code: String,
// }

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
    async fn prepare_user_data(&self) -> Result<Self::UserData> {
        let mut hasura_db_client: DbClient = get_hasura_pool()
            .await
            .get()
            .await
            .with_context(|| "Error getting hasura db pool")?;

        let hasura_transaction = hasura_db_client
            .transaction()
            .await
            .with_context(|| "Error starting hasura transaction")?;

        // Fetch election event data
        let election_event = get_election_event_by_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
        )
        .await
        .with_context(|| "Error obtaining election event")?;

        // Fetch election event data
        let start_election_event = find_scheduled_event_by_election_event_id_and_event_processor(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
            "START_VOTING_PERIOD",
        )
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)));

        // TODO: replace mock data with actual data
        let mut election_start_date: String;
        // if let Some(cron_config) = start_election_event.get(0).and_then(|event| event.cron_config.clone()) {
        //     // Now cron_config is a CronConfig, not an Option
        //     if let Some(scheduled_date) = cron_config.scheduled_date {
        //         election_start_date = scheduled_date;
        //     }

        // }

        // Placeholder values for fetching external data (e.g., total ballots)
        let total_registered_voters = 1000; // Replace with actual query
        let total_ballots_counted = 800; // Replace with actual query

        // Calculate voter turnout
        let voters_turnout = ((total_ballots_counted as f32 / total_registered_voters as f32) * 100.0).to_string();

        // Placeholder values for server data
        let server_code = "123456".to_string();
        let transmitted = true;
        let transmitted_datetime = Some("2024-10-09T12:00:00Z".to_string());
        let received = true;
        let received_datetime = Some("2024-10-09T12:05:00Z".to_string());

        let temp_val: &str = "test";
        Ok(UserData {
            date_printed: "2024-10-09T14:30:00-04:00".to_string(),
            election_date: "2024-05-10T14:30:00-04:00".to_string(),
            election_title: election_event.name.clone(),
            geographical_region: "North America".to_string(),
            post: temp_val.to_string(),
            country: temp_val.to_string(),
            registered_voters: total_registered_voters,
            ballots_counted: total_ballots_counted,
            voters_turnout,
            voting_period: "April 10 - May 10, 2024".to_string(),
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
            voting_center: temp_val.to_string(),
            precinct_code: "P12345".to_string(),
            chairperson_name: temp_val.to_string(),
            chairperson_digital_signature: temp_val.to_string(),
            poll_clerk_name: temp_val.to_string(),
            poll_clerk_digital_signature: temp_val.to_string(),
            third_member_name: temp_val.to_string(),
            third_member_digital_signature: temp_val.to_string(),
            report_hash: String::new(),
            ovcs_version: String::new(),
            system_hash: String::new(),
            qr_codes: vec![
                "String 1".to_string(),
                "String 2".to_string(),
                "String 3".to_string(),
                "String 4".to_string(),
            ],
            software_version: "1.0".to_string(),
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
        )
        .await
}
