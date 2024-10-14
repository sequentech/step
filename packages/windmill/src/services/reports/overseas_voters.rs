// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::template_renderer::*;
use crate::services::database::get_hasura_pool;
use crate::{postgres::election_event::get_election_event_by_id};
use crate::services::temp_path::*;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use chrono::{DateTime, Utc};
use chrono::offset::TimeZone;
use sequent_core::types::templates::EmailConfig;
use crate::postgres::reports::ReportType;


/// Struct for Overseas Voter Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VoterData {
    pub index: u32,
    pub first_name: String,
    pub last_name: String,
    pub middle_name: Option<String>,
    pub voted: bool,
    pub date_time_voted: Option<DateTime<Utc>>,
}

/// Struct for Report Metadata
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub election_start_date: String,
    pub election_title: String,
    pub geograpic_region: String,
    pub area: String,
    pub country: String,
    pub voting_center: String,
    pub total_voted: u32,
    pub total_not_voted: u32,
    pub total_not_enrolled: u32,
    pub total_eb_voted: u32,
    pub total_ov: u32,
    pub voters_list: Vec<VoterData>,
    pub chairperson_name: String,
    pub poll_clerk_name: String,
    pub third_member_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub report_hash: String,
    pub ovcs_version: String,
    pub system_hash: String,
    pub file_logo: String,
    pub file_qrcode_lib: String,
    pub date_time_printed: String,
    pub printing_code: String,
}

/// Main struct for generating Overseas Voters Report
#[derive(Debug)]
pub struct OverseasVotersReport {
    tenant_id: String,
    election_event_id: String,
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

    fn base_name() -> String {
        "overseas_voters_report".to_string()
    }

    fn prefix(&self) -> String {
        format!("overseas_voters_report_{}", self.election_event_id)
    }

    fn get_email_config() -> EmailConfig {
        EmailConfig {
            subject: "Sequent Online Voting - Overseas Voters".to_string(),
            plaintext_body: "".to_string(),
            html_body: None,
        }
    }

    // TODO: replace mock data with actual data
    /// Prepare user data for the report
    async fn prepare_user_data(&self) -> Result<Option<Self::UserData>>{
        let mut db_client: DbClient = get_hasura_pool().await.get().await.with_context(|| "Error getting hasura db pool")?;
        let transaction = db_client.transaction().await.with_context(|| "Error starting transaction")?;

        // Fetch election event data
        let election_event = get_election_event_by_id(&transaction, &self.tenant_id, &self.election_event_id)
            .await
            .with_context(|| "Error getting election event")?;

        // Example Voters list (should be fetched from the database or external service)
        let voters_list = vec![
            VoterData {
                index: 1,
                first_name: "John".to_string(),
                last_name: "Doe".to_string(),
                middle_name: Some("M".to_string()),
                voted: true,
                date_time_voted: Some(Utc::now()),  // Replace with actual DB data
            },
            VoterData {
                index: 2,
                first_name: "Jane".to_string(),
                last_name: "Smith".to_string(),
                middle_name: None,
                voted: false,
                date_time_voted: None,  // Replace with actual DB data
            },
        ];

        // Aggregate voter statistics
        let total_voted = voters_list.iter().filter(|v| v.voted).count() as u32;
        let total_not_voted = voters_list.len() as u32 - total_voted;
        let total_not_enrolled = 10;  // Replace with actual data
        let total_eb_voted = 5;  // Replace with actual data
        let total_ov = voters_list.len() as u32 + total_not_enrolled;  // Total OV = registered + not enrolled

        let temp_val: &str = "test";
        let user_data = UserData {
            election_start_date: temp_val.to_string(),
            election_title: temp_val.to_string(),
            geograpic_region: "Asia".to_string(),  // Replace with actual data
            area: "Region 1".to_string(),  // Replace with actual data
            country: "Philippines".to_string(),  // Replace with actual data
            voting_center: "Manila".to_string(),  // Replace with actual data
            total_voted,
            total_not_voted,
            total_not_enrolled,
            total_eb_voted,
            total_ov,
            voters_list,
            chairperson_name: temp_val.to_string(),
            poll_clerk_name: temp_val.to_string(),
            third_member_name: temp_val.to_string(),
        };

        Ok(Some(user_data))
    }

    /// Prepare system metadata for the report
    async fn prepare_system_data(&self, _rendered_user_template: String) -> Result<Self::SystemData> {
        let now = Utc::now();
        let date_printed = now.format("%Y-%m-%d").to_string();
        let time_printed = now.format("%H:%M:%S").to_string();

        let system_data = SystemData {
            report_hash: String::new(),  // Placeholder, should be computed
            ovcs_version: "1.0".to_string(),  // Replace with actual version
            system_hash: String::new(),  // Placeholder, should be computed
            file_logo: String::new(),  // Placeholder for file logo path
            file_qrcode_lib: String::new(),  // Placeholder for QR code file path
            date_time_printed: format!("{} {}", date_printed, time_printed),
            printing_code: String::new(),  // Placeholder, should be computed
        };

        Ok(system_data)
    }
}
