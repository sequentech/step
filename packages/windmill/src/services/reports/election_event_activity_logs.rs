// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::template_renderer::*;
use crate::services::database::{get_hasura_pool, PgConfig};
use crate::services::documents::upload_and_return_document_postgres;
use crate::services::electoral_log::{
    list_electoral_log, ElectoralLogRow, GetElectoralLogBody, StatementHeadDataString,
};
use crate::services::providers::transactions_provider::provide_hasura_transaction;
use crate::services::s3::get_minio_url;
use crate::services::temp_path::*;
use crate::services::temp_path::{generate_temp_file, get_file_size};
use crate::types::resources::DataList;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use chrono::{DateTime, Utc};
use csv::WriterBuilder;
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::hasura::core::Document;
use sequent_core::types::templates::EmailConfig;
use serde::{Deserialize, Serialize};
use std::env;
use tempfile::NamedTempFile;
use tracing::{info, instrument};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ReportFormat {
    CSV,
    PDF,
}

impl ReportFormat {
    pub fn from_str(format_str: &str) -> Result<ReportFormat> {
        match format_str {
            "PDF" => Ok(ReportFormat::PDF),
            "CSV" => Ok(ReportFormat::CSV),
            _ => Err(anyhow!("Invalid report format: {}", format_str)),
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActivityLogRow {
    id: i64,
    created: String,
    statement_timestamp: String,
    statement_kind: String,
    event_type: String,
    log_type: String,
    description: String,
    message: String,
    user_id: String,
}

/// Struct for User Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub act_log: Vec<ActivityLogRow>,
    pub logo: String,
}

/// Struct for System Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_logo: String,
}

/// Implementation of TemplateRenderer for Manual Verification
#[derive(Debug)]
pub struct ActivityLogsTemplate {
    tenant_id: String,
    election_event_id: String,
}

#[async_trait]
impl TemplateRenderer for ActivityLogsTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type() -> ReportType {
        ReportType::ManualVerification
    }
    fn get_tenant_id(&self) -> String {
        self.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.election_event_id.clone()
    }

    fn base_name() -> String {
        "activity_logs".to_string()
    }

    fn prefix(&self) -> String {
        format!("activity_logs")
    }

    // Not needed for activity logs
    fn get_email_config() -> EmailConfig {
        EmailConfig {
            subject: "".to_string(),
            plaintext_body: "".to_string(),
            html_body: None,
        }
    }

    async fn prepare_user_data(&self) -> Result<Self::UserData> {
        let mut act_log: Vec<ActivityLogRow> = vec![];
        let mut offset = 0;
        let limit = PgConfig::from_env()
            .with_context(|| "Error obtaining Pg config from env.")?
            .default_sql_batch_size as i64;

        loop {
            let electoral_logs: DataList<ElectoralLogRow> =
                list_electoral_log(GetElectoralLogBody {
                    tenant_id: self.tenant_id.clone(),
                    election_event_id: self.election_event_id.clone(),
                    limit: Some(limit),
                    offset: Some(offset),
                    filter: None,
                    order_by: None,
                })
                .await?;

            let is_empty = electoral_logs.items.is_empty();

            for electoral_log in electoral_logs.items {
                let user_id = match electoral_log.user_id() {
                    Some(user_id) => user_id.to_string(),
                    None => "-".to_string(),
                };

                let timestamp = electoral_log.statement_timestamp();
                let dt =
                    DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(timestamp, 0), Utc);
                let statement_timestamp = dt.to_rfc2822();

                let creation_timestamp = electoral_log.created();
                let dt = DateTime::<Utc>::from_utc(
                    NaiveDateTime::from_timestamp(creation_timestamp, 0),
                    Utc,
                );
                let created = dt.format("%Y-%m-%d").to_string();
                let head_data = electoral_log
                    .statement_head_data()
                    .with_context(|| "Error to get head data.")?;
                let event_type = head_data.event_type;
                let log_type = head_data.log_type;
                let description = head_data.description;

                act_log.push(ActivityLogRow {
                    id: electoral_log.id(),
                    created,
                    statement_timestamp,
                    statement_kind: electoral_log.statement_kind().to_string(),
                    event_type,
                    log_type,
                    description,
                    message: electoral_log.message().to_string(),
                    user_id: user_id,
                });
            }

            let total = electoral_logs.total.aggregate.count;
            if is_empty || offset >= total {
                break;
            }

            offset += limit;
        }

        Ok(UserData {
            act_log,
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
        })
    }
}

pub async fn write_export_document(
    transaction: &Transaction<'_>,
    temp_file: NamedTempFile,
    name: &str,
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Document> {
    let temp_path = temp_file.into_temp_path();
    let temp_path_string = temp_path.to_string_lossy().to_string();
    let file_size =
        get_file_size(temp_path_string.as_str()).with_context(|| "Error obtaining file size")?;

    upload_and_return_document_postgres(
        transaction,
        &temp_path_string,
        file_size,
        "text/csv",
        tenant_id,
        election_event_id,
        &name,
        Some(document_id.to_string()),
        false, // is_public: bool,
    )
    .await
}

pub async fn generate_activity_logs_report_csv(
    tenant_id: &str,
    election_event_id: &str,
    document_id: &str,
    template: &ActivityLogsTemplate,
) -> Result<()> {
    // Prepare user data
    let act_log = template
        .prepare_user_data()
        .await
        .map_err(|e| anyhow!("Error preparing activity logs data into csv: {e:?}"))?
        .act_log;

    provide_hasura_transaction(|hasura_transaction| {
        let document_id = document_id.to_string();
        let tenant_id = tenant_id.to_string();
        let election_event_id = election_event_id.to_string();
        Box::pin(async move {
            // Your async code here
            let name = format!("export-election-event-logs-{}", election_event_id);
            // Create a temporary file to write CSV data
            let mut temp_file = generate_temp_file(&name, ".csv")
                .with_context(|| "Error creating named temp file")?;
            let mut csv_writer = WriterBuilder::new().from_writer(temp_file.as_file_mut());
            for item in &act_log {
                csv_writer.serialize(item)?; // Serialize each item to CSV
            }
            // Flush and finish writing to the temporary file
            csv_writer.flush()?;
            drop(csv_writer);

            write_export_document(
                hasura_transaction,
                temp_file,
                &name,
                &document_id,
                &tenant_id,
                &election_event_id,
            )
            .await?;
            Ok(())
        })
    })
    .await
}

pub async fn generate_activity_logs_report(
    tenant_id: &str,
    election_event_id: &str,
    document_id: &str,
    format: ReportFormat,
) -> Result<()> {
    let template = ActivityLogsTemplate {
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
    };

    match format {
        ReportFormat::CSV => {
            generate_activity_logs_report_csv(tenant_id, election_event_id, document_id, &template)
                .await
        }
        ReportFormat::PDF => {
            template
                .execute_report(document_id, tenant_id, election_event_id, false, None)
                .await
        }
    }
}
