// cargo run --bin gen_config
// cargo run --bin bb_helper -- --cache-dir /tmp/cache -s http://immudb:3322 -i defaultindexboard -b defaultboard  -u immudb -p immudb upsert-init-db -l debug
// cargo run --bin bb_helper -- --cache-dir /tmp/cache -s http://immudb:3322 -i defaultindexboard -b defaultboard  -u immudb -p immudb upsert-board-db -l debug
// cargo run --bin bb_client -- --server-url https://localhost:3000 --cache-dir /tmp init
// cargo run --bin main -- --server-url https://localhost:3000 --cache-dir /tmp --trustee-config trustee1.toml
// cargo run --bin bb_client -- --server-url https://localhost:3000 --cache-dir /tmp ballots

cfg_if::cfg_if! {
    if #[cfg(feature = "bb-test")] {
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use clap::Parser;
use generic_array::typenum::U32;
use generic_array::GenericArray;
use std::fs;
use std::path::PathBuf;
use tokio::time::{sleep, Duration};
use tracing::instrument;

use braid::protocol2::board::immudb::ImmudbBoard;
use braid::protocol2::session::Session;
use braid::protocol2::trustee::Trustee;
use braid::run::config::TrusteeConfig;
use bulletin_board::client::{Client, FileCache};
use braid::util::init_log;
use strand::backend::ristretto::RistrettoCtx;
use strand::serialization::StrandDeserialize;
use strand::signature::StrandSignatureSk;

const BOARD_NAME: &str = "defaultboard";
const INDEX_BOARD_NAME: &str = "defaultindexboard";
const IMMUDB_USER: &str = "immudb";
const IMMUDB_PW: &str = "immudb";

#[derive(Parser)]
struct Cli {
    #[arg(long)]
    server_url: String,

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

    let trustee: Trustee<RistrettoCtx> = Trustee::new(sk, ek);
    
    let board = ImmudbBoard::new(&args.server_url, IMMUDB_USER, IMMUDB_PW, INDEX_BOARD_NAME.to_string(), BOARD_NAME.to_string()).await.unwrap();
    let mut session = Session::new(trustee, board);

    loop {
        session.step().await?;
        sleep(Duration::from_millis(1000)).await;
    }
}
}
else {
    fn main() {
        println!("Requires the 'bb-test' feature");
    }
}
}
