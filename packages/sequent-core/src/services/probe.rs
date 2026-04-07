// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Kubernetes probe server and health-check infrastructure.
//!
//! # Architecture
//!
//! This module follows a port-and-adapter structure:
//!
//! - [`HealthCheck`] is the **port**: an async predicate callers implement to
//!   inject domain logic without depending on HTTP primitives. Use [`FnCheck`]
//!   to adapt any async closure, or [`All`]/[`Any`] to compose multiple checks.
//!
//! - [`ProbeState`] is the **domain object**: holds the active live/ready checks
//!   behind an `Arc` so any part of the application can update them at runtime.
//!
//! - [`ProbeServer`] is the **HTTP adapter**: translates `GET` requests into
//!   `ProbeState` queries and returns `200 OK` or `503 Service Unavailable`.
//!   Prometheus metrics are served at [`METRICS_PATH`]. All unmatched routes
//!   return `404 Not Found`.
//!
//! # Graceful shutdown
//!
//! Pass any `Future<Output = ()>` to [`ProbeServer::serve`]. Use
//! [`std::future::pending()`] for a server that runs until the process exits,
//! or a [`tokio::sync::oneshot`] receiver for clean pod restarts.
//!
//! # Example
//!
//! ```rust,no_run
//! # use std::sync::Arc;
//! # use sequent_core::services::probe::{FnCheck, ProbeServer, ProbeState};
//! # async fn run() {
//! let state = Arc::new(ProbeState::new());
//! state.set_live(FnCheck(|| async { true })).await;
//! state.set_ready(FnCheck(|| async { true })).await;
//!
//! let server = ProbeServer::builder()
//!     .address(([0, 0, 0, 0], 3030))
//!     .live_path("live")
//!     .ready_path("ready")
//!     .state(Arc::clone(&state))
//!     .build();
//!
//! tokio::spawn(server.serve(std::future::pending()));
//! # }
//! ```

use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

use prometheus::{Encoder, TextEncoder};
use tokio::sync::RwLock;
use tracing::error;
use warp::{http::Response, Filter};

// ── HealthCheck port ──────────────────────────────────────────────────────────

/// Domain port for a single health check.
///
/// Implement this trait to provide liveness or readiness logic. Use [`FnCheck`]
/// to adapt an async closure without defining a struct, or [`All`]/[`Any`] to
/// compose multiple checks.
pub trait HealthCheck: Send + Sync + 'static {
    fn is_healthy(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>>;
}

/// Default check used until [`ProbeState::set_live`] or [`ProbeState::set_ready`]
/// is called. Always reports unhealthy so the pod is not marked live before
/// the application has finished initialising.
struct AlwaysUnhealthy;

impl HealthCheck for AlwaysUnhealthy {
    fn is_healthy(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>> {
        Box::pin(async { false })
    }
}

/// Adapts an async closure to the [`HealthCheck`] port.
///
/// # Example
///
/// ```rust
/// # use sequent_core::services::probe::FnCheck;
/// let check = FnCheck(|| async { true });
/// ```
pub struct FnCheck<F>(pub F);

impl<F, Fut> HealthCheck for FnCheck<F>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = bool> + Send + 'static,
{
    fn is_healthy(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>> {
        Box::pin((self.0)())
    }
}

/// Healthy only if **all** inner checks report healthy.
///
/// Evaluation short-circuits on the first failure.
pub struct All(pub Vec<Arc<dyn HealthCheck>>);

impl HealthCheck for All {
    fn is_healthy(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>> {
        let checks = self.0.clone();
        Box::pin(async move {
            for check in &checks {
                if !check.is_healthy().await {
                    return false;
                }
            }
            true
        })
    }
}

/// Healthy if **any** inner check reports healthy.
///
/// Evaluation short-circuits on the first success.
pub struct Any(pub Vec<Arc<dyn HealthCheck>>);

impl HealthCheck for Any {
    fn is_healthy(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>> {
        let checks = self.0.clone();
        Box::pin(async move {
            for check in &checks {
                if check.is_healthy().await {
                    return true;
                }
            }
            false
        })
    }
}

// ── ProbeState (domain) ───────────────────────────────────────────────────────

type SharedCheck = Arc<RwLock<Arc<dyn HealthCheck>>>;

fn default_check() -> SharedCheck {
    Arc::new(RwLock::new(Arc::new(AlwaysUnhealthy)))
}

/// Holds the active liveness and readiness checks.
///
/// Share via `Arc<ProbeState>` across the application to update checks after
/// the server has started. Both checks default to [`AlwaysUnhealthy`] until
/// explicitly set, ensuring the pod is not marked live before initialisation
/// completes.
pub struct ProbeState {
    is_live: SharedCheck,
    is_ready: SharedCheck,
}

impl Default for ProbeState {
    fn default() -> Self {
        Self::new()
    }
}

impl ProbeState {
    /// Creates a new [`ProbeState`] with both checks defaulting to unhealthy.
    pub fn new() -> Self {
        Self {
            is_live: default_check(),
            is_ready: default_check(),
        }
    }

