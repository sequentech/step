// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Exporting what a dashboard shows, as CSV or SQL.
//!
//! An export reads the snapshot revision the viewer was shown, with the
//! configuration that revision was counted under, and evaluates every
//! widget's governed queries the way the render route does. It counts only
//! the elections the viewer may see: Harvest computes them from the viewer's
//! labels before sending the task, and the figures read are those of that
//! set of elections. So what a file holds matches what the dashboard showed.
//!
//! Every widget and query goes in one table in long form: the widget, the
//! query, the row's place in the query, then the query's own columns. Reading
//! is split from formatting ([`collect_export`], [`build_file`]) so the file
//! can be checked without uploading it.

use super::config_store::{get_config_at_generation, EventRef};
use super::snapshot::{
    complete_snapshot, empty_payload, read_scope, scope_catalogue, LiveSnapshot, ScopeRead,
};
use anyhow::anyhow;
use chrono::{DateTime, NaiveDateTime, Utc};
use deadpool_postgres::Transaction;
use indexmap::IndexMap;
use sequent_core::monitoring::compute::{evaluate, event_days, ColumnKind, QueryResult};
use sequent_core::monitoring::config::{ConfigSet, ScopeSelector, Settings, Widget};
use sequent_core::monitoring::payload::ScopePayload;
use sequent_core::monitoring::problem::Report;
use sequent_core::monitoring::resolve::{resolve_widget, DynamicOptionValues};
use sequent_core::monitoring::scope::{election_set_key, PostPinning, ScopeSelection};
use sequent_core::monitoring::sources::{PendingProducer, Producer};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use strum_macros::{Display, EnumString};
use uuid::Uuid;

/// The file an export writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display, EnumString)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum MonitoringExportFormat {
    Csv,
    /// PostgreSQL: a `CREATE TABLE` and its `INSERT`s, in one transaction.
    Sql,
}

impl MonitoringExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            MonitoringExportFormat::Csv => "csv",
            MonitoringExportFormat::Sql => "sql",
        }
    }

    pub fn media_type(self) -> &'static str {
        match self {
            MonitoringExportFormat::Csv => "text/csv",
            MonitoringExportFormat::Sql => "application/sql",
        }
    }
}

/// What to export, for whom. Sent by Harvest's `/monitoring/export` route.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonitoringExportRequest {
    pub tenant_id: String,
    pub election_event_id: String,
    pub dashboard_id: String,
    /// One widget of the dashboard; `None` exports every widget on it.
    #[serde(default)]
    pub widget_id: Option<String>,
    /// The elections the viewer may see, as their labels allow: only the
    /// figures counted for this set are read.
    pub election_ids: Vec<String>,
    /// The Post an election's own page pins.
    #[serde(default)]
    pub pinned_post: Option<String>,
    #[serde(default)]
    pub scope: ScopeSelection,
    #[serde(default)]
    pub selector_values: IndexMap<String, String>,
    /// The revision the dashboard showed.
    pub snapshot_revision: i64,
    pub format: MonitoringExportFormat,
    /// Series buckets starting in `[from, to)` are kept; the rest of the
    /// figures are as of the revision.
    #[serde(default)]
    pub from: Option<DateTime<Utc>>,
    #[serde(default)]
    pub to: Option<DateTime<Utc>>,
    pub document_id: String,
}

/// Why an export writes no file. Each message starts with its code, which
/// is what the task execution records.
#[derive(Debug)]
pub enum MonitoringExportError {
    /// The revision is not a complete run, or has been pruned.
    SnapshotPruned {
        revision: i64,
    },
    NotFound(String),
    /// A Post, region or country outside the viewer's elections.
    ForbiddenScope(String),
    /// The viewer's set of elections was not counted at the revision.
    ScopePending {
        widget_id: String,
    },
    Invalid(String),
    Internal(anyhow::Error),
}

impl MonitoringExportError {
    pub fn code(&self) -> &'static str {
        match self {
            MonitoringExportError::SnapshotPruned { .. } => "SNAPSHOT_PRUNED",
            MonitoringExportError::NotFound(_) => "NOT_FOUND",
            MonitoringExportError::ForbiddenScope(_) => "FORBIDDEN_SCOPE",
            MonitoringExportError::ScopePending { .. } => "SCOPE_PENDING",
            MonitoringExportError::Invalid(_) => "INVALID",
            MonitoringExportError::Internal(_) => "INTERNAL",
        }
    }
}

