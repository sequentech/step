// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What the monitoring routes share: who the viewer is and which elections
//! they may see, how errors are answered, and drawing one widget.
//!
//! Every read route works out the elections a viewer may see from their
//! permission labels before it reads any configuration or snapshot row, and
//! only ever reads figures counted for exactly that set of elections, so a
//! restricted viewer never receives a figure that includes an election they
//! may not see.

use crate::ports::monitoring_renderer::{
    ColorScheme, RenderBoard, RendererError,
};
use crate::ports::monitoring_snapshots::{ScopeRead, SnapshotHead};
use crate::services::dependencies::HarvestServices;
use crate::services::monitoring_cache::RenderKeyParts;
use crate::services::monitoring_svg::{sanitize_svg, UnsafeSvg};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use indexmap::IndexMap;
use rocket::http::{ContentType, Header, Status};
use rocket::response::{self, Responder, Response};
use rocket::serde::json::{self, Json};
use rocket::Request;
use sequent_core::ballot::{ElectionEventPresentation, LockedDown};
use sequent_core::monitoring::compute::{evaluate, event_days, QueryResult};
use sequent_core::monitoring::config::{
    ConfigKind, ConfigSet, DimensionMapping, DimensionOrigin, ScopeSelector,
    Settings, Theme, Widget, DEFAULT_QUERY_NAME, DEFAULT_THEME,
};
use sequent_core::monitoring::payload::ScopePayload;
use sequent_core::monitoring::policy::{parse_theme, parse_widget};
use sequent_core::monitoring::presets::SETTINGS_KEY;
use sequent_core::monitoring::problem::Code;
use sequent_core::monitoring::problem::{Problem, Report, Severity};
use sequent_core::monitoring::render_request::build_board;
use sequent_core::monitoring::resolve::{resolve_widget, DynamicOptionValues};
use sequent_core::monitoring::scope::{
    election_set_key, PostPinning, ScopeSelection,
};
use sequent_core::monitoring::sources::Producer;
use sequent_core::monitoring::voter::dimension_value;
use sequent_core::services::jwt::{decode_permission_labels, JwtClaims};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::io::Cursor;
use std::sync::Arc;
use tracing::{error, warn};
use uuid::Uuid;
use windmill::services::monitoring::config_store::{
    get_config_at_generation, get_live_config, Author, EventRef, LiveConfig,
    StoredRevision,
};
use windmill::services::monitoring::snapshot::empty_payload;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// An error answer, as Hasura passes it on: `{message, extensions: {code,
/// ...}}`, so the Admin Portal reads the details from the GraphQL error.
#[derive(Debug)]
pub struct MonitoringError {
    pub status: Status,
    pub code: &'static str,
    pub message: String,
    pub details: Map<String, Value>,
    pub retry_after: Option<u32>,
}

impl MonitoringError {
    pub fn new(
        status: Status,
        code: &'static str,
        message: impl Into<String>,
    ) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            details: Map::new(),
            retry_after: None,
        }
    }

    pub fn with(mut self, name: &str, value: Value) -> Self {
        self.details.insert(name.to_string(), value);
        self
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(Status::BadRequest, "MONITORING_BAD_REQUEST", message)
    }

    /// A value the request holds that reads, but not as what it must be.
    pub fn unprocessable(message: impl Into<String>) -> Self {
        Self::new(
            Status::UnprocessableEntity,
            "MONITORING_BAD_REQUEST",
            message,
        )
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(Status::NotFound, "MONITORING_NOT_FOUND", message)
    }

    pub fn forbidden_scope(message: impl Into<String>) -> Self {
        Self::new(Status::Forbidden, "MONITORING_FORBIDDEN_SCOPE", message)
    }

    pub fn internal(error: impl std::fmt::Display) -> Self {
        error!("Monitoring route failed: {error:#}");
        Self::new(
            Status::InternalServerError,
            "InternalServerError",
            // The first line only: the causes stay in the log.
            error.to_string(),
        )
    }

    pub fn invalid(report: &Report) -> Self {
        Self::new(
            Status::UnprocessableEntity,
            "MONITORING_INVALID",
            "The configuration is not valid.",
        )
        .with("problems", json!(problems(report.problems.iter())))
    }
}

/// A refusal from `authorize`.
impl From<(Status, String)> for MonitoringError {
    fn from((status, message): (Status, String)) -> Self {
        Self::new(status, "Unauthorized", message)
    }
}

