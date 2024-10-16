// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::template_renderer::*;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::reports::ReportType;
use crate::services::database::get_hasura_pool;
use crate::services::temp_path::*;
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::offset::TimeZone;
use chrono::{DateTime, Utc};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use sequent_core::types::templates::EmailConfig;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub date_printed: String,
    pub time_printed: String,
    pub election_date: String,
    pub election_title: String,
    pub voting_period: String,
    pub post: String,
    pub country: String,
    pub voters: Vec<Voter>,       // Voter list field
    pub ov_voted: u32,            // Number of overseas voters who voted
    pub ov_not_voted: u32,        // Number of overseas voters who did not vote
    pub ov_not_pre_enrolled: u32, // Number of overseas voters not pre-enrolled
    pub eb_voted: u32,            // Election board voted count
    pub ov_total: u32,            // Total overseas voters
    pub precinct_code: String,
    pub goverment_time: String,
    pub local_time: String,
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
    pub time_voted: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_qrcode_lib: String
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
    #[instrument]
    async fn prepare_user_data(&self) -> Result<Self::UserData> {
        let data: UserData = self.prepare_preview_data().await
        .map_err(|e| 
            anyhow::anyhow!(format!(
                "Error preparing report preview {:?}", e
            )
        ))?;
        Ok(data)
    }

    /// Prepare system metadata for the report
    #[instrument]
    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        let temp_val: &str = "test";
        Ok(SystemData {
            rendered_user_template,
            file_qrcode_lib: temp_val.to_string()
        })
    }
}

/// Generate Overseas Voters Report
#[instrument]
pub async fn generate_overseas_voters_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    mode: GenerateReportMode,
) -> Result<()> {
    let template = OverseasVotersReport {
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
