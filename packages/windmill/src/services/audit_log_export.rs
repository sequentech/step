// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Exports of the tenant's log and scoped exports of an election event's.
//!
//! An export honors its scope and its range of `created` (both ends
//! included, to the minute), so it holds exactly the entries the list shows
//! for the same scope and range. One account's export can add that account's
//! entries of every election event, deleted ones included, merged by time,
//! each row naming its log.
//! The SQL export carries the signed statement, both signatures and the
//! public keys, so that every entry can be verified after loading it. Each
//! export is itself recorded, as `LogsExported`, in the log it exports.

use crate::postgres::audit_log_outbox::{AuditLogTarget, NewAuditLogEntry};
use crate::services::audit_log::{stage_entry, AuditActor};
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log::MinuteRange;
use crate::services::electoral_log_board::{
    ensure_tenant_log, get_board_client, get_board_signing_key, get_event_databases,
};
use crate::services::time_zones::parse_zone;
use anyhow::{anyhow, ensure, Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use chrono_tz::Tz;
use deadpool_postgres::Transaction;
use electoral_log::adapters::events::RetiredEventDatabase;
use electoral_log::adapters::postgres::LogScope;
use electoral_log::messages::message::Message;
use electoral_log::messages::newtypes::AuditStatementKind;
use electoral_log::messages::statement::{StatementEventType, StatementLogType};
use electoral_log::{
    ElectoralLogMessage, Filter, LogQuery, NumberColumn, NumberComparison, OrderColumn,
    SortDirection, SqlCompOperators, TextColumn,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::future::Future;
use std::io::Write;
use strum_macros::{Display, EnumString};
use tracing::instrument;
use uuid::Uuid;
use voting_crypto::signing::VerificationKey;
use voting_crypto::{RecordDeserialize, RecordSerialize};

/// Entries read from a log at a time.
const PAGE: i64 = 2500;
/// The table an SQL export fills.
pub const SQL_TABLE: &str = "audit_log";
/// The table with what an SQL export holds: its scope, range and the logs'
/// system keys.
pub const SQL_MANIFEST_TABLE: &str = "audit_log_export";
/// The tenant setting with the zone the tenant's log is shown in.
pub const LOGS_TIMEZONE_SETTING: &str = "logs_timezone";

/// The file an export writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display, EnumString)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum AuditExportFormat {
    Csv,
    Sql,
    Pdf,
}

impl AuditExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Sql => "sql",
            Self::Pdf => "pdf",
        }
    }

    pub fn media_type(self) -> &'static str {
        match self {
            Self::Csv => "text/csv",
            Self::Sql => "application/sql",
            Self::Pdf => "application/pdf",
        }
    }
}

/// What an export covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display, EnumString)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum AuditExportScope {
    /// The whole tenant log.
    TenantLog,
    /// One account's entries of the log.
    Account,
    /// One account's entries of the tenant log and of every election event.
    AccountWithEvents,
    /// The whole log of an election event.
    All,
    /// The entries of one election of an event.
    Election,
    /// The entries of one area of an event.
    Area,
}

/// An export to write.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditExportRequest {
    pub tenant_id: String,
    /// `None` for the tenant's log.
    #[serde(default)]
    pub election_event_id: Option<String>,
    pub format: AuditExportFormat,
    pub scope: AuditExportScope,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub election_id: Option<String>,
    #[serde(default)]
    pub area_id: Option<String>,
    /// The first minute to export (RFC 3339 instant), inclusive.
    #[serde(default)]
    pub created_from: Option<String>,
    /// The last minute to export (RFC 3339 instant), inclusive.
    #[serde(default)]
    pub created_to: Option<String>,
    /// The IANA zone the times are shown in; the log's zone when absent.
    #[serde(default)]
    pub time_zone: Option<String>,
    /// Who asked for the export.
    #[serde(default)]
    pub requested_by: AuditActor,
}

fn given(value: &Option<String>) -> Option<&str> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

