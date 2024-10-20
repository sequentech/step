// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::template_renderer::*;
use crate::postgres::reports::ReportType;
use crate::services::database::get_hasura_pool;
use crate::services::s3::get_minio_url;
use crate::services::temp_path::*;
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
    pub date_printed: String,
    pub election_date: String,
    pub election_title: String,
    pub voting_period: String,
    pub post: String,
    pub country: String,
    pub voters: Vec<Voter>,
    pub ovcs_version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Voter {
    pub number: u32,
    pub last_name: String,
    pub first_name: String,
    pub middle_name: String,
    pub suffix: String,
    pub date_disapproved: String,
    pub disapproved_by: String,
    pub reason: String,
}

/// Struct for System Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_qrcode_lib: String,
}

#[derive(Debug)]
pub struct PreEnrolledDisapprovedTemplate {
    tenant_id: String,
    election_event_id: String,
}

#[async_trait]
impl TemplateRenderer for PreEnrolledDisapprovedTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type() -> ReportType {
        ReportType::PRE_ENROLLED_OV_BUT_DISAPPROVED
    }

    fn get_tenant_id(&self) -> String {
        self.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.election_event_id.clone()
    }

    fn base_name() -> String {
        "pre_enrolled_ov_but_disapproved".to_string()
    }

    fn prefix(&self) -> String {
        format!("pre_enrolled_ov_but_disapproved_{}", self.election_event_id)
    }

    fn get_email_config() -> EmailConfig {
        EmailConfig {
            subject: "Sequent Online Voting - Pre Enrolled OV But Disapproved".to_string(),
            plaintext_body: "".to_string(),
            html_body: None,
        }
    }

    /// TODO: fetch the real data
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
pub async fn generate_pre_enrolled_ov_but_disapproved_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    mode: GenerateReportMode,
) -> Result<()> {
    let template = PreEnrolledDisapprovedTemplate {
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
