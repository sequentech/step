// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! HTTP readiness for the bulletin board: PostgreSQL reachability and B3 index reachability.

use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio_postgres::config::Config;
use tracing::{error, info, instrument, warn};
use warp::Future;
use warp::{http::Response, Filter};

use crate::client::pgsql::{PgsqlB3Client, PgsqlDbConnectionParams};
use sequent_core::services::setup_probe::check_postgres_select_one_no_tls;

const PROBE_ADDR: &str = "0.0.0.0:3030";
const PROBE_LIVE_PATH: &str = "live";
const PROBE_READY_PATH: &str = "ready";

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

/// `SELECT 1` over a fresh `NoTls` connection (see `sequent_core::services::setup_probe`).
#[instrument(ret, skip(pg))]
async fn check_postgres_select_one(pg: &PgsqlDbConnectionParams) -> bool {
    let config = match Config::from_str(&pg.connection_string()) {
        Ok(c) => c,
        Err(e) => {
            error!("probe postgres: invalid connection string: {e:?}");
            return false;
        }
    };
    check_postgres_select_one_no_tls(&config).await
}

#[instrument(ret, skip(pg))]
async fn check_boards(pg: &PgsqlDbConnectionParams) -> bool {
    let client = match PgsqlB3Client::new(pg).await {
        Ok(c) => c,
        Err(e) => {
            error!("probe boards: connect failed: {e:?}");
            return false;
        }
    };
    match client.get_boards().await {
        Ok(_) => true,
        Err(e) => {
            error!("probe boards: get_boards failed: {e:?}");
            false
        }
    }
}

#[instrument(ret, skip(pg))]
async fn readiness(pg: PgsqlDbConnectionParams) -> bool {
    let (select_ok, boards_ok) = tokio::join!(check_postgres_select_one(&pg), check_boards(&pg),);
    info!("probe readiness: postgres_select_one={select_ok}, boards={boards_ok}");
    select_ok && boards_ok
}

/// Listen for readiness probes on `B3_PROBE_ADDR` and set live/ready status based on PostgreSQL
/// and B3 index reachability.
pub async fn setup_probe(pg: PgsqlDbConnectionParams) {
    let addr_s = std::env::var("B3_PROBE_ADDR").unwrap_or_else(|_| PROBE_ADDR.to_string());
    let live_path =
        std::env::var("B3_PROBE_LIVE_PATH").unwrap_or_else(|_| PROBE_LIVE_PATH.to_string());
    let ready_path =
        std::env::var("B3_PROBE_READY_PATH").unwrap_or_else(|_| PROBE_READY_PATH.to_string());

    let addr: Result<SocketAddr, _> = addr_s.parse();

    if let Ok(addr) = addr {
        let ph = ProbeHandler::new(&live_path, &ready_path, addr);
        let f = ph.future();
        let pg0 = pg.clone();
        ph.set_live(move || {
            let pg = pg0.clone();
            Box::pin(async move { readiness(pg).await })
        })
        .await;

        let pg1 = pg;
        ph.set_ready(move || {
            let pg = pg1.clone();
            Box::pin(async move { readiness(pg).await })
        })
        .await;
        tokio::spawn(f);
        info!("b3 probe listening on {addr_s}/{live_path}/{ready_path}");
    } else {
        warn!("Could not parse address for b3 probe '{addr_s}'");
    }
}
