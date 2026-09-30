// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Changing an event's monitoring configuration: checking a document,
//! saving or removing it, resetting the event to a preset and switching the
//! Dashboard tab; and reading the documents and their revisions.
//!
//! Reading and checking need `monitoring-configure`; a change also needs
//! `election-event-write`, and the event must not be locked down. A change
//! is by the token's subject. Windmill's configuration store makes each
//! change as one transaction, after the policy and the chart engine have
//! accepted it, and has the electoral log record it before it commits.

use crate::ports::monitoring_renderer::{ColorScheme, RenderBoard};
use crate::routes::monitoring::authorize_monitoring;
use crate::services::dependencies::HarvestServices;
use crate::services::monitoring::{
    author, event_ref, hasura_client, is_locked_down, live_config, problems,
    refuse_when_locked_down, MonitoringError, MonitoringResult, ProblemView,
    QueryTableView, RenderResponse, RenderState, TableView,
};
use crate::services::monitoring_checks::{
    check_boards, sample_board, RendererChecks,
};
use crate::services::monitoring_svg::sanitize_svg;
use chrono::{DateTime, Utc};
use indexmap::IndexMap;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::monitoring::config::{ConfigKind, ConfigSet, DEFAULT_THEME};
use sequent_core::monitoring::presets::{self, PRESETS};
use sequent_core::monitoring::problem::{Report, Severity};
use sequent_core::monitoring::revision::{
    check_edit, DashboardMode, DocumentChange, Edit, RevisionOrigin,
};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::instrument;
use windmill::services::monitoring::config_store::{
    get_document, history, reset_to_preset, save, set_mode, Author,
    DocumentEdit, EventRef, ExpectedHead, ModeError, ModeOutcome, ResetError,
    ResetOutcome, SaveError, SaveOutcome, StoredRevision,
};

fn configure(claims: &JwtClaims) -> MonitoringResult<()> {
    authorize_monitoring(claims, vec![Permissions::MONITORING_CONFIGURE])
}

fn configure_and_write(claims: &JwtClaims) -> MonitoringResult<()> {
    authorize_monitoring(
        claims,
        vec![
            Permissions::MONITORING_CONFIGURE,
            Permissions::ELECTION_EVENT_WRITE,
        ],
    )
}

/// The event's configuration as one set; empty for an event never
/// configured.
async fn live_set(
    services: &HarvestServices,
    event: EventRef,
) -> MonitoringResult<(ConfigSet, Option<i64>)> {
    let mut client = hasura_client(services).await?;
    let transaction = client
        .transaction()
        .await
        .map_err(MonitoringError::internal)?;
    let live = live_config(&transaction, event).await?;
    transaction
        .commit()
        .await
        .map_err(MonitoringError::internal)?;
    Ok(match live {
        Some(live) => (live.assembled.set, Some(live.generation)),
        None => (ConfigSet::default(), None),
    })
}

/// Refuses a change to an event that is locked down.
async fn check_not_locked_down(
    client: &mut deadpool_postgres::Client,
    event: EventRef,
) -> MonitoringResult<()> {
    let transaction = client
        .transaction()
        .await
        .map_err(MonitoringError::internal)?;
    let locked = is_locked_down(&transaction, event).await?;
    transaction
        .commit()
        .await
        .map_err(MonitoringError::internal)?;
    refuse_when_locked_down(locked)
}

#[derive(Debug, Serialize)]
pub struct AuthorView {
    id: String,
    name: Option<String>,
}

