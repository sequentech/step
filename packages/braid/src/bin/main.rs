// cargo run --bin demo_election_config
// cargo run --bin bb_helper -- --cache-dir /tmp/cache -s http://immudb:3322 -i defaultboardindex -b defaultboard  -u immudb -p immudb upsert-init-db -l debug
// cargo run --bin bb_helper -- --cache-dir /tmp/cache -s http://immudb:3322 -i defaultboardindex -b defaultboard  -u immudb -p immudb upsert-board-db -l debug
// cargo run --bin bb_client -- --indexdb defaultboardindex --dbname defaultboard --server-url http://immudb:3322 init
// cargo run --bin main -- --server-url http://immudb:3322 --board-index defaultboardindex --trustee-config trustee1.toml
// cargo run --bin bb_client -- --server-url http://immudb:3322 --indexdb defaultboardindex --dbname defaultboard ballots
use anyhow::Result;
use clap::Parser;
use std::fs;
use std::path::PathBuf;
use tokio::time::{sleep, Duration};
use tracing::instrument;
use tracing::{error, info};

use braid::protocol2::board::immudb::{ImmudbBoard, ImmudbBoardIndex};
use braid::protocol2::session::{BoardParams, Session};
use braid::protocol2::trustee::Trustee;
use braid::run::config::TrusteeConfig;
use braid::util::assert_folder;
use sequent_core::util::init_log::init_log;
use strand::backend::ristretto::RistrettoCtx;
use strand::signature::StrandSignatureSk;
use strand::symm;

const IMMUDB_USER: &str = "immudb";
const IMMUDB_PW: &str = "immudb";

#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    server_url: String,

    #[arg(short, long)]
    board_index: String,

    #[arg(short, long)]
    trustee_config: PathBuf,

    #[arg(short, long, default_value_t = IMMUDB_USER.to_string())]
    user: String,

    #[arg(short, long, default_value_t = IMMUDB_PW.to_string())]
    password: String,

    #[arg(long, default_value_t = true)]
    strict: bool,
}

// PROJECT_VERSION=$(git rev-parse HEAD) cargo run --bin main -- --server-url http://immudb:3322 --board-index defaultboardindex --trustee-config trustee1.toml
// let version = option_env!("PROJECT_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"));
// info!("Running braid version = {}", version);

fn get_ignored_boards() -> Vec<String> {
    let boards_str: String = std::env::var("IGNORE_BOARDS").unwrap_or_else(|_| "".into());
    boards_str.split(',').map(|s| s.to_string()).collect()
}

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

    let ignored_boards = get_ignored_boards();
    info!("ignored boards {:?}", ignored_boards);

    let mut board_index: ImmudbBoardIndex = ImmudbBoardIndex::new(
        &args.server_url,
        &args.user,
        &args.password,
        args.board_index,
    )
    .await?;
    let store_root = std::env::current_dir().unwrap().join("message_store");
    assert_folder(store_root.clone())?;
    loop {
        info!(">");
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
                Trustee::new("Self".to_string(), sk.clone(), ek.clone());
            let board = BoardParams::new(
                &args.server_url,
                IMMUDB_USER,
                IMMUDB_PW,
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
            let session_result = session.step().await;
            match session_result {
                Ok(_) => (),
                Err(error) => {
                    // FIXME should handle a bulletin board refusing messages maliciously
                    error!(
                        "Error executing step for board '{}': '{:?}'",
                        board_name.clone(),
                        error
                    );
                    step_error = true;
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
