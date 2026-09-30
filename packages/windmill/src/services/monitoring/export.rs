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
//! Every widget and query goes in one table in long form. Each row says what
//! it was read from (`snapshot_revision`, `as_of`, the widget's `scope`),
//! then the widget, the query, the row's place in the query and the query's
//! own columns, then `range_from`/`range_to`, `ignored_selectors` and
//! `notice`. The SQL file also says this in comments; a CSV has no comments,
//! so its columns carry it.
//!
//! A range `[from, to)` limits only the activity series: the hours starting
//! in it are kept before the queries run, so a daily bucket sums its kept
//! hours and a running total counts from the range's start, and a Day
//! selector no longer narrows (it is listed as ignored). Totals, statuses and
//! groups are as of the snapshot revision: a snapshot holds no history of
//! them. Series rows carry `range_from`/`range_to`; other rows leave them
//! empty.
//!
//! Reading is split from formatting ([`collect_export`], [`build_file`]) so
//! the file can be checked without uploading it.

use super::config_store::{get_config_at_generation, EventRef};
use super::snapshot::{
    complete_snapshot, empty_payload, read_scope, scope_catalogue, LiveSnapshot, ScopeRead,
};
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use indexmap::IndexMap;
use sequent_core::monitoring::compute::{evaluate, event_days, ColumnKind, QueryResult};
use sequent_core::monitoring::config::{
    ConfigSet, DynamicOptions, Param, ScopeSelector, Settings, Widget,
};
use sequent_core::monitoring::payload::ScopePayload;
use sequent_core::monitoring::problem::Report;
use sequent_core::monitoring::resolve::{resolve_widget, DynamicOptionValues};
use sequent_core::monitoring::scope::{
    election_set_key, PostPinning, ScopeKey, ScopeSelection, WidgetScope,
};
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
    /// A single widget's export reads these as the render route does: an
    /// unknown selector or option is refused. A whole dashboard's gives each
    /// widget those of its selectors it has, and skips a value that is not
    /// one of that widget's options.
    #[serde(default)]
    pub selector_values: IndexMap<String, String>,
    /// Each widget's own values, by widget id, over `selector_values`; read
    /// strictly, as a single widget's are. A widget the export does not
    /// include is skipped.
    #[serde(default)]
    pub widget_selector_values: IndexMap<String, IndexMap<String, String>>,
    /// The revision the dashboard showed.
    pub snapshot_revision: i64,
    pub format: MonitoringExportFormat,
    /// Series hours starting in `[from, to)` are kept; the rest of the
    /// figures are as of the revision. Instants; any RFC 3339 offset reads.
    #[serde(default)]
    pub from: Option<DateTime<Utc>>,
    #[serde(default)]
    pub to: Option<DateTime<Utc>>,
    pub document_id: String,
}

impl MonitoringExportRequest {
    fn has_range(&self) -> bool {
        self.from.is_some() || self.to.is_some()
    }
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
    /// The export did not finish in the time the task has.
    TimedOut {
        seconds: u64,
    },
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
            MonitoringExportError::TimedOut { .. } => "TIMED_OUT",
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
            MonitoringExportError::TimedOut { seconds } => write!(
                formatter,
                "{code}: the export did not finish within {seconds} seconds; export one \
                 widget or a shorter range"
            ),
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

/// One widget of an export: what it holds, and what it was read at.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportedWidget {
    pub id: String,
    /// The scope its figures were read at, as [`ScopeKey::canonical`] writes
    /// it: the dashboard's selection narrowed to what the widget follows.
    pub scope: String,
    /// The selectors that did not apply to it: dashboard scope selectors its
    /// source cannot be narrowed by, and its Day selector when a range is
    /// given.
    pub ignored_selectors: Vec<String>,
    pub data: WidgetData,
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
    pub widgets: Vec<ExportedWidget>,
}

