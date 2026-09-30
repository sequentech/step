// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use braid::native::board::{HttpB3, HttpB3BoardParams, HttpB3Index};
use braid::util::ProtocolError;
use clap::Parser;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;
use tokio::time::{sleep, Duration};
use tracing::instrument;
use tracing::{error, info};

use braid::native::session::Session;
use braid::protocol::trustee::Trustee;
use braid::protocol::trustee::TrusteeConfig;
use strand::backend::ristretto::RistrettoCtx;
use strand::signature::StrandSignatureSk;
use strand::symm;

cfg_if::cfg_if! {
    if #[cfg(feature = "jemalloc")] {
        use tikv_jemalloc_ctl::{stats, epoch};

        #[global_allocator]
        static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
    }
}

#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    b4_url: String,

    #[arg(short, long)]
    trustee_config: PathBuf,

    #[arg(long, default_value_t = false)]
    strict: bool,

    /// Exit successfully once no board has produced work for this many seconds.
    ///
    /// Unset (the default) keeps the historical behaviour: poll forever and let
    /// something outside the process decide when to stop. Also settable with
    /// EXIT_WHEN_IDLE_SECS, matching how TRUSTEE_NAME and IGNORE_BOARDS are read.
    #[arg(long)]
    exit_when_idle_secs: Option<u64>,
}

// How often the session map (which contains trustee's memory board) is cleared
const SESSION_RESET_PERIOD: i64 = 20 * 60;