impl fmt::Display for MonitoringExportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = self.code();
        match self {
            MonitoringExportError::SnapshotPruned { revision } => write!(
                formatter,
                "{code}: snapshot revision {revision} is no longer kept; export the revision \
                 the dashboard shows now"
            ),
            MonitoringExportError::NotFound(what) => write!(formatter, "{code}: {what}"),
            MonitoringExportError::ForbiddenScope(what) => write!(formatter, "{code}: {what}"),
            MonitoringExportError::ScopePending { widget_id } => write!(
                formatter,
                "{code}: the figures of '{widget_id}' for these elections are not counted yet; \
                 try again after the next snapshot"
            ),
            MonitoringExportError::Invalid(what) => write!(formatter, "{code}: {what}"),
            MonitoringExportError::Internal(error) => write!(formatter, "{code}: {error:#}"),
        }
    }
}

impl std::error::Error for MonitoringExportError {}

impl From<anyhow::Error> for MonitoringExportError {
    fn from(error: anyhow::Error) -> Self {
        MonitoringExportError::Internal(error)
    }
}

fn invalid_report(widget_id: &str, report: &Report) -> MonitoringExportError {
    let problems: Vec<String> = report
        .problems
        .iter()
        .map(|problem| format!("{}: {}", problem.path, problem.message))
        .collect();
    MonitoringExportError::Invalid(format!("'{widget_id}': {}", problems.join("; ")))
}

/// What one widget holds at the revision.
#[derive(Debug, Clone, PartialEq)]
pub enum WidgetData {
    Evaluated {
        queries: IndexMap<String, QueryResult>,
    },
    /// Its producer is not connected: no rows, a notice.
    NotConnected { reason: PendingProducer },
}

/// Everything an export file holds, before it is formatted.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportData {
    pub election_event_id: String,
    pub dashboard_id: String,
    pub widget_id: Option<String>,
    pub snapshot_revision: i64,
    pub as_of: DateTime<Utc>,
    pub scope: ScopeSelection,
    pub pinned_post: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    /// In the dashboard's layout order.
    pub widgets: Vec<(String, WidgetData)>,
}

fn parse_uuid(name: &str, text: &str) -> Result<Uuid, MonitoringExportError> {
    Uuid::parse_str(text)
        .map_err(|_| MonitoringExportError::Invalid(format!("'{text}' is not a {name}")))
}

/// Reads and evaluates what the request exports. Reads several statements:
/// give it a REPEATABLE READ transaction, so a prune in between cannot make
/// one widget's figures vanish.
pub async fn collect_export(
    transaction: &Transaction<'_>,
    request: &MonitoringExportRequest,
) -> Result<ExportData, MonitoringExportError> {
    let event = EventRef {
        tenant_id: parse_uuid("tenant id", &request.tenant_id)?,
        election_event_id: parse_uuid("election event id", &request.election_event_id)?,
    };
    if let (Some(from), Some(to)) = (request.from, request.to) {
        if from >= to {
            return Err(MonitoringExportError::Invalid(
                "the range must start before it ends".to_string(),
            ));
        }
    }
    let allowed: Vec<String> = request
        .election_ids
        .iter()
        .map(|id| id.to_ascii_lowercase())
        .collect();
    let set_key = election_set_key(&allowed)
        .map_err(|error| MonitoringExportError::Invalid(error.to_string()))?;
    let snapshot = complete_snapshot(transaction, event, request.snapshot_revision)
        .await?
        .ok_or(MonitoringExportError::SnapshotPruned {
            revision: request.snapshot_revision,
        })?;
    let config = get_config_at_generation(transaction, event, snapshot.config_generation)
        .await?
        .ok_or_else(|| {
            MonitoringExportError::NotFound(format!(
                "the event has no configuration at generation {}",
                snapshot.config_generation
            ))
        })?;
    let set = &config.assembled.set;
    let pinning = check_scope(transaction, event, &snapshot, &set_key, &allowed, request).await?;
    let items = layout_items(set, request)?;
    let mut widgets = Vec::new();
    for (widget_id, widget, dashboard_values) in items {
        let data = collect_widget(
            transaction,
            event,
            &snapshot,
            &set_key,
            &pinning,
            request,
            widget,
            dashboard_values,
            set.settings.as_ref(),
        )
        .await?;
        widgets.push((widget_id, data));
    }
    Ok(ExportData {
        election_event_id: event.election_event_id.to_string(),
        dashboard_id: request.dashboard_id.clone(),
        widget_id: request.widget_id.clone(),
        snapshot_revision: snapshot.revision,
        as_of: snapshot.as_of,
        scope: request.scope.clone(),
        pinned_post: request.pinned_post.clone(),
        from: request.from,
        to: request.to,
        widgets,
    })
}

