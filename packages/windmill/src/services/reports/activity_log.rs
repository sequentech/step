// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::template_renderer::*;
use crate::postgres::reports::{Report, ReportType};
use crate::services::database::PgConfig;
use crate::services::documents::upload_and_return_document;
use crate::services::electoral_log::{list_electoral_log, ElectoralLogRow, GetElectoralLogBody};
use crate::services::providers::email_sender::{Attachment, EmailSender};
use crate::services::s3::get_minio_url;
use crate::services::temp_path::*;
use crate::types::resources::DataList;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use csv::WriterBuilder;
use deadpool_postgres::Transaction;
use headless_chrome::types::PrintToPdfOptions;
use sequent_core::services::date::ISO8601;
use sequent_core::services::keycloak::{self, get_event_realm, KeycloakAdminClient};
use sequent_core::types::hasura::core::{Document, TasksExecution};
use sequent_core::types::templates::ReportExtraConfig;
use serde::{Deserialize, Serialize};
use strum_macros::EnumString;
use tempfile::NamedTempFile;
use tracing::{debug, info, instrument, warn};

#[derive(Serialize, Deserialize, Debug, Clone, EnumString, PartialEq)]
pub enum ReportFormat {
    CSV,
    PDF,
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

/// Implementation of TemplateRenderer for Activity Logs
#[derive(Debug)]
pub struct ActivityLogsTemplate {
    tenant_id: String,
    election_event_id: String,
    report_format: ReportFormat,
}

impl ActivityLogsTemplate {
    pub fn new(tenant_id: String, election_event_id: String, report_format: ReportFormat) -> Self {
        ActivityLogsTemplate {
            tenant_id,
            election_event_id,
            report_format,
        }
    }
}

#[async_trait]
impl TemplateRenderer for ActivityLogsTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type(&self) -> ReportType {
        ReportType::ACTIVITY_LOGS
    }

    fn get_tenant_id(&self) -> String {
        self.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.election_event_id.clone()
    }

    fn base_name(&self) -> String {
        "activity_logs".to_string()
    }

    fn prefix(&self) -> String {
        format!("activity_logs_{}", rand::random::<u64>())
    }

    #[instrument(err, skip(self, hasura_transaction, keycloak_transaction))]
    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
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
                .await
                .map_err(|e| anyhow!("Error listing electoral logs: {e:?}"))?;

            let is_empty = electoral_logs.items.is_empty();

