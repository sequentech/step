// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::template_renderer::*;
use crate::postgres::reports::{Report, ReportType};
use crate::services::database::PgConfig;
use crate::services::documents::upload_and_return_document;
use crate::services::electoral_log::{
    count_electoral_log, list_electoral_log, ElectoralLogRow, GetElectoralLogBody,
};
use crate::services::providers::email_sender::{Attachment, EmailSender};
use crate::services::temp_path::*;
use crate::types::resources::DataList;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use csv::WriterBuilder;
use deadpool_postgres::Transaction;
use sequent_core::services::date::ISO8601;
use sequent_core::services::keycloak::{self};
use sequent_core::services::s3::get_minio_url;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::templates::{ReportExtraConfig, SendTemplateBody};
use sequent_core::util::temp_path::*;
use serde::{Deserialize, Serialize};
use std::mem;
use strum_macros::EnumString;
use tempfile::NamedTempFile;
use tracing::{debug, info, instrument, warn};

const KB: f64 = 1024.0;
const MB: f64 = 1024.0 * KB;
const GB: f64 = 1024.0 * MB;

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
    pub electoral_log: Vec<ElectoralLogRow>,
}

/// Struct for System Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
}

/// Implementation of TemplateRenderer for Activity Logs
#[derive(Debug)]
pub struct ActivityLogsTemplate {
    ids: ReportOrigins,
    report_format: ReportFormat,
}

impl ActivityLogsTemplate {
    pub fn new(ids: ReportOrigins, report_format: ReportFormat) -> Self {
        ActivityLogsTemplate { ids, report_format }
    }
}

impl TryFrom<ElectoralLogRow> for ActivityLogRow {
    type Error = anyhow::Error;

    fn try_from(electoral_log: ElectoralLogRow) -> Result<Self, Self::Error> {
        let user_id = match electoral_log.user_id() {
            Some(user_id) => user_id.to_string(),
            None => "-".to_string(),
        };

        let statement_timestamp: String = if let Ok(datetime_parsed) =
            ISO8601::timestamp_secs_utc_to_date_opt(electoral_log.statement_timestamp())
        {
            datetime_parsed.to_rfc3339()
        } else {
            return Err(anyhow::anyhow!("Error parsing statement_timestamp"));
        };

        let created: String = if let Ok(datetime_parsed) =
            ISO8601::timestamp_secs_utc_to_date_opt(electoral_log.created())
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

        Ok(ActivityLogRow {
            id: electoral_log.id(),
            user_id: user_id,
            created,
            statement_timestamp,
            statement_kind: electoral_log.statement_kind().to_string(),
            event_type,
            log_type,
            description,
            message: electoral_log.message().to_string(),
        })
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
        self.ids.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.ids.election_event_id.clone()
    }

    fn get_initial_template_alias(&self) -> Option<String> {
        self.ids.template_alias.clone()
    }

    fn get_report_origin(&self) -> ReportOriginatedFrom {
        self.ids.report_origin
    }

    fn base_name(&self) -> String {
        "activity_logs".to_string()
    }

    fn prefix(&self) -> String {
        format!("activity_logs_{}", rand::random::<u64>())
    }

    async fn count_items(&self, _hasura_transaction: &Transaction<'_>) -> Result<Option<i64>> {
        let input = GetElectoralLogBody {
            tenant_id: self.ids.tenant_id.clone(),
            election_event_id: self.ids.election_event_id.clone(),
            limit: None,
            offset: None,
            filter: None,
            order_by: None,
            area_ids: None,
            only_with_user: None,
            election_id: None,
        };
        Ok(count_electoral_log(input).await.ok())
    }