/// The Post, region and country asked for are the viewer's: a Post among
/// their elections, a region or country their elections were counted in.
async fn check_scope(
    transaction: &Transaction<'_>,
    event: EventRef,
    snapshot: &LiveSnapshot,
    set_key: &str,
    allowed: &[String],
    request: &MonitoringExportRequest,
) -> Result<PostPinning, MonitoringExportError> {
    let is_allowed = |post: &str| allowed.contains(&post.to_ascii_lowercase());
    let pinning = match request
        .pinned_post
        .as_deref()
        .filter(|post| !post.is_empty())
    {
        Some(post) if !is_allowed(post) => {
            return Err(MonitoringExportError::ForbiddenScope(
                "the Post is not one of the viewer's elections".to_string(),
            ))
        }
        // Posts are stored under their id's lowercase text.
        Some(post) => PostPinning::Pinned(post.to_ascii_lowercase()),
        None => PostPinning::Selectable,
    };
    if let Some(post) = request
        .scope
        .post
        .as_deref()
        .filter(|post| !post.is_empty())
    {
        if !is_allowed(post) {
            return Err(MonitoringExportError::ForbiddenScope(
                "the Post is not one of the viewer's elections".to_string(),
            ));
        }
    }
    let region = request
        .scope
        .region
        .as_ref()
        .filter(|value| !value.is_empty());
    let country = request
        .scope
        .country
        .as_ref()
        .filter(|value| !value.is_empty());
    if region.is_some() || country.is_some() {
        let catalogue = scope_catalogue(transaction, event, snapshot.revision, set_key).await?;
        let known = region.map_or(true, |region| catalogue.regions.contains(region))
            && country.map_or(true, |country| catalogue.countries.contains(country));
        if !known {
            return Err(MonitoringExportError::ForbiddenScope(
                "the region or country is not one of the viewer's elections".to_string(),
            ));
        }
    }
    Ok(pinning)
}

/// The widgets to export, with the dashboard's values for their selectors.
fn layout_items<'a>(
    set: &'a ConfigSet,
    request: &MonitoringExportRequest,
) -> Result<Vec<(String, &'a Widget, &'a IndexMap<String, String>)>, MonitoringExportError> {
    let dashboard = set.dashboards.get(&request.dashboard_id).ok_or_else(|| {
        MonitoringExportError::NotFound(format!(
            "the dashboard '{}' is not configured at this revision",
            request.dashboard_id
        ))
    })?;
    let mut items = Vec::new();
    for item in &dashboard.layout {
        if let Some(wanted) = &request.widget_id {
            if &item.widget != wanted {
                continue;
            }
        }
        let widget = set.widgets.get(&item.widget).ok_or_else(|| {
            MonitoringExportError::NotFound(format!(
                "the widget '{}' is not configured at this revision",
                item.widget
            ))
        })?;
        items.push((item.widget.clone(), widget, &item.values));
        if request.widget_id.is_some() {
            break;
        }
    }
    if items.is_empty() {
        return Err(MonitoringExportError::NotFound(match &request.widget_id {
            Some(widget) => format!(
                "the dashboard '{}' has no widget '{widget}'",
                request.dashboard_id
            ),
            None => format!("the dashboard '{}' has no widgets", request.dashboard_id),
        }));
    }
    Ok(items)
}

