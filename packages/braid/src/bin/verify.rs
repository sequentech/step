// cargo run --bin verify -- --server-url http://immudb:3322 --board defaultboard --trustee-config trustee1.toml
use anyhow::Result;
use chacha20poly1305::KeyInit;
use clap::Parser;
use tracing::info;
use tracing::instrument;

use braid::protocol2::board::immudb::ImmudbBoard;
use braid::protocol2::trustee::Trustee;
use braid::util::init_log;
use braid::verify::verifier::Verifier;
use strand::backend::ristretto::RistrettoCtx;
use strand::signature::StrandSignatureSk;

const IMMUDB_USER: &str = "immudb";
const IMMUDB_PW: &str = "immudb";

/// Verifies election data on a bulletin board
#[derive(Parser)]
struct Cli {
    /// URL of the bulletin board server
    #[arg(long)]
    server_url: String,

    /// Name of the board to audit
    #[arg(long)]
    board: String,

    /// Checks inclusion of the given ballot
    #[arg(long)]
    ballot_hash: Option<String>,
}

#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    // generate dummy values, these are not important
    let mut csprng = strand::rng::StrandRng;
    let dummy_sk = StrandSignatureSk::new().unwrap();
    let dummy_encryption_key = chacha20poly1305::ChaCha20Poly1305::generate_key(&mut csprng);

    init_log(true);
    let args = Cli::parse();

    let store_root = std::env::current_dir().unwrap().join("message_store");

    info!("Connecting to board '{}'..", args.board);
    let trustee: Trustee<RistrettoCtx> = Trustee::new(dummy_sk, dummy_encryption_key);
    let board = ImmudbBoard::new(
        &args.server_url,
        IMMUDB_USER,
        IMMUDB_PW,
        args.board,
        store_root.clone(),
    )
    .await?;
    let mut session = Verifier::new(trustee, board);
    session.run().await?;

    Ok(())
}