    #[instrument(err, skip_all)]
    async fn prepare_user_data_batch(
        &self,
        _hasura_transaction: &Transaction<'_>,
        _keycloak_transaction: &Transaction<'_>,
        offset: &mut i64,
        limit: i64,
    ) -> Result<Self::UserData> {
        let mut act_log: Vec<ActivityLogRow> = vec![];
        let mut elect_logs: Vec<ElectoralLogRow> = vec![];

        let electoral_logs: DataList<ElectoralLogRow> = list_electoral_log(GetElectoralLogBody {
            tenant_id: self.ids.tenant_id.clone(),
            election_event_id: self.ids.election_event_id.clone(),
            limit: Some(limit),
            offset: Some(*offset),
            filter: None,
            order_by: None,
            area_ids: None,
            only_with_user: None,
            election_id: None,
        })
        .await
        .map_err(|e| anyhow!("Error listing electoral logs: {e:?}"))?;

        for electoral_log in electoral_logs.items {
            elect_logs.push(electoral_log.clone());
            let head_data = electoral_log
                .statement_head_data()
                .with_context(|| "Error to get head data.")?;
            let event_type = head_data.event_type;
            let log_type = head_data.log_type;
            let description = head_data.description;
            let activity_log = electoral_log.try_into()?;
            info!("activity_log = {activity_log:?}");
            let activity_log = ActivityLogRow {
                event_type,
                log_type,
                description,
                ..activity_log
            };
            info!("activity_log = {activity_log:?}");
            act_log.push(activity_log);
        }

        Ok(UserData {
            act_log,
            electoral_log: elect_logs,
        })
    }

    #[instrument(err, skip_all)]
    async fn prepare_user_data(
        &self,
        _hasura_transaction: &Transaction<'_>,
        _keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        let mut act_log: Vec<ActivityLogRow> = vec![];
        let mut elect_logs: Vec<ElectoralLogRow> = vec![];
        let mut offset = 0;
        let limit = PgConfig::from_env()
            .with_context(|| "Error obtaining Pg config from env.")?
            .default_sql_batch_size as i64;

        loop {
            let electoral_logs: DataList<ElectoralLogRow> =
                list_electoral_log(GetElectoralLogBody {
                    tenant_id: self.ids.tenant_id.clone(),
                    election_event_id: self.ids.election_event_id.clone(),
                    limit: Some(limit),
                    offset: Some(offset),
                    filter: None,
                    order_by: None,
                    area_ids: None,
                    only_with_user: None,
                    election_id: None,
                })
                .await
                .map_err(|e| anyhow!("Error listing electoral logs: {e:?}"))?;

            let is_empty = electoral_logs.items.is_empty();

            for electoral_log in electoral_logs.items {
                elect_logs.push(electoral_log.clone());
                let head_data = electoral_log
                    .statement_head_data()
                    .with_context(|| "Error to get head data.")?;
                let event_type = head_data.event_type;
                let log_type = head_data.log_type;
                let description = head_data.description;
                let activity_log = electoral_log.try_into()?;
                let activity_log = ActivityLogRow {
                    event_type,
                    log_type,
                    description,
                    ..activity_log
                };
                act_log.push(activity_log);
            }

            let total = electoral_logs.total.aggregate.count;
            if is_empty || offset >= total {
                break;
            }

            offset += limit;
        }

        Ok(UserData {
            act_log,
            electoral_log: elect_logs,
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
        })
    }

