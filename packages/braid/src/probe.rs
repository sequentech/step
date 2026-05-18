// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! HTTP readiness for the mixnet trustee: B3 gRPC reachability and disk headroom for the
//! on-disk message store

use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use tokio::sync::Mutex;
use tracing::{error, info, instrument, warn};
use warp::Future;
use warp::{http::Response, Filter};

use crate::protocol::board::grpc_m::GrpcB3Index;

/// Default listen address for the trustee probe HTTP server (distinct from b3’s `3030`).
const PROBE_ADDR: &str = "0.0.0.0:3031";
const PROBE_LIVE_PATH: &str = "live";
const PROBE_READY_PATH: &str = "ready";

/// Default minimum free space on the filesystem that hosts `message_store` (64 MiB).
const DEFAULT_MIN_FREE_BYTES_MESSAGE_STORE: u64 = 64 * 1024 * 1024;

/// Get minimum free space on the filesystem that hosts `message_store` from environment variable.
fn min_free_bytes_message_store_from_env() -> u64 {
    match std::env::var("BRAID_MESSAGE_STORE_MIN_FREE_BYTES") {
        Ok(s) => match s.parse::<u64>() {
            Ok(n) => n,
            Err(e) => {
                warn!(
                    "invalid BRAID_MESSAGE_STORE_MIN_FREE_BYTES '{s}': {e:?}, using default {}",
                    DEFAULT_MIN_FREE_BYTES_MESSAGE_STORE
                );
                DEFAULT_MIN_FREE_BYTES_MESSAGE_STORE
            }
        },
        Err(_) => DEFAULT_MIN_FREE_BYTES_MESSAGE_STORE,
    }
}

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

#[instrument(ret)]
async fn check_b3_index(url: &str) -> bool {
    let index = GrpcB3Index::new(url);
    match index.get_boards().await {
        Ok(_) => true,
        Err(e) => {
            error!("probe b3: get_boards failed: {e:?}");
            false
        }
    }
}

/// Free space on the filesystem backing `store_root` (directory for per-board SQLite stores).
#[instrument(ret)]
fn check_message_store_space(store_root: &Path, min_free_bytes: u64) -> bool {
    #[cfg(unix)]
    {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        let path_for_stat = if store_root.is_dir() {
            store_root
        } else {
            match store_root.parent() {
                Some(p) if !p.as_os_str().is_empty() => p,
                _ => store_root,
            }
        };

        let c_path = match CString::new(path_for_stat.as_os_str().as_bytes()) {
            Ok(p) => p,
            Err(e) => {
                error!("probe disk: invalid store path: {e:?}");
                return false;
            }
        };

        let mut vfs: libc::statvfs = unsafe { std::mem::zeroed() };
        let rc = unsafe { libc::statvfs(c_path.as_ptr(), &mut vfs) };
        if rc != 0 {
            error!("probe disk: statvfs failed for {:?}", path_for_stat);
            return false;
        }

        let free = (vfs.f_bavail as u128).saturating_mul(vfs.f_frsize as u128);
        let min = min_free_bytes as u128;
        if free < min {
            error!(
                "probe disk: free space {} bytes below minimum {} bytes on {:?}",
                free, min_free_bytes, path_for_stat
            );
            return false;
        }
        true
    }
    #[cfg(not(unix))]
    {
        let _ = store_root;
        warn!("probe disk: free-space check skipped (non-unix)");
        true
    }
}

#[instrument(ret, skip(store_root))]
async fn readiness(b3_url: String, store_root: std::path::PathBuf, min_free_bytes: u64) -> bool {
    let (b3_ok, disk_ok) = tokio::join!(check_b3_index(&b3_url), async move {
        check_message_store_space(&store_root, min_free_bytes)
    });
    info!("probe readiness: b3={b3_ok}, message_store_disk={disk_ok}");
    b3_ok && disk_ok
}

/// Serves `GET /live` and `GET /ready` probes. Readiness checks B3 connectivity and
/// disk headroom under `store_root`.
pub async fn setup_probe(b3_url: String, store_root: std::path::PathBuf) {
    let addr_s = std::env::var("BRAID_PROBE_ADDR").unwrap_or_else(|_| PROBE_ADDR.to_string());
    let live_path =
        std::env::var("BRAID_PROBE_LIVE_PATH").unwrap_or_else(|_| PROBE_LIVE_PATH.to_string());
    let ready_path =
        std::env::var("BRAID_PROBE_READY_PATH").unwrap_or_else(|_| PROBE_READY_PATH.to_string());
    let min_free_bytes = min_free_bytes_message_store_from_env();

    let addr: Result<SocketAddr, _> = addr_s.parse();

    if let Ok(addr) = addr {
        let ph = ProbeHandler::new(&live_path, &ready_path, addr);
        let f = ph.future();
        let url0 = b3_url.clone();
        let root0 = store_root.clone();
        let min0 = min_free_bytes;
        ph.set_live(move || {
            let url = url0.clone();
            let root = root0.clone();
            let min = min0;
            Box::pin(async move { readiness(url, root, min).await })
        })
        .await;

        let url1 = b3_url;
        let root1 = store_root;
        let min1 = min_free_bytes;
        ph.set_ready(move || {
            let url = url1.clone();
            let root = root1.clone();
            let min = min1;
            Box::pin(async move { readiness(url, root, min).await })
        })
        .await;
        tokio::spawn(f);
        info!("braid trustee probe on {addr_s}/{live_path}/{ready_path}");
    } else {
        warn!("Could not parse address for braid probe '{addr_s}'");
    }
}
