// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// cargo run --bin verify -- --b3-url http://[::1]:50051 --board testboard
use anyhow::Result;
use clap::Parser;
use tracing::info;
use tracing::instrument;

use braid_b5::native::board::{HttpB3, HttpB3BoardParams};
use braid_b5::protocol::trustee::Trustee;
use braid_b5::native::verify::verifier::Verifier;

use cryptography::context::{RistrettoCtx, Context};
use cryptography::utils::signatures::SignatureScheme;

/// Verifies election data on a bulletin board
#[derive(Parser)]
struct Cli {
    /// URL of the grpc bulletin board server
    #[arg(long)]
    server_url: String,

    /// Name of the board to audit
    #[arg(long)]
    board: String,

    /// Checks inclusion of the given ballot
    ///
    /// NOT YET IMPLEMENTED
    #[arg(long)]
    ballot_hash: Option<String>,
}

/// Entry point for the braid verifier.
///
/// Executes verification against the specified
/// board on a grpc bulletin board.
#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    braid_b5::native::logging::init_log(true);

    // generate dummy values, these are not important
    let mut rng = RistrettoCtx::get_rng();
    let dummy_sk = <<RistrettoCtx as Context>::SignatureScheme as SignatureScheme<_>>::gen_signing_key(&mut rng);
    let dummy_encryption_key = cryptography::utils::symm::gen_key();

    let args = Cli::parse();

    let _store_root = std::env::current_dir().unwrap().join("message_store");

    info!("Connecting to board '{}'..", args.board);
    
    let trustee: Trustee<RistrettoCtx, braid_b5::native::board::NoOpStorage> = Trustee::new(
        "Verifier".to_string(),
        args.board.to_string(),
        dummy_sk,
        dummy_encryption_key,
        braid_b5::native::board::NoOpStorage::new(),
        None,
    );
    let board_params = HttpB3BoardParams::new(&args.server_url).await;
    let board: HttpB3 = board_params.create_board(&args.board, None);
    let mut session: Verifier<RistrettoCtx, HttpB3, braid_b5::native::board::NoOpStorage> = 
        Verifier::new(trustee, board, &args.board);
    let _result = session.run().await?;

    Ok(())
}
