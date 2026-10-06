// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::generation::{
    attach_report_manifest, write_report_manifest, GeneratedFile, ReportRequester,
};
use super::report_variables::configuration_stamp_without_template;
use super::template_renderer::*;
use super::template_time::load_i18n_defaults;
use crate::postgres::election::get_elections;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::reports::{Report, ReportFormat as OutputFormat, ReportType};
use crate::services::documents::upload_and_return_document_with_annotations;
use crate::services::electoral_log::{
    count_electoral_log, ElectoralLogRow, GetElectoralLogBody, MinuteRange, OrderField,
    IMMUDB_ROWS_LIMIT,
};
use crate::services::protocol_manager::{get_board_client, get_event_board};
use crate::services::providers::email_sender::{Attachment, EmailSender};
use crate::services::time_zones::parse_zone;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, SecondsFormat, Utc};
use chrono_tz::Tz;
use csv::WriterBuilder;
use deadpool_postgres::Transaction;
use electoral_log::messages::message::Message;
use electoral_log::ElectoralLogMessage;
use sequent_core::ballot::{ElectionEventPresentation, ElectionPresentation};
use sequent_core::election_config::manifest::ConfigurationStamp;
use sequent_core::services::reports::{template_time_variables, DateTimeZoneStyle, TimeZoneTexts};
use sequent_core::services::s3::get_minio_url;
use sequent_core::time_zones::log_time_zone;
use sequent_core::types::hasura::core::{DocumentAnnotations, TasksExecution};
use sequent_core::types::templates::{ReportExtraConfig, SendTemplateBody};
use sequent_core::util::temp_path::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::mem;
use std::path::Path;
use strand::serialization::StrandDeserialize;
use strum_macros::EnumString;
use tempfile::NamedTempFile;
use tokio::sync::OnceCell;
use tracing::{debug, info, instrument, warn};

#[derive(Serialize, Deserialize, Debug, Clone, EnumString, PartialEq, Copy)]
pub enum ReportFormat {
    CSV,
    PDF,
}

/// What the Logs tab's export dialog asks for: a range of `created`
/// (RFC 3339 instants, inclusive to the minute) and the zone to show the
/// times in. Absent fields: every row, each in its log zone.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct ActivityLogExportOptions {
    #[serde(default)]
    pub created_from: Option<String>,
    #[serde(default)]
    pub created_to: Option<String>,
    #[serde(default)]
    pub time_zone: Option<String>,
}

impl ActivityLogExportOptions {
    /// The range of `created` to export.
    pub fn range(&self) -> Result<MinuteRange> {
        MinuteRange::parse(self.created_from.as_deref(), self.created_to.as_deref())
    }

    /// The chosen zone, if any.
    pub fn zone(&self) -> Result<Option<Tz>> {
        self.time_zone
            .as_deref()
            .map(str::trim)
            .filter(|zone| !zone.is_empty())
            .map(parse_zone)
            .transpose()
    }
}

/// The zone each exported row is shown in: the zone the export chose, else
/// the log zone (`sequent_core::time_zones::log_time_zone`: the primary, or
/// with the ELECTION policy the row's election zone).
#[derive(Debug, Clone, PartialEq)]
pub struct LogZones {
    chosen: Option<Tz>,
    event: Tz,
    elections: HashMap<String, Tz>,
    texts: TimeZoneTexts,
}

/// A stored zone name chrono-tz can't read: UTC, as the resolver's last
/// fallback, with a warning.
fn zone_or_utc(name: &str) -> Tz {
    parse_zone(name).unwrap_or_else(|err| {
        warn!("{err:?}; showing log times in UTC");
        Tz::UTC
    })
}

impl LogZones {
    pub fn new(
        chosen: Option<Tz>,
        event: Option<&ElectionEventPresentation>,
        elections: &[(String, Option<ElectionPresentation>)],
    ) -> Self {
        Self::with_defaults(chosen, event, elections, &serde_json::Value::Null)
    }

    fn with_defaults(
        chosen: Option<Tz>,
        event: Option<&ElectionEventPresentation>,
        elections: &[(String, Option<ElectionPresentation>)],
        defaults: &serde_json::Value,
    ) -> Self {
        LogZones {
            chosen,
            texts: TimeZoneTexts::from_variables(&template_time_variables(event, None, defaults)),
            event: zone_or_utc(&log_time_zone(event, None)),
            elections: elections
                .iter()
                .map(|(id, presentation)| {
                    (
                        id.clone(),
                        zone_or_utc(&log_time_zone(event, presentation.as_ref())),
                    )
                })
                .collect(),
        }
    }

    /// Uses the shared template formatter's defaults and scoped overrides.
    pub fn label(&self, zone: Tz, instant: DateTime<Utc>) -> String {
        self.texts.label(zone.name(), instant)
    }

    /// "April 09, 2028 06:00:03 PhST": a time in `zone` with its label.
    pub fn labelled(&self, ts: i64, zone: Tz, what: &str) -> Result<String> {
        let at = instant(ts, what)?;
        Ok(self.texts.date_time_zone(
            at,
            zone.name(),
            "%B %d, %Y %H:%M:%S",
            DateTimeZoneStyle::Label,
        ))
    }