            for electoral_log in electoral_logs.items {
                let user_id = match electoral_log.user_id() {
                    Some(user_id) => user_id.to_string(),
                    None => "-".to_string(),
                };

                let statement_timestamp: String = if let Ok(datetime_parsed) =
                    ISO8601::timestamp_ms_utc_to_date_opt(
                        electoral_log.statement_timestamp() * 1000,
                    ) {
                    datetime_parsed.to_rfc3339()
                } else {
                    return Err(anyhow::anyhow!("Error parsing statement_timestamp"));
                };

                let created: String = if let Ok(datetime_parsed) =
                    ISO8601::timestamp_ms_utc_to_date_opt(electoral_log.created() * 1000)
                {
                    datetime_parsed.to_rfc3339()
                } else {
                    return Err(anyhow::anyhow!("Error parsing created"));
                };

                let head_data = electoral_log
                    .statement_head_data()
                    .with_context(|| "Error to get head data.")?;
                let event_type = head_data.event_type;
                let log_type = head_data.log_type;
                let description = head_data.description;

                act_log.push(ActivityLogRow {
                    id: electoral_log.id(),
                    user_id: user_id,
                    created,
                    statement_timestamp,
                    statement_kind: electoral_log.statement_kind().to_string(),
                    event_type,
                    log_type,
                    description,
                    message: electoral_log.message().to_string(),
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

    #[instrument(err, skip_all)]
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

    #[instrument(err, skip(self, hasura_transaction, keycloak_transaction))]
    async fn execute_report(
        &self,
        document_id: &str,
        tenant_id: &str,
        election_event_id: &str,
        is_scheduled_task: bool,
        recipients: Vec<String>,
        pdf_options: Option<PrintToPdfOptions>,
        generate_mode: GenerateReportMode,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
        task_execution: Option<TasksExecution>,
    ) -> Result<()> {
        if self.report_format == ReportFormat::PDF {
            // Call the default implementation for PDF
            self.execute_report_inner(
                document_id,
                tenant_id,
                election_event_id,
                is_scheduled_task,
                recipients,
                pdf_options,
                generate_mode,
                hasura_transaction,
                keycloak_transaction,
                task_execution,
            )
            .await
        } else {
            // Generate CSV report
            // Prepare user data
            let user_data = self
                .prepare_user_data(hasura_transaction, keycloak_transaction)
                .await
                .map_err(|e| anyhow!("Error preparing activity logs data into CSV: {e:?}"))?;

            // Generate CSV file using generate_export_data
            let name = format!("export-election-event-logs-{}", election_event_id);
            let temp_file = generate_export_data(&user_data.act_log, &name)
                .await
                .map_err(|e| anyhow!("Error generating export data: {e:?}"))?;

            // Upload document
            let temp_path = temp_file.into_temp_path();
            let temp_path_string = temp_path.to_string_lossy().to_string();
            let file_size =
                get_file_size(&temp_path_string).with_context(|| "Error obtaining file size")?;

            let auth_headers = keycloak::get_client_credentials()
                .await
                .map_err(|err| anyhow!("Error getting client credentials: {err:?}"))?;

            let _document = upload_and_return_document(
                temp_path_string.clone(),
                file_size,
                "text/csv".to_string(),
                auth_headers.clone(),
                tenant_id.to_string(),
                election_event_id.to_string(),
                name.clone(),
                Some(document_id.to_string()),
                false,
            )
            .await
            .map_err(|err| anyhow!("Error uploading document: {err:?}"))?;

            // Send email if needed
            if self.should_send_email(is_scheduled_task) {
                let ext_cfg: ReportExtraConfig = self
                    .get_default_extra_config()
                    .await
                    .map_err(|e| anyhow!("Error getting default extra config: {e:?}"))?;
                let email_config = ext_cfg.communication_templates.email_config;
                let email_recipients = self
                    .get_email_recipients(recipients, tenant_id, election_event_id)
                    .await
                    .map_err(|err| anyhow!("Error getting email recipients: {err:?}"))?;
                let email_sender = EmailSender::new()
                    .await
                    .map_err(|e| anyhow!("Error getting email sender: {e:?}"))?;
                let content_bytes = std::fs::read(&temp_path_string)
                    .map_err(|e| anyhow!("Error reading file content: {e:?}"))?;

                email_sender
                    .send(
                        email_recipients,
                        email_config.subject,
                        email_config.plaintext_body,
                        email_config.html_body,
                        vec![Attachment {
                            filename: name,
                            mimetype: "text/csv".to_string(),
                            content: content_bytes,
                        }],
                    )
                    .await
                    .map_err(|err| anyhow!("Error sending email: {err:?}"))?;
            }

            Ok(())
        }
    }
}

/// Maintains the generate_export_data function as before.
/// This function can be used by other report types that need to generate CSV files.
#[instrument(err)]
pub async fn generate_export_data(act_log: &[ActivityLogRow], name: &str) -> Result<NamedTempFile> {
    // Create a temporary file to write CSV data
    let mut temp_file =
        generate_temp_file(&name, ".csv").with_context(|| "Error creating named temp file")?;
    let mut csv_writer = WriterBuilder::new().from_writer(temp_file.as_file_mut());

    for item in act_log {
        // Serialize each item to CSV
        csv_writer
            .serialize(item)
            .map_err(|e| anyhow!("Error serializing to CSV: {e:?}"))?;
    }
    // Flush and finish writing to the temporary file
    csv_writer
        .flush()
        .map_err(|e| anyhow!("Error flushing CSV writer: {e:?}"))?;
    drop(csv_writer);

    Ok(temp_file)
}
