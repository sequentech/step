// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Exporting a dashboard's figures, or one widget's, as CSV or SQL: Harvest
//! checks what the viewer may see and that the run is kept, then Windmill's
//! `export_monitoring_data` task writes the document.
//!
//! `from` and `to` are instants: RFC 3339 with an offset, any offset; the
//! portal turns the local times it shows into instants with the settings'
//! time zone. A time without an offset names no instant and is refused.
//! The dashboard and widget are looked up in the configuration the dashboard
//! draws the run with ([`config_at_snapshot`]): the live one unless the
//! settings changed since the run was counted. The request names that
//! configuration's generation, which is what the task reads, so a widget
//! saved a moment ago exports as it is drawn, and the file does not change
//! with later saves.

use crate::routes::monitoring::{authorize_monitoring, viewer_and_config};
use crate::services::dependencies::HarvestServices;
use crate::services::monitoring::{
    check_selector_values, config_at_snapshot, parse_instant, pinned_snapshot,
    request_body, MonitoringBody, MonitoringError, MonitoringResult,
};
use indexmap::IndexMap;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::monitoring::config::ConfigSet;
use sequent_core::monitoring::problem::{Code, Problem, Report};
use sequent_core::monitoring::scope::ScopeSelection;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{error, instrument};
use uuid::Uuid;
use windmill::services::monitoring::export::{
    MonitoringExportFormat, MonitoringExportRequest,
};
use windmill::types::tasks::ETasksExecution;

