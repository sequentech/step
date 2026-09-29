// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The monitoring dashboards an administrator views: which dashboards the
//! event has, one dashboard with everything its widgets need, and one
//! widget drawn.
//!
//! Each route authorizes first, then works out the elections the viewer
//! may see from their permission labels, and only then reads configuration
//! or snapshot rows. Figures are only read for exactly that set of
//! elections; a Post, region or country outside it is refused.

use crate::ports::monitoring_renderer::ColorScheme;
use crate::services::authorization::authorize;
use crate::services::dependencies::HarvestServices;
use crate::services::monitoring::{
    dashboard_theme_id, dimension_label, draft_revision, draft_theme,
    draft_widget, draw_widget, election_region, hasura_client, live_config,
    revision_of, viewer, Draft, DrawPlan, MonitoringError, MonitoringResult,
    RenderResponse, SnapshotView, Viewer,
};
use indexmap::IndexMap;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::monitoring::config::{
    ConfigKind, ScopeSelector, SelectorWords, Settings,
};
use sequent_core::monitoring::presets::SETTINGS_KEY;
use sequent_core::monitoring::revision::DashboardMode;
use sequent_core::monitoring::scope::ScopeSelection;
use sequent_core::monitoring::sources::{
    CountingUnit, DataSourceId, Measure, Producer, QueryTemplate,
    VoterDimensions,
};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::IntoEnumIterator;
use tracing::instrument;
use windmill::services::monitoring::config_store::LiveConfig;

pub(crate) fn authorize_monitoring(
    claims: &JwtClaims,
    permissions: Vec<Permissions>,
) -> MonitoringResult<()> {
    authorize(
        claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        permissions,
    )?;
    Ok(())
}

/// The viewer, and the event's configuration, read in one transaction that
/// is over before anything slow happens.
async fn viewer_and_config(
    services: &HarvestServices,
    claims: &JwtClaims,
    election_event_id: &str,
    election_id: Option<&str>,
) -> MonitoringResult<(Viewer, Option<LiveConfig>)> {
    let mut client = hasura_client(services).await?;
    let transaction = client
        .transaction()
        .await
        .map_err(MonitoringError::internal)?;
    let viewer =
        viewer(&transaction, claims, election_event_id, election_id).await?;
    let live = live_config(&transaction, viewer.event).await?;
    transaction
        .commit()
        .await
        .map_err(MonitoringError::internal)?;
    Ok((viewer, live))
}

