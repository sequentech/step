// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The renderer service's HTTP answers, as the adapter reads them, from a
//! scripted local peer.

use super::HttpMonitoringRenderer;
use crate::ports::monitoring_renderer::{
    ColorScheme, MonitoringRenderer, RenderBoard, RendererError,
};
use crate::route_services::http::{Exchange, HttpServer};
use serde_json::json;
use std::time::Duration;

fn renderer(peer: &HttpServer) -> HttpMonitoringRenderer {
    HttpMonitoringRenderer::new(
        Some(format!("{}/", peer.url)),
        "secret-token".into(),
        Duration::from_secs(2),
    )
}

fn board() -> RenderBoard {
    RenderBoard {
        board: json!({"charts": {}}),
        width: 640,
        height: Some(260),
        color_scheme: ColorScheme::Dark,
        locale: "en".into(),
    }
}

fn error_problem() -> serde_json::Value {
    json!({
        "severity": "error",
        "code": "chart_schema",
        "path": "charts.a",
        "message": "No such type.",
        "engine_code": "ERR-CHART-TYPE",
    })
}

#[tokio::test]
async fn a_drawn_board_is_read_and_the_request_carries_the_token() {
    let peer = HttpServer::start(vec![Exchange::json(
        "POST",
        "/render",
        200,
        json!({
            "svg": "<svg/>",
            "render_ms": 12,
            "dbt_charts_version": "0.8.0",
            "cache": "miss",
            "warnings": [{
                "severity": "warning",
                "code": "chart_schema",
                "path": "",
                "message": "A pinned colour is unseen.",
                "engine_code": null,
            }],
        }),
    )]);
    let drawn = renderer(&peer).render(board()).await.unwrap();
    assert_eq!(drawn.svg, "<svg/>");
    assert_eq!(drawn.render_ms, 12);
    assert_eq!(drawn.warnings.len(), 1);
    let requests = peer.finish();
    assert_eq!(requests.len(), 1);
    let token = requests[0]
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("x-renderer-token"))
        .map(|(_, value)| value.as_str());
    assert_eq!(token, Some("secret-token"));
    assert_eq!(
        requests[0].json(),
        json!({
            "board": {"charts": {}},
            "width": 640,
            "height": 260,
            "color_scheme": "dark",
            "locale": "en",
        })
    );
}

#[tokio::test]
async fn refusals_timeouts_and_failures_are_told_apart() {
    let peer = HttpServer::start(vec![
        Exchange::json(
            "POST",
            "/render",
            422,
            json!({"problems": [error_problem()]}),
        ),
        Exchange::json(
            "POST",
            "/render",
            504,
            json!({"error": "RENDER_TIMEOUT"}),
        ),
        Exchange::json("POST", "/render", 500, json!({"error": "INTERNAL"})),
        Exchange::json(
            "POST",
            "/validate",
            422,
            json!({"problems": [error_problem()]}),
        ),
        Exchange::json("POST", "/validate", 200, json!({"problems": []})),
    ]);
    let renderer = renderer(&peer);
    match renderer.render(board()).await {
        Err(RendererError::Refused(problems)) => {
            assert_eq!(
                problems[0].engine_code.as_deref(),
                Some("ERR-CHART-TYPE")
            )
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        renderer.render(board()).await,
        Err(RendererError::Timeout)
    ));
    assert!(matches!(
        renderer.render(board()).await,
        Err(RendererError::Unavailable(_))
    ));
    assert_eq!(renderer.validate(&json!({})).await.unwrap().len(), 1);
    assert!(renderer.validate(&json!({})).await.unwrap().is_empty());
    peer.finish();
}

#[tokio::test]
async fn without_a_url_the_renderer_is_unavailable() {
    let renderer = HttpMonitoringRenderer::new(
        None,
        String::new(),
        Duration::from_secs(1),
    );
    assert!(matches!(
        renderer.render(board()).await,
        Err(RendererError::Unavailable(_))
    ));
}