impl<'r> Responder<'r, 'static> for MonitoringError {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        let mut extensions = self.details;
        extensions.insert("code".into(), Value::String(self.code.into()));
        let body = json!({"message": self.message, "extensions": extensions})
            .to_string();
        let mut response = Response::build();
        response
            .status(self.status)
            .header(ContentType::JSON)
            .sized_body(body.len(), Cursor::new(body));
        if let Some(seconds) = self.retry_after {
            response.header(Header::new("Retry-After", seconds.to_string()));
        }
        response.ok()
    }
}

pub type MonitoringResult<T> = Result<T, MonitoringError>;

/// A monitoring route's body as Rocket reads it: kept even when it is not
/// JSON or not what the route takes, so the route refuses it as it refuses
/// anything else, rather than Rocket answering without a code.
pub type MonitoringBody<'r, T> = Result<Json<T>, json::Error<'r>>;

/// The body a route takes, or a MONITORING_BAD_REQUEST that says what is
/// wrong with it: 400 when it is not JSON, 422 when it is JSON but not what
/// the route takes (a field missing or of the wrong type).
pub fn request_body<T>(body: MonitoringBody<'_, T>) -> MonitoringResult<T> {
    match body {
        Ok(body) => Ok(body.into_inner()),
        Err(json::Error::Io(error)) => Err(MonitoringError::bad_request(
            format!("The request body could not be read: {error}"),
        )),
        Err(json::Error::Parse(_, error)) if error.is_data() => {
            Err(MonitoringError::unprocessable(format!(
                "The request body is not what this route takes: {error}"
            )))
        }
        Err(json::Error::Parse(_, error)) => Err(MonitoringError::bad_request(
            format!("The request body is not JSON: {error}"),
        )),
    }
}

/// What a request under `/monitoring` that no route answered, or that
/// failed before its route ran (no valid credentials, no such route, a
/// body Rocket refused), is answered with: the monitoring refusal shape,
/// never the bare "Unknown Error" Hasura cannot explain.
#[catch(default)]
pub fn monitoring_catcher(status: Status, _: &Request<'_>) -> MonitoringError {
    let (code, message) = match status.code {
        401 => ("Unauthorized", "The request carries no valid credentials."),
        404 => (
            "MONITORING_NOT_FOUND",
            "No monitoring route takes this request; check its path and that its body is JSON.",
        ),
        400..=499 => (
            "MONITORING_BAD_REQUEST",
            "The request is not one this route takes.",
        ),
        _ => ("InternalServerError", "Internal error"),
    };
    MonitoringError::new(status, code, message)
}

// ---------------------------------------------------------------------------
// Problems
// ---------------------------------------------------------------------------

/// A problem as the Admin Portal reads it: the severity in capitals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProblemView {
    pub severity: ProblemSeverity,
    pub code: Value,
    pub path: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine_code: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProblemSeverity {
    Error,
    Warning,
}

impl From<&Problem> for ProblemView {
    fn from(problem: &Problem) -> Self {
        Self {
            severity: match problem.severity {
                Severity::Error => ProblemSeverity::Error,
                Severity::Warning => ProblemSeverity::Warning,
            },
            code: serde_json::to_value(problem.code).unwrap_or(Value::Null),
            path: problem.path.clone(),
            message: problem.message.clone(),
            engine_code: problem.engine_code.clone(),
        }
    }
}

pub fn problems<'a>(
    problems: impl IntoIterator<Item = &'a Problem>,
) -> Vec<ProblemView> {
    problems.into_iter().map(ProblemView::from).collect()
}

// ---------------------------------------------------------------------------
// Who is asking, and what they may see
// ---------------------------------------------------------------------------

/// One election of the event, as the dashboards name it.
#[derive(Debug, Clone, PartialEq)]
pub struct EventElection {
    pub id: Uuid,
    pub name: String,
    pub permission_label: Option<String>,
    pub annotations: Option<Value>,
}

/// A monitoring request's event, and the elections its viewer may see.
#[derive(Debug, Clone)]
pub struct Viewer {
    pub event: EventRef,
    /// The elections the viewer may see, in the event's order.
    pub elections: Vec<EventElection>,
    /// Whether the viewer may see fewer than every election of the event.
    pub restricted: bool,
    /// The election whose page the dashboard is shown on.
    pub pinned: Option<Uuid>,
    /// The key the snapshot job stores this set of elections' figures under.
    pub election_set_key: String,
}

impl Viewer {
    pub fn may_see(&self, election_id: &str) -> bool {
        Uuid::parse_str(election_id).is_ok_and(|id| {
            self.elections.iter().any(|election| election.id == id)
        })
    }

    pub fn election_ids(&self) -> Vec<Uuid> {
        self.elections.iter().map(|election| election.id).collect()
    }

    pub fn pinning(&self) -> PostPinning {
        match self.pinned {
            Some(id) => PostPinning::Pinned(id.to_string()),
            None => PostPinning::Selectable,
        }
    }
}

