// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The checks only the chart engine can make of a configuration change:
//! that each chart the change would draw passes dbt Charts' schema, and
//! draws. Each affected widget is drawn with the made-up figures of
//! `sequent_core::monitoring::sample`, so a save does not wait for, or
//! depend on, a snapshot.

use crate::ports::monitoring_renderer::MonitoringRenderer;
use indexmap::IndexMap;
use sequent_core::monitoring::compute::{evaluate, event_days, QueryResult};
use sequent_core::monitoring::config::{
    ConfigKind, ConfigSet, Widget, DEFAULT_QUERY_NAME, DEFAULT_THEME,
};
use sequent_core::monitoring::problem::Report;
use sequent_core::monitoring::render_request::build_board;
use sequent_core::monitoring::resolve::{resolve_widget, DynamicOptionValues};
use sequent_core::monitoring::revision::Edit;
use sequent_core::monitoring::sample::sample_payload;
use serde_json::Value;
use std::sync::Arc;
use tracing::warn;
use windmill::services::monitoring::config_store::{
    MonitoringConfigChecks, ProposedChange,
};

/// How many charts one change is checked with at most: a theme on every
/// dashboard reaches every widget, and each draw takes the engine a moment.
const MAX_BOARDS: usize = 24;

/// One chart a change would draw.
#[derive(Debug, Clone)]
pub struct SampleBoard {
    pub widget_id: String,
    pub board: Value,
    /// The first rows of its default query.
    pub data: Option<QueryResult>,
    pub height: Option<u32>,
}

/// `widget` drawn on the sample figures, with the dashboard's values for it
/// and the theme given; `None` when it cannot be evaluated.
pub fn sample_board(
    set: &ConfigSet,
    widget: &Widget,
    theme_id: &str,
    values: &IndexMap<String, String>,
) -> Option<SampleBoard> {
    let payload = sample_payload(widget.source, set.settings.as_ref());
    let dynamic = DynamicOptionValues {
        event_days: event_days(&payload),
    };
    let resolved = resolve_widget(widget, values, &IndexMap::new(), &dynamic)
        .map_err(|report| {
            warn!(widget = %widget.id, "sample resolve failed: {report:?}")
        })
        .ok()?;
    let mut data: IndexMap<String, QueryResult> = IndexMap::new();
    for (name, query) in &resolved.queries {
        let result = evaluate(
            widget.source,
            query,
            &payload,
            set.settings.as_ref(),
        )
        .map_err(|report| {
            warn!(widget = %widget.id, "sample evaluation failed: {report:?}")
        })
        .ok()?;
        data.insert(name.clone(), result);
    }
    let theme = set.themes.get(theme_id);
    Some(SampleBoard {
        widget_id: widget.id.clone(),
        board: build_board(widget, theme, &data),
        data: data
            .get(DEFAULT_QUERY_NAME)
            .or_else(|| data.values().next())
            .cloned(),
        height: widget.height,
    })
}

/// The charts saving `kind`/`key` would change, drawn on the sample figures,
/// in `set` as it would be once saved.
pub fn boards_for(
    set: &ConfigSet,
    kind: ConfigKind,
    key: &str,
) -> Vec<SampleBoard> {
    let mut placed: Vec<(String, String, IndexMap<String, String>)> = vec![];
    match kind {
        ConfigKind::Widget => {
            // With the first dashboard that shows it, or on its own.
            let shown = set.dashboards.values().find_map(|dashboard| {
                dashboard.layout.iter().find(|item| item.widget == key).map(
                    |item| {
                        (
                            dashboard
                                .theme
                                .clone()
                                .unwrap_or_else(|| DEFAULT_THEME.to_string()),
                            item.values.clone(),
                        )
                    },
                )
            });
            let (theme, values) = shown.unwrap_or_else(|| {
                (DEFAULT_THEME.to_string(), IndexMap::new())
            });
            placed.push((key.to_string(), theme, values));
        }
        ConfigKind::Dashboard => {
            if let Some(dashboard) = set.dashboards.get(key) {
                let theme = dashboard
                    .theme
                    .clone()
                    .unwrap_or_else(|| DEFAULT_THEME.to_string());
                for item in &dashboard.layout {
                    placed.push((
                        item.widget.clone(),
                        theme.clone(),
                        item.values.clone(),
                    ));
                }
            }
        }
        ConfigKind::Theme => {
            for dashboard in set.dashboards.values() {
                let theme = dashboard
                    .theme
                    .clone()
                    .unwrap_or_else(|| DEFAULT_THEME.to_string());
                if theme != key {
                    continue;
                }
                for item in &dashboard.layout {
                    placed.push((
                        item.widget.clone(),
                        theme.clone(),
                        item.values.clone(),
                    ));
                }
            }
        }
        ConfigKind::Settings => {}
    }
    let mut seen: Vec<String> = vec![];
    let mut boards = vec![];
    for (widget_id, theme, values) in placed {
        if boards.len() >= MAX_BOARDS {
            break;
        }
        let Some(widget) = set.widgets.get(&widget_id) else {
            continue;
        };
        let Some(board) = sample_board(set, widget, &theme, &values) else {
            continue;
        };
        let text = board.board.to_string();
        if seen.contains(&text) {
            continue;
        }
        seen.push(text);
        boards.push(board);
    }
    boards
}

/// Asks the renderer about each chart of [`boards_for`]: errors and
/// warnings alike, the widget named where it is not the one edited.
pub async fn check_boards(
    renderer: &dyn MonitoringRenderer,
    set: &ConfigSet,
    kind: ConfigKind,
    key: &str,
) -> anyhow::Result<Report> {
    let mut report = Report::default();
    for board in boards_for(set, kind, key) {
        let found = renderer
            .validate(&board.board)
            .await
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        for mut problem in found {
            if kind == ConfigKind::Widget {
                problem.path = if problem.path.is_empty() {
                    "chart".to_string()
                } else {
                    format!("chart.{}", problem.path)
                };
            } else {
                problem.message =
                    format!("{}: {}", board.widget_id, problem.message);
            }
            report.push(problem);
        }
    }
    Ok(report)
}

/// [`MonitoringConfigChecks`] by the renderer service.
pub struct RendererChecks {
    pub renderer: Arc<dyn MonitoringRenderer>,
}

#[rocket::async_trait]
impl MonitoringConfigChecks for RendererChecks {
    async fn check(
        &self,
        change: &ProposedChange<'_>,
    ) -> anyhow::Result<Report> {
        if matches!(change.edit, Edit::Delete) {
            return Ok(Report::default());
        }
        check_boards(
            self.renderer.as_ref(),
            change.set,
            change.kind,
            change.key,
        )
        .await
    }
}

#[cfg(test)]
#[path = "../../tests/support/monitoring_live_renderer.rs"]
mod monitoring_live_renderer;
