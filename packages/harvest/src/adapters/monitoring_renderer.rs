// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::monitoring_renderer::{
    ColorScheme, MonitoringRenderer, RenderBoard, RenderedBoard, RendererError,
};
use reqwest::StatusCode;
use sequent_core::monitoring::problem::Problem;
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;
use tracing::{instrument, warn};

pub const URL_VARIABLE: &str = "HARVEST_MONITORING_RENDERER_URL";
pub const TOKEN_VARIABLE: &str = "HARVEST_MONITORING_RENDERER_TOKEN";
pub const TIMEOUT_VARIABLE: &str = "HARVEST_MONITORING_RENDERER_TIMEOUT_MS";
const DEFAULT_TIMEOUT_MS: u64 = 5000;
const CONNECT_TIMEOUT: Duration = Duration::from_millis(500);
/// The dbt Charts release the renderer pins; part of every cache key.
pub const ENGINE_VERSION: &str = "dbt-charts-0.8.0";

/// The renderer service over HTTP: one attempt per call, never retried, so
/// a slow engine costs a viewer one timeout and not several.
pub struct HttpMonitoringRenderer {
    base_url: Option<String>,
    token: String,
    client: reqwest::Client,
}

impl HttpMonitoringRenderer {
    pub fn from_env() -> Self {
        let timeout_ms = std::env::var(TIMEOUT_VARIABLE)
            .ok()
            .and_then(|value| value.trim().parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(DEFAULT_TIMEOUT_MS);
        Self::new(
            std::env::var(URL_VARIABLE).ok(),
            std::env::var(TOKEN_VARIABLE).unwrap_or_default(),
            Duration::from_millis(timeout_ms),
        )
    }

    pub fn new(
        base_url: Option<String>,
        token: String,
        timeout: Duration,
    ) -> Self {
        // An internal service: never through a proxy.
        let client = reqwest::Client::builder()
            .no_proxy()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(timeout)
            .build()
            .expect("an HTTP client with timeouts");
        Self {
            base_url: base_url
                .map(|url| url.trim().trim_end_matches('/').to_string())
                .filter(|url| !url.is_empty()),
            token,
            client,
        }
    }

    async fn post(
        &self,
        path: &str,
        body: &Value,
    ) -> Result<Value, RendererError> {
        let Some(base_url) = &self.base_url else {
            return Err(RendererError::Unavailable(format!(
                "{URL_VARIABLE} is not set"
            )));
        };
        let response = self
            .client
            .post(format!("{base_url}{path}"))
            .header("X-Renderer-Token", &self.token)
            .json(body)
            .send()
            .await
            .map_err(transport_error)?;
        let status = response.status();
        let body: Value = response.json().await.map_err(transport_error)?;
        match status {
            StatusCode::OK => Ok(body),
            StatusCode::UNPROCESSABLE_ENTITY => {
                Err(RendererError::Refused(problems_in(&body)))
            }
            StatusCode::GATEWAY_TIMEOUT => Err(RendererError::Timeout),
            other => {
                let code = body
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("UNKNOWN");
                Err(RendererError::Unavailable(format!(
                    "the renderer answered {other} {code}"
                )))
            }
        }
    }
}

fn transport_error(error: reqwest::Error) -> RendererError {
    if error.is_timeout() {
        RendererError::Timeout
    } else {
        warn!("Monitoring renderer request failed: {error}");
        RendererError::Unavailable(error.to_string())
    }
}

fn problems_in(body: &Value) -> Vec<Problem> {
    body.get("problems")
        .cloned()
        .and_then(|problems| serde_json::from_value(problems).ok())
        .unwrap_or_default()
}

#[derive(Deserialize)]
struct RenderAnswer {
    svg: String,
    render_ms: u64,
    #[serde(default)]
    warnings: Vec<Problem>,
}

#[rocket::async_trait]
impl MonitoringRenderer for HttpMonitoringRenderer {
    fn version(&self) -> String {
        ENGINE_VERSION.to_string()
    }

    #[instrument(skip_all, fields(width = board.width))]
    async fn render(
        &self,
        board: RenderBoard,
    ) -> Result<RenderedBoard, RendererError> {
        let body = json!({
            "board": board.board,
            "width": board.width,
            "height": board.height,
            "color_scheme": match board.color_scheme {
                ColorScheme::Light => "light",
                ColorScheme::Dark => "dark",
            },
            "locale": board.locale,
        });
        let answer = self.post("/render", &body).await?;
        let answer: RenderAnswer =
            serde_json::from_value(answer).map_err(|error| {
                RendererError::Unavailable(format!(
                    "unreadable render answer: {error}"
                ))
            })?;
        Ok(RenderedBoard {
            svg: answer.svg,
            render_ms: answer.render_ms,
            warnings: answer.warnings,
        })
    }

    #[instrument(skip_all)]
    async fn validate(
        &self,
        board: &Value,
    ) -> Result<Vec<Problem>, RendererError> {
        match self.post("/validate", &json!({ "board": board })).await {
            Ok(answer) => Ok(problems_in(&answer)),
            // A malformed request is still an answer about the board.
            Err(RendererError::Refused(problems)) if !problems.is_empty() => {
                Ok(problems)
            }
            Err(RendererError::Refused(_)) => Err(RendererError::Unavailable(
                "the renderer refused the request without saying why".into(),
            )),
            Err(other) => Err(other),
        }
    }
}

#[cfg(test)]
#[path = "../../tests/support/monitoring_renderer_http.rs"]
mod monitoring_renderer_http;