impl AuditExportRequest {
    /// Refuses a scope that doesn't fit the log, or lacks what it names.
    pub fn validate(&self) -> Result<()> {
        Uuid::parse_str(&self.tenant_id).context("Invalid tenant ID")?;
        let of_event = match given(&self.election_event_id) {
            Some(event) => {
                Uuid::parse_str(event).context("Invalid election event ID")?;
                true
            }
            None => false,
        };
        match (self.scope, of_event) {
            (AuditExportScope::TenantLog | AuditExportScope::AccountWithEvents, true) => {
                return Err(anyhow!("{} is a scope of the tenant log", self.scope));
            }
            (
                AuditExportScope::All | AuditExportScope::Election | AuditExportScope::Area,
                false,
            ) => {
                return Err(anyhow!(
                    "{} is a scope of an election event's log",
                    self.scope
                ));
            }
            _ => {}
        }
        match self.scope {
            AuditExportScope::Account | AuditExportScope::AccountWithEvents => {
                ensure!(
                    given(&self.username).is_some(),
                    "The account's username is missing"
                );
            }
            AuditExportScope::Election => {
                ensure!(
                    given(&self.election_id).is_some(),
                    "The election is missing"
                );
            }
            AuditExportScope::Area => {
                ensure!(given(&self.area_id).is_some(), "The area is missing");
            }
            AuditExportScope::TenantLog | AuditExportScope::All => {}
        }
        self.range()?;
        if let Some(zone) = given(&self.time_zone) {
            parse_zone(zone)?;
        }
        Ok(())
    }

    pub fn range(&self) -> Result<MinuteRange> {
        MinuteRange::parse(self.created_from.as_deref(), self.created_to.as_deref())
    }

    /// The log the export is recorded in.
    pub fn target(&self) -> Result<AuditLogTarget> {
        Ok(AuditLogTarget {
            tenant_id: Uuid::parse_str(&self.tenant_id)?,
            election_event_id: given(&self.election_event_id)
                .map(Uuid::parse_str)
                .transpose()?,
        })
    }

    /// The filters every log of the export is read with.
    pub fn filters(&self) -> Result<Vec<Filter>> {
        let mut filters: Vec<Filter> = self.range()?.filters(NumberColumn::Created).collect();
        let exactly =
            |column, value: &str| Filter::Text(column, SqlCompOperators::Equal, value.to_string());
        match self.scope {
            AuditExportScope::Account | AuditExportScope::AccountWithEvents => {
                if let Some(username) = given(&self.username) {
                    filters.push(exactly(TextColumn::Username, username));
                }
            }
            AuditExportScope::Election => {
                if let Some(election) = given(&self.election_id) {
                    filters.push(exactly(TextColumn::ElectionId, election));
                }
            }
            AuditExportScope::Area => {
                if let Some(area) = given(&self.area_id) {
                    filters.push(exactly(TextColumn::AreaId, area));
                }
            }
            AuditExportScope::TenantLog | AuditExportScope::All => {}
        }
        Ok(filters)
    }
}

/// A log an export reads: its board, the name its rows carry and the key that
/// verifies its system signatures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportedLog {
    #[serde(skip)]
    pub board: String,
    /// The database of a deleted election event's log, read in place of the
    /// board.
    #[serde(skip)]
    pub retired: Option<RetiredEventDatabase>,
    /// `tenant`, `event <election event ID>`, or `retired event <election
    /// event ID>` for a deleted one's.
    pub log: String,
    /// `None` for a deleted election event's log: its key was deleted with the
    /// event, and the tenant log's entry of the deletion names it.
    pub system_public_key: Option<String>,
    pub entries: u64,
}

/// A row of an export.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditExportRow {
    pub log: String,
    pub id: i64,
    pub created: i64,
    pub statement_timestamp: i64,
    pub statement_kind: String,
    pub event_type: String,
    pub log_type: String,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub description: String,
    /// The signed message as JSON, on one line.
    pub message: String,
    /// The signed statement's bytes, in base64.
    pub statement: String,
    pub sender_name: String,
    pub sender_public_key: String,
    pub sender_signature: String,
    pub system_signature: String,
    /// `created` as an ISO 8601 UTC instant.
    pub created_utc: String,
    /// `created` in the export's zone, with its offset.
    pub created_local: String,
    pub time_zone: String,
}

fn instant(seconds: i64) -> Result<DateTime<Utc>> {
    DateTime::<Utc>::from_timestamp(seconds, 0)
        .ok_or_else(|| anyhow!("Invalid time {seconds} in a log entry"))
}

