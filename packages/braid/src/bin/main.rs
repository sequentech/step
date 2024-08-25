// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use braid::protocol::board::grpc::{GrpcB3, GrpcB3BoardParams, GrpcB3Index};
use clap::Parser;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tokio::time::{sleep, Duration};
use tracing::instrument;
use tracing::{error, info};

use braid::protocol::board::BoardFactory;
use braid::protocol::session::Session;
use braid::protocol::trustee::Trustee;
use braid::protocol::trustee::TrusteeConfig;
use braid::util::assert_folder;
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

const IMMUDB_USER: &str = "immudb";
const IMMUDB_PW: &str = "immudb";
const IMMUDB_URL: &str = "http://immudb:3322";

#[derive(Parser)]
struct Cli {
    #[arg(short, long, default_value_t = IMMUDB_URL.to_string())]
    server_url: String,

    #[arg(short, long)]
    trustee_config: PathBuf,

    #[arg(short, long, default_value_t = IMMUDB_USER.to_string())]
    user: String,

    #[arg(short, long, default_value_t = IMMUDB_PW.to_string())]
    password: String,

    #[arg(long, default_value_t = false)]
    strict: bool,

    #[arg(long, default_value_t = 240)]
    session_reset_period: u64,
}

fn get_ignored_boards() -> Vec<String> {
    let boards_str: String = std::env::var("IGNORE_BOARDS").unwrap_or_else(|_| "".into());
    boards_str.split(',').map(|s| s.to_string()).collect()
}

/*
Entry point for a braid mixnet trustee.

Example run command

cargo run --release --bin main  -- --server-url http://immudb:3322 --board-index defaultboardindex--trustee-config trustee.toml

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
    braid::util::init_log(true);

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

    info!("{}", strand::info_string());

    let tc: TrusteeConfig = toml::from_str(&contents).unwrap();
    let sk: StrandSignatureSk = StrandSignatureSk::from_der_b64_string(&tc.signing_key_sk).unwrap();

    let bytes = braid::util::decode_base64(&tc.encryption_key)?;
    let ek = symm::sk_from_bytes(&bytes)?;

    let mut ignored_boards = get_ignored_boards();
    info!("ignored boards {:?}", ignored_boards);

    let store_root = std::env::current_dir().unwrap().join("message_store");
    assert_folder(store_root.clone())?;

    let mut session_map: HashMap<String, Session<RistrettoCtx, GrpcB3>> = HashMap::new();
    let mut loop_count: u64 = 0;
    loop {
        info!("{} >", loop_count);

        let b3index = GrpcB3Index::new(&args.server_url);

        let boards_result = b3index.get_boards().await;
        let boards: Vec<String> = match boards_result {
            Ok(boards) => boards,
            Err(error) => {
                error!(
                    "Error listing board names: '{}' ({})",
                    error, args.server_url
                );
                sleep(Duration::from_millis(1000)).await;
                continue;
            }
        };

        if loop_count % args.session_reset_period == 0 {
            info!("* Session memory reset");
            session_map = HashMap::new();
        }

        let mut step_error = false;
        for board_name in &boards {
            if ignored_boards.contains(&board_name) {
                info!("Ignoring board '{}'..", board_name);
                continue;
            }
            if session_map.contains_key(board_name) {
                continue;
            }

            info!("Creating new session for board '{}'..", board_name.clone());

            let trustee: Trustee<RistrettoCtx> = Trustee::new(
                std::env::var("TRUSTEE_NAME").unwrap_or_else(|_| "Self".to_string()),
                sk.clone(),
                ek.clone(),
            );
            let board =
                GrpcB3BoardParams::new(&args.server_url, &board_name, Some(store_root.clone()));

            // Try to connect to detect errors early
            let board_result = board.get_board().await;
            match board_result {
                Ok(_) => (),
                Err(error) => {
                    error!(
                        "Error connecting to board '{}': '{}'",
                        board_name.clone(),
                        error
                    );
                    continue;
                }
            };

            let session = Session::new(&board_name, trustee, board);
            session_map.insert(board_name.clone(), session);
        }

        let mut session_map_next = HashMap::new();
        // This code is currently sequential, see protocol_test_grpc for an example of
        // handling sessions in parallel by spawning threads.
        for s in session_map.into_values() {
            let board_name = s.name.clone();
            info!("* Running trustee for board '{}'..", board_name);
            let (session, result) = s.step(loop_count).await;
            match result {
                Ok(_) => (),
                Err(error) => {
                    // FIXME should handle a bulletin board refusing messages maliciously
                    error!(
                        "Error executing step for board '{}': '{:?}'",
                        board_name.clone(),
                        error
                    );
                    // FIXME identify this condition properly
                    if error.to_string().contains("Self authority not found") {
                        ignored_boards.push(board_name);
                    } else {
                        step_error = true;
                    }
                }
            };
            info!("");
            session_map_next.insert(session.name.clone(), session);
        }
        session_map = session_map_next;

        if args.strict && step_error {
            break;
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

        loop_count = (loop_count + 1) % u64::MAX;
        sleep(Duration::from_millis(1000)).await;
    }

    Ok(())
}