pub fn parse_uuid(value: &str, name: &str) -> MonitoringResult<Uuid> {
    Uuid::parse_str(value.trim()).map_err(|_| {
        MonitoringError::bad_request(format!("{name} is not a UUID"))
    })
}

/// The caller's tenant and the event named in the body.
pub fn event_ref(
    claims: &JwtClaims,
    election_event_id: &str,
) -> MonitoringResult<EventRef> {
    Ok(EventRef {
        tenant_id: parse_uuid(&claims.hasura_claims.tenant_id, "tenant_id")?,
        election_event_id: parse_uuid(election_event_id, "election_event_id")?,
    })
}

/// Who a configuration change is by: the token's subject.
pub fn author(claims: &JwtClaims) -> Author {
    Author {
        id: claims.hasura_claims.user_id.clone(),
        name: claims
            .name
            .clone()
            .or_else(|| claims.preferred_username.clone()),
    }
}

/// The elections a viewer with `labels` may see: every one when they have no
/// label, else the unlabeled ones and those with one of their labels.
pub fn allowed_elections(
    elections: Vec<EventElection>,
    labels: &[String],
) -> Vec<EventElection> {
    if labels.is_empty() {
        return elections;
    }
    elections
        .into_iter()
        .filter(|election| match &election.permission_label {
            Some(label) => labels.contains(label),
            None => true,
        })
        .collect()
}

pub async fn event_elections(
    transaction: &Transaction<'_>,
    event: EventRef,
) -> anyhow::Result<Vec<EventElection>> {
    let rows = transaction
        .query(
            "SELECT id, external_id, presentation, permission_label, annotations
             FROM sequent_backend.election
             WHERE tenant_id = $1 AND election_event_id = $2
             ORDER BY id",
            &[&event.tenant_id, &event.election_event_id],
        )
        .await?;
    let mut elections: Vec<EventElection> = rows
        .iter()
        .map(|row| {
            let id: Uuid = row.get("id");
            EventElection {
                id,
                name: post_name(
                    row.get("presentation"),
                    row.get("external_id"),
                    id,
                ),
                permission_label: row
                    .get::<_, Option<String>>("permission_label")
                    .filter(|label| !label.is_empty()),
                annotations: row.get("annotations"),
            }
        })
        .collect();
    elections.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
    Ok(elections)
}

/// The name the portal shows for an election, as the snapshot job names its
/// Posts: its English alias or name, else any language's; else its
/// external id; else its id.
fn post_name(
    presentation: Option<Value>,
    external_id: Option<String>,
    id: Uuid,
) -> String {
    let i18n = presentation
        .as_ref()
        .and_then(|presentation| presentation.get("i18n"))
        .and_then(Value::as_object);
    let field = |lang: &str, field: &str| {
        i18n?
            .get(lang)?
            .get(field)?
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    };
    field("en", "alias")
        .or_else(|| field("en", "name"))
        .or_else(|| {
            i18n?.keys().find_map(|lang| {
                field(lang, "alias").or_else(|| field(lang, "name"))
            })
        })
        .or_else(|| external_id.filter(|value| !value.trim().is_empty()))
        .unwrap_or_else(|| id.to_string())
}

/// A connection to the Hasura database.
pub async fn hasura_client(
    services: &HarvestServices,
) -> MonitoringResult<deadpool_postgres::Client> {
    services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(MonitoringError::internal)
}

/// The viewer of `election_event_id`, and the elections they may see; the
/// first thing each read route works out, before any configuration or
/// snapshot row. An election page they may not see is refused.
pub async fn viewer(
    transaction: &Transaction<'_>,
    claims: &JwtClaims,
    election_event_id: &str,
    election_id: Option<&str>,
) -> MonitoringResult<Viewer> {
    let event = event_ref(claims, election_event_id)?;
    let pinned = election_id
        .filter(|id| !id.trim().is_empty())
        .map(|id| parse_uuid(id, "election_id"))
        .transpose()?;
    let all = event_elections(transaction, event)
        .await
        .map_err(MonitoringError::internal)?;
    let total = all.len();
    let labels = decode_permission_labels(claims);
    let elections = allowed_elections(all, &labels);
    let restricted = elections.len() < total;
    if let Some(pinned) = pinned {
        if !elections.iter().any(|election| election.id == pinned) {
            return Err(MonitoringError::forbidden_scope(
                "The election is not one the viewer may see.",
            ));
        }
    }
    let election_set_key = election_set_key(
        elections.iter().map(|election| election.id.to_string()),
    )
    .map_err(MonitoringError::internal)?;
    Ok(Viewer {
        event,
        elections,
        restricted,
        pinned,
        election_set_key,
    })
}

