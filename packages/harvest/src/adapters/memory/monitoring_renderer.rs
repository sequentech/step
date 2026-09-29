// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::monitoring_renderer::{
    MonitoringRenderer, RenderBoard, RenderedBoard, RendererError,
};
use sequent_core::monitoring::problem::Problem;
use serde_json::Value;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

/// A renderer that draws every board as the same small SVG, after an
/// optional delay, and answers checks with scripted problems.
pub struct MemoryRenderer {
    pub svg: Mutex<String>,
    pub delay: Duration,
    pub failure: Mutex<Option<RendererError>>,
    pub problems: Mutex<Vec<Problem>>,
    renders: AtomicUsize,
    validations: AtomicUsize,
    boards: Mutex<Vec<RenderBoard>>,
}

impl Default for MemoryRenderer {
    fn default() -> Self {
        Self {
            svg: Mutex::new(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="640" height="280"><g><rect width="10" height="10"/></g></svg>"#
                    .to_string(),
            ),
            delay: Duration::ZERO,
            failure: Mutex::new(None),
            problems: Mutex::new(vec![]),
            renders: AtomicUsize::new(0),
            validations: AtomicUsize::new(0),
            boards: Mutex::new(vec![]),
        }
    }
}

impl MemoryRenderer {
    pub fn slow(delay: Duration) -> Self {
        Self {
            delay,
            ..Default::default()
        }
    }

    pub fn drawing(svg: &str) -> Self {
        let renderer = Self::default();
        *renderer.svg.lock().unwrap() = svg.to_string();
        renderer
    }

    pub fn failing(failure: RendererError) -> Self {
        let renderer = Self::default();
        *renderer.failure.lock().unwrap() = Some(failure);
        renderer
    }

    pub fn finding(problems: Vec<Problem>) -> Self {
        let renderer = Self::default();
        *renderer.problems.lock().unwrap() = problems;
        renderer
    }

    pub fn renders(&self) -> usize {
        self.renders.load(Ordering::SeqCst)
    }

    pub fn validations(&self) -> usize {
        self.validations.load(Ordering::SeqCst)
    }

    pub fn boards(&self) -> Vec<RenderBoard> {
        self.boards.lock().unwrap().clone()
    }
}

#[rocket::async_trait]
impl MonitoringRenderer for MemoryRenderer {
    fn version(&self) -> String {
        "memory".to_string()
    }

    async fn render(
        &self,
        board: RenderBoard,
    ) -> Result<RenderedBoard, RendererError> {
        self.renders.fetch_add(1, Ordering::SeqCst);
        self.boards.lock().unwrap().push(board);
        if !self.delay.is_zero() {
            tokio::time::sleep(self.delay).await;
        }
        if let Some(failure) = self.failure.lock().unwrap().clone() {
            return Err(failure);
        }
        Ok(RenderedBoard {
            svg: self.svg.lock().unwrap().clone(),
            render_ms: 7,
            warnings: vec![],
        })
    }

    async fn validate(
        &self,
        _board: &Value,
    ) -> Result<Vec<Problem>, RendererError> {
        self.validations.fetch_add(1, Ordering::SeqCst);
        if let Some(failure) = self.failure.lock().unwrap().clone() {
            return Err(failure);
        }
        Ok(self.problems.lock().unwrap().clone())
    }
}