impl AuditExportRow {
    pub fn new(log: &str, entry: &ElectoralLogMessage, zone: Tz) -> Result<Self> {
        let message = Message::record_deserialize(&entry.message)
            .map_err(|error| anyhow!("Error reading log entry {}: {error:?}", entry.id))?;
        let created = instant(entry.created)?;
        let head = &message.statement.head;
        Ok(AuditExportRow {
            log: log.to_string(),
            id: entry.id,
            created: entry.created,
            statement_timestamp: entry.statement_timestamp,
            statement_kind: entry.statement_kind.clone(),
            event_type: head.event_type.to_string(),
            log_type: head.log_type.to_string(),
            user_id: entry.user_id.clone(),
            username: entry.username.clone(),
            description: head.description.clone(),
            message: serde_json::to_string(&message)?,
            statement: base64_text(&message.statement.record_serialize()?),
            sender_name: message.sender.name.clone(),
            sender_public_key: message.sender.pk.to_b64_string()?,
            sender_signature: message.sender_signature.to_b64_string()?,
            system_signature: message.system_signature.to_b64_string()?,
            created_utc: created.to_rfc3339_opts(SecondsFormat::Secs, true),
            created_local: created
                .with_timezone(&zone)
                .to_rfc3339_opts(SecondsFormat::Secs, false),
            time_zone: zone.name().to_string(),
        })
    }
}

fn base64_text(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// The zone a tenant's log is shown in: its `logs_timezone` setting, or UTC.
pub fn tenant_log_zone(settings: Option<&Value>) -> Tz {
    settings
        .and_then(|settings| settings[LOGS_TIMEZONE_SETTING].as_str())
        .and_then(|zone| parse_zone(zone).ok())
        .unwrap_or(Tz::UTC)
}

async fn tenant_settings(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
) -> Result<Option<Value>> {
    let tenant = Uuid::parse_str(tenant_id)?;
    Ok(hasura_transaction
        .query_opt(
            "SELECT settings FROM sequent_backend.tenant WHERE id = $1",
            &[&tenant],
        )
        .await
        .context("Error reading the tenant's settings")?
        .and_then(|row| row.get(0)))
}

/// The logs an export reads, with their system keys: the tenant's, an
/// election event's, or the tenant's and every election event's.
#[instrument(skip(hasura_transaction), err)]
pub async fn exported_logs(
    hasura_transaction: &Transaction<'_>,
    request: &AuditExportRequest,
) -> Result<Vec<ExportedLog>> {
    let tenant = Uuid::parse_str(&request.tenant_id)?;
    let public = |key: &voting_crypto::signing::SigningKey| -> Result<String> {
        VerificationKey::from_sk(key)?.to_b64_string()
    };
    let event_log = |event: Uuid, board: String, key: String| ExportedLog {
        board,
        retired: None,
        log: format!("event {event}"),
        system_public_key: Some(key),
        entries: 0,
    };
    if let Some(event) = given(&request.election_event_id) {
        let event = Uuid::parse_str(event)?;
        let reference: Option<Value> = hasura_transaction
            .query_opt(
                "SELECT bulletin_board_reference FROM sequent_backend.election_event
                 WHERE tenant_id = $1 AND id = $2",
                &[&tenant, &event],
            )
            .await?
            .ok_or_else(|| anyhow!("Election event {event} does not exist"))?
            .get(0);
        let board = get_election_event_board(reference)
            .ok_or_else(|| anyhow!("Election event {event} has no log"))?;
        let key = get_board_signing_key(
            hasura_transaction,
            &request.tenant_id,
            Some(&event.to_string()),
            &board,
        )
        .await?;
        return Ok(vec![event_log(event, board, public(&key)?)]);
    }
    let tenant_log = ensure_tenant_log(hasura_transaction, &request.tenant_id).await?;
    let mut logs = vec![ExportedLog {
        board: tenant_log.board,
        retired: None,
        log: "tenant".to_string(),
        system_public_key: Some(public(&tenant_log.system_key)?),
        entries: 0,
    }];
    if request.scope == AuditExportScope::AccountWithEvents {
        let events = hasura_transaction
            .query(
                "SELECT id, bulletin_board_reference FROM sequent_backend.election_event
                 WHERE tenant_id = $1 ORDER BY created_at, id",
                &[&tenant],
            )
            .await
            .context("Error listing the tenant's election events")?;
        for row in events {
            let event: Uuid = row.get(0);
            let Some(board) = get_election_event_board(row.get(1)) else {
                continue;
            };
            let key = get_board_signing_key(
                hasura_transaction,
                &request.tenant_id,
                Some(&event.to_string()),
                &board,
            )
            .await;
            // An event without its key has no log to read.
            if let Ok(key) = key {
                logs.push(event_log(event, board, public(&key)?));
            }
        }
        // Deleted events' logs, kept in their retired databases.
        for retired in get_event_databases()
            .await?
            .retired_events(&request.tenant_id)
            .await?
        {
            logs.push(ExportedLog {
                board: String::new(),
                log: format!("retired event {}", retired.election_event_id),
                retired: Some(retired),
                system_public_key: None,
                entries: 0,
            });
        }
    }
    Ok(logs)
}

/// Every entry of `log` that matches `filters`, oldest first, read in pages:
/// from its board, or every log of a deleted event's retired database, which
/// is opened for the read alone.
async fn read_log(
    log: &ExportedLog,
    filters: &[Filter],
    zone: Tz,
    each: impl FnMut(AuditExportRow) -> Result<()>,
) -> Result<u64> {
    match &log.retired {
        None => {
            let client = get_board_client().await?;
            read_pages(log, filters, zone, each, |query| {
                let client = client.clone();
                async move { client.query(&log.board, &query).await }
            })
            .await
        }
        Some(retired) => {
            get_event_databases()
                .await?
                .read_retired_event(retired, |store| async move {
                    read_pages(log, filters, zone, each, |query| {
                        let store = store.clone();
                        async move { store.query_scope(LogScope::Database, &query).await }
                    })
                    .await
                })
                .await
        }
    }
}

async fn read_pages<Read, Page>(
    log: &ExportedLog,
    filters: &[Filter],
    zone: Tz,
    mut each: impl FnMut(AuditExportRow) -> Result<()>,
    page: Read,
) -> Result<u64>
where
    Read: Fn(LogQuery) -> Page,
    Page: Future<Output = Result<Vec<ElectoralLogMessage>>>,
{
    let (mut last_id, mut read) = (0i64, 0u64);
    loop {
        let query = LogQuery {
            filters: std::iter::once(Filter::Number(
                NumberColumn::Id,
                NumberComparison::GreaterThan,
                last_id,
            ))
            .chain(filters.iter().cloned())
            .collect(),
            order: vec![(OrderColumn::Id, SortDirection::Asc)],
            limit: PAGE,
            ..LogQuery::default()
        };
        let entries = page(query)
            .await
            .with_context(|| format!("Error reading log {}", log.log))?;
        let last_page = i64::try_from(entries.len())? < PAGE;
        for entry in &entries {
            last_id = entry.id;
            each(AuditExportRow::new(&log.log, entry, zone)?)?;
            read += 1;
        }
        if last_page {
            return Ok(read);
        }
    }
}

/// What an export holds, for its file and for its `LogsExported` entry.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExportManifest {
    pub format: AuditExportFormat,
    pub scope: AuditExportScope,
    pub username: Option<String>,
    pub election_id: Option<String>,
    pub area_id: Option<String>,
    pub created_from: Option<String>,
    pub created_to: Option<String>,
    pub time_zone: String,
    pub entries: u64,
    pub logs: Vec<ExportedLog>,
}