/*
Entry point for a braid mixnet trustee.

Example run command

cargo run --release --bin main  -- --b4-url http://127.0.0.1:50051 --trustee-config trustee.toml

A mixnet trustee will periodically:

    1) Poll the board index for active protocol boards
    2) For each protocol board
        a) Poll the protocol board for new messages
        b) Update the local store with new messages
        c) Execute the protocol with the existing messages in the local store

The process will loop indefinitely unless an error is encountered and the 'strict'
command line option is set to true.
*/
#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    braid::native::logging::init_log(true);

    cfg_if::cfg_if! {
        if #[cfg(feature = "jemalloc")] {
            let e = epoch::mib().unwrap();
            let allocated = stats::allocated::mib().unwrap();
            let resident = stats::resident::mib().unwrap();
        }
    }

    let args = Cli::parse();

    let contents = fs::read_to_string(args.trustee_config)
        .expect("Should have been able to read the trustee configuration file");

    let tc: TrusteeConfig = toml::from_str(&contents).unwrap();
    let sk: StrandSignatureSk = StrandSignatureSk::from_der_b64_string(&tc.signing_key_sk)?;

    let bytes = braid::util::decode_base64(&tc.encryption_key)?;
    let ek = symm::sk_from_bytes(&bytes)?;

    let ignored_boards = get_ignored_boards();
    info!("ignored boards {:?}", ignored_boards);

    let store_root = std::env::current_dir().unwrap().join("message_store");
    ensure_directory(store_root.clone())?;

    let mut session_map: HashMap<
        String,
        Session<RistrettoCtx, HttpB3, braid::native::board::SqliteStorage>,
    > = HashMap::new();
    let mut loop_count: i64 = 0;

    let exit_when_idle_secs = get_exit_when_idle_secs(args.exit_when_idle_secs);
    match exit_when_idle_secs {
        Some(secs) => info!("Will exit after {}s with no activity on any board", secs),
        None => info!("No idle deadline set, will run until stopped"),
    }
    // None means "currently active"; Some(t) is when the quiet period began.
    let mut idle_since: Option<Instant> = None;

    loop {
        info!("{} >", loop_count);

        let b3index = HttpB3Index::new(&args.b4_url);

        let boards_result = b3index.get_boards().await;
        let boards: Vec<String> = match boards_result {
            Ok(boards) => boards,
            Err(error) => {
                error!("Error listing board names: '{}' ({})", error, args.b4_url);
                sleep(Duration::from_millis(1000)).await;
                continue;
            }
        };

        if loop_count % SESSION_RESET_PERIOD == 0 {
            info!("* Session memory reset");
            session_map = HashMap::new();
        }

        let mut step_error = false;
        let mut did_work = false;
        for board_name in &boards {
            if ignored_boards.contains(&board_name) {
                info!("Ignoring board '{}'..", board_name);
                continue;
            }
            if session_map.contains_key(board_name) {
                continue;
            }

            info!(
                "* Creating new session for board '{}'..",
                board_name.clone()
            );

            let storage =
                braid::native::board::SqliteStorage::new(store_root.join(board_name), None);
            let trustee = Trustee::new(
                std::env::var("TRUSTEE_NAME").unwrap_or_else(|_| "Self".to_string()),
                board_name.to_string(),
                sk.clone(),
                ek.clone(),
                storage,
                None,
            );
            let board = HttpB3BoardParams::new(&args.b4_url).await;

            let session = Session::new(&board_name, trustee, board);
            session_map.insert(board_name.clone(), session);
        }

        // This code is sequential, see main_concurrent for an alternative implementation
        for s in session_map.values_mut() {
            let board_name = s.board_name.clone();

            let result = s.step().await;
            match result {
                Ok((posted_count, step_result)) => {
                    // Three independent signs of a live protocol: we published,
                    // a peer published, or the datalog still has work queued for
                    // us. Actions matter on their own because a trustee can have
                    // a pending action while no message moves in that tick.
                    if posted_count > 0
                        || step_result.added_messages > 0
                        || !step_result.actions.is_empty()
                    {
                        did_work = true;
                    }
                }
                Err(error) => {
                    let mut show_error = true;
                    let error_msg = format!("{:?}", error);
                    if let ProtocolError::BootstrapError(msg) = error {
                        show_error = !msg.starts_with("Zero messages received");
                    }
                    if show_error {
                        error!(
                            "Error executing step for board '{}': '{}'",
                            board_name.clone(),
                            error_msg
                        );
                    }
                    step_error = true;
                }
            };
        }

        if args.strict && step_error {
            break;
        }

        // Idle shutdown.
        //
        // Without this the trustee has no end: it polls B3 forever and is stopped
        // from outside, so a ceremony that wedges holds the pod until whatever is
        // watching gives up on it. Ending the run when the protocol goes quiet is
        // what lets the workload be a Job rather than a scaled-to-zero Deployment.
        //
        // A step error counts as activity rather than idleness. A trustee that
        // cannot reach the board has not finished, and exiting 0 there would
        // report failure as success; the run should keep retrying and be bounded
        // by the Job's own deadline instead.
        if let Some(limit) = exit_when_idle_secs {
            if did_work || step_error {
                idle_since = None;
            } else {
                let since = *idle_since.get_or_insert_with(Instant::now);
                if since.elapsed() >= Duration::from_secs(limit) {
                    info!(
                        "No activity on {} board(s) for {}s, exiting",
                        session_map.len(),
                        limit
                    );
                    break;
                }
            }
        }

        cfg_if::cfg_if! {
            if #[cfg(feature = "jemalloc")] {
                // Many statistics are cached and only updated
                // when the epoch is advanced:
                let e_ = e.advance();
                let alloc = allocated.read();
                let res = resident.read();
                let mb = 1024 * 1024;

                if let(Ok(_), Ok(alloc), Ok(res)) = (e_, alloc, res) {
                    info!("{} MB allocated / {} MB resident ({} boards)", (alloc / mb), (res / mb), boards.len());
                }
            }
        }

        loop_count = (loop_count + 1) % i64::MAX;
        println!("");
        sleep(Duration::from_millis(1000)).await;
    }

    Ok(())
}

/// Seconds of inactivity after which the run ends, from the flag or
/// EXIT_WHEN_IDLE_SECS. An unparseable value is ignored rather than fatal, so a
/// bad env var degrades to the historical run-forever behaviour instead of
/// crash-looping a trustee mid-ceremony.
fn get_exit_when_idle_secs(from_arg: Option<u64>) -> Option<u64> {
    from_arg.or_else(|| {
        let raw = std::env::var("EXIT_WHEN_IDLE_SECS").ok()?;
        match raw.parse() {
            Ok(secs) => Some(secs),
            Err(_) => {
                error!("Ignoring unparseable EXIT_WHEN_IDLE_SECS '{}'", raw);
                None
            }
        }
    })
}

fn get_ignored_boards() -> Vec<String> {
    let boards_str: String = std::env::var("IGNORE_BOARDS").unwrap_or_else(|_| "".into());
    boards_str.split(',').map(|s| s.to_string()).collect()
}

/// Checks for and creates a directory if needed.
fn ensure_directory(folder: PathBuf) -> Result<()> {
    let path = folder.as_path();
    if path.exists() {
        if path.is_dir() {
            Ok(())
        } else {
            Err(anyhow!("Path is not a folder: {}", path.display()))
        }
    } else {
        fs::create_dir(path).map_err(|err| anyhow!(err))
    }
}