    /// The zone of a row of `election_id` (event-wide rows: the event's).
    pub fn zone_of(&self, election_id: Option<&str>) -> Tz {
        self.chosen.unwrap_or_else(|| {
            election_id
                .and_then(|id| self.elections.get(id))
                .copied()
                .unwrap_or(self.event)
        })
    }
}

/// A Unix time in seconds as an instant.
fn instant(ts: i64, what: &str) -> Result<DateTime<Utc>> {
    DateTime::<Utc>::from_timestamp(ts, 0).ok_or_else(|| anyhow!("Error parsing {what}: {ts}"))
}

/// RFC 3339 in `zone`'s offset: "2028-04-09T06:00:03+08:00".
fn rfc3339_in(ts: i64, zone: Tz, what: &str) -> Result<String> {
    Ok(instant(ts, what)?
        .with_timezone(&zone)
        .to_rfc3339_opts(SecondsFormat::Secs, false))
}

/// A row of the PDF. The template prints `created_label` and
/// `statement_timestamp_label` (the time in the row's zone with the zone's
/// label), their UTC instants and the IANA name. `created` and
/// `statement_timestamp` stay RFC 3339 for custom templates.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActivityLogRow {
    id: i64,
    created: String,
    created_label: String,
    #[serde(default)]
    created_utc: String,
    statement_timestamp: String,
    statement_timestamp_label: String,
    #[serde(default)]
    statement_timestamp_utc: String,
    statement_kind: String,
    event_type: String,
    log_type: String,
    description: String,
    message: String,
    user_id: String,
    time_zone: String,
}

/// A row of the CSV export: the board row, then the head's event type and
/// log type, then `created` as an ISO 8601 UTC instant, as the time in the
/// row's zone (RFC 3339 with its offset) and the zone's IANA name. New
/// columns go at the end, and the importer reads the columns it needs by
/// name, so files with and without them import.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ElectoralLogCsvRow {
    pub id: i64,
    pub created: i64,
    pub statement_timestamp: i64,
    pub statement_kind: String,
    pub message: String,
    pub data: String,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub event_type: String,
    pub log_type: String,
    pub created_utc: String,
    pub created_local: String,
    pub time_zone: String,
}

impl ElectoralLogCsvRow {
    pub fn new(entry: ElectoralLogMessage, zones: &LogZones) -> Result<Self> {
        let zone = zones.zone_of(entry.election_id.as_deref());
        let row = ElectoralLogRow::try_from(entry)?;
        let head = row
            .statement_head_data()
            .context("Error reading the statement head")?;
        Ok(ElectoralLogCsvRow {
            id: row.id,
            created: row.created,
            statement_timestamp: row.statement_timestamp,
            statement_kind: row.statement_kind,
            message: row.message.replace('\n', " ").replace('\r', " "),
            data: row.data,
            user_id: row.user_id,
            username: row.username,
            event_type: head.event_type,
            log_type: head.log_type,
            created_utc: instant(row.created, "created")?
                .to_rfc3339_opts(SecondsFormat::Secs, true),
            created_local: rfc3339_in(row.created, zone, "created")?,
            time_zone: zone.name().to_string(),
        })
    }
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
    formats: Vec<OutputFormat>,
    options: ActivityLogExportOptions,
    zones: OnceCell<LogZones>,
}

impl ActivityLogsTemplate {
    /// An export in the one format its caller chose.
    pub fn new(ids: ReportOrigins, report_format: ReportFormat) -> Self {
        ActivityLogsTemplate {
            ids,
            report_format,
            formats: vec![match report_format {
                ReportFormat::CSV => OutputFormat::Csv,
                ReportFormat::PDF => OutputFormat::Pdf,
            }],
            options: ActivityLogExportOptions::default(),
            zones: OnceCell::new(),
        }
    }

    /// The report in the formats its definition asks for.
    pub fn in_formats(ids: ReportOrigins, formats: Vec<OutputFormat>) -> Self {
        ActivityLogsTemplate {
            formats,
            ..Self::new(ids, ReportFormat::PDF)
        }
    }

    fn export_name(&self) -> String {
        format!("export-election-event-logs-{}", self.ids.election_event_id)
    }

    /// The export's range as the log list's filter (inclusive to the minute).
    fn range_filter(&self) -> Option<HashMap<OrderField, String>> {
        let mut filter = HashMap::new();
        let bounds = [
            (OrderField::CreatedFrom, &self.options.created_from),
            (OrderField::CreatedTo, &self.options.created_to),
        ];
        for (field, value) in bounds {
            if let Some(value) = value.as_ref().filter(|value| !value.trim().is_empty()) {
                filter.insert(field, value.clone());
            }
        }
        (!filter.is_empty()).then_some(filter)
    }

    /// The Logs tab's export: a range and a zone.
    pub fn with_options(mut self, options: ActivityLogExportOptions) -> Self {
        self.options = options;
        self
    }