    /// Replaces the liveness check.
    pub async fn set_live(&self, check: impl HealthCheck) {
        *self.is_live.write().await = Arc::new(check);
    }

    /// Replaces the readiness check.
    pub async fn set_ready(&self, check: impl HealthCheck) {
        *self.is_ready.write().await = Arc::new(check);
    }

    async fn check_live(&self) -> bool {
        self.is_live.read().await.is_healthy().await
    }

    async fn check_ready(&self) -> bool {
        self.is_ready.read().await.is_healthy().await
    }
}

// ── HTTP adapter ──────────────────────────────────────────────────────────────

/// URL path at which Prometheus metrics are served.
pub const METRICS_PATH: &str = "metrics";

fn probe_response(healthy: bool) -> Response<String> {
    let (status, body) = if healthy {
        (warp::http::StatusCode::OK, "OK")
    } else {
        (
            warp::http::StatusCode::SERVICE_UNAVAILABLE,
            "Service Unavailable",
        )
    };
    Response::builder()
        .status(status)
        .body(body.to_string())
        .expect("static probe response headers are always valid")
}

async fn handle_metrics() -> Result<Response<String>, warp::Rejection> {
    let encoder = TextEncoder::new();
    let mut buf = Vec::new();

    if let Err(err) = encoder.encode(&prometheus::gather(), &mut buf) {
        error!("Failed to encode prometheus metrics: {err}");
        return Err(warp::reject::reject());
    }

    let body = String::from_utf8(buf).map_err(|err| {
        error!("Failed to convert prometheus metrics to UTF-8: {err}");
        warp::reject::reject()
    })?;

    Response::builder()
        .status(warp::http::StatusCode::OK)
        .header("Content-Type", encoder.format_type())
        .body(body)
        .map_err(|err| {
            error!("Failed to build metrics response: {err}");
            warp::reject::reject()
        })
}

/// Warp-based HTTP adapter for Kubernetes probe and metrics endpoints.
///
/// Build via [`ProbeServer::builder()`]. Keep a separate `Arc<ProbeState>` to
/// update live/ready checks after the server has started.
///
/// Endpoints:
/// - `GET /<live_path>` — liveness probe (`200` / `503`)
/// - `GET /<ready_path>` — readiness probe (`200` / `503`)
/// - `GET /metrics` — Prometheus metrics
/// - Anything else — `404 Not Found`
pub struct ProbeServer {
    state: Arc<ProbeState>,
    address: SocketAddr,
    live_path: String,
    ready_path: String,
}

impl ProbeServer {
    /// Returns a new [`ProbeServerBuilder`] with default settings.
    pub fn builder() -> ProbeServerBuilder {
        ProbeServerBuilder::default()
    }

    /// Returns the composed warp filter.
    ///
    /// Exposed for in-process testing via [`warp::test`]. The returned filter
    /// is fully self-contained: all rejections are converted to `404 Not Found`
    /// so the error type is [`Infallible`].
    pub fn filter(
        &self,
    ) -> impl Filter<Extract = impl warp::Reply, Error = Infallible>
           + Clone
           + Send
           + Sync
           + 'static {
        let live_state = Arc::clone(&self.state);
        let ready_state = Arc::clone(&self.state);

        let live = warp::path(self.live_path.clone())
            .and(warp::path::end())
            .and_then(move || {
                let s = Arc::clone(&live_state);
                async move {
                    Ok::<_, warp::Rejection>(probe_response(
                        s.check_live().await,
                    ))
                }
            });

        let ready = warp::path(self.ready_path.clone())
            .and(warp::path::end())
            .and_then(move || {
                let s = Arc::clone(&ready_state);
                async move {
                    Ok::<_, warp::Rejection>(probe_response(
                        s.check_ready().await,
                    ))
                }
            });

        let metrics = warp::path(METRICS_PATH)
            .and(warp::path::end())
            .and_then(handle_metrics);

        warp::get().and(live.or(ready).or(metrics)).recover(
            |_: warp::Rejection| async {
                Ok::<_, Infallible>(
                    Response::builder()
                        .status(warp::http::StatusCode::NOT_FOUND)
                        .body("Not found".to_string())
                        .expect("static 404 response headers are always valid"),
                )
            },
        )
    }

