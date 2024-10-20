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
    pub num_of_registered_voters: u32,
    pub num_of_ballots_counted: u32,
    pub voter_turnout: f32,
    pub server_code: String,
    pub transmitted: bool,
    pub transmitted_datetime: Option<String>,
    pub received: bool,
    pub received_datetime: Option<String>,
    pub election_start_date: String,
    pub election_title: String,
    pub geograpic_region: String,
    pub area: String,
    pub country: String,
    pub voting_center: String,
    pub chairperson_name: String,
    pub poll_clerk_name: String,
    pub third_member_name: String,
    pub report_hash: String,
    pub ovsc_version: String,
    pub system_hash: String,
    pub file_logo: String,
    pub file_qrcode_lib: String,
    pub date_time_printed: String,
    pub printing_code: String,
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
        let voter_turnout = (total_ballots_counted as f32 / total_registered_voters as f32) * 100.0;

        // Placeholder values for server data
        let server_code = "123456".to_string();
        let transmitted = true;
        let transmitted_datetime = Some("2024-10-09T12:00:00Z".to_string());
        let received = true;
        let received_datetime = Some("2024-10-09T12:05:00Z".to_string());

        let temp_val: &str = "test";
        Ok(UserData {
            num_of_registered_voters: total_registered_voters,
            num_of_ballots_counted: total_ballots_counted,
            voter_turnout,
            server_code,
            transmitted,
            transmitted_datetime,
            received,
            received_datetime,
            election_start_date: temp_val.to_string(),
            election_title: election_event.name.clone(),
            geograpic_region: temp_val.to_string(),
            area: temp_val.to_string(),
            country: temp_val.to_string(),
            voting_center: temp_val.to_string(),
            chairperson_name: temp_val.to_string(),
            poll_clerk_name: temp_val.to_string(),
            third_member_name: temp_val.to_string(),
            report_hash: String::new(),
            ovsc_version: String::new(),
            system_hash: String::new(),
            file_logo: String::new(),
            file_qrcode_lib: String::new(),
            date_time_printed: String::new(),
            printing_code: String::new(),
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
