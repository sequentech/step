// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

use warp::Future;
use warp::{http::Response, Filter};

pub struct ProbeHandler {
    address: SocketAddr,
    live_path: String,
    ready_path: String,
    is_live: Arc<
        Mutex<
            Box<
                dyn Fn() -> std::pin::Pin<
                        Box<dyn std::future::Future<Output = bool> + Send>,
                    > + Send
                    + Sync,
            >,
        >,
    >,
    is_ready: Arc<
        Mutex<
            Box<
                dyn Fn() -> std::pin::Pin<
                        Box<dyn std::future::Future<Output = bool> + Send>,
                    > + Send
                    + Sync,
            >,
        >,
    >,
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

    pub fn future(&self) -> impl Future<Output = ()> {
        let il = Arc::clone(&self.is_live);
        let ir = Arc::clone(&self.is_ready);

        let filter =
            warp::get().and(
                warp::path(self.live_path.to_string())
                    .and_then(move || {
                        // "Any code greater than or equal to 200 and less than
                        // 400 indicates success. Any other code indicates failure". https://kubernetes.io/docs/tasks/configure-pod-container/configure-liveness-readiness-startup-probes/
                        let il = Arc::clone(&il);
                        async move {
                            let is_live = il.lock().await;
                            let is_live_future = is_live();
                            if is_live_future.await {
                                Ok::<_, warp::Rejection>(
                                    Response::builder()
                                        .status(warp::http::StatusCode::OK)
                                        .body("Live")
                                        .unwrap(),
                                )
                            } else {
                                Ok::<_, warp::Rejection>(
                                    Response::builder()
                                        .status(
                                            warp::http::StatusCode::BAD_REQUEST,
                                        )
                                        .body("Not live")
                                        .unwrap(),
                                )
                            }
                        }
                    })
                    .or(warp::path(self.ready_path.to_string()).and_then(
                        move || {
                            let ir = Arc::clone(&ir);
                            async move {
                                let is_ready = ir.lock().await;
                                let is_ready_future = is_ready();
                                if is_ready_future.await {
                                    Ok::<_, warp::Rejection>(
                                        Response::builder()
                                            .status(warp::http::StatusCode::OK)
                                            .body("Ready")
                                            .unwrap(),
                                    )
                                } else {
                                    Ok::<_, warp::Rejection>(Response::builder()
                                .status(warp::http::StatusCode::BAD_REQUEST)
                                .body("Not ready")
                                .unwrap())
                                }
                            }
                        },
                    )),
            );

        warp::serve(filter).bind(self.address)
    }

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
