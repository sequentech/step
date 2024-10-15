// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::template_renderer::*;
use crate::postgres::reports::ReportType;
use crate::services::database::get_hasura_pool;
use crate::services::s3::get_minio_url;
use crate::services::temp_path::*;
use anyhow::{Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Client as DbClient;
use sequent_core::types::templates::EmailConfig;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

// Struct to hold user data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub report_hash: String,
    pub ovcs_version: String,
    pub system_hash: String,
    pub election_date: String,
    pub election_title: String,
    pub voting_period: String,
    pub regions: Vec<Region>,
    pub ofov_disapproved: u32,
    pub sbei_disapproved: u32,
    pub system_disapproved: u32,
    pub qr_codes: Vec<String>,
}

// Struct to hold system data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegionData {
    pub post: String,
    pub country: String,
    pub total: u32,
    pub not_pre_enrolled: u32,
    pub pre_enrolled: u32,
    pub pre_enrolled_not_voted: u32,
    pub pre_enrolled_voted: u32,
    pub voted: u32,
    pub password_reset_request: u32,
    pub remarks: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Region {
    pub name: String,
    pub data: Vec<RegionData>,
}

#[derive(Debug)]
pub struct OVCSStatisticsTemplate {
    tenant_id: String,
    election_event_id: String,
}

#[async_trait]
impl TemplateRenderer for OVCSStatisticsTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type() -> ReportType {
        ReportType::OVCS_STATISTICS
    }

    fn base_name() -> String {
        "ovcs_statistics".to_string()
    }

    fn prefix(&self) -> String {
        format!("ovcs_statistics_{}", self.election_event_id)
    }

    fn get_tenant_id(&self) -> String {
        self.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.election_event_id.clone()
    }

    fn get_email_config() -> EmailConfig {
        EmailConfig {
            subject: "Sequent Online Voting - OVCS Statistics".to_string(),
            plaintext_body: "".to_string(),
            html_body: None,
        }
    }

    #[instrument]
    async fn prepare_user_data(&self) -> Result<Self::UserData> {
        let data: UserData = self.prepare_preview_data().await?;
        Ok(data)
    }

    /// Prepare system metadata for the report
    #[instrument]
    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        Ok(SystemData {
            rendered_user_template
        })
    }
}

#[instrument]
pub async fn generate_ovcs_statistics_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    mode: GenerateReportMode,
) -> Result<()> {
    let template = OVCSStatisticsTemplate {
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