impl ExportManifest {
    fn new(request: &AuditExportRequest, zone: Tz, logs: Vec<ExportedLog>) -> Self {
        ExportManifest {
            format: request.format,
            scope: request.scope,
            username: given(&request.username).map(str::to_string),
            election_id: given(&request.election_id).map(str::to_string),
            area_id: given(&request.area_id).map(str::to_string),
            created_from: given(&request.created_from).map(str::to_string),
            created_to: given(&request.created_to).map(str::to_string),
            time_zone: zone.name().to_string(),
            entries: logs.iter().map(|log| log.entries).sum(),
            logs,
        }
    }
}

/// A text as an SQL literal. PostgreSQL text holds no NUL.
fn sql_text(value: &str) -> String {
    format!("'{}'", value.replace('\0', "").replace('\'', "''"))
}

fn sql_optional_text(value: Option<&str>) -> String {
    value.map(sql_text).unwrap_or_else(|| "NULL".to_string())
}

fn sql_bytes(base64: &str) -> String {
    format!("decode({}, 'base64')", sql_text(base64))
}

/// What an SQL export starts with: one transaction and the entries' table.
pub fn sql_header() -> String {
    format!(
        "BEGIN;\n\
         CREATE TABLE {SQL_TABLE} (\n  \
         log TEXT NOT NULL,\n  \
         id BIGINT NOT NULL,\n  \
         created BIGINT NOT NULL,\n  \
         statement_timestamp BIGINT NOT NULL,\n  \
         statement_kind TEXT NOT NULL,\n  \
         event_type TEXT NOT NULL,\n  \
         log_type TEXT NOT NULL,\n  \
         user_id TEXT,\n  \
         username TEXT,\n  \
         description TEXT NOT NULL,\n  \
         message JSONB NOT NULL,\n  \
         statement BYTEA NOT NULL,\n  \
         sender_name TEXT NOT NULL,\n  \
         sender_public_key TEXT NOT NULL,\n  \
         sender_signature BYTEA NOT NULL,\n  \
         system_signature BYTEA NOT NULL,\n  \
         created_utc TIMESTAMPTZ NOT NULL,\n  \
         created_local TEXT NOT NULL,\n  \
         time_zone TEXT NOT NULL,\n  \
         PRIMARY KEY (log, id)\n\
         );\n"
    )
}