#[derive(Debug, Deserialize)]
pub struct ExportInput {
    election_event_id: String,
    #[serde(default)]
    election_id: Option<String>,
    dashboard_id: String,
    #[serde(default)]
    widget_id: Option<String>,
    #[serde(default)]
    scope: Option<ScopeSelection>,
    #[serde(default)]
    selector_values: Option<IndexMap<String, String>>,
    /// Each widget's own picks, by widget id.
    #[serde(default)]
    widget_selector_values: Option<IndexMap<String, IndexMap<String, String>>>,
    snapshot_revision: i64,
    format: MonitoringExportFormat,
    /// Read in the handler, so a time without an offset is answered as
    /// such rather than as a body that does not parse.
    #[serde(default)]
    from: Option<String>,
    #[serde(default)]
    to: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExportOutput {
    document_id: String,
    task_execution: TasksExecution,
}

fn out_of_scope() -> MonitoringError {
    MonitoringError::forbidden_scope("The scope is not one the viewer may see.")
}

#[instrument(skip(claims, services))]
#[post("/monitoring/export", format = "json", data = "<body>")]
pub async fn export_monitoring(
    body: MonitoringBody<'_, ExportInput>,
    claims: JwtClaims,
    services: &State<HarvestServices>,
) -> MonitoringResult<Json<ExportOutput>> {
    authorize_monitoring(&claims, vec![Permissions::MONITORING_VIEW])?;
    let input = request_body(body)?;
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
            return Err(out_of_scope());
        }
    }
    let from = parse_instant("from", input.from.as_deref())?;
    let to = parse_instant("to", input.to.as_deref())?;
    if let (Some(from), Some(to)) = (from, to) {
        if from >= to {
            const MESSAGE: &str =
                "The end of the range must come after its start.";
            let mut report = Report::default();
            report.push(Problem::error(Code::InvalidValue, "to", MESSAGE));
            return Err(MonitoringError {
                message: MESSAGE.to_string(),
                ..MonitoringError::invalid(&report)
            });
        }
    }
    let Some(live) = live else {
        return Err(MonitoringError::not_found(
            "The event has no monitoring configuration.",
        ));
    };

    let snapshots = &services.monitoring_snapshots;
    let kept = pinned_snapshot(
        services,
        viewer.event,
        input.snapshot_revision,
        "That snapshot is no longer kept; export the current one.",
    )
    .await?;
    let (dashboard_id, widget_id) = (&input.dashboard_id, &input.widget_id);
    let config = config_at_snapshot(
        services,
        viewer.event,
        Some(&kept),
        live,
        |set: &ConfigSet| {
            set.dashboards.contains_key(dashboard_id)
                && widget_id.as_ref().map_or(true, |widget_id| {
                    set.widgets.contains_key(widget_id)
                })
        },
    )
    .await?;
    // The dashboard draws such a run with the live configuration and says
    // so; a file cannot say so, and would hold the live queries over
    // figures counted under another configuration.
    if config.notice.is_some() {
        return Err(MonitoringError::not_found(
            "That snapshot was counted under a configuration no longer kept; export again after the next update.",
        ));
    }
    let set = &config.set;
    let Some(dashboard) = set.dashboards.get(&input.dashboard_id) else {
        return Err(MonitoringError::not_found("There is no such dashboard."));
    };
    if let Some(widget_id) = &input.widget_id {
        if !dashboard
            .layout
            .iter()
            .any(|item| &item.widget == widget_id)
        {
            return Err(MonitoringError::not_found(
                "The dashboard has no such widget.",
            ));
        }
    }
    // Each widget's picks are its own: a selector it lacks, or an option it
    // does not list, is refused now rather than failing the task.
    let widget_selector_values =
        input.widget_selector_values.clone().unwrap_or_default();
    let mut report = Report::default();
    for (widget_id, values) in &widget_selector_values {
        let placed = dashboard
            .layout
            .iter()
            .any(|item| &item.widget == widget_id);
        if let Some(widget) = set.widgets.get(widget_id).filter(|_| placed) {
            check_selector_values(widget, values, &mut report);
        }
    }
    if let Some(widget) = input
        .widget_id
        .as_ref()
        .and_then(|widget_id| set.widgets.get(widget_id))
    {
        check_selector_values(
            widget,
            input.selector_values.as_ref().unwrap_or(&IndexMap::new()),
            &mut report,
        );
    }
    if !report.is_accepted() {
        return Err(MonitoringError {
            message: "A selector value is not one of its widget's options."
                .to_string(),
            ..MonitoringError::invalid(&report)
        });
    }
    let region = scope.region.as_ref().filter(|value| !value.is_empty());
    let country = scope.country.as_ref().filter(|value| !value.is_empty());
    if region.is_some() || country.is_some() {
        let catalogue = snapshots
            .catalogue(
                viewer.event,
                input.snapshot_revision,
                &viewer.election_set_key,
            )
            .await
            .map_err(MonitoringError::internal)?;
        if region.is_some_and(|region| !catalogue.regions.contains(region))
            || country
                .is_some_and(|country| !catalogue.countries.contains(country))
        {
            return Err(out_of_scope());
        }
    }

    let tenant_id = viewer.event.tenant_id.to_string();
    let election_event_id = viewer.event.election_event_id.to_string();
    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());
    let task_execution = services
        .ledger
        .post(
            &tenant_id,
            Some(&election_event_id),
            ETasksExecution::EXPORT_MONITORING_DATA,
            &executer_name,
        )
        .await
        .map_err(MonitoringError::internal)?;
    let document_id = Uuid::new_v4().to_string();
    let request = MonitoringExportRequest {
        tenant_id,
        election_event_id,
        dashboard_id: input.dashboard_id,
        widget_id: input.widget_id,
        election_ids: viewer
            .election_ids()
            .iter()
            .map(Uuid::to_string)
            .collect(),
        pinned_post: viewer.pinned.map(|pinned| pinned.to_string()),
        scope,
        selector_values: input.selector_values.unwrap_or_default(),
        widget_selector_values,
        snapshot_revision: input.snapshot_revision,
        config_generation: Some(config.generation),
        format: input.format,
        from,
        to,
        document_id: document_id.clone(),
    };
    let broker = services.tasks.connect().await;
    if let Err(send_error) = broker
        .send_task(
            windmill::tasks::export_monitoring_data::export_monitoring_data::new(
                request,
                task_execution.clone(),
            ),
        )
        .await
    {
        error!(task_id = %task_execution.id, "Failed to enqueue the monitoring export: {send_error}");
        services
            .ledger
            .update_fail(&task_execution, "Failed to enqueue the export")
            .await
            .ok();
        return Err(MonitoringError::internal("Failed to enqueue the export"));
    }
    Ok(Json(ExportOutput {
        document_id,
        task_execution,
    }))
}

#[cfg(test)]
#[path = "../../tests/support/monitoring_export_routes.rs"]
mod monitoring_export_routes;