#[allow(clippy::too_many_arguments)]
async fn collect_widget(
    transaction: &Transaction<'_>,
    event: EventRef,
    snapshot: &LiveSnapshot,
    set_key: &str,
    pinning: &PostPinning,
    request: &MonitoringExportRequest,
    widget: &Widget,
    dashboard_values: &IndexMap<String, String>,
    settings: Option<&Settings>,
) -> Result<WidgetData, MonitoringExportError> {
    let source = widget.source;
    if let Producer::Pending(reason) = source.spec().producer {
        return Ok(WidgetData::NotConnected { reason });
    }
    let mut selection = request.scope.clone();
    selection.post = selection.post.map(|post| post.to_ascii_lowercase());
    let scope = selection.for_widget(widget, pinning);
    let read = read_scope(
        transaction,
        event,
        snapshot.revision,
        source,
        set_key,
        &scope.key.canonical(),
    )
    .await?;
    let payload: ScopePayload = match read {
        ScopeRead::NotCounted => {
            return Err(MonitoringExportError::ScopePending {
                widget_id: widget.id.clone(),
            })
        }
        ScopeRead::NotConnected { reason } => return Ok(WidgetData::NotConnected { reason }),
        ScopeRead::Empty => empty_payload(
            source,
            settings.ok_or_else(|| {
                MonitoringExportError::Internal(anyhow!("a counted event has no settings"))
            })?,
        ),
        ScopeRead::Payload { text, .. } => serde_json::from_str(&text).map_err(|error| {
            MonitoringExportError::Internal(anyhow!(
                "the stored figures of '{}' do not read: {error}",
                widget.id
            ))
        })?,
    };
    let requested = selector_values_for(widget, request);
    let dynamic = DynamicOptionValues {
        event_days: event_days(&payload),
    };
    let resolved = resolve_widget(widget, dashboard_values, &requested, &dynamic)
        .map_err(|report| invalid_report(&widget.id, &report))?;
    let mut queries = IndexMap::new();
    for (name, query) in &resolved.queries {
        let mut result = evaluate(source, query, &payload, settings)
            .map_err(|report| invalid_report(&widget.id, &report))?;
        keep_buckets(&mut result, request.from, request.to);
        queries.insert(name.clone(), result);
    }
    Ok(WidgetData::Evaluated { queries })
}

/// A single widget's export reads the values as the render route does, so
/// an unknown selector is refused; a dashboard's gives each widget the
/// values of the selectors it has.
fn selector_values_for(
    widget: &Widget,
    request: &MonitoringExportRequest,
) -> IndexMap<String, String> {
    if request.widget_id.is_some() {
        return request.selector_values.clone();
    }
    request
        .selector_values
        .iter()
        .filter(|(name, _)| widget.selectors.contains_key(*name))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect()
}

/// The column of a series row that says when its bucket starts, in UTC.
const BUCKET_UTC: &str = "bucket_utc";

/// Keeps the series buckets starting in `[from, to)`; other results have no
/// buckets and are kept whole.
pub fn keep_buckets(
    result: &mut QueryResult,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
) {
    if from.is_none() && to.is_none() {
        return;
    }
    let Some(index) = result
        .columns
        .iter()
        .position(|column| column.name == BUCKET_UTC)
    else {
        return;
    };
    result.rows.retain(|row| {
        let Some(start) = row
            .get(index)
            .and_then(Value::as_str)
            .and_then(|text| NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M:%SZ").ok())
            .map(|start| start.and_utc())
        else {
            return false;
        };
        from.map_or(true, |from| start >= from) && to.map_or(true, |to| start < to)
    });
}

/// The type of an exported column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportColumnType {
    Text,
    Integer,
    Number,
}

impl ExportColumnType {
    fn from_kind(kind: ColumnKind) -> Self {
        match kind {
            ColumnKind::Text => ExportColumnType::Text,
            ColumnKind::Integer => ExportColumnType::Integer,
            ColumnKind::Number => ExportColumnType::Number,
        }
    }

    /// A column two queries fill differently holds both.
    fn widen(self, other: Self) -> Self {
        use ExportColumnType::*;
        match (self, other) {
            (Text, _) | (_, Text) => Text,
            (Number, _) | (_, Number) => Number,
            (Integer, Integer) => Integer,
        }
    }

    fn sql(self) -> &'static str {
        match self {
            ExportColumnType::Text => "TEXT",
            ExportColumnType::Integer => "BIGINT",
            ExportColumnType::Number => "DOUBLE PRECISION",
        }
    }
}

/// Every widget and query in one table.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportTable {
    pub columns: Vec<(String, ExportColumnType)>,
    pub rows: Vec<Vec<Value>>,
}

const WIDGET_COLUMN: &str = "widget_id";
const QUERY_COLUMN: &str = "query";
const ROW_COLUMN: &str = "row";
const NOTICE_COLUMN: &str = "notice";

/// A query's column named like one of the table's own, renamed.
fn own_name(name: &str) -> String {
    if [WIDGET_COLUMN, QUERY_COLUMN, ROW_COLUMN, NOTICE_COLUMN].contains(&name) {
        format!("{name}_value")
    } else {
        name.to_string()
    }
}