// ---------------------------------------------------------------------------
// list-dashboards
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ListDashboardsInput {
    election_event_id: String,
    #[serde(default)]
    election_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DashboardSummary {
    id: String,
    title: String,
    requirements: Vec<String>,
    widget_count: usize,
}

#[derive(Debug, Serialize)]
pub struct ListDashboardsOutput {
    mode: DashboardMode,
    dashboards: Vec<DashboardSummary>,
    snapshot: Option<SnapshotView>,
}

#[instrument(skip(claims, services))]
#[post("/monitoring/list-dashboards", format = "json", data = "<body>")]
pub async fn list_dashboards(
    body: Json<ListDashboardsInput>,
    claims: JwtClaims,
    services: &State<HarvestServices>,
) -> MonitoringResult<Json<ListDashboardsOutput>> {
    authorize_monitoring(&claims, vec![Permissions::MONITORING_VIEW])?;
    let input = body.into_inner();
    let (viewer, live) = viewer_and_config(
        services,
        &claims,
        &input.election_event_id,
        input.election_id.as_deref(),
    )
    .await?;
    let Some(live) = live else {
        return Ok(Json(ListDashboardsOutput {
            mode: DashboardMode::Legacy,
            dashboards: vec![],
            snapshot: None,
        }));
    };
    let mut dashboards: Vec<_> =
        live.assembled.set.dashboards.values().collect();
    dashboards.sort_by(|a, b| (a.order, &a.id).cmp(&(b.order, &b.id)));
    let dashboards = dashboards
        .into_iter()
        .map(|dashboard| DashboardSummary {
            id: dashboard.id.clone(),
            title: dashboard.title.clone(),
            requirements: dashboard.requirements.clone(),
            widget_count: dashboard.layout.len(),
        })
        .collect();
    let snapshot = match live.mode {
        DashboardMode::Legacy => None,
        DashboardMode::Configured => services
            .monitoring_snapshots
            .live(viewer.event)
            .await
            .map_err(MonitoringError::internal)?
            .as_ref()
            .map(SnapshotView::from),
    };
    Ok(Json(ListDashboardsOutput {
        mode: live.mode,
        dashboards,
        snapshot,
    }))
}

// ---------------------------------------------------------------------------
// get-dashboard
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct GetDashboardInput {
    election_event_id: String,
    #[serde(default)]
    election_id: Option<String>,
    dashboard_id: String,
}

#[derive(Debug, Serialize)]
pub struct WidgetEntry {
    definition: Value,
    revision: i32,
}

#[derive(Debug, Serialize)]
pub struct CatalogEntry {
    id: String,
    title: String,
    source: DataSourceId,
    requirements: Vec<String>,
    revision: i32,
}

#[derive(Debug, Serialize)]
pub struct DocumentRef {
    id: String,
    revision: i32,
}

#[derive(Debug, Serialize)]
pub struct SettingsView {
    time_zone: String,
    unknown_label: String,
    selectors: IndexMap<ScopeSelector, SelectorWords>,
}

#[derive(Debug, Serialize)]
pub struct ScopeOption {
    key: String,
    label: String,
}

#[derive(Debug, Serialize)]
pub struct PostOption {
    key: String,
    label: String,
    region: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct ScopeOptions {
    regions: Vec<ScopeOption>,
    posts: Vec<PostOption>,
    countries: Vec<ScopeOption>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProducerState {
    Connected,
    NotConnected,
}

#[derive(Debug, Serialize)]
pub struct SourceView {
    counting_unit: CountingUnit,
    measures: Vec<Measure>,
    templates: Vec<QueryTemplate>,
    dimensions: Vec<String>,
    producer: ProducerState,
    reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GetDashboardOutput {
    dashboard: Value,
    dashboard_revision: i32,
    widgets: IndexMap<String, WidgetEntry>,
    catalog: Vec<CatalogEntry>,
    theme: Option<DocumentRef>,
    settings: SettingsView,
    settings_revision: i32,
    scope_options: ScopeOptions,
    restricted: bool,
    pinned_post: Option<String>,
    sources: IndexMap<DataSourceId, SourceView>,
    snapshot: Option<SnapshotView>,
}

fn json_of<T: Serialize>(value: &T) -> MonitoringResult<Value> {
    serde_json::to_value(value).map_err(MonitoringError::internal)
}

fn sources(settings: Option<&Settings>) -> IndexMap<DataSourceId, SourceView> {
    DataSourceId::iter()
        .map(|source| {
            let spec = source.spec();
            let mut dimensions: Vec<String> = spec
                .builtin_dimensions
                .iter()
                .map(|dimension| dimension.to_string())
                .collect();
            if spec.voter_dimensions == VoterDimensions::Configured {
                if let Some(settings) = settings {
                    dimensions.extend(settings.dimensions.keys().cloned());
                }
            }
            let (producer, reason) = match spec.producer {
                Producer::Available => (ProducerState::Connected, None),
                Producer::Pending(reason) => {
                    (ProducerState::NotConnected, Some(reason.to_string()))
                }
            };
            (
                source,
                SourceView {
                    counting_unit: spec.counting_unit,
                    measures: spec.measures.to_vec(),
                    templates: spec.templates.to_vec(),
                    dimensions,
                    producer,
                    reason,
                },
            )
        })
        .collect()
}

#[instrument(skip(claims, services))]
#[post("/monitoring/get-dashboard", format = "json", data = "<body>")]
pub async fn get_dashboard(
    body: Json<GetDashboardInput>,
    claims: JwtClaims,
    services: &State<HarvestServices>,
) -> MonitoringResult<Json<GetDashboardOutput>> {
    authorize_monitoring(&claims, vec![Permissions::MONITORING_VIEW])?;
    let input = body.into_inner();
    let (viewer, live) = viewer_and_config(
        services,
        &claims,
        &input.election_event_id,
        input.election_id.as_deref(),
    )
    .await?;
    let Some(live) = live else {
        return Err(MonitoringError::not_found(
            "The event has no monitoring configuration.",
        ));
    };
    let set = &live.assembled.set;
    let Some(dashboard) = set.dashboards.get(&input.dashboard_id) else {
        return Err(MonitoringError::not_found("There is no such dashboard."));
    };
    let revision = |kind: ConfigKind, key: &str| {
        revision_of(&live.documents, kind, key).unwrap_or(0)
    };
    let mut widgets = IndexMap::new();
    for item in &dashboard.layout {
        if let Some(widget) = set.widgets.get(&item.widget) {
            widgets.insert(
                item.widget.clone(),
                WidgetEntry {
                    definition: json_of(widget)?,
                    revision: revision(ConfigKind::Widget, &item.widget),
                },
            );
        }
    }
    let catalog = set
        .widgets
        .values()
        .map(|widget| CatalogEntry {
            id: widget.id.clone(),
            title: widget.title.clone(),
            source: widget.source,
            requirements: widget.requirements.clone(),
            revision: revision(ConfigKind::Widget, &widget.id),
        })
        .collect();
    let theme_id = dashboard_theme_id(set, Some(&dashboard.id));
    let theme = set.themes.get(&theme_id).map(|_| DocumentRef {
        revision: revision(ConfigKind::Theme, &theme_id),
        id: theme_id.clone(),
    });
    let settings = set.settings.as_ref();
    let settings_view = SettingsView {
        time_zone: settings
            .map(|settings| settings.time_zone.clone())
            .unwrap_or_else(|| "UTC".to_string()),
        unknown_label: settings
            .map(|settings| settings.unknown_label().to_string())
            .unwrap_or_else(|| "Unknown".to_string()),
        selectors: settings
            .map(|settings| settings.selectors.clone())
            .unwrap_or_default(),
    };

    let snapshot = services
        .monitoring_snapshots
        .live(viewer.event)
        .await
        .map_err(MonitoringError::internal)?;
    let region_mapping = settings.map(|settings| &settings.scope.region);
    let country_mapping = settings.map(|settings| &settings.scope.country);
    let mut scope_options = ScopeOptions::default();
    let posts = viewer.elections.iter().filter(|election| {
        viewer.pinned.map_or(true, |pinned| pinned == election.id)
    });
    scope_options.posts = posts
        .map(|election| PostOption {
            key: election.id.to_string(),
            label: election.name.clone(),
            region: election_region(election, settings),
        })
        .collect();
    let (regions, countries) = match &snapshot {
        Some(head) => {
            let catalogue = services
                .monitoring_snapshots
                .catalogue(
                    viewer.event,
                    head.revision,
                    &viewer.election_set_key,
                )
                .await
                .map_err(MonitoringError::internal)?;
            (catalogue.regions, catalogue.countries)
        }
        None => {
            let mut regions: Vec<String> = scope_options
                .posts
                .iter()
                .filter_map(|post| post.region.clone())
                .collect();
            regions.sort();
            regions.dedup();
            (regions, vec![])
        }
    };
    if viewer.pinned.is_none() {
        scope_options.regions = regions
            .into_iter()
            .map(|key| ScopeOption {
                label: dimension_label(region_mapping, &key),
                key,
            })
            .collect();
    }
    scope_options.countries = countries
        .into_iter()
        .map(|key| ScopeOption {
            label: dimension_label(country_mapping, &key),
            key,
        })
        .collect();
    // A restricted viewer's elections are counted as long as someone asks.
    if viewer.restricted && snapshot.is_some() {
        services
            .monitoring_snapshots
            .request_election_set(viewer.event, &viewer.election_ids())
            .await
            .map_err(MonitoringError::internal)?;
    }

    Ok(Json(GetDashboardOutput {
        dashboard: json_of(dashboard)?,
        dashboard_revision: revision(ConfigKind::Dashboard, &dashboard.id),
        widgets,
        catalog,
        theme,
        settings: settings_view,
        settings_revision: revision(ConfigKind::Settings, SETTINGS_KEY),
        scope_options,
        restricted: viewer.restricted,
        pinned_post: viewer.pinned.map(|pinned| pinned.to_string()),
        sources: sources(settings),
        snapshot: snapshot.as_ref().map(SnapshotView::from),
    }))
}

// ---------------------------------------------------------------------------
// render-widget
// ---------------------------------------------------------------------------

fn default_width() -> i64 {
    640
}

fn default_color_scheme() -> ColorScheme {
    ColorScheme::Light
}

fn default_locale() -> String {
    "en".to_string()
}

#[derive(Debug, Deserialize)]
pub struct RenderWidgetInput {
    election_event_id: String,
    #[serde(default)]
    election_id: Option<String>,
    dashboard_id: String,
    widget_id: String,
    #[serde(default)]
    scope: Option<ScopeSelection>,
    #[serde(default)]
    selector_values: Option<IndexMap<String, String>>,
    #[serde(default)]
    snapshot_revision: Option<i64>,
    #[serde(default = "default_width")]
    width: i64,
    #[serde(default = "default_color_scheme")]
    color_scheme: ColorScheme,
    #[serde(default = "default_locale")]
    locale: String,
    #[serde(default)]
    draft: Option<Draft>,
}

#[instrument(skip(claims, services))]
#[post("/monitoring/render-widget", format = "json", data = "<body>")]
pub async fn render_widget(
    body: Json<RenderWidgetInput>,
    claims: JwtClaims,
    services: &State<HarvestServices>,
) -> MonitoringResult<Json<RenderResponse>> {
    let input = body.into_inner();
    let draft = input.draft.clone().unwrap_or_default();
    let drafting = draft.widget_yaml.is_some() || draft.theme_yaml.is_some();
    let mut permissions = vec![Permissions::MONITORING_VIEW];
    if drafting {
        permissions.push(Permissions::MONITORING_CONFIGURE);
    }
    authorize_monitoring(&claims, permissions)?;
    let (viewer, live) = viewer_and_config(
        services,
        &claims,
        &input.election_event_id,
        input.election_id.as_deref(),
    )
    .await?;
    let scope = input.scope.clone().unwrap_or_default();
    if let Some(post) = scope.post.as_ref().filter(|post| !post.is_empty()) {
        if !viewer.may_see(post) {
            return Err(MonitoringError::forbidden_scope(
                "The Post is not one the viewer may see.",
            ));
        }
    }
    let Some(live) = live else {
        return Err(MonitoringError::not_found(
            "The event has no monitoring configuration.",
        ));
    };
    let set = &live.assembled.set;
    let Some(dashboard) = set.dashboards.get(&input.dashboard_id) else {
        return Err(MonitoringError::not_found("There is no such dashboard."));
    };
    let revision = |kind: ConfigKind, key: &str| {
        revision_of(&live.documents, kind, key)
            .map(|revision| revision.to_string())
            .unwrap_or_else(|| "none".to_string())
    };
    let drafted_widget;
    let (widget, widget_revision) = match &draft.widget_yaml {
        Some(yaml) => match draft_widget(yaml) {
            Ok(widget) => {
                drafted_widget = widget;
                (&drafted_widget, draft_revision(yaml))
            }
            Err(report) => return Ok(Json(RenderResponse::invalid(&report))),
        },
        None => match set.widgets.get(&input.widget_id) {
            Some(widget) => {
                (widget, revision(ConfigKind::Widget, &input.widget_id))
            }
            None => {
                return Err(MonitoringError::not_found(
                    "There is no such widget.",
                ))
            }
        },
    };
    let theme_id = dashboard_theme_id(set, Some(&dashboard.id));
    let drafted_theme;
    let (theme, theme_revision) = match &draft.theme_yaml {
        Some(yaml) => match draft_theme(yaml) {
            Ok(theme) => {
                drafted_theme = theme;
                (Some(&drafted_theme), draft_revision(yaml))
            }
            Err(report) => return Ok(Json(RenderResponse::invalid(&report))),
        },
        None => (
            set.themes.get(&theme_id),
            revision(ConfigKind::Theme, &theme_id),
        ),
    };
    let dashboard_values = dashboard
        .layout
        .iter()
        .find(|item| item.widget == input.widget_id)
        .map(|item| item.values.clone())
        .unwrap_or_default();
    let snapshot = match input.snapshot_revision {
        Some(revision) => {
            let head = services
                .monitoring_snapshots
                .complete(viewer.event, revision)
                .await
                .map_err(MonitoringError::internal)?;
            if head.is_none() {
                return Err(MonitoringError::new(
                    Status::Gone,
                    "MONITORING_SNAPSHOT_PRUNED",
                    "That snapshot is no longer kept.",
                ));
            }
            head
        }
        None => services
            .monitoring_snapshots
            .live(viewer.event)
            .await
            .map_err(MonitoringError::internal)?,
    };
    let response = draw_widget(
        services,
        DrawPlan {
            viewer: &viewer,
            dashboard: (
                dashboard.id.as_str(),
                revision(ConfigKind::Dashboard, &dashboard.id),
            ),
            widget,
            widget_revision,
            theme,
            theme_ref: (theme_id.as_str(), theme_revision),
            settings: set.settings.as_ref(),
            settings_revision: revision(ConfigKind::Settings, SETTINGS_KEY),
            dashboard_values,
            scope,
            selector_values: input.selector_values.clone().unwrap_or_default(),
            snapshot,
            width: input.width,
            color_scheme: input.color_scheme,
            locale: &input.locale,
        },
    )
    .await?;
    Ok(Json(response))
}

#[cfg(test)]
#[path = "../../tests/support/monitoring_routes.rs"]
mod monitoring_routes;