    /// The zone of each row, read once per export.
    #[instrument(err, skip_all)]
    async fn log_zones(&self, hasura_transaction: &Transaction<'_>) -> Result<&LogZones> {
        self.zones
            .get_or_try_init(|| async {
                let chosen = self.options.zone()?;
                let tenant_id = self.ids.tenant_id.as_str();
                let election_event_id = self.ids.election_event_id.as_str();
                let event =
                    get_election_event_by_id(hasura_transaction, tenant_id, election_event_id)
                        .await
                        .context("Error reading the election event")?;
                // A presentation that doesn't read never blocks the export:
                // its rows show in UTC, the resolver's last fallback.
                let event_presentation = event.get_presentation().unwrap_or_else(|err| {
                    warn!("Error reading the event presentation: {err:?}");
                    None
                });
                let defaults = load_i18n_defaults().await;
                if chosen.is_some() {
                    // Every row in the chosen zone; the event gives the labels.
                    return Ok(LogZones::with_defaults(
                        chosen,
                        event_presentation.as_ref(),
                        &[],
                        &defaults,
                    ));
                }
                let elections: Vec<(String, Option<ElectionPresentation>)> =
                    get_elections(hasura_transaction, tenant_id, election_event_id)
                        .await
                        .context("Error reading the elections")?
                        .into_iter()
                        .map(|election| {
                            let presentation = election.get_presentation();
                            (election.id, presentation)
                        })
                        .collect();
                Ok(LogZones::with_defaults(
                    None,
                    event_presentation.as_ref(),
                    &elections,
                    &defaults,
                ))
            })
            .await
    }

    /// The CSV of every row with the zone columns in UTC, as the election
    /// event export (a backup the importer reads) writes it.
    pub async fn generate_export_csv_data(&self, name: &str) -> Result<NamedTempFile> {
        self.generate_export_csv_data_in(name, &LogZones::new(Some(Tz::UTC), None, &[]))
            .await
    }

    /// The CSV of the rows in the export's range, each row's time in its
    /// zone, streamed from the electoral-log board in batches.
    #[instrument(err, skip(self, zones))]
    pub async fn generate_export_csv_data_in(
        &self,
        name: &str,
        zones: &LogZones,
    ) -> Result<NamedTempFile> {
        let mut temp_file =
            generate_temp_file(name, ".csv").with_context(|| "Error creating named temp file")?;
        let mut csv_writer = WriterBuilder::new().from_writer(temp_file.as_file_mut());
        self.export_rows(zones, |row| {
            csv_writer
                .serialize(row)
                .map_err(|e| anyhow!("Error serializing to CSV: {e:?}"))
        })
        .await?;
        csv_writer
            .flush()
            .map_err(|e| anyhow!("Error flushing CSV writer: {e:?}"))?;
        drop(csv_writer);

        Ok(temp_file)
    }

    /// The same rows as PostgreSQL statements: the table and an `INSERT` for
    /// each row, in one transaction. Its first line names the configuration
    /// of an event imported from a signed one.
    #[instrument(err, skip(self, zones, stamp))]
    pub async fn generate_export_sql_data_in(
        &self,
        name: &str,
        zones: &LogZones,
        stamp: Option<&ConfigurationStamp>,
    ) -> Result<NamedTempFile> {
        let mut temp_file =
            generate_temp_file(name, ".sql").with_context(|| "Error creating named temp file")?;
        let file = temp_file.as_file_mut();
        file.write_all(sql_header(stamp).as_bytes())
            .context("Error writing the SQL export")?;
        self.export_rows(zones, |row| {
            file.write_all(sql_insert(&row).as_bytes())
                .context("Error writing the SQL export")
        })
        .await?;
        file.write_all(SQL_FOOTER.as_bytes())
            .context("Error writing the SQL export")?;
        file.flush().context("Error flushing the SQL export")?;

        Ok(temp_file)
    }

    /// Each row in the export's range, in the order of the board, streamed
    /// from it in batches.
    async fn export_rows(
        &self,
        zones: &LogZones,
        mut write: impl FnMut(ElectoralLogCsvRow) -> Result<()>,
    ) -> Result<()> {
        let range = self.options.range()?;
        let limit = IMMUDB_ROWS_LIMIT as i64;
        let mut last_id: i64 = 0;
        let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;
        let board_name = get_event_board(
            self.ids.tenant_id.as_str(),
            self.ids.election_event_id.as_str(),
            &slug,
        );

        let mut board_client = get_board_client().await?;

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
                if !range.contains(entry.created) {
                    continue;
                }
                let row = ElectoralLogCsvRow::new(entry, zones)
                    .map_err(|e| anyhow!("Error converting log entry to row: {e:?}"))?;
                write(row)?;
            }

            if is_last_batch {
                break;
            }
        }

        Ok(())
    }
}

/// The table an SQL export of the log fills.
pub const SQL_TABLE: &str = "electoral_log";
const SQL_FOOTER: &str = "COMMIT;\n";

/// A text as an SQL literal. PostgreSQL text holds no NUL.
fn sql_text(value: &str) -> String {
    format!("'{}'", value.replace('\0', "").replace('\'', "''"))
}

fn sql_optional_text(value: Option<&str>) -> String {
    value.map(sql_text).unwrap_or_else(|| "NULL".to_string())
}