/// The long table: `widget_id`, `query`, `row` (from 1 within the query),
/// every query's columns in the order first met, then `notice`. A query
/// without a column leaves it null.
pub fn export_table(data: &ExportData) -> ExportTable {
    let mut shared: IndexMap<String, ExportColumnType> = IndexMap::new();
    for (_, widget) in &data.widgets {
        if let WidgetData::Evaluated { queries } = widget {
            for result in queries.values() {
                for column in &result.columns {
                    let kind = ExportColumnType::from_kind(column.kind);
                    shared
                        .entry(own_name(&column.name))
                        .and_modify(|known| *known = known.widen(kind))
                        .or_insert(kind);
                }
            }
        }
    }
    let width = shared.len() + 4;
    let mut rows = Vec::new();
    for (widget_id, widget) in &data.widgets {
        match widget {
            WidgetData::NotConnected { reason } => {
                let mut row = vec![Value::Null; width];
                row[0] = Value::String(widget_id.clone());
                row[width - 1] = Value::String(format!("NOT_CONNECTED: {reason}"));
                rows.push(row);
            }
            WidgetData::Evaluated { queries } => {
                for (name, result) in queries {
                    let notices: Vec<String> =
                        result.notices.iter().map(ToString::to_string).collect();
                    let notice = if notices.is_empty() {
                        Value::Null
                    } else {
                        Value::String(notices.join("; "))
                    };
                    let places: Vec<usize> = result
                        .columns
                        .iter()
                        .map(|column| shared.get_index_of(&own_name(&column.name)).unwrap_or(0) + 3)
                        .collect();
                    for (number, cells) in (1_u64..).zip(&result.rows) {
                        let mut row = vec![Value::Null; width];
                        row[0] = Value::String(widget_id.clone());
                        row[1] = Value::String(name.clone());
                        row[2] = Value::from(number);
                        for (place, cell) in places.iter().zip(cells) {
                            row[*place] = cell.clone();
                        }
                        row[width - 1] = notice.clone();
                        rows.push(row);
                    }
                }
            }
        }
    }
    let mut columns = vec![
        (WIDGET_COLUMN.to_string(), ExportColumnType::Text),
        (QUERY_COLUMN.to_string(), ExportColumnType::Text),
        (ROW_COLUMN.to_string(), ExportColumnType::Integer),
    ];
    columns.extend(shared);
    columns.push((NOTICE_COLUMN.to_string(), ExportColumnType::Text));
    ExportTable { columns, rows }
}

/// The file's bytes in `format`.
pub fn build_file(
    data: &ExportData,
    format: MonitoringExportFormat,
) -> Result<Vec<u8>, MonitoringExportError> {
    let table = export_table(data);
    match format {
        MonitoringExportFormat::Csv => to_csv(&table),
        MonitoringExportFormat::Sql => to_sql(data, &table),
    }
}

/// The file's name: the dashboard, the widget, the revision.
pub fn file_name(data: &ExportData, format: MonitoringExportFormat) -> String {
    let safe = |text: &str| -> String {
        text.chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    };
    let widget = data
        .widget_id
        .as_deref()
        .map(|widget| format!("-{}", safe(widget)))
        .unwrap_or_default();
    format!(
        "monitoring-{}{widget}-r{}.{}",
        safe(&data.dashboard_id),
        data.snapshot_revision,
        format.extension()
    )
}

/// A cell a spreadsheet would run as a formula starts with one of these; it
/// is written after a `'`, so opening the file runs nothing.
fn defuse_formula(text: &str) -> String {
    match text.chars().next() {
        Some('=' | '+' | '-' | '@' | '\t' | '\r') => format!("'{text}"),
        _ => text.to_string(),
    }
}

fn csv_cell(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(text) => defuse_formula(text),
        other => defuse_formula(&other.to_string()),
    }
}

pub fn to_csv(table: &ExportTable) -> Result<Vec<u8>, MonitoringExportError> {
    let mut writer = csv::WriterBuilder::new()
        .terminator(csv::Terminator::CRLF)
        .from_writer(Vec::new());
    let internal = |error: csv::Error| MonitoringExportError::Internal(anyhow!(error));
    writer
        .write_record(table.columns.iter().map(|(name, _)| name.as_str()))
        .map_err(internal)?;
    for row in &table.rows {
        writer
            .write_record(row.iter().map(csv_cell))
            .map_err(internal)?;
    }
    writer
        .into_inner()
        .map_err(|error| MonitoringExportError::Internal(anyhow!("{error}")))
}

/// How many rows each `INSERT` carries.
const INSERT_BATCH: usize = 500;

