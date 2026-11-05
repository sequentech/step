// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Every shipped preset's widgets through a running renderer service and
//! the SVG sanitizer. Run it with the renderer up:
//!
//! ```text
//! HARVEST_MONITORING_RENDERER_URL=http://monitoring-renderer:8080 \
//! HARVEST_MONITORING_RENDERER_TOKEN=... \
//! cargo test -p harvest -- --ignored every_preset_widget
//! ```

use super::boards_for;
use crate::adapters::monitoring_renderer::HttpMonitoringRenderer;
use crate::ports::monitoring_renderer::{
    ColorScheme, MonitoringRenderer, RenderBoard,
};
use crate::services::monitoring_svg::sanitize_svg;
use sequent_core::monitoring::config::ConfigKind;
use sequent_core::monitoring::presets::PRESETS;
use sequent_core::monitoring::problem::Severity;

#[tokio::test]
#[ignore = "needs a running renderer service"]
async fn every_preset_widget_passes_the_real_engine_and_the_sanitizer() {
    let renderer = HttpMonitoringRenderer::from_env();
    let mut failures = vec![];
    let mut drawn_count = 0;
    for source in PRESETS {
        let preset = source.load().expect("a shipped preset loads");
        for key in preset.set.widgets.keys() {
            let boards = boards_for(&preset.set, ConfigKind::Widget, key);
            if boards.is_empty() {
                failures.push(format!("{}/{key}: no sample board", source.id));
            }
            for board in boards {
                let place = format!("{}/{key}", source.id);
                match renderer.validate(&board.board).await {
                    Ok(problems) => {
                        for problem in problems.iter().filter(|problem| {
                            problem.severity == Severity::Error
                        }) {
                            failures.push(format!("{place}: {problem:?}"));
                        }
                    }
                    Err(error) => failures.push(format!("{place}: {error}")),
                }
                for color_scheme in [ColorScheme::Light, ColorScheme::Dark] {
                    let drawn = renderer
                        .render(RenderBoard {
                            board: board.board.clone(),
                            width: 640,
                            height: board.height,
                            color_scheme,
                            locale: "en".into(),
                        })
                        .await;
                    match drawn {
                        Ok(drawn) => match sanitize_svg(&drawn.svg) {
                            Ok(clean) => {
                                drawn_count += 1;
                                eprintln!(
                                    "{place} {color_scheme:?}: {} -> {} bytes, {} ms",
                                    drawn.svg.len(),
                                    clean.len(),
                                    drawn.render_ms
                                );
                            }
                            Err(error) => {
                                if let Ok(dump) =
                                    std::env::var("MONITORING_SVG_DUMP")
                                {
                                    std::fs::write(dump, &drawn.svg).ok();
                                }
                                failures.push(format!(
                                    "{place}: unsafe SVG {error:?}"
                                ))
                            }
                        },
                        Err(error) => {
                            failures.push(format!("{place}: {error}"))
                        }
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(drawn_count > 0);
}
