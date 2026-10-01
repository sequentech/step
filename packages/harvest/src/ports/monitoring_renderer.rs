// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use sequent_core::monitoring::problem::Problem;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Which palette a chart is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ColorScheme {
    Light,
    Dark,
}

/// One board to draw.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderBoard {
    /// The dbt Charts board, from `sequent_core::monitoring::render_request`.
    pub board: Value,
    /// The width the SVG is drawn at, in pixels.
    pub width: u32,
    /// The least height the chart takes, in pixels.
    pub height: Option<u32>,
    pub color_scheme: ColorScheme,
    pub locale: String,
}

/// A drawn board.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderedBoard {
    pub svg: String,
    pub render_ms: u64,
    /// What the engine warns of; the board was still drawn.
    pub warnings: Vec<Problem>,
}

/// Why a board was not drawn or checked.
#[derive(Debug, Clone, PartialEq)]
pub enum RendererError {
    /// The renderer refused the board, and said why.
    Refused(Vec<Problem>),
    /// No answer within the time Harvest waits.
    Timeout,
    /// The renderer could not be reached, or failed.
    Unavailable(String),
}

impl std::fmt::Display for RendererError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RendererError::Refused(problems) => {
                write!(
                    formatter,
                    "the renderer refused the board: {problems:?}"
                )
            }
            RendererError::Timeout => {
                write!(formatter, "the renderer did not answer in time")
            }
            RendererError::Unavailable(why) => {
                write!(formatter, "the renderer is unavailable: {why}")
            }
        }
    }
}

impl std::error::Error for RendererError {}

/// The internal service that draws monitoring widgets with dbt Charts.
#[rocket::async_trait]
pub trait MonitoringRenderer: Send + Sync {
    /// Which engine draws, so a cached chart is not served across versions.
    fn version(&self) -> String;

    async fn render(
        &self,
        board: RenderBoard,
    ) -> Result<RenderedBoard, RendererError>;

    /// The engine's schema check of `board`, which also draws it: every
    /// problem it has, errors and warnings.
    async fn validate(
        &self,
        board: &Value,
    ) -> Result<Vec<Problem>, RendererError>;
}
