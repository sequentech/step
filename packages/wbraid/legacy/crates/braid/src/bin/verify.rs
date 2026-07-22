// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// cargo run --bin verify -- --b3-url http://[::1]:50051 --board testboard
use anyhow::Result;
use clap::Parser;
use tracing::info;
use tracing::instrument;

use braid::native::board::{HttpB4, HttpB4BoardParams};
use braid::protocol::trustee::Trustee;
use braid::protocol::verify::verifier::Verifier;
use braid::protocol::board::NoOpStorage;

use cryptography::context::{RistrettoCtx, Context};
use cryptography::utils::signatures::SignatureScheme;

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
    ///
    /// NOT YET IMPLEMENTED
    #[arg(long)]
    ballot_hash: Option<String>,
}

/// Entry point for the braid verifier.
///
/// Executes verification against the specified board.
#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    braid::native::logging::init_log(true);

    // generate dummy values, these are not important
    let mut rng = RistrettoCtx::get_rng();
    let dummy_sk = <<RistrettoCtx as Context>::SignatureScheme as SignatureScheme<_>>::gen_signing_key(&mut rng);
    let dummy_encryption_key = cryptography::utils::symm::gen_key().unwrap();

    let args = Cli::parse();

    let _store_root = std::env::current_dir().unwrap().join("message_store");

    info!("Connecting to board '{}'..", args.board);
    
    let trustee: Trustee<RistrettoCtx, NoOpStorage> = Trustee::new(
        "Verifier".to_string(),
        args.board.to_string(),
        dummy_sk,
        dummy_encryption_key,
        NoOpStorage::new(),
        None,
    );
    let board_params = HttpB4BoardParams::new(&args.server_url);
    let board: HttpB4 = board_params.create_board(&args.board, None);
    let mut session: Verifier<RistrettoCtx, HttpB4, NoOpStorage> = 
        Verifier::new(trustee, board, &args.board);
    let _result = session.run().await?;

    Ok(())
}