impl From<&Author> for AuthorView {
    fn from(author: &Author) -> Self {
        Self {
            id: author.id.clone(),
            name: author.name.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// validate-config
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ValidateConfigInput {
    election_event_id: String,
    kind: ConfigKind,
    key: String,
    yaml: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValidationResult {
    Valid,
    Invalid,
}

#[derive(Debug, Serialize)]
pub struct ValidateConfigOutput {
    result: ValidationResult,
    problems: Vec<ProblemView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    preview: Option<RenderResponse>,
}

fn verdict(
    report: Report,
    preview: Option<RenderResponse>,
) -> ValidateConfigOutput {
    let result = if report.is_accepted() {
        ValidationResult::Valid
    } else {
        ValidationResult::Invalid
    };
    ValidateConfigOutput {
        result,
        problems: problems(report.problems.iter()),
        preview,
    }
}

#[instrument(skip(claims, services, body))]
#[post("/monitoring/validate-config", format = "json", data = "<body>")]
pub async fn validate_config(
    body: Json<ValidateConfigInput>,
    claims: JwtClaims,
    services: &State<HarvestServices>,
) -> MonitoringResult<Json<ValidateConfigOutput>> {
    configure(&claims)?;
    let input = body.into_inner();
    let event = event_ref(&claims, &input.election_event_id)?;
    let (set, _) = live_set(services, event).await?;
    let checked = match check_edit(
        &set,
        input.kind,
        &input.key,
        Edit::Upsert(&input.yaml),
    ) {
        Ok(checked) => checked,
        Err(report) => return Ok(Json(verdict(report, None))),
    };
    let mut report = checked.report.clone();
    let engine = check_boards(
        services.monitoring_renderer.as_ref(),
        &checked.set,
        input.kind,
        &input.key,
    )
    .await
    .map_err(|error| {
        MonitoringError::new(
            Status::ServiceUnavailable,
            "MONITORING_CHECKS_UNAVAILABLE",
            format!(
                "The chart engine could not check the configuration: {error}"
            ),
        )
    })?;
    report.extend(engine);
    let preview = if report.is_accepted() && input.kind == ConfigKind::Widget {
        preview_widget(services, &checked.set, &input.key).await
    } else {
        None
    };
    Ok(Json(verdict(report, preview)))
}

/// The widget drawn on the sample figures, as a preview of what was checked.
async fn preview_widget(
    services: &HarvestServices,
    set: &ConfigSet,
    key: &str,
) -> Option<RenderResponse> {
    let widget = set.widgets.get(key)?;
    let board = sample_board(set, widget, DEFAULT_THEME, &IndexMap::new())?;
    let table = board.data.as_ref().map(TableView::from);
    let tables = QueryTableView::all(&board.queries);
    let drawn = services
        .monitoring_renderer
        .render(RenderBoard {
            board: board.board,
            width: 640,
            height: board.height,
            color_scheme: ColorScheme::Light,
            locale: "en".to_string(),
        })
        .await;
    let mut response = match drawn {
        Ok(drawn) => match sanitize_svg(&drawn.svg) {
            Ok(svg) => {
                let mut response = RenderResponse::state(RenderState::Rendered);
                response.svg = Some(svg);
                response.render_ms = Some(drawn.render_ms);
                response.diagnostics = problems(drawn.warnings.iter());
                response
            }
            Err(_) => {
                let mut response =
                    RenderResponse::state(RenderState::RenderFailed);
                response.reason = Some("UNSAFE_OUTPUT".into());
                response
            }
        },
        Err(error) => {
            let mut response = RenderResponse::state(RenderState::RenderFailed);
            response.reason = Some(error.to_string());
            response
        }
    };
    response.table = table;
    response.tables = tables;
    Some(response)
}

// ---------------------------------------------------------------------------
// save-config
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChangeInput {
    Upsert,
    Delete,
}

#[derive(Debug, Deserialize)]
pub struct SaveConfigInput {
    election_event_id: String,
    kind: ConfigKind,
    key: String,
    #[serde(default)]
    yaml: Option<String>,
    /// The revision the editor started from; none for a new document.
    #[serde(default)]
    expected_revision: Option<i32>,
    change: ChangeInput,
}

#[derive(Debug, Serialize)]
pub struct SaveConfigOutput {
    revision: i32,
    generation: i64,
    warnings: Vec<ProblemView>,
}

fn save_error(error: SaveError) -> MonitoringError {
    match error {
        SaveError::NotConfigured => MonitoringError::not_found(
            "The event has no monitoring configuration; reset it to a preset first.",
        ),
        SaveError::Conflict { current } => MonitoringError::new(
            Status::Conflict,
            "MONITORING_CONFLICT",
            "The document changed since it was opened.",
        )
        .with(
            "current_revision",
            json!(current.as_ref().map(|current| current.revision)),
        )
        .with(
            "author",
            json!(current.as_ref().map(|current| AuthorView::from(&current.author))),
        )
        .with(
            "time",
            json!(current.as_ref().map(|current| current.created_at)),
        ),
        SaveError::Invalid(report) => MonitoringError::invalid(&report),
        SaveError::ChecksUnavailable(error) => MonitoringError::new(
            Status::ServiceUnavailable,
            "MONITORING_CHECKS_UNAVAILABLE",
            format!("The chart engine could not check the change: {error}"),
        ),
        SaveError::Busy => {
            let mut error = MonitoringError::new(
                Status::ServiceUnavailable,
                "MONITORING_BUSY",
                "Other changes to the event came first; try again.",
            );
            error.retry_after = Some(1);
            error
        }
        SaveError::Internal(error) => MonitoringError::internal(error),
    }
}

#[instrument(skip(claims, services, body))]
#[post("/monitoring/save-config", format = "json", data = "<body>")]
pub async fn save_config(
    body: Json<SaveConfigInput>,
    claims: JwtClaims,
    services: &State<HarvestServices>,
) -> MonitoringResult<Json<SaveConfigOutput>> {
    configure_and_write(&claims)?;
    let input = body.into_inner();
    let event = event_ref(&claims, &input.election_event_id)?;
    let edit = match (input.change, input.yaml.as_deref()) {
        (ChangeInput::Upsert, Some(yaml)) => Edit::Upsert(yaml),
        (ChangeInput::Upsert, None) => {
            return Err(MonitoringError::bad_request(
                "An UPSERT needs its yaml.",
            ))
        }
        (ChangeInput::Delete, _) => Edit::Delete,
    };
    let expected = match input.expected_revision {
        Some(revision) => ExpectedHead::At(revision),
        None => ExpectedHead::Absent,
    };
    let mut client = hasura_client(services).await?;
    check_not_locked_down(&mut client, event).await?;
    let checks = RendererChecks {
        renderer: services.monitoring_renderer.clone(),
    };
    let outcome = save(
        &mut client,
        &checks,
        services.monitoring_audit.as_ref(),
        event,
        &author(&claims),
        DocumentEdit {
            kind: input.kind,
            key: &input.key,
            edit,
            expected,
        },
    )
    .await
    .map_err(save_error)?;
    let (revision, warnings) = match outcome {
        SaveOutcome::Saved { revision, warnings } => (revision, warnings),
        SaveOutcome::Unchanged(revision) => (revision, Report::default()),
    };
    Ok(Json(SaveConfigOutput {
        revision: revision.revision,
        generation: revision.config_generation,
        warnings: problems(
            warnings
                .problems
                .iter()
                .filter(|problem| problem.severity == Severity::Warning),
        ),
    }))
}

// ---------------------------------------------------------------------------
// reset-to-preset, list-presets, set-mode
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ResetToPresetInput {
    election_event_id: String,
    preset_id: String,
    /// What the Dashboard tab shows once reset; the configured dashboards
    /// unless said otherwise.
    #[serde(default)]
    mode: Option<DashboardMode>,
}

#[derive(Debug, Serialize)]
pub struct GenerationOutput {
    generation: i64,
    /// Always sent, even empty: Hasura passes a missing list on as missing.
    warnings: Vec<ProblemView>,
}

async fn current_generation(
    services: &HarvestServices,
    event: EventRef,
) -> MonitoringResult<i64> {
    Ok(live_set(services, event).await?.1.unwrap_or(0))
}

#[instrument(skip(claims, services))]
#[post("/monitoring/reset-to-preset", format = "json", data = "<body>")]
pub async fn reset_config_to_preset(
    body: Json<ResetToPresetInput>,
    claims: JwtClaims,
    services: &State<HarvestServices>,
) -> MonitoringResult<Json<GenerationOutput>> {
    configure_and_write(&claims)?;
    let input = body.into_inner();
    let event = event_ref(&claims, &input.election_event_id)?;
    let mut client = hasura_client(services).await?;
    check_not_locked_down(&mut client, event).await?;
    let outcome = reset_to_preset(
        &mut client,
        services.monitoring_audit.as_ref(),
        event,
        &author(&claims),
        &input.preset_id,
        input.mode.unwrap_or(DashboardMode::Configured),
    )
    .await
    .map_err(|error| match error {
        ResetError::NotFound => {
            MonitoringError::not_found("There is no such election event.")
        }
        ResetError::UnknownPreset => {
            MonitoringError::not_found("There is no such preset.")
        }
        ResetError::Invalid(report) => MonitoringError::invalid(&report),
        ResetError::Internal(error) => MonitoringError::internal(error),
    })?;
    Ok(Json(match outcome {
        ResetOutcome::Reset {
            generation,
            warnings,
            ..
        } => GenerationOutput {
            generation,
            warnings: problems(warnings.problems.iter()),
        },
        ResetOutcome::Unchanged => GenerationOutput {
            generation: current_generation(services, event).await?,
            warnings: vec![],
        },
    }))
}

#[derive(Debug, Default, Deserialize)]
pub struct ListPresetsInput {
    #[serde(default)]
    #[allow(dead_code)]
    election_event_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PresetSummary {
    id: String,
    version: u32,
    title: String,
    description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListPresetsOutput {
    presets: Vec<PresetSummary>,
}

#[instrument(skip(claims))]
#[post("/monitoring/list-presets", format = "json", data = "<body>")]
pub async fn list_presets(
    body: Json<ListPresetsInput>,
    claims: JwtClaims,
) -> MonitoringResult<Json<ListPresetsOutput>> {
    configure(&claims)?;
    let _ = body.into_inner();
    let presets = PRESETS
        .iter()
        .filter_map(|source| presets::load(source.id)?.ok())
        .map(|preset| PresetSummary {
            id: preset.manifest.id,
            version: preset.manifest.version,
            title: preset.manifest.title,
            description: preset.manifest.description,
        })
        .collect();
    Ok(Json(ListPresetsOutput { presets }))
}

#[derive(Debug, Serialize)]
pub struct SetModeOutput {
    mode: DashboardMode,
    generation: i64,
}

#[derive(Debug, Deserialize)]
pub struct SetModeInput {
    election_event_id: String,
    mode: DashboardMode,
}

#[instrument(skip(claims, services))]
#[post("/monitoring/set-mode", format = "json", data = "<body>")]
pub async fn set_dashboard_mode(
    body: Json<SetModeInput>,
    claims: JwtClaims,
    services: &State<HarvestServices>,
) -> MonitoringResult<Json<SetModeOutput>> {
    configure_and_write(&claims)?;
    let input = body.into_inner();
    let event = event_ref(&claims, &input.election_event_id)?;
    let mut client = hasura_client(services).await?;
    let outcome = set_mode(
        &mut client,
        services.monitoring_audit.as_ref(),
        event,
        &author(&claims),
        input.mode,
    )
    .await
    .map_err(|error| match error {
        ModeError::NotConfigured => MonitoringError::not_found(
            "The event has no monitoring configuration; reset it to a preset first.",
        ),
        ModeError::Invalid(report) => MonitoringError::invalid(&report),
        ModeError::Internal(error) => MonitoringError::internal(error),
    })?;
    let generation = match outcome {
        ModeOutcome::Switched { generation } => generation,
        ModeOutcome::Unchanged => current_generation(services, event).await?,
    };
    Ok(Json(SetModeOutput {
        mode: input.mode,
        generation,
    }))
}

// ---------------------------------------------------------------------------
// list-config, get-config
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ListConfigInput {
    election_event_id: String,
}

#[derive(Debug, Serialize)]
pub struct DocumentSummary {
    kind: ConfigKind,
    key: String,
    revision: i32,
    origin: RevisionOrigin,
    author: AuthorView,
    created_at: DateTime<Utc>,
    sha256: Option<String>,
}

impl From<&StoredRevision> for DocumentSummary {
    fn from(revision: &StoredRevision) -> Self {
        Self {
            kind: revision.kind,
            key: revision.key.clone(),
            revision: revision.revision,
            origin: revision.origin,
            author: AuthorView::from(&revision.author),
            created_at: revision.created_at,
            sha256: revision.sha256.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PresetRef {
    id: String,
    version: i32,
}

#[derive(Debug, Serialize)]
pub struct ListConfigOutput {
    mode: DashboardMode,
    generation: i64,
    preset: Option<PresetRef>,
    documents: Vec<DocumentSummary>,
}

#[instrument(skip(claims, services))]
#[post("/monitoring/list-config", format = "json", data = "<body>")]
pub async fn list_config(
    body: Json<ListConfigInput>,
    claims: JwtClaims,
    services: &State<HarvestServices>,
) -> MonitoringResult<Json<ListConfigOutput>> {
    configure(&claims)?;
    let input = body.into_inner();
    let event = event_ref(&claims, &input.election_event_id)?;
    let mut client = hasura_client(services).await?;
    let transaction = client
        .transaction()
        .await
        .map_err(MonitoringError::internal)?;
    let live = live_config(&transaction, event).await?;
    transaction
        .commit()
        .await
        .map_err(MonitoringError::internal)?;
    Ok(Json(match live {
        None => ListConfigOutput {
            mode: DashboardMode::Legacy,
            generation: 0,
            preset: None,
            documents: vec![],
        },
        Some(live) => ListConfigOutput {
            mode: live.mode,
            generation: live.generation,
            preset: live.preset.map(|preset| PresetRef {
                id: preset.id,
                version: preset.version,
            }),
            documents: live
                .documents
                .iter()
                .map(DocumentSummary::from)
                .collect(),
        },
    }))
}

const DEFAULT_HISTORY: usize = 20;
const MAX_HISTORY: usize = 100;

#[derive(Debug, Deserialize)]
pub struct GetConfigInput {
    election_event_id: String,
    kind: ConfigKind,
    key: String,
    #[serde(default)]
    revision: Option<i32>,
    /// History older than this revision, for the next page.
    #[serde(default)]
    before_revision: Option<i32>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct HistoryEntry {
    revision: i32,
    change: DocumentChange,
    origin: RevisionOrigin,
    author: AuthorView,
    created_at: DateTime<Utc>,
    sha256: Option<String>,
    generation: i64,
}

#[derive(Debug, Serialize)]
pub struct GetConfigOutput {
    kind: ConfigKind,
    key: String,
    yaml: Option<String>,
    revision: i32,
    change: DocumentChange,
    origin: RevisionOrigin,
    author: AuthorView,
    created_at: DateTime<Utc>,
    history: Vec<HistoryEntry>,
}

#[instrument(skip(claims, services))]
#[post("/monitoring/get-config", format = "json", data = "<body>")]
pub async fn get_config(
    body: Json<GetConfigInput>,
    claims: JwtClaims,
    services: &State<HarvestServices>,
) -> MonitoringResult<Json<GetConfigOutput>> {
    configure(&claims)?;
    let input = body.into_inner();
    let event = event_ref(&claims, &input.election_event_id)?;
    let mut client = hasura_client(services).await?;
    let transaction = client
        .transaction()
        .await
        .map_err(MonitoringError::internal)?;
    let document = get_document(
        &transaction,
        event,
        input.kind,
        &input.key,
        input.revision,
    )
    .await
    .map_err(MonitoringError::internal)?;
    let Some(document) = document else {
        return Err(MonitoringError::not_found("There is no such document."));
    };
    let revisions = history(&transaction, event, input.kind, &input.key)
        .await
        .map_err(MonitoringError::internal)?;
    transaction
        .commit()
        .await
        .map_err(MonitoringError::internal)?;
    let limit = input.limit.unwrap_or(DEFAULT_HISTORY).clamp(1, MAX_HISTORY);
    let history = revisions
        .iter()
        .filter(|revision| {
            input
                .before_revision
                .map_or(true, |before| revision.revision < before)
        })
        .take(limit)
        .map(|revision| HistoryEntry {
            revision: revision.revision,
            change: revision.change,
            origin: revision.origin,
            author: AuthorView::from(&revision.author),
            created_at: revision.created_at,
            sha256: revision.sha256.clone(),
            generation: revision.config_generation,
        })
        .collect();
    Ok(Json(GetConfigOutput {
        kind: document.kind,
        key: document.key.clone(),
        yaml: document.yaml.clone(),
        revision: document.revision,
        change: document.change,
        origin: document.origin,
        author: AuthorView::from(&document.author),
        created_at: document.created_at,
        history,
    }))
}

#[cfg(test)]
#[path = "../../tests/support/monitoring_config_routes.rs"]
mod monitoring_config_routes;

#[cfg(test)]
mod generation_output_tests {
    use super::GenerationOutput;
    use serde_json::json;

    /// Hasura passes an action's reply on as it is: a field left out is
    /// missing from the answer, which Apollo reports as an error.
    #[test]
    fn a_reset_without_warnings_lists_none() {
        let output = GenerationOutput {
            generation: 3,
            warnings: vec![],
        };
        assert_eq!(
            serde_json::to_value(&output).unwrap(),
            json!({"generation": 3, "warnings": []})
        );
    }
}
