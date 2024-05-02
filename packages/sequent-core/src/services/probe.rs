// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;

use warp::Future;
use warp::{http::Response, Filter};

pub struct ProbeHandler {
    address: SocketAddr,
    live_path: String,
    ready_path: String,
    is_live: Arc<Mutex<Box<dyn Fn() -> bool + Send + Sync>>>,
    is_ready: Arc<Mutex<Box<dyn Fn() -> bool + Send + Sync>>>,
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
            is_live: Arc::new(Mutex::new(Box::new(|| false))),
            is_ready: Arc::new(Mutex::new(Box::new(|| false))),
        }
    }

    pub fn future(&self) -> impl Future<Output = ()> {
        let il = Arc::clone(&self.is_live);
        let ir = Arc::clone(&self.is_ready);

        let filter = warp::get().and(
            warp::path(self.live_path.to_string())
                .map(move || {
                    let is_live = il.lock().unwrap();
                    // "Any code greater than or equal to 200 and less than 400
                    // indicates success. Any other code indicates failure". https://kubernetes.io/docs/tasks/configure-pod-container/configure-liveness-readiness-startup-probes/
                    if is_live() {
                        Response::builder()
                            .status(warp::http::StatusCode::OK)
                            .body("Live")
                    } else {
                        Response::builder()
                            .status(warp::http::StatusCode::BAD_REQUEST)
                            .body("Not live")
                    }
                })
                .or(warp::path(self.ready_path.to_string()).map(move || {
                    let is_ready = ir.lock().unwrap();
                    // "Any code greater than or equal to 200 and less than 400
                    // indicates success. Any other code indicates failure". https://kubernetes.io/docs/tasks/configure-pod-container/configure-liveness-readiness-startup-probes/
                    if is_ready() {
                        Response::builder()
                            .status(warp::http::StatusCode::OK)
                            .body("Ready")
                    } else {
                        Response::builder()
                            .status(warp::http::StatusCode::BAD_REQUEST)
                            .body("Not ready")
                    }
                })),
        );

        warp::serve(filter).bind(self.address)
    }

    pub fn set_live(&mut self, f: impl Fn() -> bool + Send + Sync + 'static) {
        let mut l = self.is_live.lock().unwrap();
        *l = Box::new(f);
    }

    pub fn set_ready(&mut self, f: impl Fn() -> bool + Send + Sync + 'static) {
        let mut r = self.is_ready.lock().unwrap();
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
        let mut ph = ProbeHandler::new("live", "ready", ([127, 0, 0, 1], 3030));

        let f = ph.future();
        let handle = tokio::spawn(f);

        let t = true;
        sleep(Duration::from_secs(20)).await;
        // curl localhost:3030/live
        ph.set_live(move || t);
        sleep(Duration::from_secs(20)).await;
        // curl localhost:3030/ready
        ph.set_ready(move || t);
        handle.await.unwrap()
    }
}
