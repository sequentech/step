// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio_postgres::config::Config;
use tracing::{error, info, instrument, warn};
use warp::Future;
use warp::{http::Response, Filter};

use crate::client::grpc::B3Client;
use crate::client::pgsql::PgsqlDbConnectionParams;
use sequent_core::services::setup_probe::check_postgres_select_one_no_tls;

const DEFAULT_PROBE_ADDR: &str = "0.0.0.0:3030";
const DEFAULT_PROBE_LIVE_PATH: &str = "live";
const DEFAULT_PROBE_READY_PATH: &str = "ready";
const DEFAULT_GRPC_CHECK_TIMEOUT_SECS: u64 = 5;

pub struct ProbeHandler {
    address: SocketAddr,
    live_path: String,
    ready_path: String,
    is_live: Arc<
        Mutex<
            Box<
                dyn Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send>>
                    + Send
                    + Sync,
            >,
        >,
    >,
    is_ready: Arc<
        Mutex<
            Box<
                dyn Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send>>
                    + Send
                    + Sync,
            >,
        >,
    >,
}

impl ProbeHandler {
    pub fn new(live_path: &str, ready_path: &str, address: impl Into<SocketAddr>) -> ProbeHandler {
        ProbeHandler {
            address: address.into(),
            live_path: live_path.to_string(),
            ready_path: ready_path.to_string(),
            is_live: Arc::new(Mutex::new(Box::new(|| Box::pin(async { false })))),
            is_ready: Arc::new(Mutex::new(Box::new(|| Box::pin(async { false })))),
        }
    }

    pub fn future(&self) -> impl Future<Output = ()> {
        let il = Arc::clone(&self.is_live);
        let ir = Arc::clone(&self.is_ready);

        let filter = warp::get().and(
            warp::path(self.live_path.to_string())
                .and_then(move || {
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
                                    .status(warp::http::StatusCode::BAD_REQUEST)
                                    .body("Not live")
                                    .unwrap(),
                            )
                        }
                    }
                })
                .or(warp::path(self.ready_path.to_string()).and_then(move || {
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
                            Ok::<_, warp::Rejection>(
                                Response::builder()
                                    .status(warp::http::StatusCode::BAD_REQUEST)
                                    .body("Not ready")
                                    .unwrap(),
                            )
                        }
                    }
                })),
        );

        warp::serve(filter).bind(self.address)
    }

    pub async fn set_live(
        &self,
        f: impl Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send>>
            + Send
            + Sync
            + 'static,
    ) {
        let mut l = self.is_live.lock().await;
        *l = Box::new(f);
    }

    pub async fn set_ready(
        &self,
        f: impl Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send>>
            + Send
            + Sync
            + 'static,
    ) {
        let mut r = self.is_ready.lock().await;
        *r = Box::new(f);
    }
}

/// Builds an `http://…` URL suitable for dialing this server's gRPC from the same host.
/// When the listen address is all-interfaces (`0.0.0.0` / `[::]`), uses loopback for the client.
pub fn grpc_health_url_for_bind(bind: &str) -> String {
    let trimmed = bind.trim();
    let hostport = if let Some(port) = trimmed.strip_prefix("0.0.0.0:") {
        format!("127.0.0.1:{port}")
    } else if trimmed == "0.0.0.0" {
        "127.0.0.1".to_string()
    } else if let Some(port) = trimmed.strip_prefix("[::]:") {
        format!("127.0.0.1:{port}")
    } else {
        trimmed.to_string()
    };
    if hostport.starts_with("http://") || hostport.starts_with("https://") {
        hostport
    } else {
        format!("http://{hostport}")
    }
}

#[instrument(ret, skip(pg))]
async fn check_postgres(pg: &PgsqlDbConnectionParams) -> bool {
    let config = match Config::from_str(&pg.connection_string()) {
        Ok(c) => c,
        Err(e) => {
            error!("probe postgres: invalid config: {e:?}");
            return false;
        }
    };
    check_postgres_select_one_no_tls(&config).await
}

#[instrument(ret)]
async fn check_grpc(grpc_url: &str, max_message_size: usize, timeout_secs: u64) -> bool {
    let c = B3Client::new(grpc_url, max_message_size, timeout_secs);
    match c.get_boards().await {
        Ok(_) => true,
        Err(e) => {
            error!("probe grpc: get_boards failed: {e:?}");
            false
        }
    }
}

#[instrument(ret, skip(pg))]
async fn readiness(
    pg: PgsqlDbConnectionParams,
    grpc_url: String,
    max_message_size: usize,
    grpc_timeout_secs: u64,
) -> bool {
    let (pg_ok, grpc_ok) = tokio::join!(
        check_postgres(&pg),
        check_grpc(&grpc_url, max_message_size, grpc_timeout_secs),
    );
    info!("probe readiness: postgres={pg_ok}, grpc={grpc_ok}");
    pg_ok && grpc_ok
}

/// Spawns HTTP `/live` and `/ready` endpoints (Kubernetes-style).
///
/// Environment (optional): `B3_PROBE_ADDR`, `B3_PROBE_LIVE_PATH`, `B3_PROBE_READY_PATH`,
/// `B3_PROBE_GRPC_TIMEOUT_SECS`.
pub async fn setup_probe(pg: PgsqlDbConnectionParams, grpc_url: String, max_message_size: usize) {
    let addr_s = std::env::var("B3_PROBE_ADDR").unwrap_or_else(|_| DEFAULT_PROBE_ADDR.to_string());
    let live_path =
        std::env::var("B3_PROBE_LIVE_PATH").unwrap_or_else(|_| DEFAULT_PROBE_LIVE_PATH.to_string());
    let ready_path = std::env::var("B3_PROBE_READY_PATH")
        .unwrap_or_else(|_| DEFAULT_PROBE_READY_PATH.to_string());
    let grpc_timeout_secs = std::env::var("B3_PROBE_GRPC_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_GRPC_CHECK_TIMEOUT_SECS);

    let addr: Result<SocketAddr, _> = addr_s.parse();

    if let Ok(addr) = addr {
        let ph = ProbeHandler::new(&live_path, &ready_path, addr);
        let f = ph.future();
        let pg0 = pg.clone();
        let url0 = grpc_url.clone();
        ph.set_live(move || {
            let pg = pg0.clone();
            let url = url0.clone();
            Box::pin(async move { readiness(pg, url, max_message_size, grpc_timeout_secs).await })
        })
        .await;
        info!("probe live set");

        let pg1 = pg.clone();
        let url1 = grpc_url;
        ph.set_ready(move || {
            let pg = pg1.clone();
            let url = url1.clone();
            Box::pin(async move { readiness(pg, url, max_message_size, grpc_timeout_secs).await })
        })
        .await;
        tokio::spawn(f);
        info!("probe ready set");
    } else {
        warn!("Could not parse address for b3 probe '{addr_s}'");
    }
}
