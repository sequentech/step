// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::template_renderer::*;
use crate::postgres::reports::{Report, ReportType};
use crate::services::documents::upload_and_return_document;
use crate::services::electoral_log::{ElectoralLogRow, IMMUDB_ROWS_LIMIT};
use crate::services::protocol_manager::{get_board_client, get_event_board};
use crate::services::providers::email_sender::{Attachment, EmailSender};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use csv::WriterBuilder;
use deadpool_postgres::Transaction;
use electoral_log::messages::message::Message;
use electoral_log::ElectoralLogMessage;
use sequent_core::services::date::ISO8601;
use sequent_core::services::s3::get_minio_url;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::templates::{ReportExtraConfig, SendTemplateBody};
use sequent_core::util::temp_path::*;
use serde::{Deserialize, Serialize};
use std::mem;
use strand::serialization::StrandDeserialize;
use strum_macros::EnumString;
use tempfile::NamedTempFile;
use tracing::{debug, info, instrument, warn};

#[derive(Serialize, Deserialize, Debug, Clone, EnumString, PartialEq, Copy)]
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
/// act_log is for PDF
/// electoral_log is for CSV
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

    // Export data using the electoral-log board client, streaming in batches
    #[instrument(err, skip(self))]
    pub async fn generate_export_csv_data(&self, name: &str) -> Result<NamedTempFile> {
        let limit = IMMUDB_ROWS_LIMIT as i64;
        let mut last_id: i64 = 0;
        let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;
        let board_name = get_event_board(
            self.ids.tenant_id.as_str(),
            self.ids.election_event_id.as_str(),
            &slug,
        );

        let mut board_client = get_board_client().await?;

        let mut temp_file =
            generate_temp_file(name, ".csv").with_context(|| "Error creating named temp file")?;
        let mut csv_writer = WriterBuilder::new().from_writer(temp_file.as_file_mut());

        loop {
            info!("last_id: {last_id}");
            let msgs = board_client
                .get_electoral_log_messages_batch(&board_name, limit, last_id)
                .await
                .map_err(|e| anyhow!("Error fetching electoral log batch: {e:?}"))?;

            let batch_size = msgs.len() * mem::size_of::<ElectoralLogMessage>();
            info!(
                "Logs batch size: {} entries ({} bytes)",
                msgs.len(),
                batch_size
            );
            let is_last_batch = msgs.len() < limit as usize;

            for entry in msgs {
                last_id = entry.id;
                let mut row: ElectoralLogRow = entry
                    .try_into()
                    .map_err(|e| anyhow!("Error converting log entry to row: {e:?}"))?;
                row.message = row.message.replace('\n', " ").replace('\r', " ");
                csv_writer
                    .serialize(row)
                    .map_err(|e| anyhow!("Error serializing to CSV: {e:?}"))?;
            }

            if is_last_batch {
                break;
            }
        }

        csv_writer
            .flush()
            .map_err(|e| anyhow!("Error flushing CSV writer: {e:?}"))?;
        drop(csv_writer);

        Ok(temp_file)
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
            user_id,
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

impl TryFrom<ElectoralLogMessage> for ActivityLogRow {
    type Error = anyhow::Error;

    fn try_from(electoral_log: ElectoralLogMessage) -> Result<Self, Self::Error> {
        let user_id = match electoral_log.user_id {
            Some(user_id) => user_id.to_string(),
            None => "-".to_string(),
        };

        let statement_timestamp: String = if let Ok(datetime_parsed) =
            ISO8601::timestamp_secs_utc_to_date_opt(electoral_log.statement_timestamp)
        {
            datetime_parsed.to_rfc3339()
        } else {
            return Err(anyhow::anyhow!("Error parsing statement_timestamp"));
        };

        let created: String = if let Ok(datetime_parsed) =
            ISO8601::timestamp_secs_utc_to_date_opt(electoral_log.created)
        {
            datetime_parsed.to_rfc3339()
        } else {
            return Err(anyhow::anyhow!("Error parsing created"));
        };

        let deserialized_message = Message::strand_deserialize(&electoral_log.message)
            .map_err(|e| anyhow!("Error deserializing message: {e:?}"))?;

        let head_data = deserialized_message.statement.head.clone();
        let event_type = head_data.event_type.to_string();
        let log_type = head_data.log_type.to_string();
        let description = head_data.description;

        Ok(ActivityLogRow {
            id: electoral_log.id,
            user_id,
            created,
            statement_timestamp,
            statement_kind: electoral_log.statement_kind,
            event_type,
            log_type,
            description,
            message: deserialized_message.to_string(),
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
        let mut client = get_board_client().await?;
        let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;
        let board_name = get_event_board(
            self.ids.tenant_id.as_str(),
            self.ids.election_event_id.as_str(),
            &slug,
        );
        let total = client
            .count_electoral_log_messages(&board_name, None)
            .await
            .map_err(|e| anyhow!("Error counting electoral log messages: {e:?}"))?;
        Ok(Some(total))
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
        let mut electoral_log: Vec<ElectoralLogRow> = vec![];
        let mut client = get_board_client().await?;
        let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;
        let board_name = get_event_board(
            self.ids.tenant_id.as_str(),
            self.ids.election_event_id.as_str(),
            &slug,
        );
        // Uses offset-based pagination because the caller framework pre-computes
        // `offset = batch_index * limit` and processes batches in parallel (rayon),
        // which is incompatible with cursor-based pagination.
        let msgs = client
            .get_electoral_log_messages_at_offset(&board_name, limit, *offset)
            .await
            .map_err(|e| anyhow!("Failed to get electoral log messages batch: {e:?}"))?;
        info!("Format: {:#?}", self.report_format);
        for entry in msgs {
            match self.report_format {
                ReportFormat::PDF => {
                    act_log.push(entry.try_into()?);
                }
                ReportFormat::CSV => {
                    electoral_log.push(entry.try_into()?);
                }
            }
        }

        Ok(UserData {
            act_log,
            electoral_log,
        })
    }

    #[instrument(err, skip_all)]
    async fn prepare_user_data(
        &self,
        _hasura_transaction: &Transaction<'_>,
        _keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        Err(anyhow!(
            "prepare_user_data should not be used for this report type, use prepare_user_data_batch instead"
        ))
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
            // Generate CSV file using generate_export_csv_data
            let name = format!("export-election-event-logs-{}", election_event_id);
            let full_name = format!("{}.csv", name);
            let temp_file = self
                .generate_export_csv_data(&name)
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
                &full_name.clone(),
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

// Export data
#[instrument(err, skip(act_log))]
pub async fn generate_export_data(
    act_log: &[ElectoralLogRow],
    name: &str,
) -> Result<NamedTempFile> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::protocol_manager::get_event_board;
    use crate::services::reports::template_renderer::ReportOriginatedFrom;
    use chrono::Utc;
    use electoral_log::BoardClient;
    use sequent_core::util::external_config::load_external_config;
    use std::env;
    use std::io::BufRead;
    use std::process::Command;

    const NUM_LOGS: usize = 120_000;
    const STEP_CLI_DATA_DIR: &str = "/workspaces/step/packages/step-cli/data";
    const STEP_CLI_BIN: &str =
        "/workspaces/step/packages/step-cli/rust-local-target/release/step-cli";

    // Run: cargo test --release test_generate_export_csv_data_120k_memory -- --nocapture --ignored
    // To visualize results, open DHAT Viewer in a browser and upload the generated dhat-heap.json file.
    #[tokio::test]
    #[ignore]
    async fn test_generate_export_csv_data_120k_memory() -> Result<()> {
        let config = load_external_config(STEP_CLI_DATA_DIR)
            .map_err(|e| anyhow!("Failed to load external config: {e}"))?;
        let tenant_id = config.tenant_id;
        let election_event_id = config.election_event_id;

        let test_env_slug = format!("t{}", chrono::Utc::now().timestamp());
        env::set_var("ENV_SLUG", &test_env_slug);

        let immudb_user = env::var("IMMUDB_USER").context("IMMUDB_USER must be set")?;
        let immudb_password = env::var("IMMUDB_PASSWORD").context("IMMUDB_PASSWORD must be set")?;
        let immudb_server_url =
            env::var("IMMUDB_SERVER_URL").context("IMMUDB_SERVER_URL must be set")?;

        let board_name = get_event_board(&tenant_id, &election_event_id, &test_env_slug);
        println!("board_name: {board_name}");

        let mut board_client = BoardClient::new(&immudb_server_url, &immudb_user, &immudb_password)
            .await
            .map_err(|e| anyhow!("Failed to create BoardClient: {e:?}"))?;
        board_client
            .upsert_electoral_log_db(&board_name)
            .await
            .map_err(|e| anyhow!("Failed to create immudb database: {e:?}"))?;
        println!("Set up immudb database: {board_name}");

        let output = Command::new(STEP_CLI_BIN)
            .args([
                "step",
                "create-electoral-logs",
                "--working-directory",
                STEP_CLI_DATA_DIR,
                "--num-logs",
                &NUM_LOGS.to_string(),
            ])
            .env("ENV_SLUG", &test_env_slug)
            .env("IMMUDB_USER", &immudb_user)
            .env("IMMUDB_PASSWORD", &immudb_password)
            .env("IMMUDB_SERVER_URL", &immudb_server_url)
            .env("DEFAULT_SQL_BATCH_SIZE", "500")
            .env(
                "KC_DB_URL_HOST",
                env::var("KC_DB_URL_HOST").context("KC_DB_URL_HOST must be set")?,
            )
            .env(
                "KC_DB_URL_PORT",
                env::var("KC_DB_URL_PORT").context("KC_DB_URL_PORT must be set")?,
            )
            .env(
                "KC_DB_USERNAME",
                env::var("KC_DB_USERNAME").context("KC_DB_USERNAME must be set")?,
            )
            .env(
                "KC_DB_PASSWORD",
                env::var("KC_DB_PASSWORD").context("KC_DB_PASSWORD must be set")?,
            )
            .env("KC_DB", env::var("KC_DB").context("KC_DB must be set")?)
            .output()
            .map_err(|e| anyhow!("Failed to run step-cli: {e:?}"))?;

        assert!(
            output.status.success(),
            "step-cli failed with status: {}",
            output.status
        );

        let ids = ReportOrigins {
            tenant_id: tenant_id.clone(),
            election_event_id: election_event_id.clone(),
            election_id: None,
            template_alias: None,
            voter_id: None,
            report_origin: ReportOriginatedFrom::ReportsTab,
            executer_username: None,
            tally_session_id: None,
        };
        let template = ActivityLogsTemplate::new(ids, ReportFormat::CSV);
        let name = format!("test-export-{election_event_id}");

        // Start the profiler here so stats reflect only generate_export_csv_data
        let _profiler = dhat::Profiler::new_heap();
        let temp_file = template
            .generate_export_csv_data(&name)
            .await
            .map_err(|e| anyhow!("generate_export_csv_data failed: {e:?}"))?;
        let stats = dhat::HeapStats::get();

        println!("Peak live heap:   {} bytes", stats.max_bytes);
        println!(
            "Total allocated:  {} bytes in {} blocks",
            stats.total_bytes, stats.total_blocks,
        );
        println!(
            "Current live:     {} bytes in {} blocks",
            stats.curr_bytes, stats.curr_blocks
        );

        let metadata = std::fs::metadata(temp_file.path())
            .map_err(|e| anyhow!("Failed to get temp file metadata: {e:?}"))?;
        println!("CSV file size:    {} bytes", metadata.len());

        let file = std::fs::File::open(temp_file.path())
            .map_err(|e| anyhow!("Failed to open temp file: {e:?}"))?;
        let line_count = std::io::BufReader::new(file).lines().count();
        println!(
            "CSV line count:   {} lines ({} data rows)",
            line_count,
            line_count.saturating_sub(1)
        );
        assert_eq!(
            line_count - 1,
            NUM_LOGS,
            "expected {NUM_LOGS} data rows but got {}",
            line_count - 1
        );

        // _profiler drops here → writes dhat-heap.json in the working directory
        Ok(())
    }
}
