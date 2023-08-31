// cargo run --bin verify -- --server-url http://immudb:3322 --board defaultboard --trustee-config trustee1.toml
use anyhow::Result;
use clap::Parser;
use generic_array::typenum::U32;
use generic_array::GenericArray;
use std::fs;
use std::path::PathBuf;
use tracing::info;
use tracing::instrument;

use braid::protocol2::board::immudb::ImmudbBoard;
use braid::protocol2::trustee::Trustee;
use braid::run::config::TrusteeConfig;
use braid::util::init_log;
use braid::verify::verifier::VerifyingSession;
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
    board: String,

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

    let bytes = braid::util::decode_base64(&tc.signing_key_sk)?;
    let sk = StrandSignatureSk::strand_deserialize(&bytes).unwrap();

    let bytes = braid::util::decode_base64(&tc.encryption_key)?;
    let ek = GenericArray::<u8, U32>::from_slice(&bytes).to_owned();

    let store_root = std::env::current_dir().unwrap().join("message_store");

    info!(">");
    info!("Connecting to board '{}'..", args.board);
    let trustee: Trustee<RistrettoCtx> = Trustee::new(sk.clone(), ek.clone());
    let board = ImmudbBoard::new(
        &args.server_url,
        IMMUDB_USER,
        IMMUDB_PW,
        args.board,
        store_root.clone(),
    )
    .await?;
    let mut session = VerifyingSession::new(trustee, board);
    session.run().await?;

    Ok(())
}
