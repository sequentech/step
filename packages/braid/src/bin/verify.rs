// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// cargo run --bin verify -- --b3-url http://[::1]:50051 --board testboard
use anyhow::{anyhow, Result};
use clap::Parser;
use tracing::info;
use tracing::instrument;

use b3::messages::newtypes::ConfigurationHash;
use braid::protocol::board::grpc_m::GrpcB3;
use braid::protocol::trustee2::Trustee;
use braid::verify::verifier::{VerificationOutcome, Verifier};

use strand::backend::ristretto::RistrettoCtx;
use strand::signature::StrandSignatureSk;

/// Verifies election data on a bulletin board
#[derive(Parser)]
struct Cli {
    /// URL of the grpc bulletin board server
    #[arg(long)]
    server_url: String,

    /// Name of the board to audit
    #[arg(long)]
    board: String,

    /// Hex encoded hash of the configuration the board must use
    #[arg(long, value_parser = parse_cfg_hash)]
    expected_cfg_hash: Option<ConfigurationHash>,
}

fn parse_cfg_hash(value: &str) -> Result<ConfigurationHash> {
    let bytes = hex::decode(value)?;
    Ok(ConfigurationHash(braid::util::hash_from_vec(&bytes)?))
}

/// Entry point for the braid verifier.
///
/// Executes verification against the specified
/// board on a grpc bulletin board.
#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    braid::util::init_log(true);

    // generate dummy values, these are not important
    let dummy_sk = StrandSignatureSk::gen().unwrap();
    let dummy_encryption_key = strand::symm::gen_key();

    let args = Cli::parse();

    let _store_root = std::env::current_dir().unwrap().join("message_store");

    info!("Connecting to board '{}'..", args.board);
    let trustee: Trustee<RistrettoCtx> = Trustee::new(
        "Verifier".to_string(),
        args.board.to_string(),
        dummy_sk,
        dummy_encryption_key,
        None,
        None,
    );
    let board = GrpcB3::new(&args.server_url);
    let mut session = Verifier::new(trustee, board, &args.board, args.expected_cfg_hash);

    match session.run().await? {
        VerificationOutcome::Passed => Ok(()),
        VerificationOutcome::Failed => {
            Err(anyhow!("Verification of board '{}' failed", args.board))
        }
    }
}