/// Whether the event is locked down, when its configuration may not change.
pub async fn is_locked_down(
    transaction: &Transaction<'_>,
    event: EventRef,
) -> MonitoringResult<bool> {
    let row = transaction
        .query_opt(
            "SELECT presentation FROM sequent_backend.election_event
             WHERE tenant_id = $1 AND id = $2",
            &[&event.tenant_id, &event.election_event_id],
        )
        .await
        .map_err(MonitoringError::internal)?;
    let Some(row) = row else {
        return Err(MonitoringError::not_found(
            "There is no such election event.",
        ));
    };
    let presentation: Option<Value> = row.get("presentation");
    let Some(presentation) = presentation else {
        return Ok(false);
    };
    let presentation: ElectionEventPresentation =
        serde_json::from_value(presentation)
            .map_err(MonitoringError::internal)?;
    Ok(presentation.locked_down == Some(LockedDown::LOCKED_DOWN))
}

pub fn refuse_when_locked_down(locked: bool) -> MonitoringResult<()> {
    if locked {
        return Err(MonitoringError::new(
            Status::Forbidden,
            "MONITORING_LOCKED_DOWN",
            "The election event is locked down; its monitoring configuration cannot change.",
        ));
    }
    Ok(())
}

pub async fn live_config(
    transaction: &Transaction<'_>,
    event: EventRef,
) -> MonitoringResult<Option<LiveConfig>> {
    get_live_config(transaction, event)
        .await
        .map_err(MonitoringError::internal)
}

/// An instant a request names: RFC 3339 with its offset (`Z`, `+08:00`).
/// A time without one names no instant, so it is refused.
pub fn parse_instant(
    name: &str,
    text: Option<&str>,
) -> MonitoringResult<Option<DateTime<Utc>>> {
    let Some(text) = text.map(str::trim).filter(|text| !text.is_empty()) else {
        return Ok(None);
    };
    DateTime::parse_from_rfc3339(text)
        .map(|at| Some(at.with_timezone(&Utc)))
        .map_err(|_| {
            MonitoringError::unprocessable(format!(
                "`{name}` must be an RFC 3339 date and time with a UTC offset, \
                 such as 2026-05-04T08:00:00+08:00; '{text}' has no offset or does not read"
            ))
        })
}

/// A notice a drawing carries when it is not drawn with the configuration
/// its snapshot was counted under.
pub const CONFIG_AT_SNAPSHOT_UNAVAILABLE: &str =
    "CONFIG_AT_SNAPSHOT_UNAVAILABLE";
pub const CONFIG_NEWER_THAN_SNAPSHOT: &str = "CONFIG_NEWER_THAN_SNAPSHOT";

/// The configuration a snapshot is read with.
#[derive(Debug, Clone)]
pub struct SnapshotConfig {
    pub generation: i64,
    pub documents: Vec<StoredRevision>,
    pub set: ConfigSet,
    /// Why it is the live configuration rather than the snapshot's own.
    pub notice: Option<&'static str>,
}

impl SnapshotConfig {
    pub fn live(live: LiveConfig) -> Self {
        Self {
            generation: live.generation,
            documents: live.documents,
            set: live.assembled.set,
            notice: None,
        }
    }

    pub fn settings_revision(&self) -> Option<i32> {
        revision_of(&self.documents, ConfigKind::Settings, SETTINGS_KEY)
    }
}

/// The complete run `revision` a request pins. One the event no longer keeps
/// is gone (410, MONITORING_SNAPSHOT_PRUNED, saying `gone`); one after the
/// event's live run, or of an event with no run, was never issued (404).
pub async fn pinned_snapshot(
    services: &HarvestServices,
    event: EventRef,
    revision: i64,
    gone: &'static str,
) -> MonitoringResult<SnapshotHead> {
    let snapshots = &services.monitoring_snapshots;
    if let Some(head) = snapshots
        .complete(event, revision)
        .await
        .map_err(MonitoringError::internal)?
    {
        return Ok(head);
    }
    let live = snapshots
        .live(event)
        .await
        .map_err(MonitoringError::internal)?;
    if live.map_or(true, |live| revision > live.revision) {
        return Err(MonitoringError::not_found(
            "There is no such snapshot: the event has not counted it yet.",
        ));
    }
    Err(MonitoringError::new(
        Status::Gone,
        "MONITORING_SNAPSHOT_PRUNED",
        gone,
    ))
}

