// cargo run --bin demo_election_config
// cargo run --bin bb_helper -- --cache-dir /tmp/cache -s http://immudb:3322 -i defaultboardindex -b defaultboard  -u immudb -p immudb upsert-init-db -l debug
// cargo run --bin bb_helper -- --cache-dir /tmp/cache -s http://immudb:3322 -i defaultboardindex -b defaultboard  -u immudb -p immudb upsert-board-db -l debug
// cargo run --bin bb_client -- --server-url http://immudb:3322 init
// cargo run --bin main -- --server-url http://immudb:3322 --board-index defaultboardindex --trustee-config trustee1.toml
// cargo run --bin bb_client -- --server-url http://immudb:3322 ballots
use anyhow::Result;
use clap::Parser;
use generic_array::typenum::U32;
use generic_array::GenericArray;
use std::fs;
use std::path::PathBuf;
use tokio::time::{sleep, Duration};
use tracing::info;
use tracing::instrument;

use braid::protocol2::board::immudb::{ImmudbBoard, ImmudbBoardIndex};
use braid::protocol2::session::Session;
use braid::protocol2::trustee::Trustee;
use braid::run::config::TrusteeConfig;
use braid::util::init_log;
use strand::backend::ristretto::RistrettoCtx;
use strand::serialization::StrandDeserialize;
use strand::signature::StrandSignatureSk;

const IMMUDB_USER: &str = "immudb";
const IMMUDB_PW: &str = "immudb";

#[derive(Parser)]
struct Cli {
    #[arg(long)]
    server_url: String,

    #[arg(long)]
    board_index: String,

    #[arg(long)]
    trustee_config: PathBuf,
}

#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    init_log(true);

    // PROJECT_VERSION=$(git rev-parse HEAD) cargo run --bin main -- --server-url http://immudb:3322 --board-index defaultboardindex --trustee-config trustee1.toml
    // let version = option_env!("PROJECT_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"));
    // info!("Running braid version = {}", version);

    let args = Cli::parse();

    let contents = fs::read_to_string(args.trustee_config)
        .expect("Should have been able to read the trustee configuration file");

    let tc: TrusteeConfig = toml::from_str(&contents).unwrap();

    let bytes = braid::util::decode_base64(&tc.signing_key_sk)?;
    let sk = StrandSignatureSk::strand_deserialize(&bytes).unwrap();

    let bytes = braid::util::decode_base64(&tc.encryption_key)?;
    let ek = GenericArray::<u8, U32>::from_slice(&bytes).to_owned();

    let mut board_index =
        ImmudbBoardIndex::new(&args.server_url, IMMUDB_USER, IMMUDB_PW, args.board_index).await?;
    let store_root = std::env::current_dir().unwrap().join("message_store");
    loop {
        info!(">");
        let boards: Vec<String> = board_index.get_board_names().await?;

        for board_name in boards {
            info!("Connecting to board '{}'..", board_name.clone());
            let trustee: Trustee<RistrettoCtx> = Trustee::new(sk.clone(), ek.clone());
            let board = ImmudbBoard::new(
                &args.server_url,
                IMMUDB_USER,
                IMMUDB_PW,
                board_name.clone(),
                store_root.clone(),
            )
            .await?;

            // FIXME error should be handled to prevent loop termination
            let mut session = Session::new(trustee, board);
            info!("Running trustee for board '{}'..", board_name);
            session.step().await?;
        }
        sleep(Duration::from_millis(1000)).await;
    }
}
