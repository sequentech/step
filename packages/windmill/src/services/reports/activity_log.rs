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
        info!("prepare_user_data_batch: offset = {offset}, limit = {limit}");
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

        let total = electoral_logs.total.aggregate.count;

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

        info!("prepare_user_data: initial limit = {limit}, offset = {offset}");

        loop {
            info!("prepare_user_data loop: iteration with offset = {offset}, limit = {limit}");
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
    use std::env;
    use std::process::Command;

    const TENANT_ID: &str = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
    const ELECTION_EVENT_ID: &str = "bb6eabc3-e66b-4201-bfef-6d60544fa803";
    const NUM_LOGS: usize = 120_000;

    #[tokio::test]
    #[ignore]
    async fn test_prepare_user_data_120k() -> Result<()> {
        // Use a unique slug per run to get a clean database (immudb delete is unreliable)
        let test_env_slug = format!("t{}", Utc::now().timestamp());
        env::set_var("ENV_SLUG", &test_env_slug);

        let immudb_user = env::var("IMMUDB_USER").context("IMMUDB_USER must be set")?;
        let immudb_password = env::var("IMMUDB_PASSWORD").context("IMMUDB_PASSWORD must be set")?;
        let immudb_server_url =
            env::var("IMMUDB_SERVER_URL").context("IMMUDB_SERVER_URL must be set")?;

        let board_name = get_event_board(TENANT_ID, ELECTION_EVENT_ID, &test_env_slug);
        println!("board_name: {board_name}");

        // Set up immudb database (unique slug ensures a fresh database each run)
        let mut board_client = BoardClient::new(&immudb_server_url, &immudb_user, &immudb_password)
            .await
            .map_err(|e| anyhow!("Failed to create BoardClient: {e:?}"))?;
        board_client
            .upsert_electoral_log_db(&board_name)
            .await
            .map_err(|e| anyhow!("Failed to create immudb database: {e:?}"))?;
        println!("Set up immudb database: {board_name}");

        // Seed electoral logs using step-cli binary
        let step_cli_bin = "/workspaces/step/packages/step-cli/rust-local-target/release/step-cli";
        let working_dir = "/workspaces/step/packages/step-cli/data";
        let output = Command::new(step_cli_bin)
            .args([
                "step",
                "create-electoral-logs",
                "--working-directory",
                working_dir,
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

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("step-cli stdout: {stdout}");
        println!("step-cli stderr: {stderr}");
        assert!(
            output.status.success(),
            "step-cli failed with status: {}",
            output.status
        );

        // Create ActivityLogsTemplate with matching IDs
        let ids = ReportOrigins {
            tenant_id: TENANT_ID.to_string(),
            election_event_id: ELECTION_EVENT_ID.to_string(),
            election_id: None,
            template_alias: None,
            voter_id: None,
            report_origin: ReportOriginatedFrom::ReportsTab,
            executer_username: None,
            tally_session_id: None,
        };
        let template = ActivityLogsTemplate::new(ids, ReportFormat::CSV);

        // Create dummy deadpool postgres transactions (unused by prepare_user_data)
        let hasura_pg_host = env::var("HASURA_PG_HOST").context("HASURA_PG_HOST must be set")?;
        let hasura_pg_port: u16 = env::var("HASURA_PG_PORT")
            .context("HASURA_PG_PORT must be set")?
            .parse()
            .context("HASURA_PG_PORT must be a valid port number")?;
        let hasura_pg_user = env::var("HASURA_PG_USER").context("HASURA_PG_USER must be set")?;
        let hasura_pg_password =
            env::var("HASURA_PG_PASSWORD").context("HASURA_PG_PASSWORD must be set")?;
        let hasura_pg_dbname =
            env::var("HASURA_PG_DBNAME").context("HASURA_PG_DBNAME must be set")?;

        let mut hasura_cfg = deadpool_postgres::Config::new();
        hasura_cfg.host = Some(hasura_pg_host);
        hasura_cfg.port = Some(hasura_pg_port);
        hasura_cfg.user = Some(hasura_pg_user);
        hasura_cfg.password = Some(hasura_pg_password);
        hasura_cfg.dbname = Some(hasura_pg_dbname);
        let hasura_pool = hasura_cfg
            .create_pool(
                Some(deadpool_postgres::Runtime::Tokio1),
                tokio_postgres::NoTls,
            )
            .map_err(|e| anyhow!("Failed to create hasura pool: {e:?}"))?;
        let mut hasura_client = hasura_pool
            .get()
            .await
            .map_err(|e| anyhow!("Failed to get hasura client: {e:?}"))?;
        let hasura_tx = hasura_client
            .transaction()
            .await
            .map_err(|e| anyhow!("Failed to start hasura transaction: {e:?}"))?;

        let kc_db_host = env::var("KC_DB_URL_HOST").context("KC_DB_URL_HOST must be set")?;
        let kc_db_port: u16 = env::var("KC_DB_URL_PORT")
            .context("KC_DB_URL_PORT must be set")?
            .parse()
            .context("KC_DB_URL_PORT must be a valid port number")?;
        let kc_db_user = env::var("KC_DB_USERNAME").context("KC_DB_USERNAME must be set")?;
        let kc_db_password = env::var("KC_DB_PASSWORD").context("KC_DB_PASSWORD must be set")?;
        let kc_db_dbname = env::var("KC_DB_URL_DATABASE").unwrap_or("postgres".to_string());

        let mut kc_cfg = deadpool_postgres::Config::new();
        kc_cfg.host = Some(kc_db_host);
        kc_cfg.port = Some(kc_db_port);
        kc_cfg.user = Some(kc_db_user);
        kc_cfg.password = Some(kc_db_password);
        kc_cfg.dbname = Some(kc_db_dbname);
        let kc_pool = kc_cfg
            .create_pool(
                Some(deadpool_postgres::Runtime::Tokio1),
                tokio_postgres::NoTls,
            )
            .map_err(|e| anyhow!("Failed to create keycloak pool: {e:?}"))?;
        let mut kc_client = kc_pool
            .get()
            .await
            .map_err(|e| anyhow!("Failed to get keycloak client: {e:?}"))?;
        let kc_tx = kc_client
            .transaction()
            .await
            .map_err(|e| anyhow!("Failed to start keycloak transaction: {e:?}"))?;

        // Call prepare_user_data
        let user_data = template
            .prepare_user_data(&hasura_tx, &kc_tx)
            .await
            .map_err(|e| anyhow!("prepare_user_data failed: {e:?}"))?;

        println!("act_log.len() = {}", user_data.act_log.len());
        println!("electoral_log.len() = {}", user_data.electoral_log.len());

        assert_eq!(user_data.act_log.len(), NUM_LOGS);
        assert_eq!(user_data.electoral_log.len(), NUM_LOGS);

        Ok(())
    }
}