/// Whether the figures `head` counted are the ones the live configuration
/// reads. A run counts every source, at every scope, for every election set,
/// from the settings and the event's elections alone: no widget, dashboard or
/// theme is read. Those only read the figures (their queries, measures,
/// dimensions, grains and scopes are evaluated when drawn, and the dimensions
/// they may group by are the settings'). So the figures serve the live
/// configuration exactly when it has the settings revision they were counted
/// with.
pub fn counted_for_live(head: &SnapshotHead, live: &LiveConfig) -> bool {
    revision_of(&live.documents, ConfigKind::Settings, SETTINGS_KEY)
        == Some(head.settings_revision)
}

/// The configuration a dashboard draws `head` with. The live one when there
/// is no run, or when the run's figures are the ones it reads
/// ([`counted_for_live`]), so a saved title, chart, layout or theme shows at
/// once. Otherwise (the settings changed since) the one the run was counted
/// under, so what is drawn is what the figures were counted for; the live one
/// again when that generation is no longer kept, or has not what `has` asks
/// for (a widget added since), and those two say so.
///
/// Exports do not use this: an export of a run always reads the
/// configuration that run was counted under, so exporting the same revision
/// twice gives the same file.
pub async fn config_at_snapshot(
    services: &HarvestServices,
    event: EventRef,
    head: Option<&SnapshotHead>,
    live: LiveConfig,
    has: impl Fn(&ConfigSet) -> bool,
) -> MonitoringResult<SnapshotConfig> {
    let Some(head) = head.filter(|head| {
        head.config_generation != live.generation
            && !counted_for_live(head, &live)
    }) else {
        return Ok(SnapshotConfig::live(live));
    };
    let mut client = hasura_client(services).await?;
    let transaction = client
        .transaction()
        .await
        .map_err(MonitoringError::internal)?;
    let at =
        get_config_at_generation(&transaction, event, head.config_generation)
            .await
            .map_err(MonitoringError::internal)?;
    transaction
        .commit()
        .await
        .map_err(MonitoringError::internal)?;
    let notice = match at {
        Some(at) if has(&at.assembled.set) => {
            return Ok(SnapshotConfig {
                generation: at.generation,
                documents: at.documents,
                set: at.assembled.set,
                notice: None,
            })
        }
        Some(_) => CONFIG_NEWER_THAN_SNAPSHOT,
        None => CONFIG_AT_SNAPSHOT_UNAVAILABLE,
    };
    Ok(SnapshotConfig {
        notice: Some(notice),
        ..SnapshotConfig::live(live)
    })
}

/// Checks the values a request gives a widget's selectors against the
/// options it lists: a selector it lacks, or an option not listed, is a
/// problem. Days come from the figures, so a day is checked when the
/// figures are read.
pub fn check_selector_values(
    widget: &Widget,
    values: &IndexMap<String, String>,
    report: &mut Report,
) {
    for (name, value) in values {
        let path = format!("widget_selector_values.{}.{name}", widget.id);
        match widget.selectors.get(name) {
            None => report.push(Problem::error(
                Code::UnknownSelector,
                path,
                format!("'{}' has no selector '{name}'.", widget.id),
            )),
            Some(selector)
                if selector.options_from.is_none()
                    && !selector.options.contains_key(value) =>
            {
                report.push(Problem::error(
                    Code::UnknownOption,
                    path,
                    format!("'{value}' is not an option of '{name}'."),
                ))
            }
            Some(_) => {}
        }
    }
}

/// The revision of a document the event has.
pub fn revision_of(
    documents: &[StoredRevision],
    kind: ConfigKind,
    key: &str,
) -> Option<i32> {
    documents
        .iter()
        .find(|document| document.kind == kind && document.key == key)
        .map(|document| document.revision)
}

// ---------------------------------------------------------------------------
// Snapshots and scope options
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SnapshotView {
    pub revision: i64,
    pub as_of: DateTime<Utc>,
    pub checked_at: Option<DateTime<Utc>>,
}

impl From<&SnapshotHead> for SnapshotView {
    fn from(head: &SnapshotHead) -> Self {
        Self {
            revision: head.revision,
            as_of: head.as_of,
            checked_at: head.checked_at,
        }
    }
}

/// The region an election lies in, when the settings read it from an
/// election annotation.
pub fn election_region(
    election: &EventElection,
    settings: Option<&Settings>,
) -> Option<String> {
    let mapping = &settings?.scope.region;
    let Some(DimensionOrigin::ElectionAnnotation(key)) = mapping.origin()
    else {
        return None;
    };
    let raw = election.annotations.as_ref()?.get(key)?.as_str()?;
    dimension_value(mapping, Some(raw), Utc::now().date_naive())
}