    /// Starts the HTTP server, running until `shutdown` resolves.
    ///
    /// Use [`std::future::pending()`] for a server that runs until the process
    /// exits. Pass a [`tokio::sync::oneshot`] receiver for clean pod restarts.
    pub async fn serve(
        self,
        shutdown: impl Future<Output = ()> + Send + 'static,
    ) {
        warp::serve(self.filter())
            .bind_with_graceful_shutdown(self.address, shutdown)
            .1
            .await;
    }
}

/// Builder for [`ProbeServer`].
///
/// Obtain via [`ProbeServer::builder()`].
pub struct ProbeServerBuilder {
    state: Option<Arc<ProbeState>>,
    address: SocketAddr,
    live_path: String,
    ready_path: String,
}

impl Default for ProbeServerBuilder {
    fn default() -> Self {
        Self {
            state: None,
            address: ([0, 0, 0, 0], 3030).into(),
            live_path: "live".to_string(),
            ready_path: "ready".to_string(),
        }
    }
}

impl ProbeServerBuilder {
    /// Sets the socket address the server will listen on.
    pub fn address(mut self, addr: impl Into<SocketAddr>) -> Self {
        self.address = addr.into();
        self
    }

    /// Sets the URL path for the liveness probe (default: `"live"`).
    pub fn live_path(mut self, path: impl Into<String>) -> Self {
        self.live_path = path.into();
        self
    }

    /// Sets the URL path for the readiness probe (default: `"ready"`).
    pub fn ready_path(mut self, path: impl Into<String>) -> Self {
        self.ready_path = path.into();
        self
    }

    /// Shares an existing [`ProbeState`] with the server.
    ///
    /// If omitted, a fresh (always-unhealthy) state is created internally.
    pub fn state(mut self, state: Arc<ProbeState>) -> Self {
        self.state = Some(state);
        self
    }

    /// Builds the [`ProbeServer`].
    pub fn build(self) -> ProbeServer {
        ProbeServer {
            state: self.state.unwrap_or_else(|| Arc::new(ProbeState::new())),
            address: self.address,
            live_path: self.live_path,
            ready_path: self.ready_path,
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_server(state: Arc<ProbeState>) -> ProbeServer {
        ProbeServer::builder()
            .live_path("live")
            .ready_path("ready")
            .state(state)
            .build()
    }

    #[tokio::test]
    async fn live_returns_200_when_healthy() {
        let state = Arc::new(ProbeState::new());
        state.set_live(FnCheck(|| async { true })).await;
        let res = warp::test::request()
            .method("GET")
            .path("/live")
            .reply(&make_server(state).filter())
            .await;
        assert_eq!(res.status(), 200);
    }

    #[tokio::test]
    async fn live_returns_503_when_unhealthy() {
        let state = Arc::new(ProbeState::new());
        let res = warp::test::request()
            .method("GET")
            .path("/live")
            .reply(&make_server(state).filter())
            .await;
        assert_eq!(res.status(), 503);
    }

    #[tokio::test]
    async fn ready_returns_200_when_healthy() {
        let state = Arc::new(ProbeState::new());
        state.set_ready(FnCheck(|| async { true })).await;
        let res = warp::test::request()
            .method("GET")
            .path("/ready")
            .reply(&make_server(state).filter())
            .await;
        assert_eq!(res.status(), 200);
    }

    #[tokio::test]
    async fn ready_returns_503_when_unhealthy() {
        let state = Arc::new(ProbeState::new());
        let res = warp::test::request()
            .method("GET")
            .path("/ready")
            .reply(&make_server(state).filter())
            .await;
        assert_eq!(res.status(), 503);
    }

    #[tokio::test]
    async fn unknown_path_returns_404() {
        let state = Arc::new(ProbeState::new());
        let res = warp::test::request()
            .method("GET")
            .path("/unknown")
            .reply(&make_server(state).filter())
            .await;
        assert_eq!(res.status(), 404);
    }

    #[tokio::test]
    async fn all_requires_every_check_to_pass() {
        let state = Arc::new(ProbeState::new());
        state
            .set_live(All(vec![
                Arc::new(FnCheck(|| async { true })),
                Arc::new(FnCheck(|| async { false })),
            ]))
            .await;
        assert!(!state.check_live().await);
    }

    #[tokio::test]
    async fn all_passes_when_every_check_passes() {
        let state = Arc::new(ProbeState::new());
        state
            .set_live(All(vec![
                Arc::new(FnCheck(|| async { true })),
                Arc::new(FnCheck(|| async { true })),
            ]))
            .await;
        assert!(state.check_live().await);
    }

    #[tokio::test]
    async fn any_passes_when_one_check_passes() {
        let state = Arc::new(ProbeState::new());
        state
            .set_live(Any(vec![
                Arc::new(FnCheck(|| async { false })),
                Arc::new(FnCheck(|| async { true })),
            ]))
            .await;
        assert!(state.check_live().await);
    }

    #[tokio::test]
    async fn any_fails_when_all_checks_fail() {
        let state = Arc::new(ProbeState::new());
        state
            .set_live(Any(vec![
                Arc::new(FnCheck(|| async { false })),
                Arc::new(FnCheck(|| async { false })),
            ]))
            .await;
        assert!(!state.check_live().await);
    }
}
