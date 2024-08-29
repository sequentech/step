// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use braid::protocol::board::grpc2::{
    BoardFactoryMulti, BoardMulti, GrpcB3, GrpcB3BoardParams, GrpcB3Index,
};
use clap::Parser;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tokio::time::{sleep, Duration};
use tracing::instrument;
use tracing::{error, info};

use braid::protocol::session2::Session2;
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

    #[arg(long, default_value_t = 300)]
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

    let mut session_map: HashMap<String, Session2<RistrettoCtx>> = HashMap::new();
    let mut loop_count: u64 = 0;
    loop {
        info!("{} >", loop_count);

        let b3index = GrpcB3Index::new(&args.server_url);

        let boards_result = b3index.get_boards().await;

        let Ok(boards) = boards_result else {
            error!(
                "Error listing board names: '{}' ({})",
                boards_result.err().unwrap(),
                args.server_url
            );
            sleep(Duration::from_millis(1000)).await;
            continue;
        };

        if loop_count % args.session_reset_period == 0 {
            info!("* Session memory reset");
            session_map = HashMap::new();
        }

        let mut requests: Vec<(String, i64)> = vec![];
        for board_name in &boards {
            if ignored_boards.contains(&board_name) {
                info!("Ignoring board '{}'..", board_name);
                continue;
            }

            let session = session_map.get_mut(board_name);
            let last_id = if let Some(s) = session {
                s.get_last_external_id().await?
            } else {
                info!(
                    "* Creating new session for board '{}'..",
                    board_name.clone()
                );

                let trustee: Trustee<RistrettoCtx> = Trustee::new(
                    std::env::var("TRUSTEE_NAME").unwrap_or_else(|_| "Self".to_string()),
                    sk.clone(),
                    ek.clone(),
                );

                let mut session = Session2::new(&board_name, trustee, &store_root);
                let last_id = session.get_last_external_id().await?;

                session_map.insert(board_name.clone(), session);

                last_id
            };

            requests.push((board_name.to_string(), last_id));
        }

        info!("gathered {} requests", requests.len());
        let board = GrpcB3BoardParams::new(&args.server_url);
        let board = board.get_board();
        let responses = board.get_messages_multi(&requests).await;
        let Ok(responses) = responses else {
            error!(
                "Error retrieving messages for {} requests: {}",
                requests.len(),
                responses.err().unwrap()
            );
            sleep(Duration::from_millis(1000)).await;
            continue;
        };
        info!("received {} keyed messages", responses.len());

        let mut step_error = false;
        let mut post_messages = vec![];
        let mut total_bytes: u32 = 0;

        for km in responses {
            if km.messages.len() == 0 {
                continue;
            }

            let s = session_map.get_mut(&km.board);
            let Some(s) = s else {
                error!("Could not retrieve session with name: '{}'", km.board);
                continue;
            };
            // println!("Step for {} with {} messages", km.board, km.messages.len());
            let messages = s.step(km.messages, loop_count).await;

            let Ok(messages) = messages else {
                let _ = messages.inspect_err(|error| {
                    error!(
                        "Error executing step for board '{}': '{:?}'",
                        km.board, error
                    );
                    // FIXME identify this condition properly
                    if error.to_string().contains("Self authority not found") {
                        ignored_boards.push(km.board);
                    } else {
                        step_error = true;
                    }
                });

                continue;
            };

            if messages.len() > 0 {
                let next_bytes: usize = messages
                    .iter()
                    .map(|m| m.artifact.as_ref().map(|v| v.len()).unwrap_or(0))
                    .sum();
                total_bytes += next_bytes as u32;
                post_messages.push((km.board, messages));
            }
        }

        if post_messages.len() > 0 {
            info!(
                "Posting {} keyed messages with {:.2} MB",
                post_messages.len(),
                f64::from(total_bytes) / (1024.0 * 1024.0)
            );
            let result = board.insert_messages_multi(post_messages).await;
            if let Err(err) = result {
                error!("Error posting messages: '{:?}'", err);
                step_error = true;
            }
        } else {
            info!("No messages to post on this step");
        }

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
        println!("");
        sleep(Duration::from_millis(1000)).await;
    }

    Ok(())
}