/// What an SQL export starts with: the configuration it came from, as a
/// comment, for an event imported from a signed one, and the table.
pub fn sql_header(stamp: Option<&ConfigurationStamp>) -> String {
    let comment = stamp
        .map(|stamp| format!("-- {}\n", stamp.line()))
        .unwrap_or_default();
    format!(
        "{comment}BEGIN;\n\
         CREATE TABLE {SQL_TABLE} (\n  \
         id BIGINT PRIMARY KEY,\n  \
         created BIGINT NOT NULL,\n  \
         statement_timestamp BIGINT NOT NULL,\n  \
         statement_kind TEXT NOT NULL,\n  \
         message TEXT NOT NULL,\n  \
         data TEXT NOT NULL,\n  \
         user_id TEXT,\n  \
         username TEXT,\n  \
         event_type TEXT NOT NULL,\n  \
         log_type TEXT NOT NULL,\n  \
         created_utc TIMESTAMPTZ NOT NULL,\n  \
         created_local TEXT NOT NULL,\n  \
         time_zone TEXT NOT NULL\n\
         );\n"
    )
}

/// A row of the log as its `INSERT`, on one line.
pub fn sql_insert(row: &ElectoralLogCsvRow) -> String {
    format!(
        "INSERT INTO {SQL_TABLE} VALUES ({}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {});\n",
        row.id,
        row.created,
        row.statement_timestamp,
        sql_text(&row.statement_kind),
        sql_text(&row.message),
        sql_text(&row.data.replace('\n', " ").replace('\r', " ")),
        sql_optional_text(row.user_id.as_deref()),
        sql_optional_text(row.username.as_deref()),
        sql_text(&row.event_type),
        sql_text(&row.log_type),
        sql_text(&row.created_utc),
        sql_text(&row.created_local),
        sql_text(&row.time_zone),
    )
}

