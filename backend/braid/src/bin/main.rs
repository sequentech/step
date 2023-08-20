// cargo run --bin gen_config
// cargo run --bin bb_helper -- --cache-dir /tmp/cache -s http://immudb:3322 -i defaultindexboard -b defaultboard  -u immudb -p immudb upsert-init-db -l debug
// cargo run --bin bb_helper -- --cache-dir /tmp/cache -s http://immudb:3322 -i defaultindexboard -b defaultboard  -u immudb -p immudb upsert-board-db -l debug
// cargo run --bin bb_client --features=bb-test -- --server-url http://immudb:3322 init
// cargo run --bin main --features=bb-test -- --server-url http://immudb:3322 --truste-config trustee1.toml 
//  cargo run --bin bb_client --features=bb-test -- --server-url http://immudb:3322 ballots
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use tracing::{info};
use clap::Parser;
use generic_array::typenum::U32;
use generic_array::GenericArray;
use std::fs;
use std::path::PathBuf;
use tokio::time::{sleep, Duration};
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
    let args = Cli::parse();

    let contents = fs::read_to_string(args.trustee_config)
        .expect("Should have been able to read the trustee configuration file");

    let tc: TrusteeConfig = toml::from_str(&contents).unwrap();

    let bytes = general_purpose::STANDARD_NO_PAD
        .decode(&tc.signing_key)
        .map_err(|error| anyhow!(error))?;
    let sk = StrandSignatureSk::strand_deserialize(&bytes).unwrap();

    let bytes = general_purpose::STANDARD_NO_PAD
        .decode(&tc.encryption_key)
        .map_err(|error| anyhow!(error))?;
    let ek = GenericArray::<u8, U32>::from_slice(&bytes).to_owned();

    let mut board_index = ImmudbBoardIndex::new(&args.server_url, IMMUDB_USER, IMMUDB_PW, args.board_index).await?;
    loop {
        info!(">");
        let boards: Vec<String> = board_index.get_board_names().await?;
        for board_name in boards {
            info!("Connecting to board '{}'..", board_name.clone());
            let trustee: Trustee<RistrettoCtx> = Trustee::new(sk.clone(), ek.clone());
            let board = ImmudbBoard::new(&args.server_url, IMMUDB_USER, IMMUDB_PW, board_name.clone()).await?;
            let mut session = Session::new(trustee, board); 
            info!("Running trustee for board '{}'..", board_name);
            // FIXME error should be handled to prevent loop termination
            session.step().await?;
        }
        sleep(Duration::from_millis(1000)).await;
    }
}