    #[instrument(err, skip_all)]
    async fn execute_report(
        &self,
        document_id: &str,
        tenant_id: &str,
        election_event_id: &str,
        is_scheduled_task: bool,
        recipients: Vec<String>,
        generate_mode: GenerateReportMode,
        report: Option<Report>,
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
                generate_mode,
                report,
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

            // Generate CSV file using generate_report_data
            let name = format!("export-election-event-logs-{}", election_event_id);
            let temp_file = generate_report_data(&user_data.act_log, &name)
                .await
                .map_err(|e| anyhow!("Error generating export data: {e:?}"))?;

            // Upload document
            let temp_path = temp_file.into_temp_path();
            let temp_path_string = temp_path.to_string_lossy().to_string();
            let file_size =
                get_file_size(&temp_path_string).with_context(|| "Error obtaining file size")?;

            let _document = upload_and_return_document(
                hasura_transaction,
                &temp_path_string.clone(),
                file_size,
                "text/csv",
                tenant_id,
                Some(election_event_id.to_string()),
                &name.clone(),
                Some(document_id.to_string()),
                false,
            )
            .await
            .map_err(|err| anyhow!("Error uploading document: {err:?}"))?;

            // Send email if needed
            if self.should_send_email(is_scheduled_task) {
                // Do the query to get the user template data
                let template_data_opt: Option<SendTemplateBody> = self
                    .get_custom_user_template_data(hasura_transaction)
                    .await
                    .map_err(|e| anyhow!("Error getting custom user template: {e:?}"))?;

                // Set the data from the user or fill extra config if needed with default data
                let email_config = match template_data_opt {
                    Some(template) if template.email.is_some() => template.email.unwrap(),
                    _ => {
                        let ext_cfg: ReportExtraConfig = self
                            .get_default_extra_config()
                            .await
                            .map_err(|e| anyhow!("Error getting default extra config: {e:?}"))?;
                        ext_cfg.communication_templates.email_config
                    }
                };

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
#[instrument(err, skip(act_log))]
pub async fn generate_report_data(act_log: &[ActivityLogRow], name: &str) -> Result<NamedTempFile> {
    // Create a temporary file to write CSV data
    let mut temp_file =
        generate_temp_file(&name, ".csv").with_context(|| "Error creating named temp file")?;
    let mut csv_writer = WriterBuilder::new().from_writer(temp_file.as_file_mut());

    for item in act_log {
        let mut item_clean = item.clone();

        // Replace newline characters in the message field
        item_clean.message = item_clean.message.replace('\n', " ").replace('\r', " ");
        // Serialize each item to CSV
        csv_writer
            .serialize(item_clean)
            .map_err(|e| anyhow!("Error serializing to CSV: {e:?}"))?;
    }
    // Flush and finish writing to the temporary file
    csv_writer
        .flush()
        .map_err(|e| anyhow!("Error flushing CSV writer: {e:?}"))?;
    drop(csv_writer);

    Ok(temp_file)
}

impl ActivityLogsTemplate {
    // Export data
    #[instrument(err, skip(self, hasura_transaction))]
    pub async fn generate_export_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        name: &str,
    ) -> Result<NamedTempFile> {
        let limit = PgConfig::from_env()
            .with_context(|| "Error obtaining Pg config from env.")?
            .default_sql_batch_size as i64;
        let mut offset: i64 = 0;

        // Create a temporary file to write CSV data
        let mut temp_file =
            generate_temp_file(&name, ".csv").with_context(|| "Error creating named temp file")?;
        let mut csv_writer = WriterBuilder::new().from_writer(temp_file.as_file_mut());
        let total = self
            .count_items(hasura_transaction)
            .await
            .map_err(|e| anyhow!("Error count_items in activity logs data: {e:?}"))?
            .unwrap_or(0);

        while offset < total {
            // Prepare user data
            let user_data = self
                .prepare_user_data_batch(hasura_transaction, hasura_transaction, &mut offset, limit)
                .await
                .map_err(|e| anyhow!("Error preparing activity logs data: {e:?}"))?;

            let s1 = user_data.electoral_log.len() * (mem::size_of::<ElectoralLogRow>());
            let s2 = user_data.act_log.len() * (mem::size_of::<ActivityLogRow>());
            let kb = (s1 + s2) as f64 / KB;
            let mb = (s1 + s2) as f64 / MB;
            info!("Logs batch size: {kb:.2} KB, {mb:.2} MB");

            for item in user_data.electoral_log {
                let mut item_clone = item.clone();

                // Replace newline characters in the message field
                item_clone.message = item_clone.message.replace('\n', " ").replace('\r', " ");
                // Serialize each item to CSV
                csv_writer
                    .serialize(item_clone)
                    .map_err(|e| anyhow!("Error serializing to CSV: {e:?}"))?;
            }
            offset += limit;
        }

        // Flush and finish writing to the temporary file
        csv_writer
            .flush()
            .map_err(|e| anyhow!("Error flushing CSV writer: {e:?}"))?;
        drop(csv_writer);

        Ok(temp_file)
    }
}
