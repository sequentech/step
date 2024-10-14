// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::template_renderer::*;
use crate::postgres::reports::ReportType;
use crate::services::s3::get_minio_url;
use crate::services::temp_path::*;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use sequent_core::types::templates::EmailConfig;
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{info, instrument};

/// Struct returned by the API call for manual verification URL
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StatisticalReportOutput {
    pub link: String,
}

/// Struct for User Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub qrcode: String,
    pub logo: String,
}

/// Struct for System Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_logo: String,
    pub file_qrcode_lib: String,
}

/// Implementation of TemplateRenderer for Manual Verification
#[derive(Debug)]
pub struct StatisticalReportTemplate {
    tenant_id: String,
    election_event_id: String,
    election_id: String,
    contest_id: String,
}

#[async_trait]
impl TemplateRenderer for StatisticalReportTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_tenant_id(&self) -> String {
        self.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.election_event_id.clone()
    }

    fn base_name() -> String {
        "statistical_report".to_string()
    }

    fn prefix(&self) -> String {
        format!("statistical_report_{}", self.contest_id)
    }

    fn get_report_type() -> ReportType {
        ReportType::STATISTICAL_REPORT
    }

    fn get_email_config() -> EmailConfig {
        EmailConfig {
            subject: "Statistical Report".to_string(),
            plaintext_body: "".to_string(),
            html_body: None,
        }
    }

    async fn prepare_user_data(&self) -> Result<Self::UserData> {
        Ok(UserData {
            qrcode: QR_CODE_TEMPLATE.to_string(),
            logo: LOGO_TEMPLATE.to_string(),
        })
    }

    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        let public_asset_path = get_public_assets_path_env_var()?;
        let minio_endpoint_base =
            get_minio_url().with_context(|| "Error getting minio endpoint")?;

        Ok(SystemData {
            rendered_user_template,
            file_logo: format!(
                "{}/{}/{}",
                minio_endpoint_base, public_asset_path, PUBLIC_ASSETS_LOGO_IMG
            ),
            file_qrcode_lib: format!(
                "{}/{}/{}",
                minio_endpoint_base, public_asset_path, PUBLIC_ASSETS_QRCODE_LIB
            ),
        })
    }
}

/// Function to generate the manual verification report using the TemplateRenderer
#[instrument(err)]
pub async fn generate_statistical_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    contest_id: &str,
) -> Result<()> {
    let template = StatisticalReportTemplate {
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        contest_id: contest_id.to_string(),
        election_id: election_id.to_string(),
    };
    template
        .execute_report(
            document_id,
            tenant_id,
            election_event_id,
            /* is_scheduled_task */ false,
            /* receiver */ None,
            /* pdf_options */ None,
        )
        .await
}