pub fn dimension_label(
    mapping: Option<&DimensionMapping>,
    key: &str,
) -> String {
    mapping
        .and_then(|mapping| mapping.labels.get(key))
        .cloned()
        .unwrap_or_else(|| key.to_string())
}

// ---------------------------------------------------------------------------
// Drawing a widget
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RenderState {
    Rendered,
    NotConnected,
    NoSnapshot,
    ScopePending,
    RenderFailed,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TableColumnView {
    pub name: String,
    pub kind: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TableView {
    pub columns: Vec<TableColumnView>,
    pub rows: Vec<Vec<Value>>,
}

impl From<&QueryResult> for TableView {
    fn from(result: &QueryResult) -> Self {
        Self {
            columns: result
                .columns
                .iter()
                .map(|column| TableColumnView {
                    name: column.name.clone(),
                    kind: serde_json::to_value(column.kind)
                        .unwrap_or(Value::Null),
                })
                .collect(),
            rows: result.rows.clone(),
        }
    }
}

/// One query's figures, named as in the widget.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct QueryTableView {
    pub query: String,
    pub table: TableView,
}

impl QueryTableView {
    /// Every query's table, in the widget's query order.
    pub fn all(data: &IndexMap<String, QueryResult>) -> Vec<Self> {
        data.iter()
            .map(|(query, result)| Self {
                query: query.clone(),
                table: TableView::from(result),
            })
            .collect()
    }
}

/// What drawing a widget came to.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RenderResponse {
    pub state: RenderState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub svg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table: Option<TableView>,
    /// Every query's table, in the widget's query order; `table` is the
    /// default query's.
    pub tables: Vec<QueryTableView>,
    pub notices: Vec<String>,
    pub diagnostics: Vec<ProblemView>,
    pub ignored_selectors: Vec<ScopeSelector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub render_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_revision: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub as_of: Option<DateTime<Utc>>,
}

impl RenderResponse {
    pub fn state(state: RenderState) -> Self {
        Self {
            state,
            reason: None,
            svg: None,
            table: None,
            tables: vec![],
            notices: vec![],
            diagnostics: vec![],
            ignored_selectors: vec![],
            render_ms: None,
            snapshot_revision: None,
            as_of: None,
        }
    }

    pub fn invalid(report: &Report) -> Self {
        Self {
            diagnostics: problems(report.problems.iter()),
            ..Self::state(RenderState::Invalid)
        }
    }

    fn reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    fn at(mut self, head: &SnapshotHead) -> Self {
        self.snapshot_revision = Some(head.revision);
        self.as_of = Some(head.as_of);
        self
    }
}

/// A chart as drawn, checked and kept.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawnChart {
    pub svg: String,
    pub render_ms: u64,
    pub warnings: Vec<Problem>,
}

/// Why a chart was not drawn.
#[derive(Debug, Clone, PartialEq)]
pub enum DrawFailure {
    Renderer(RendererError),
    Unsafe(UnsafeSvg),
}

/// The width a chart is drawn at: the next multiple of 40 pixels, within
/// what the renderer draws.
pub fn width_bucket(width: i64) -> u32 {
    let width = width.clamp(200, 2400) as u32;
    width.div_ceil(40) * 40
}

/// The widget and theme documents a draw uses: as stored, or a draft.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Draft {
    #[serde(default)]
    pub widget_yaml: Option<String>,
    #[serde(default)]
    pub theme_yaml: Option<String>,
}

pub fn draft_revision(yaml: &str) -> String {
    format!("draft:{}", hex::encode(Sha256::digest(yaml.as_bytes())))
}