/// The exported table's name.
const SQL_TABLE: &str = "monitoring_export";

/// A value as a PostgreSQL literal (with `standard_conforming_strings`, the
/// default, so a backslash is only a backslash): NULL, TRUE/FALSE, a number
/// as JSON wrote it, text in single quotes with each quote doubled. Text
/// cannot hold a NUL, so a value with one is refused rather than cut short.
/// Lists and maps are written as their JSON text.
pub fn sql_literal(value: &Value) -> Result<String, MonitoringExportError> {
    let quoted = |text: &str| -> Result<String, MonitoringExportError> {
        if text.contains('\0') {
            return Err(MonitoringExportError::Invalid(
                "a value holds a NUL character, which SQL text cannot".to_string(),
            ));
        }
        Ok(format!("'{}'", text.replace('\'', "''")))
    };
    match value {
        Value::Null => Ok("NULL".to_string()),
        Value::Bool(true) => Ok("TRUE".to_string()),
        Value::Bool(false) => Ok("FALSE".to_string()),
        Value::Number(number) => Ok(number.to_string()),
        Value::String(text) => quoted(text),
        other => quoted(&other.to_string()),
    }
}

/// A name as a quoted identifier.
pub fn sql_identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// Text for a `--` comment: a line break would end it.
fn sql_comment(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

/// A cell in its column's type: in a text column a number is its text.
fn sql_cell(value: &Value, column: ExportColumnType) -> Result<String, MonitoringExportError> {
    match (value, column) {
        (Value::Number(number), ExportColumnType::Text) => {
            sql_literal(&Value::String(number.to_string()))
        }
        (Value::Bool(flag), ExportColumnType::Text) => {
            sql_literal(&Value::String(flag.to_string()))
        }
        _ => sql_literal(value),
    }
}

pub fn to_sql(data: &ExportData, table: &ExportTable) -> Result<Vec<u8>, MonitoringExportError> {
    let mut out = String::new();
    let scope = |value: &Option<String>| value.clone().unwrap_or_else(|| "all".to_string());
    let range = |value: Option<DateTime<Utc>>| {
        value.map_or("-".to_string(), |at| {
            at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        })
    };
    let header = [
        "Monitoring export (PostgreSQL)".to_string(),
        format!("Election event: {}", data.election_event_id),
        format!(
            "Dashboard: {}; widget: {}",
            data.dashboard_id,
            data.widget_id.as_deref().unwrap_or("all")
        ),
        format!(
            "Snapshot revision {} as of {}",
            data.snapshot_revision,
            data.as_of
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        ),
        format!(
            "Scope: {} {}; {} {}; {} {}",
            ScopeSelector::Region,
            scope(&data.scope.region),
            ScopeSelector::Post,
            scope(&data.pinned_post.clone().or(data.scope.post.clone())),
            ScopeSelector::Country,
            scope(&data.scope.country),
        ),
        format!(
            "Series buckets from {} up to, not including, {}",
            range(data.from),
            range(data.to)
        ),
    ];
    for line in header {
        out.push_str(&format!("-- {}\n", sql_comment(&line)));
    }
    out.push_str("BEGIN;\n");
    let table_name = sql_identifier(SQL_TABLE);
    out.push_str(&format!("CREATE TABLE {table_name} (\n"));
    let definitions: Vec<String> = table
        .columns
        .iter()
        .map(|(name, kind)| format!("  {} {}", sql_identifier(name), kind.sql()))
        .collect();
    out.push_str(&definitions.join(",\n"));
    out.push_str("\n);\n");
    let names: Vec<String> = table
        .columns
        .iter()
        .map(|(name, _)| sql_identifier(name))
        .collect();
    for batch in table.rows.chunks(INSERT_BATCH) {
        out.push_str(&format!(
            "INSERT INTO {table_name} ({}) VALUES\n",
            names.join(", ")
        ));
        let mut values = Vec::with_capacity(batch.len());
        for row in batch {
            let cells = row
                .iter()
                .zip(&table.columns)
                .map(|(value, (_, kind))| sql_cell(value, *kind))
                .collect::<Result<Vec<String>, MonitoringExportError>>()?;
            values.push(format!("  ({})", cells.join(", ")));
        }
        out.push_str(&values.join(",\n"));
        out.push_str(";\n");
    }
    out.push_str("COMMIT;\n");
    Ok(out.into_bytes())
}

#[cfg(test)]
#[path = "export_tests.rs"]
mod export_tests;