impl ActivityLogRow {
    pub fn new(electoral_log: ElectoralLogMessage, zones: &LogZones) -> Result<Self> {
        let zone = zones.zone_of(electoral_log.election_id.as_deref());
        let user_id = match electoral_log.user_id {
            Some(user_id) => user_id.to_string(),
            None => "-".to_string(),
        };

        let statement_timestamp = rfc3339_in(
            electoral_log.statement_timestamp,
            zone,
            "statement_timestamp",
        )?;
        let created = rfc3339_in(electoral_log.created, zone, "created")?;
        let statement_timestamp_label = zones.labelled(
            electoral_log.statement_timestamp,
            zone,
            "statement_timestamp",
        )?;
        let created_label = zones.labelled(electoral_log.created, zone, "created")?;
        let created_utc =
            instant(electoral_log.created, "created")?.to_rfc3339_opts(SecondsFormat::Secs, true);
        let statement_timestamp_utc =
            instant(electoral_log.statement_timestamp, "statement_timestamp")?
                .to_rfc3339_opts(SecondsFormat::Secs, true);

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
            created_label,
            created_utc,
            statement_timestamp,
            statement_timestamp_label,
            statement_timestamp_utc,
            statement_kind: electoral_log.statement_kind,
            event_type,
            log_type,
            description,
            message: deserialized_message.to_string(),
            time_zone: zone.name().to_string(),
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
        // A ranged export counts the rows in its range, so each batch has rows.
        if let Some(filter) = self.range_filter() {
            let total = count_electoral_log(GetElectoralLogBody {
                tenant_id: self.ids.tenant_id.clone(),
                election_event_id: self.ids.election_event_id.clone(),
                filter: Some(filter),
                ..Default::default()
            })
            .await
            .map_err(|e| anyhow!("Error counting electoral log messages in range: {e:?}"))?;
            return Ok(Some(total));
        }
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
        hasura_transaction: &Transaction<'_>,
        _keycloak_transaction: &Transaction<'_>,
        offset: &mut i64,
        limit: i64,
    ) -> Result<Self::UserData> {
        let mut act_log: Vec<ActivityLogRow> = vec![];
        let mut electoral_log: Vec<ElectoralLogRow> = vec![];
        let range = self.options.range()?;
        let zones = self.log_zones(hasura_transaction).await?;
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
        let msgs = if range.is_unbounded() {
            client
                .get_electoral_log_messages_at_offset(&board_name, limit, *offset)
                .await
        } else {
            // The batch's rows within the range (the count was of the range too).
            client
                .get_electoral_log_messages_filtered(
                    &board_name,
                    None,
                    range.start,
                    range.end.map(|end| end - 1),
                    Some(limit),
                    Some(*offset),
                    Some(HashMap::from([("id".to_string(), "asc".to_string())])),
                )
                .await
        }
        .map_err(|e| anyhow!("Failed to get electoral log messages batch: {e:?}"))?;
        info!("Format: {:#?}", self.report_format);
        for entry in msgs {
            if !range.contains(entry.created) {
                continue;
            }
            match self.report_format {
                ReportFormat::PDF => {
                    act_log.push(ActivityLogRow::new(entry, zones)?);
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

    /// The formats it was built for: the one an export from the Logs tab
    /// asked for, or those the Reports tab read from its report.
    fn output_formats(&self, _report: Option<&Report>) -> Result<Vec<OutputFormat>> {
        Ok(self.formats.clone())
    }

    fn requested_by(&self) -> Option<String> {
        self.ids.executer_username.clone()
    }

    #[instrument(err, skip_all)]
    async fn write_format(
        &self,
        format: OutputFormat,
        hasura_transaction: &Transaction<'_>,
        stamp: Option<&ConfigurationStamp>,
        directory: &Path,
    ) -> Result<Option<GeneratedFile>> {
        let name = self.export_name();
        let zones = self.log_zones(hasura_transaction).await?;
        let temp_file = match format {
            // A line that is not a row would be read as one by the importer
            // of this file: its configuration is named in the hash manifest
            // stored with it.
            OutputFormat::Csv => self.generate_export_csv_data_in(&name, zones).await,
            OutputFormat::Sql => self.generate_export_sql_data_in(&name, zones, stamp).await,
            OutputFormat::Pdf | OutputFormat::Xml => return Ok(None),
        }
        .map_err(|e| anyhow!("Error generating export data: {e:?}"))?;
        let full_name = format!("{name}.{}", format.extension());
        let path = directory.join(&full_name);
        std::fs::copy(temp_file.path(), &path)
            .with_context(|| format!("Error writing {full_name}"))?;
        Ok(Some(GeneratedFile {
            name: full_name,
            path: path.to_string_lossy().to_string(),
            media_type: format.media_type().to_string(),
        }))
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
        may_read_secret_attributes: bool,
    ) -> Result<()> {
        if self.report_format == ReportFormat::PDF {
            // Read the zones now: the batches render on a worker pool that
            // must not wait on this transaction.
            self.log_zones(hasura_transaction).await?;
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
                may_read_secret_attributes,
            )
            .await
        } else {
            // Generate CSV file using generate_export_csv_data
            let name = format!("export-election-event-logs-{}", election_event_id);
            let full_name = format!("{}.csv", name);
            let zones = self.log_zones(hasura_transaction).await?;
            let temp_file = self
                .generate_export_csv_data_in(&name, zones)
                .await
                .map_err(|e| anyhow!("Error generating export data: {e:?}"))?;

            // Upload document
            let temp_path = temp_file.into_temp_path();
            let temp_path_string = temp_path.to_string_lossy().to_string();
            let file_size =
                get_file_size(&temp_path_string).with_context(|| "Error obtaining file size")?;

            // A CSV has no footer to print the configuration in: its
            // document's hash manifest names it.
            let stamp = configuration_stamp_without_template(
                hasura_transaction,
                tenant_id,
                election_event_id,
            )
            .await?;
            let report_manifest = stamp
                .as_ref()
                .map(|stamp| {
                    write_report_manifest(
                        &self.get_report_type(),
                        stamp,
                        &[GeneratedFile {
                            name: full_name.clone(),
                            path: temp_path_string.clone(),
                            media_type: OutputFormat::Csv.media_type().to_string(),
                        }],
                    )
                })
                .transpose()?;
            let mut annotations = DocumentAnnotations::default();
            if let Some(written) = &report_manifest {
                attach_report_manifest(
                    hasura_transaction,
                    tenant_id,
                    election_event_id,
                    written,
                    &ReportRequester::named(self.requested_by()),
                    &mut annotations,
                )
                .await?;
            }

            let _document = upload_and_return_document_with_annotations(
                hasura_transaction,
                &temp_path_string.clone(),
                file_size,
                "text/csv",
                tenant_id,
                Some(election_event_id.to_string()),
                &full_name.clone(),
                Some(document_id.to_string()),
                false,
                &annotations,
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

    use sequent_core::ballot::{ElectionEventTimeZones, LogTimeZonePolicy};

    /// A board entry of `election_id`, created at `created`.
    fn entry(election_id: Option<&str>, created: &str) -> ElectoralLogMessage {
        use electoral_log::messages::message::SigningData;
        use electoral_log::messages::newtypes::{
            EventIdString, SigningLogEntry, SigningStatementKind,
        };
        use electoral_log::messages::statement::{StatementEventType, StatementLogType};
        use strand::signature::StrandSignatureSk;

        let sk = StrandSignatureSk::generate().unwrap();
        let sd = SigningData::new(sk.clone(), "", sk);
        let message = Message::signing_message(
            EventIdString("event".to_string()),
            SigningLogEntry {
                kind: SigningStatementKind::SigningActionExecuted,
                event_type: StatementEventType::SYSTEM,
                log_type: StatementLogType::INFO,
                description: "Opened voting on schedule".to_string(),
                details_json: "{}".to_string(),
                step_id: "2b7c9e40-1f5d-4a8e-9c3b-6d2e1f0a9b87".to_string(),
            },
            1_841_397_365,
            &sd,
            None,
            None,
            election_id.map(str::to_string),
            None,
        )
        .unwrap();
        let mut entry = ElectoralLogMessage::try_from(&message).unwrap();
        entry.created = DateTime::parse_from_rfc3339(created).unwrap().timestamp();
        entry
    }

    fn event(
        configured: &[&str],
        primary: &str,
        logs: LogTimeZonePolicy,
    ) -> ElectionEventPresentation {
        ElectionEventPresentation {
            timezones: Some(ElectionEventTimeZones {
                configured: configured.iter().map(|zone| zone.to_string()).collect(),
                primary: primary.to_string(),
                logs,
            }),
            ..Default::default()
        }
    }

    fn election(zone: Option<&str>) -> Option<ElectionPresentation> {
        Some(ElectionPresentation {
            timezone: zone.map(str::to_string),
            ..Default::default()
        })
    }

    /// The two configurations: a Manila primary with the logs in the primary
    /// (the COMELEC preset), and a Madrid primary with a Canary Islands
    /// office and the logs in each election's zone (the Madrid association).
    /// Expectations come from the configuration.
    fn configurations() -> Vec<(
        ElectionEventPresentation,
        Vec<(String, Option<ElectionPresentation>)>,
    )> {
        vec![
            (
                event(
                    &["Asia/Manila", "Asia/Dubai"],
                    "Asia/Manila",
                    LogTimeZonePolicy::PRIMARY,
                ),
                vec![("dubai".to_string(), election(Some("Asia/Dubai")))],
            ),
            (
                event(
                    &["Europe/Madrid", "Atlantic/Canary"],
                    "Europe/Madrid",
                    LogTimeZonePolicy::ELECTION,
                ),
                vec![
                    ("canary".to_string(), election(Some("Atlantic/Canary"))),
                    ("madrid".to_string(), election(None)),
                ],
            ),
        ]
    }

    #[test]
    fn each_row_is_shown_in_its_log_zone() {
        for (event, elections) in configurations() {
            let zones = LogZones::new(None, Some(&event), &elections);
            let timezones = event.timezones.clone().unwrap();
            for (id, presentation) in &elections {
                let expected = match timezones.logs {
                    LogTimeZonePolicy::PRIMARY => timezones.primary.clone(),
                    LogTimeZonePolicy::ELECTION => presentation
                        .as_ref()
                        .and_then(|p| p.timezone.clone())
                        .unwrap_or(timezones.primary.clone()),
                };
                assert_eq!(zones.zone_of(Some(id)).name(), expected);
            }
            // Event-wide rows and unknown elections: the primary.
            assert_eq!(zones.zone_of(None).name(), timezones.primary);
            assert_eq!(zones.zone_of(Some("gone")).name(), timezones.primary);
            // A zone chosen in the export dialog applies to every row.
            let chosen = LogZones::new(Some(Tz::America__New_York), Some(&event), &elections);
            assert_eq!(chosen.zone_of(Some(&elections[0].0)), Tz::America__New_York);
        }
    }

    #[test]
    fn the_csv_adds_the_utc_instant_the_zone_time_and_the_zone_name() {
        for (event, elections) in configurations() {
            let zones = LogZones::new(None, Some(&event), &elections);
            let election_id = elections[0].0.as_str();
            let zone = zones.zone_of(Some(election_id));
            let row =
                ElectoralLogCsvRow::new(entry(Some(election_id), "2028-04-08T22:00:03Z"), &zones)
                    .unwrap();

            assert_eq!(row.created_utc, "2028-04-08T22:00:03Z");
            let local = DateTime::parse_from_rfc3339(&row.created_local).unwrap();
            assert_eq!(local.timestamp(), row.created);
            let expected_offset = crate::services::time_zones::offset_seconds_at(
                zone,
                DateTime::parse_from_rfc3339("2028-04-08T22:00:03Z")
                    .unwrap()
                    .with_timezone(&Utc),
            );
            assert_eq!(local.offset().local_minus_utc(), expected_offset);
            assert_eq!(row.time_zone, zone.name());
        }
    }

    #[test]
    fn the_pdf_rows_carry_the_zone_offset_and_name() {
        let (event, elections) = configurations().remove(0);
        let defaults = serde_json::json!({"en": {"timezones": {"abbr": {"Asia/Manila": "PhST"}}}});
        let zones = LogZones::with_defaults(None, Some(&event), &elections, &defaults);
        let row = ActivityLogRow::new(entry(None, "2028-04-08T22:00:03Z"), &zones).unwrap();
        assert_eq!(row.created, "2028-04-09T06:00:03+08:00");
        assert_eq!(row.created_label, "April 09, 2028 06:00:03 PhST");
        assert_eq!(row.created_utc, "2028-04-08T22:00:03Z");
        assert_eq!(row.statement_timestamp_utc, "2028-05-08T11:16:05Z");
        assert_eq!(row.time_zone, "Asia/Manila");
        let template =
            include_str!("../../../../../.devcontainer/minio/public-assets/activity_logs_user.hbs");
        let variables = serde_json::json!({"act_log": [row]})
            .as_object()
            .unwrap()
            .clone();
        let rendered =
            sequent_core::services::reports::render_template_text(template, variables).unwrap();
        assert!(rendered.contains("April 09, 2028 06:00:03 PhST"));
        assert!(rendered.contains("2028-04-08T22:00:03Z"));
        assert!(rendered.contains("2028-05-08T11:16:05Z"));
        assert!(rendered.contains("Asia/Manila"));
    }

    /// The label follows the timezone texts: the event's override (templates:
    /// > unprefixed > global:), else the default, else the tz abbreviation,
    /// else the offset.
    #[test]
    fn the_pdf_label_follows_the_zone_texts() {
        let at = DateTime::parse_from_rfc3339("2028-04-08T22:00:03Z")
            .unwrap()
            .with_timezone(&Utc);
        let defaults = serde_json::json!({"en": {"timezones": {"abbr": {"Asia/Manila": "PhST"}}}});
        let zones = LogZones::with_defaults(None, None, &[], &defaults);
        assert_eq!(zones.label(Tz::Asia__Manila, at), "PhST");
        assert_eq!(zones.label(Tz::America__New_York, at), "EDT");
        assert_eq!(zones.label(Tz::Asia__Dubai, at), "GMT+4");
        assert_eq!(zones.label(Tz::Asia__Kathmandu, at), "GMT+5:45");
        assert_eq!(zones.label(Tz::America__St_Johns, at), "NDT");
        assert_eq!(zones.label(Tz::America__Noronha, at), "GMT-2");

        let mut event = event(&["Asia/Manila"], "Asia/Manila", LogTimeZonePolicy::PRIMARY);
        let texts = |entries: &[(&str, &str)]| {
            entries
                .iter()
                .map(|(key, value)| (key.to_string(), Some(value.to_string())))
                .collect::<HashMap<_, _>>()
        };
        event.i18n = Some(HashMap::from([(
            "en".to_string(),
            texts(&[("global:timezones.abbr.Asia/Manila", "PHT")]),
        )]));
        let zones = LogZones::new(None, Some(&event), &[]);
        assert_eq!(zones.label(Tz::Asia__Manila, at), "PHT");
        assert_eq!(
            zones
                .labelled(at.timestamp(), Tz::Asia__Manila, "created")
                .unwrap(),
            "April 09, 2028 06:00:03 PHT"
        );

        event.i18n = Some(HashMap::from([(
            "en".to_string(),
            texts(&[
                ("global:timezones.abbr.Asia/Manila", "PHT"),
                ("templates:timezones.abbr.Asia/Manila", "PH Time"),
                ("timezones.abbr.Asia/Manila", "Manila"),
            ]),
        )]));
        let zones = LogZones::new(None, Some(&event), &[]);
        assert_eq!(zones.label(Tz::Asia__Manila, at), "PH Time");
    }

    /// A ranged export counts and reads the log list's range, so no PDF
    /// batch is rendered for rows outside it.
    #[test]
    fn a_ranged_export_counts_its_range() {
        let ids = || ReportOrigins {
            tenant_id: "tenant".to_string(),
            election_event_id: "event".to_string(),
            election_id: None,
            template_alias: None,
            voter_id: None,
            report_origin: ReportOriginatedFrom::ExportFunction,
            executer_username: None,
            tally_session_id: None,
        };
        let template = ActivityLogsTemplate::new(ids(), ReportFormat::PDF);
        assert_eq!(template.range_filter(), None);
        let template = ActivityLogsTemplate::new(ids(), ReportFormat::PDF).with_options(
            ActivityLogExportOptions {
                created_from: Some("2028-04-01T00:00:00+08:00".to_string()),
                created_to: Some(" ".to_string()),
                time_zone: None,
            },
        );
        assert_eq!(
            template.range_filter(),
            Some(HashMap::from([(
                OrderField::CreatedFrom,
                "2028-04-01T00:00:00+08:00".to_string()
            )]))
        );
    }

    fn origins() -> ReportOrigins {
        ReportOrigins {
            tenant_id: "tenant".to_string(),
            election_event_id: "event".to_string(),
            election_id: None,
            template_alias: None,
            voter_id: None,
            report_origin: ReportOriginatedFrom::ReportsTab,
            executer_username: None,
            tally_session_id: None,
        }
    }

    #[test]
    fn an_export_writes_the_format_its_caller_chose_and_a_report_those_it_asks_for() {
        let csv = ActivityLogsTemplate::new(origins(), ReportFormat::CSV);
        assert_eq!(csv.output_formats(None).unwrap(), vec![OutputFormat::Csv]);
        let pdf = ActivityLogsTemplate::new(origins(), ReportFormat::PDF);
        assert_eq!(pdf.output_formats(None).unwrap(), vec![OutputFormat::Pdf]);

        let asked = vec![OutputFormat::Pdf, OutputFormat::Csv, OutputFormat::Sql];
        let report = ActivityLogsTemplate::in_formats(origins(), asked.clone());
        assert_eq!(report.output_formats(None).unwrap(), asked);
        assert_eq!(report.report_format, ReportFormat::PDF);
        assert_eq!(report.export_name(), "export-election-event-logs-event");
    }

    fn sql_row() -> ElectoralLogCsvRow {
        let (event, elections) = configurations().remove(0);
        let zones = LogZones::new(Some(Tz::UTC), Some(&event), &elections);
        let mut row = ElectoralLogCsvRow::new(entry(None, "2028-04-08T22:00:03Z"), &zones).unwrap();
        row.message = "the voter's ballot; DROP TABLE electoral_log; --".to_string();
        row.data = "line one\nline two\0".to_string();
        row.user_id = None;
        row.username = Some("o'brien".to_string());
        row
    }

    #[test]
    fn the_sql_export_names_the_configuration_in_its_first_line() {
        let stamp = ConfigurationStamp {
            external_id: "ov-2028".to_string(),
            revision: 3,
            manifest_sha256: "ab".repeat(32),
            template_sha256: "cd".repeat(32),
        };
        let stamped = sql_header(Some(&stamp));
        assert_eq!(
            stamped.lines().next(),
            Some(
                format!(
                    "-- Configuration revision 3, manifest SHA-256 {}",
                    "ab".repeat(32)
                )
                .as_str()
            )
        );

        let plain = sql_header(None);
        assert!(plain.starts_with("BEGIN;\nCREATE TABLE electoral_log (\n  id BIGINT PRIMARY KEY,"));
        assert!(plain.ends_with("time_zone TEXT NOT NULL\n);\n"));
        assert_eq!(
            stamped.lines().skip(1).collect::<Vec<_>>(),
            plain.lines().collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_sql_export_writes_each_row_as_one_statement_that_cannot_end_early() {
        let row = sql_row();
        let statement = sql_insert(&row);
        assert_eq!(statement.matches('\n').count(), 1);
        assert!(statement.ends_with(");\n"));
        assert_eq!(
            statement,
            format!(
                "INSERT INTO electoral_log VALUES ({}, {}, {}, '{}', \
                 'the voter''s ballot; DROP TABLE electoral_log; --', 'line one line two', \
                 NULL, 'o''brien', '{}', '{}', '2028-04-08T22:00:03Z', \
                 '2028-04-08T22:00:03+00:00', 'UTC');\n",
                row.id,
                row.created,
                row.statement_timestamp,
                row.statement_kind,
                row.event_type,
                row.log_type,
            )
        );
    }

    /// The export's range: "to" is inclusive to the minute in the chosen zone.
    #[test]
    fn the_export_range_includes_the_last_minute_in_the_chosen_zone() {
        let options = ActivityLogExportOptions {
            created_from: Some("2028-04-01T00:00:00+08:00".to_string()),
            created_to: Some("2028-04-09T23:59:00+08:00".to_string()),
            time_zone: Some("Asia/Manila".to_string()),
        };
        let range = options.range().unwrap();
        let at = |rfc3339: &str| DateTime::parse_from_rfc3339(rfc3339).unwrap().timestamp();
        assert!(range.contains(at("2028-04-09T23:59:59+08:00")));
        assert!(!range.contains(at("2028-04-10T00:00:00+08:00")));
        assert!(range.contains(at("2028-04-01T00:00:00+08:00")));
        assert!(!range.contains(at("2028-03-31T23:59:59+08:00")));
        assert_eq!(options.zone().unwrap(), Some(Tz::Asia__Manila));
        assert!(ActivityLogExportOptions {
            time_zone: Some("Mars/Olympus".to_string()),
            ..Default::default()
        }
        .zone()
        .is_err());
    }

    /// The export ends with the head's event and log types and the times,
    /// and the importer still reads it.
    #[test]
    fn the_csv_export_ends_with_event_and_log_types() {
        use electoral_log::messages::message::SigningData;
        use electoral_log::messages::newtypes::{
            EventIdString, SigningLogEntry, SigningStatementKind,
        };
        use electoral_log::messages::statement::{StatementEventType, StatementLogType};
        use strand::signature::StrandSignatureSk;

        let sk = StrandSignatureSk::generate().unwrap();
        let sd = SigningData::new(sk.clone(), "", sk);
        let message = Message::signing_message(
            EventIdString("event".to_string()),
            SigningLogEntry {
                kind: SigningStatementKind::SigningSignatureRefused,
                event_type: StatementEventType::SYSTEM,
                log_type: StatementLogType::ERROR,
                description: "Refused on 7F3A-91C2: issuer not trusted".to_string(),
                details_json: "{}".to_string(),
                step_id: "2b7c9e40-1f5d-4a8e-9c3b-6d2e1f0a9b87".to_string(),
            },
            1_841_397_365,
            &sd,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let entry = ElectoralLogMessage::try_from(&message).unwrap();
        let zones = LogZones::new(None, None, &[]);
        let row = ElectoralLogCsvRow::new(entry.clone(), &zones).unwrap();
        assert_eq!(row.event_type, "SYSTEM");
        assert_eq!(row.log_type, "ERROR");
        assert!(!row.message.contains('\n'));

        let mut writer = WriterBuilder::new().from_writer(vec![]);
        writer.serialize(&row).unwrap();
        let file = String::from_utf8(writer.into_inner().unwrap()).unwrap();
        let header = file.lines().next().unwrap();
        assert_eq!(
            header,
            "id,created,statement_timestamp,statement_kind,message,data,user_id,username,\
             event_type,log_type,created_utc,created_local,time_zone"
        );
        assert_eq!(row.time_zone, "UTC");

        let imported: ElectoralLogRow = csv::Reader::from_reader(file.as_bytes())
            .deserialize()
            .next()
            .unwrap()
            .unwrap();
        assert_eq!(imported.data, row.data);
        assert_eq!(imported.statement_kind, "SigningSignatureRefused");
    }

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