impl ExportData {
    /// The dashboard's scope as asked, a Post the page pins included.
    pub fn scope_key(&self) -> String {
        let value = |value: &Option<String>| value.clone().filter(|value| !value.is_empty());
        let post = value(&self.pinned_post).or_else(|| value(&self.scope.post));
        ScopeKey {
            // A Post lies in one region, so choosing one drops the region.
            region: if post.is_some() {
                None
            } else {
                value(&self.scope.region)
            },
            post,
            country: value(&self.scope.country),
        }
        .canonical()
    }
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
    for (widget, dashboard_values) in items {
        let exported = collect_widget(
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
        widgets.push(exported);
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
) -> Result<Vec<(&'a Widget, &'a IndexMap<String, String>)>, MonitoringExportError> {
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
        items.push((widget, &item.values));
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
) -> Result<ExportedWidget, MonitoringExportError> {
    let source = widget.source;
    let mut selection = request.scope.clone();
    selection.post = selection.post.map(|post| post.to_ascii_lowercase());
    let scope = selection.for_widget(widget, pinning);
    let not_connected = |reason| ExportedWidget {
        id: widget.id.clone(),
        scope: scope.key.canonical(),
        ignored_selectors: scope_ignored(&scope),
        data: WidgetData::NotConnected { reason },
    };
    if let Producer::Pending(reason) = source.spec().producer {
        return Ok(not_connected(reason));
    }
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
        ScopeRead::NotConnected { reason } => return Ok(not_connected(reason)),
        ScopeRead::Empty => match settings {
            Some(settings) => empty_payload(source, settings),
            // A pass counts only with settings, so a counted scope has them.
            None => {
                return Err(MonitoringExportError::Internal(anyhow!(
                    "the configuration of '{}' has no settings",
                    widget.id
                )))
            }
        },
        ScopeRead::Payload { text, .. } => serde_json::from_str(&text).map_err(|error| {
            MonitoringExportError::Internal(anyhow!(
                "the stored figures of '{}' do not read: {error}",
                widget.id
            ))
        })?,
    };
    evaluate_widget(widget, dashboard_values, request, &scope, payload, settings)
}

fn scope_ignored(scope: &WidgetScope) -> Vec<String> {
    scope.ignored.iter().map(ToString::to_string).collect()
}

/// Evaluates a widget's queries on the figures of its scope, as the request
/// asks: its selector values, and the range, which keeps the series hours
/// in it and lifts the Day selector's narrowing.
pub fn evaluate_widget(
    widget: &Widget,
    dashboard_values: &IndexMap<String, String>,
    request: &MonitoringExportRequest,
    scope: &WidgetScope,
    mut payload: ScopePayload,
    settings: Option<&Settings>,
) -> Result<ExportedWidget, MonitoringExportError> {
    // The Day selector's options are the days of the whole series, as the
    // dashboard offered them.
    let dynamic = DynamicOptionValues {
        event_days: event_days(&payload),
    };
    let requested = selector_values_for(widget, request, &dynamic);
    let mut resolved = resolve_widget(widget, dashboard_values, &requested, &dynamic)
        .map_err(|report| invalid_report(&widget.id, &report))?;
    let mut ignored_selectors = scope_ignored(scope);
    if request.has_range() {
        keep_hours(&mut payload, request.from, request.to);
        let queries = widget.named_queries();
        for (name, query) in resolved.queries.iter_mut() {
            if query.day.take().is_none() {
                continue;
            }
            let selector = match queries.get(name).and_then(|query| query.day.as_ref()) {
                Some(Param::Selector(reference)) => reference.selector.clone(),
                _ => "day".to_string(),
            };
            if !ignored_selectors.contains(&selector) {
                ignored_selectors.push(selector);
            }
        }
    }
    let mut queries = IndexMap::new();
    for (name, query) in &resolved.queries {
        let result = evaluate(widget.source, query, &payload, settings)
            .map_err(|report| invalid_report(&widget.id, &report))?;
        queries.insert(name.clone(), result);
    }
    Ok(ExportedWidget {
        id: widget.id.clone(),
        scope: scope.key.canonical(),
        ignored_selectors,
        data: WidgetData::Evaluated { queries },
    })
}

/// The values a widget's selectors are resolved with. A single widget's
/// export, and a widget's own map, are read strictly: an unknown selector or
/// option is refused. A whole dashboard's shared values reach a widget only
/// for selectors it has and options it lists.
fn selector_values_for(
    widget: &Widget,
    request: &MonitoringExportRequest,
    dynamic: &DynamicOptionValues,
) -> IndexMap<String, String> {
    let mut values: IndexMap<String, String> = if request.widget_id.is_some() {
        request.selector_values.clone()
    } else {
        request
            .selector_values
            .iter()
            .filter(|(name, value)| {
                widget
                    .selectors
                    .get(*name)
                    .is_some_and(|selector| match selector.options_from {
                        Some(DynamicOptions::EventDays) => dynamic.event_days.contains(value),
                        None => selector.options.contains_key(*value),
                    })
            })
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect()
    };
    if let Some(own) = request.widget_selector_values.get(&widget.id) {
        values.extend(
            own.iter()
                .map(|(name, value)| (name.clone(), value.clone())),
        );
    }
    values
}

/// Keeps the series hours whose UTC start is in `[from, to)`. An hour whose
/// start does not read is kept, so evaluating it reports it.
pub fn keep_hours(
    payload: &mut ScopePayload,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
) {
    payload.series.retain(|bucket| {
        let Ok(start) = DateTime::parse_from_str(
            &format!("{}{}", bucket.start, bucket.utc_offset),
            "%Y-%m-%dT%H:%M:%S%:z",
        ) else {
            return true;
        };
        let start = start.with_timezone(&Utc);
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

const REVISION_COLUMN: &str = "snapshot_revision";
const AS_OF_COLUMN: &str = "as_of";
const SCOPE_COLUMN: &str = "scope";
const WIDGET_COLUMN: &str = "widget_id";
const QUERY_COLUMN: &str = "query";
const ROW_COLUMN: &str = "row";
const RANGE_FROM_COLUMN: &str = "range_from";
const RANGE_TO_COLUMN: &str = "range_to";
const IGNORED_COLUMN: &str = "ignored_selectors";
const NOTICE_COLUMN: &str = "notice";

/// The columns before a query's own.
const LEADING: [(&str, ExportColumnType); 6] = [
    (REVISION_COLUMN, ExportColumnType::Integer),
    (AS_OF_COLUMN, ExportColumnType::Text),
    (SCOPE_COLUMN, ExportColumnType::Text),
    (WIDGET_COLUMN, ExportColumnType::Text),
    (QUERY_COLUMN, ExportColumnType::Text),
    (ROW_COLUMN, ExportColumnType::Integer),
];

/// The columns after a query's own.
const TRAILING: [(&str, ExportColumnType); 4] = [
    (RANGE_FROM_COLUMN, ExportColumnType::Text),
    (RANGE_TO_COLUMN, ExportColumnType::Text),
    (IGNORED_COLUMN, ExportColumnType::Text),
    (NOTICE_COLUMN, ExportColumnType::Text),
];

/// The column of a series row that says when its bucket starts, in UTC.
const BUCKET_UTC: &str = "bucket_utc";

/// Why a query's only row in the file carries no figures: so a query, or a
/// whole widget, with nothing to show still appears.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmptyQuery {
    /// The query has no rows at all.
    NoRows,
    /// A series with no bucket in the range asked for.
    NoRowsInRange,
}

impl EmptyQuery {
    fn notice(self) -> &'static str {
        match self {
            EmptyQuery::NoRows => "NO_ROWS",
            EmptyQuery::NoRowsInRange => "NO_ROWS_IN_RANGE",
        }
    }
}

/// A query's column named like one of the table's own, renamed.
fn own_name(name: &str) -> String {
    let fixed = LEADING.iter().chain(TRAILING.iter());
    if fixed.map(|(fixed, _)| *fixed).any(|fixed| fixed == name) {
        format!("{name}_value")
    } else {
        name.to_string()
    }
}

fn instant(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// The long table: what each row was read at (`snapshot_revision`, `as_of`,
/// `scope`), `widget_id`, `query`, `row` (from 1 within the query), every
/// query's columns in the order first met, then `range_from` and `range_to`
/// (on series rows, when a range is given), `ignored_selectors` and
/// `notice`. A query without a column leaves it null. A query with no rows
/// has one row with no `row` number and no figures, whose notice says
/// `NO_ROWS_IN_RANGE` (a series, when a range is given) or `NO_ROWS`, as a
/// widget not connected has one saying `NOT_CONNECTED`.
pub fn export_table(data: &ExportData) -> ExportTable {
    let mut shared: IndexMap<String, ExportColumnType> = IndexMap::new();
    for widget in &data.widgets {
        if let WidgetData::Evaluated { queries } = &widget.data {
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
    let lead = LEADING.len();
    let width = lead + shared.len() + TRAILING.len();
    let text = |value: Option<String>| value.map_or(Value::Null, Value::String);
    let range_from = text(data.from.map(instant));
    let range_to = text(data.to.map(instant));
    let mut rows = Vec::new();
    for widget in &data.widgets {
        let ignored = text(Some(widget.ignored_selectors.join(", ")).filter(|x| !x.is_empty()));
        let blank = || {
            let mut row = vec![Value::Null; width];
            row[0] = Value::from(data.snapshot_revision);
            row[1] = Value::String(instant(data.as_of));
            row[2] = Value::String(widget.scope.clone());
            row[3] = Value::String(widget.id.clone());
            row[width - 2] = ignored.clone();
            row
        };
        match &widget.data {
            WidgetData::NotConnected { reason } => {
                let mut row = blank();
                row[width - 1] = Value::String(format!("NOT_CONNECTED: {reason}"));
                rows.push(row);
            }
            WidgetData::Evaluated { queries } => {
                for (name, result) in queries {
                    let series = result
                        .columns
                        .iter()
                        .any(|column| column.name == BUCKET_UTC);
                    let empty = result.rows.is_empty().then(|| {
                        if series && (data.from.is_some() || data.to.is_some()) {
                            EmptyQuery::NoRowsInRange
                        } else {
                            EmptyQuery::NoRows
                        }
                    });
                    let notices: Vec<String> = empty
                        .map(|empty| empty.notice().to_string())
                        .into_iter()
                        .chain(result.notices.iter().map(ToString::to_string))
                        .collect();
                    let notice = text(Some(notices.join("; ")).filter(|x| !x.is_empty()));
                    if empty.is_some() {
                        let mut row = blank();
                        row[4] = Value::String(name.clone());
                        if series {
                            row[width - 4] = range_from.clone();
                            row[width - 3] = range_to.clone();
                        }
                        row[width - 1] = notice;
                        rows.push(row);
                        continue;
                    }
                    let places: Vec<usize> = result
                        .columns
                        .iter()
                        .map(|column| {
                            shared.get_index_of(&own_name(&column.name)).unwrap_or(0) + lead
                        })
                        .collect();
                    for (number, cells) in (1_u64..).zip(&result.rows) {
                        let mut row = blank();
                        row[4] = Value::String(name.clone());
                        row[5] = Value::from(number);
                        for (place, cell) in places.iter().zip(cells) {
                            row[*place] = cell.clone();
                        }
                        if series {
                            row[width - 4] = range_from.clone();
                            row[width - 3] = range_to.clone();
                        }
                        row[width - 1] = notice.clone();
                        rows.push(row);
                    }
                }
            }
        }
    }
    let mut columns: Vec<(String, ExportColumnType)> = LEADING
        .iter()
        .map(|(name, kind)| (name.to_string(), *kind))
        .collect();
    columns.extend(shared);
    columns.extend(
        TRAILING
            .iter()
            .map(|(name, kind)| (name.to_string(), *kind)),
    );
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

/// The longest a part of a file name taken from the request is.
const NAME_PART: usize = 80;

/// The file's name: the dashboard, the widget, the scope, the revision and
/// the range, each with only letters, digits, `-` and `_`.
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
            .take(NAME_PART)
            .collect()
    };
    let compact = |at: DateTime<Utc>| at.format("%Y%m%dT%H%MZ").to_string();
    let mut parts = vec![format!("monitoring-{}", safe(&data.dashboard_id))];
    if let Some(widget) = &data.widget_id {
        parts.push(safe(widget));
    }
    parts.push(safe(&data.scope_key()));
    parts.push(format!("r{}", data.snapshot_revision));
    if let Some(from) = data.from {
        parts.push(format!("from{}", compact(from)));
    }
    if let Some(to) = data.to {
        parts.push(format!("to{}", compact(to)));
    }
    format!("{}.{}", parts.join("-"), format.extension())
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

/// A value as a PostgreSQL literal (with `standard_conforming_strings`,
/// which the file sets, so a backslash is only a backslash): NULL,
/// TRUE/FALSE, a number as JSON wrote it, text in single quotes with each
/// quote doubled. Text cannot hold a NUL, so a value with one is refused
/// rather than cut short. Lists and maps are written as their JSON text.
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

/// What the file's figures are as of, said in its header.
pub const AS_OF_NOTE: &str = "Totals, statuses and groups are as of the snapshot revision. \
    Only activity series rows (those with range_from/range_to) are limited to the range; their \
    cumulative columns count from the range's start";

pub fn to_sql(data: &ExportData, table: &ExportTable) -> Result<Vec<u8>, MonitoringExportError> {
    let mut out = String::new();
    let scope = |value: &Option<String>| value.clone().unwrap_or_else(|| "all".to_string());
    let range = |value: Option<DateTime<Utc>>| value.map_or("-".to_string(), instant);
    let mut header = vec![
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
            instant(data.as_of)
        ),
        format!(
            "Scope: {} {}; {} {}; {} {} ({})",
            ScopeSelector::Region,
            scope(&data.scope.region),
            ScopeSelector::Post,
            scope(&data.pinned_post.clone().or(data.scope.post.clone())),
            ScopeSelector::Country,
            scope(&data.scope.country),
            data.scope_key(),
        ),
        format!(
            "Series buckets from {} up to, not including, {}",
            range(data.from),
            range(data.to)
        ),
    ];
    if data.from.is_some() || data.to.is_some() {
        header.push(AS_OF_NOTE.to_string());
    } else {
        header.push("Totals, statuses and groups are as of the snapshot revision".to_string());
    }
    for line in header {
        out.push_str(&format!("-- {}\n", sql_comment(&line)));
    }
    out.push_str("BEGIN;\n");
    // The literals below are written for these; a client's own settings do
    // not change what they read as.
    out.push_str("SET standard_conforming_strings = on;\n");
    out.push_str("SET client_encoding = 'UTF8';\n");
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
