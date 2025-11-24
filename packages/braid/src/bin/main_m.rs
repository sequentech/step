// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;

use braid::protocol::board::grpc_m::GrpcB3Index;
use clap::Parser;
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use tokio::time::{sleep, Duration};
use tracing::instrument;
use tracing::{error, info};

use braid::protocol::session::session_m::SessionFactory;
use braid::protocol::session::session_master::SessionMaster;
use braid::protocol::trustee2::TrusteeConfig;

cfg_if::cfg_if! {
    if #[cfg(feature = "jemalloc")] {
        use tikv_jemalloc_ctl::{stats, epoch};

        #[global_allocator]
        static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
    }
}

/// Runs a mixnet trustee.
///
/// This entry point supports concurrency, multiplexing and chunking.
#[derive(Parser)]
struct Cli {
    /// The url of the braid bulletin board grpc server.
    #[arg(short, long)]
    b3_url: String,

    /// The trustee configuration file including signature and symmetric encryption keys.
    #[arg(short, long)]
    trustee_config: PathBuf,

    /// Sets the tokio worker_threads parameter.
    ///
    /// Recommended to set to session_workers + 1.
    #[arg(short, long, default_value_t = 2)]
    tokio_workers: usize,

    /// The number of SessionSets that will run the protocol.
    ///
    /// SessionSets run concurrently as tokio threads and multiplex grpc b3
    /// requests. Setting this value greater than the number of cores
    /// has no effect.
    #[arg(short, long, default_value_t = 1)]
    session_workers: usize,

    /// Determines the maximum number of actions that can be executed concurrently.
    ///
    /// Higher values may increase core utilization, but also
    /// peak memory usage.
    #[arg(short, long)]
    max_concurrent_actions: Option<usize>,
}

/// Tokio entry point.
fn main() -> Result<()> {
    let args = Cli::parse();

    // let runtime = tokio::runtime::Builder::new_current_thread()
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(args.tokio_workers)
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async { run(&args).await })
}

/// Entry point for a braid mixnet trustee.
///
/// This entry point supports concurrency, multiplexing and chunking.
///
/// Concurrency
///
/// There are 3 levels of concurrency:
/// 1) Session workers (SessionSet) run as tokio threads, as per args.tokio_workers and args.session_workers.
/// 2) Inferred Actions run in a rayon collection (limited by args.max_concurrent_actions).
/// 3) Strand's extensive use of rayon collections.
///
/// The active boards are distributed to SessionSets using modulo hashing.
///
/// Multiplexing
///
/// Each SessionSet will multiplex bulletin board requests and responses across its member Sessions.
///
/// Chunking
///
/// Multiplexed requests will be chunked. Truncated responses from the bulletin board will be followed
/// up. Chunking is controlled by the value b3::grpc::MAX_MESSAGE_SIZE.
///
/// Example run command
///
/// cargo run --release --bin main_m -- --b3-url http://127.0.0.1:50051 --trustee-config trustee.toml
///
#[instrument(skip_all)]
async fn run(args: &Cli) -> Result<()> {
    braid::util::init_log(true);

    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        default_panic(info);
        std::process::exit(1);
    }));

    cfg_if::cfg_if! {
        if #[cfg(feature = "jemalloc")] {
            let mut max_allocated = 0;
            let e = epoch::mib().unwrap();
            let allocated = stats::allocated::mib().unwrap();
            let resident = stats::resident::mib().unwrap();
        }
    }

    let contents = fs::read_to_string(args.trustee_config.clone())
        .expect("Should have been able to read the trustee configuration file");

    info!("{}", strand::info_string());

    let tc: TrusteeConfig = toml::from_str(&contents).unwrap();

    let ignored_boards = get_ignored_boards();
    info!("ignored boards {:?}", ignored_boards);

    let store_root = std::env::current_dir().unwrap().join("message_store");
    braid::util::ensure_directory(store_root.clone())?;

    let store_root = std::env::current_dir().unwrap().join("message_store");

    let trustee_name = std::env::var("TRUSTEE_NAME").unwrap_or(
        args.trustee_config
            .clone()
            .into_os_string()
            .into_string()
            .unwrap(),
    );

    let factory = SessionFactory::new(&trustee_name, tc, store_root, args.max_concurrent_actions)?;
    let mut master = SessionMaster::new(&args.b3_url, factory, args.session_workers)?;

    loop {
        let b3index = GrpcB3Index::new(&args.b3_url);
        let boards_result = b3index.get_boards().await;

        let Ok(mut boards) = boards_result else {
            error!(
                "Error listing board names: '{}' ({})",
                boards_result.err().unwrap(),
                args.b3_url
            );
            sleep(Duration::from_millis(1000)).await;
            continue;
        };

        boards.retain(|b| !ignored_boards.contains(b));
        let boards_len = boards.len();
        master.refresh_sets(boards).await?;

        cfg_if::cfg_if! {
            if #[cfg(feature = "jemalloc")] {
                // Many statistics are cached and only updated
                // when the epoch is advanced:
                let e_ = e.advance();
                let alloc = allocated.read();
                let res = resident.read();
                let mb = 1024 * 1024;

                if let(Ok(_), Ok(alloc), Ok(res)) = (e_, alloc, res) {
                    if res > max_allocated {
                        max_allocated = res;
                    }
                    info!("{} MB allocated / {} MB resident (max = {} MB) ({} boards)", (alloc / mb), (res / mb), (max_allocated / mb), boards_len);
                }
            }
        }

        let _ = std::io::stdout().flush();
        sleep(Duration::from_millis(5000)).await;
    }
}

/// Returns boards that have been requested to be ignored
/// as specified by an environment variable.
///
/// Comma separated list of boards.
fn get_ignored_boards() -> HashSet<String> {
    let boards_str: String = std::env::var("IGNORE_BOARDS").unwrap_or_else(|_| "".into());
    HashSet::from_iter(boards_str.split(',').map(|s| s.to_string()))
}
