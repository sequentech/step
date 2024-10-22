// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::template_renderer::*;
use crate::postgres::reports::{Report, ReportType};
use crate::services::database::get_hasura_pool;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use sequent_core::types::templates::EmailConfig;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

/// Struct for User Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub report_hash: String,
    pub system_hash: String,
    pub election_title: String,
    pub election_date: String,
    pub voting_period: String,
    pub post: String,
    pub country: String,
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
    #[instrument]
    async fn prepare_user_data(&self) -> Result<Self::UserData> {
        let data: UserData = self
            .prepare_preview_data()
            .await
            .map_err(|e| anyhow::anyhow!(format!("Error preparing report preview {:?}", e)))?;
        Ok(data)
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

#[instrument]
pub async fn generate_pre_enrolled_ov_subject_to_manual_validation_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    mode: GenerateReportMode,
    report: Report,
) -> Result<()> {
    let template = PreEnrolledManualUsersTemplate {
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
            Some(report)
        )
        .await
}