/// A row as its `INSERT`, on one line.
pub fn sql_insert(row: &AuditExportRow) -> String {
    format!(
        "INSERT INTO {SQL_TABLE} VALUES ({}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}::jsonb, {}, \
         {}, {}, {}, {}, {}, {}, {});\n",
        sql_text(&row.log),
        row.id,
        row.created,
        row.statement_timestamp,
        sql_text(&row.statement_kind),
        sql_text(&row.event_type),
        sql_text(&row.log_type),
        sql_optional_text(row.user_id.as_deref()),
        sql_optional_text(row.username.as_deref()),
        sql_text(&row.description.replace(['\n', '\r'], " ")),
        sql_text(&row.message.replace("\\u0000", "")),
        sql_bytes(&row.statement),
        sql_text(&row.sender_name),
        sql_text(&row.sender_public_key),
        sql_bytes(&row.sender_signature),
        sql_bytes(&row.system_signature),
        sql_text(&row.created_utc),
        sql_text(&row.created_local),
        sql_text(&row.time_zone),
    )
}

/// What an SQL export ends with: its manifest, with each log's system key,
/// and the end of the transaction.
pub fn sql_footer(manifest: &ExportManifest) -> Result<String> {
    Ok(format!(
        "CREATE TABLE {SQL_MANIFEST_TABLE} (manifest JSONB NOT NULL);\n\
         INSERT INTO {SQL_MANIFEST_TABLE} VALUES ({}::jsonb);\n\
         COMMIT;\n",
        sql_text(&serde_json::to_string(manifest)?)
    ))
}

