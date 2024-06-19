// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use clap::Parser;
use std::fs;
use std::path::PathBuf;
use tokio::time::{sleep, Duration};
use tracing::instrument;
use tracing::{error, info};

use braid::protocol::board::immudb::ImmudbBoardIndex;
use braid::protocol::session::{BoardParams, Session};
use braid::protocol::trustee::Trustee;
use braid::protocol::trustee::TrusteeConfig;
use braid::util::assert_folder;
use sequent_core::util::init_log::init_log;
use strand::backend::ristretto::RistrettoCtx;
use strand::signature::StrandSignatureSk;
use strand::symm;

const IMMUDB_USER: &str = "immudb";
const IMMUDB_PW: &str = "immudb";
const IMMUDB_URL: &str = "http://immudb:3322";

#[derive(Parser)]
struct Cli {
    #[arg(short, long, default_value_t = IMMUDB_URL.to_string())]
    server_url: String,

    #[arg(short, long)]
    board_index: String,

    #[arg(short, long)]
    trustee_config: PathBuf,

    #[arg(short, long, default_value_t = IMMUDB_USER.to_string())]
    user: String,

    #[arg(short, long, default_value_t = IMMUDB_PW.to_string())]
    password: String,

    #[arg(long, default_value_t = false)]
    strict: bool,
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
    init_log(true);
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
    loop {
        info!(">");

        let mut board_index: ImmudbBoardIndex = ImmudbBoardIndex::new(
            &args.server_url,
            &args.user,
            &args.password,
            args.board_index.clone(),
        )
        .await?;

        let boards_result = board_index.get_board_names().await;
        let boards: Vec<String> = match boards_result {
            Ok(boards) => boards,
            Err(error) => {
                error!("Error listing board names: '{}'", error);
                sleep(Duration::from_millis(1000)).await;
                continue;
            }
        };

        let mut step_error = false;
        for board_name in boards {
            if ignored_boards.contains(&board_name) {
                info!("Ignoring board '{}'..", board_name);
                continue;
            }

            info!("Connecting to board '{}'..", board_name.clone());
            let trustee: Trustee<RistrettoCtx> =
                Trustee::new(std::env::var("TRUSTEE_NAME").unwrap_or_else(|_| "Self".to_string()), sk.clone(), ek.clone());
            let board = BoardParams::new(
                &args.server_url,
                &args.user,
                &args.password,
                &board_name,
                Some(store_root.clone()),
            );

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

            let session = Session::new(trustee, board);
            info!("Running trustee for board '{}'..", board_name);
            let (session, result) = session.step().await;
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
        }
        if args.strict && step_error {
            break;
        }
        sleep(Duration::from_millis(1000)).await;
    }

    Ok(())
}