/// A widget to draw, and the revisions of what it is drawn from.
pub struct DrawPlan<'a> {
    pub viewer: &'a Viewer,
    pub dashboard: (&'a str, String),
    pub widget: &'a Widget,
    pub widget_revision: String,
    pub theme: Option<&'a Theme>,
    pub theme_ref: (&'a str, String),
    pub settings: Option<&'a Settings>,
    pub settings_revision: String,
    /// The settings revision drawn with; a run counted with an older one
    /// may lack a count these settings add.
    pub current_settings_revision: Option<i32>,
    pub dashboard_values: IndexMap<String, String>,
    pub scope: ScopeSelection,
    pub selector_values: IndexMap<String, String>,
    pub snapshot: Option<SnapshotHead>,
    pub width: i64,
    pub color_scheme: ColorScheme,
    pub locale: &'a str,
}

/// Why a widget is pending: its run was counted with older settings.
pub const SETTINGS_PENDING: &str = "SETTINGS_PENDING";

/// Whether a count is missing only because the run was counted with older
/// settings than the ones drawn with: the next run counts it.
fn counted_before_these_settings(
    report: &Report,
    head: &SnapshotHead,
    plan: &DrawPlan<'_>,
) -> bool {
    let only_not_counted = !report.problems.is_empty()
        && report
            .problems
            .iter()
            .all(|problem| problem.code == Code::NotCounted);
    only_not_counted
        && plan
            .current_settings_revision
            .is_some_and(|current| head.settings_revision < current)
}

/// Draws one widget for a viewer: reads the figures counted for the
/// viewer's elections at the widget's scope, evaluates the widget's queries
/// and has the renderer draw the board, once per distinct chart.
pub async fn draw_widget(
    services: &HarvestServices,
    plan: DrawPlan<'_>,
) -> MonitoringResult<RenderResponse> {
    let widget = plan.widget;
    let source = widget.source;
    let scope = plan.scope.for_widget(widget, &plan.viewer.pinning());
    let ignored = scope.ignored.clone();
    if let Producer::Pending(reason) = source.spec().producer {
        let mut response = RenderResponse::state(RenderState::NotConnected)
            .reason(reason.to_string());
        response.ignored_selectors = ignored;
        return Ok(response);
    }
    let Some(head) = plan.snapshot.clone() else {
        let mut response = RenderResponse::state(RenderState::NoSnapshot);
        response.ignored_selectors = ignored;
        return Ok(response);
    };
    let event = plan.viewer.event;
    let scope_key = scope.key.canonical();
    let read = services
        .monitoring_snapshots
        .read_scope(
            event,
            head.revision,
            source,
            &plan.viewer.election_set_key,
            &scope_key,
        )
        .await
        .map_err(MonitoringError::internal)?;
    let payload = match read {
        ScopeRead::NotCounted => {
            services
                .monitoring_snapshots
                .request_election_set(event, &plan.viewer.election_ids())
                .await
                .map_err(MonitoringError::internal)?;
            let mut response =
                RenderResponse::state(RenderState::ScopePending).at(&head);
            response.ignored_selectors = ignored;
            return Ok(response);
        }
        ScopeRead::NotConnected { reason } => {
            let mut response = RenderResponse::state(RenderState::NotConnected)
                .reason(reason.to_string())
                .at(&head);
            response.ignored_selectors = ignored;
            return Ok(response);
        }
        ScopeRead::Empty => match plan.settings {
            Some(settings) => empty_payload(source, settings),
            // A counted event always has its settings.
            None => {
                let mut response =
                    RenderResponse::state(RenderState::RenderFailed)
                        .reason("NO_SETTINGS")
                        .at(&head);
                response.ignored_selectors = ignored;
                return Ok(response);
            }
        },
        ScopeRead::Payload { text, .. } => {
            match serde_json::from_str::<ScopePayload>(&text) {
                Ok(payload) => payload,
                Err(error) => {
                    warn!("Unreadable monitoring payload: {error}");
                    let mut response =
                        RenderResponse::state(RenderState::RenderFailed)
                            .reason("UNREADABLE_SNAPSHOT")
                            .at(&head);
                    response.ignored_selectors = ignored;
                    return Ok(response);
                }
            }
        }
    };
    // A region or country the viewer's elections were never counted in is
    // not one they can ask for.
    if scope.key.region.is_some() || scope.key.country.is_some() {
        let catalogue = services
            .monitoring_snapshots
            .catalogue(event, head.revision, &plan.viewer.election_set_key)
            .await
            .map_err(MonitoringError::internal)?;
        let region_known = scope
            .key
            .region
            .as_ref()
            .map_or(true, |region| catalogue.regions.contains(region));
        let country_known = scope
            .key
            .country
            .as_ref()
            .map_or(true, |country| catalogue.countries.contains(country));
        if !region_known || !country_known {
            return Err(MonitoringError::forbidden_scope(
                "The region or country is not one of the viewer's elections.",
            ));
        }
    }

    let dynamic = DynamicOptionValues {
        event_days: event_days(&payload),
    };
    let resolved = match resolve_widget(
        widget,
        &plan.dashboard_values,
        &plan.selector_values,
        &dynamic,
    ) {
        Ok(resolved) => resolved,
        Err(report) => {
            let mut response = RenderResponse::invalid(&report).at(&head);
            response.ignored_selectors = ignored;
            return Ok(response);
        }
    };
    let mut data: IndexMap<String, QueryResult> = IndexMap::new();
    for (name, query) in &resolved.queries {
        match evaluate(source, query, &payload, plan.settings) {
            Ok(result) => {
                data.insert(name.clone(), result);
            }
            Err(report) => {
                let mut response =
                    if counted_before_these_settings(&report, &head, &plan) {
                        RenderResponse::state(RenderState::ScopePending)
                            .reason(SETTINGS_PENDING)
                            .at(&head)
                    } else {
                        RenderResponse::invalid(&report).at(&head)
                    };
                response.ignored_selectors = ignored;
                return Ok(response);
            }
        }
    }
    let table = data
        .get(DEFAULT_QUERY_NAME)
        .or_else(|| data.values().next())
        .map(TableView::from);
    let tables = QueryTableView::all(&data);
    let mut notices: Vec<String> = vec![];
    for result in data.values() {
        for notice in &result.notices {
            let notice = notice.to_string();
            if !notices.contains(&notice) {
                notices.push(notice);
            }
        }
    }

    let board = build_board(widget, plan.theme, &data);
    let width = width_bucket(plan.width);
    let color_scheme = match plan.color_scheme {
        ColorScheme::Light => "LIGHT",
        ColorScheme::Dark => "DARK",
    };
    let tenant_id = event.tenant_id.to_string();
    let election_event_id = event.election_event_id.to_string();
    let key = RenderKeyParts {
        tenant_id: &tenant_id,
        election_event_id: &election_event_id,
        dashboard: plan.dashboard.clone(),
        widget: (widget.id.as_str(), plan.widget_revision.clone()),
        theme: plan.theme_ref.clone(),
        settings_revision: plan.settings_revision.clone(),
        snapshot_revision: head.revision,
        election_set_key: &plan.viewer.election_set_key,
        scope_key,
        selector_values: plan.selector_values.clone(),
        renderer_version: services.monitoring_renderer.version(),
        locale: plan.locale,
        width_bucket: width,
        color_scheme,
    }
    .key();
    let renderer = services.monitoring_renderer.clone();
    let request = RenderBoard {
        board,
        width,
        height: widget.height,
        color_scheme: plan.color_scheme,
        locale: plan.locale.to_string(),
    };
    let drawn = services
        .monitoring_cache
        .get_or_render(key, || async move {
            let drawn = renderer
                .render(request)
                .await
                .map_err(DrawFailure::Renderer)?;
            let svg = sanitize_svg(&drawn.svg).map_err(DrawFailure::Unsafe)?;
            Ok(DrawnChart {
                svg,
                render_ms: drawn.render_ms,
                warnings: drawn.warnings,
            })
        })
        .await;

    let mut response = match drawn {
        Ok(chart) => {
            let chart: Arc<DrawnChart> = chart;
            let mut response = RenderResponse::state(RenderState::Rendered);
            response.svg = Some(chart.svg.clone());
            response.render_ms = Some(chart.render_ms);
            response.diagnostics = problems(chart.warnings.iter());
            response
        }
        Err(DrawFailure::Renderer(RendererError::Refused(refused))) => {
            RenderResponse {
                diagnostics: problems(refused.iter()),
                ..RenderResponse::state(RenderState::Invalid)
            }
        }
        Err(DrawFailure::Renderer(RendererError::Timeout)) => {
            RenderResponse::state(RenderState::RenderFailed)
                .reason("RENDER_TIMEOUT")
        }
        Err(DrawFailure::Renderer(RendererError::Unavailable(why))) => {
            warn!("Monitoring renderer unavailable: {why}");
            RenderResponse::state(RenderState::RenderFailed)
                .reason("RENDERER_UNAVAILABLE")
        }
        Err(DrawFailure::Unsafe(_)) => {
            RenderResponse::state(RenderState::RenderFailed)
                .reason("UNSAFE_OUTPUT")
        }
    };
    response.table = table;
    response.tables = tables;
    response.notices = notices;
    response.ignored_selectors = ignored;
    Ok(response.at(&head))
}

// ---------------------------------------------------------------------------
// Drafts
// ---------------------------------------------------------------------------

/// A draft widget, or why it cannot be drawn.
pub fn draft_widget(yaml: &str) -> Result<Widget, Report> {
    let parsed = parse_widget(yaml);
    match parsed.value {
        Some(widget) if parsed.report.is_accepted() => Ok(widget),
        _ => Err(parsed.report),
    }
}

pub fn draft_theme(yaml: &str) -> Result<Theme, Report> {
    let parsed = parse_theme(yaml);
    match parsed.value {
        Some(theme) if parsed.report.is_accepted() => Ok(theme),
        _ => Err(parsed.report),
    }
}

/// The theme a dashboard draws its widgets with.
pub fn dashboard_theme_id(
    set: &ConfigSet,
    dashboard_id: Option<&str>,
) -> String {
    dashboard_id
        .and_then(|id| set.dashboards.get(id))
        .and_then(|dashboard| dashboard.theme.clone())
        .unwrap_or_else(|| DEFAULT_THEME.to_string())
}