fn html_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The page a PDF export prints: what it holds, then a row for each entry.
pub fn pdf_html(manifest: &ExportManifest, rows: &[AuditExportRow]) -> String {
    let fact = |label: &str, value: &str| {
        format!("<tr><th>{label}</th><td>{}</td></tr>", html_text(value))
    };
    let optional = |value: &Option<String>| value.clone().unwrap_or_else(|| "-".to_string());
    let mut facts = vec![
        fact("Scope", &manifest.scope.to_string()),
        fact("Created from", &optional(&manifest.created_from)),
        fact("Created to", &optional(&manifest.created_to)),
        fact("Timezone", &manifest.time_zone),
        fact("Entries", &manifest.entries.to_string()),
    ];
    if let Some(username) = &manifest.username {
        facts.insert(1, fact("Account", username));
    }
    let body: String = rows
        .iter()
        .map(|row| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}<br><small>{}</small></td><td>{}</td>\
                 <td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                html_text(&row.log),
                row.id,
                html_text(&row.created_local),
                html_text(&row.created_utc),
                html_text(row.username.as_deref().unwrap_or("-")),
                html_text(&row.statement_kind),
                html_text(&row.event_type),
                html_text(&row.log_type),
                html_text(&row.description),
            )
        })
        .collect();
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Logs</title><style>\
         body{{font-family:sans-serif;font-size:9px}}table{{border-collapse:collapse;width:100%}}\
         th,td{{border:1px solid #999;padding:2px 4px;text-align:left;vertical-align:top}}\
         .facts{{width:auto;margin-bottom:12px}}small{{color:#555}}</style></head><body>\
         <h1>Logs</h1><table class=\"facts\">{}</table><table><thead><tr><th>Log</th><th>Id</th>\
         <th>Created</th><th>Username</th><th>Statement kind</th><th>Event type</th>\
         <th>Log type</th><th>Description</th></tr></thead><tbody>{body}</tbody></table>\
         </body></html>",
        facts.concat()
    )
}

/// An export's file before it is rendered or stored.
pub struct WrittenExport {
    pub manifest: ExportManifest,
    /// The CSV or SQL text; for a PDF, the page to print.
    pub contents: Vec<u8>,
}

/// Reads the logs of an export and writes its file. A PDF comes back as the
/// page to print. Entries of several logs are merged by time.
#[instrument(skip(hasura_transaction), err)]
pub async fn write_export(
    hasura_transaction: &Transaction<'_>,
    request: &AuditExportRequest,
) -> Result<WrittenExport> {
    request.validate()?;
    let zone = match given(&request.time_zone) {
        Some(zone) => parse_zone(zone)?,
        None if given(&request.election_event_id).is_none() => tenant_log_zone(
            tenant_settings(hasura_transaction, &request.tenant_id)
                .await?
                .as_ref(),
        ),
        None => Tz::UTC,
    };
    let filters = request.filters()?;
    let mut logs = exported_logs(hasura_transaction, request).await?;
    let mut rows = Vec::new();
    for log in &mut logs {
        log.entries = read_log(log, &filters, zone, |row| {
            rows.push(row);
            Ok(())
        })
        .await?;
    }
    if logs.len() > 1 {
        rows.sort_by(|a, b| (a.created, &a.log, a.id).cmp(&(b.created, &b.log, b.id)));
    }
    let manifest = ExportManifest::new(request, zone, logs);
    let contents = match request.format {
        AuditExportFormat::Csv => {
            let mut writer = csv::Writer::from_writer(Vec::new());
            for row in &rows {
                writer
                    .serialize(row)
                    .context("Error writing the CSV export")?;
            }
            writer
                .into_inner()
                .map_err(|error| anyhow!("Error ending the CSV export: {error}"))?
        }
        AuditExportFormat::Sql => {
            let mut text = Vec::new();
            text.write_all(sql_header().as_bytes())?;
            for row in &rows {
                text.write_all(sql_insert(row).as_bytes())?;
            }
            text.write_all(sql_footer(&manifest)?.as_bytes())?;
            text
        }
        AuditExportFormat::Pdf => pdf_html(&manifest, &rows).into_bytes(),
    };
    Ok(WrittenExport { manifest, contents })
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// The `LogsExported` entry of an export: its format, scope, range, entry
/// count and the file's SHA-256, in the log it exports.
pub fn logs_exported_entry(
    request: &AuditExportRequest,
    manifest: &ExportManifest,
    file_sha256: &str,
    document_id: &str,
) -> Result<NewAuditLogEntry> {
    let what = match manifest.scope {
        AuditExportScope::TenantLog => "the tenant log".to_string(),
        AuditExportScope::All => "the election event's log".to_string(),
        AuditExportScope::Account => format!(
            "the entries of {}",
            manifest.username.as_deref().unwrap_or_default()
        ),
        AuditExportScope::AccountWithEvents => format!(
            "the entries of {} in the tenant log and every election event",
            manifest.username.as_deref().unwrap_or_default()
        ),
        AuditExportScope::Election => "the entries of an election".to_string(),
        AuditExportScope::Area => "the entries of an area".to_string(),
    };
    Ok(NewAuditLogEntry {
        target: request.target()?,
        statement_kind: AuditStatementKind::LogsExported,
        event_type: match request.requested_by.user_id {
            Some(_) => StatementEventType::USER,
            None => StatementEventType::SYSTEM,
        },
        log_type: StatementLogType::INFO,
        user_id: request.requested_by.user_id.clone(),
        username: request.requested_by.username.clone(),
        election_id: None,
        area_id: None,
        description: format!(
            "Exported {what} as {}: {} entries",
            manifest.format, manifest.entries
        ),
        details: json!({
            "format": manifest.format,
            "scope": manifest.scope,
            "username": manifest.username,
            "election_id": manifest.election_id,
            "area_id": manifest.area_id,
            "created_from": manifest.created_from,
            "created_to": manifest.created_to,
            "time_zone": manifest.time_zone,
            "entries": manifest.entries,
            "sha256": file_sha256,
            "document_id": document_id,
            "actor": request.requested_by,
        }),
    })
}

/// Queues the `LogsExported` entry of an export in the transaction that
/// stores its file.
#[instrument(skip_all, err)]
pub async fn record_export(
    hasura_transaction: &Transaction<'_>,
    request: &AuditExportRequest,
    manifest: &ExportManifest,
    file_sha256: &str,
    document_id: &str,
) -> Result<Uuid> {
    let entry = logs_exported_entry(request, manifest, file_sha256, document_id)?;
    stage_entry(hasura_transaction, &entry).await
}

#[cfg(test)]
#[path = "audit_log_export_tests.rs"]
mod tests;
