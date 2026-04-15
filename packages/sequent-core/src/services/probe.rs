// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

use warp::Future;
use warp::{http::Response, Filter};

/// Type alias for the probe future function used in liveness/readiness checks.
type ProbeFuture = dyn Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send>>
    + Send
    + Sync;
/// A handler for Kubernetes liveness and readiness probes.
pub struct ProbeHandler {
    /// The address to bind the probe server to.
    address: SocketAddr,
    /// The path for the liveness probe.
    live_path: String,
    /// The path for the readiness probe.
    ready_path: String,
    /// The liveness probe function.
    is_live: Arc<Mutex<Box<ProbeFuture>>>,
    /// The readiness probe function.
    is_ready: Arc<Mutex<Box<ProbeFuture>>>,
}

impl ProbeHandler {
    pub fn new(
        live_path: &str,
        ready_path: &str,
        address: impl Into<SocketAddr>,
    ) -> ProbeHandler {
        ProbeHandler {
            address: address.into(),
            live_path: live_path.to_string(),
            ready_path: ready_path.to_string(),
            is_live: Arc::new(Mutex::new(Box::new(|| {
                Box::pin(async { false })
            }))),
            is_ready: Arc::new(Mutex::new(Box::new(|| {
                Box::pin(async { false })
            }))),
        }
    }

    /// Returns a future that runs the probe server.
    ///
    /// # Panics
    /// Panics if the response cannot be built.
    pub fn future(&self) -> impl Future<Output = ()> {
        let il = Arc::clone(&self.is_live);
        let ir = Arc::clone(&self.is_ready);

        let filter = warp::get().and(
            warp::path(self.live_path.clone())
                .and_then(move || {
                    // "Any code greater than or equal to 200 and less than
                    // 400 indicates success. Any other code indicates failure". https://kubernetes.io/docs/tasks/configure-pod-container/configure-liveness-readiness-startup-probes/
                    let il = Arc::clone(&il);
                    async move {
                        let is_live = il.lock().await;
                        let is_live_future = is_live();
                        if is_live_future.await {
                            let response = Response::builder()
                                .status(warp::http::StatusCode::OK)
                                .body("Live")
                                .unwrap_or_else(|_| Response::new("Live"));
                            Ok::<_, warp::Rejection>(response)
                        } else {
                            let response = Response::builder()
                                .status(warp::http::StatusCode::BAD_REQUEST)
                                .body("Not live")
                                .unwrap_or_else(|_| Response::new("Not live"));
                            Ok::<_, warp::Rejection>(response)
                        }
                    }
                })
                .or(warp::path(self.ready_path.clone()).and_then(move || {
                    let ir = Arc::clone(&ir);
                    async move {
                        let is_ready = ir.lock().await;
                        let is_ready_future = is_ready();
                        if is_ready_future.await {
                            let response = Response::builder()
                                .status(warp::http::StatusCode::OK)
                                .body("Ready")
                                .unwrap_or_else(|_| Response::new("Ready"));
                            Ok::<_, warp::Rejection>(response)
                        } else {
                            let response = Response::builder()
                                .status(warp::http::StatusCode::BAD_REQUEST)
                                .body("Not ready")
                                .unwrap_or_else(|_| Response::new("Not ready"));
                            Ok::<_, warp::Rejection>(response)
                        }
                    }
                })),
        );

        warp::serve(filter).bind(self.address)
    }

    /// Sets the liveness probe function.
    ///
    /// # Errors
    /// This function does not return errors, but the probe function may fail internally.
    pub async fn set_live(
        &self,
        f: impl Fn() -> std::pin::Pin<
                Box<dyn std::future::Future<Output = bool> + Send>,
            > + Send
            + Sync
            + 'static,
    ) {
        let mut l = self.is_live.lock().await;
        *l = Box::new(f);
    }

    /// Sets the readiness probe function.
    ///
    /// # Errors
    /// This function does not return errors, but the probe function may fail internally.
    pub async fn set_ready(
        &self,
        f: impl Fn() -> std::pin::Pin<
                Box<dyn std::future::Future<Output = bool> + Send>,
            > + Send
            + Sync
            + 'static,
    ) {
        let mut r = self.is_ready.lock().await;
        *r = Box::new(f);
    }
}

#[cfg(test)]
mod tests {

    use crate::services::probe::ProbeHandler;
    use tokio::time::{sleep, Duration};

    // cargo test probe --features=probe -- --include-ignored
    #[ignore]
    #[tokio::test]
    async fn test_probe() {
        let ph = ProbeHandler::new("live", "ready", ([127, 0, 0, 1], 3030));

        let f = ph.future();
        let handle = tokio::spawn(f);

        sleep(Duration::from_secs(20)).await;
        // curl localhost:3030/live
        ph.set_live(move || Box::pin(async { true })).await;
        sleep(Duration::from_secs(20)).await;
        // curl localhost:3030/ready
        ph.set_ready(move || Box::pin(async { true })).await;
        handle.await.unwrap()
    }
}
