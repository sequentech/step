// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// cargo run --bin verify -- --server-url http://[::1]:50051 --board testboard
use anyhow::Result;
use clap::Parser;
use tracing::info;
use tracing::instrument;

use braid::native::board::{HttpB3, HttpB3BoardParams};
use braid::native::verify::verifier::Verifier;
use braid::protocol::trustee::Trustee;

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
    braid::native::logging::init_log(true);

    // generate dummy values, these are not important
    let dummy_sk = StrandSignatureSk::generate().unwrap();
    let dummy_encryption_key = strand::symm::gen_key();

    let args = Cli::parse();

    let _store_root = std::env::current_dir().unwrap().join("message_store");

    info!("Connecting to board '{}'..", args.board);

    let trustee: Trustee<RistrettoCtx, braid::native::board::NoOpStorage> = Trustee::new(
        "Verifier".to_string(),
        args.board.to_string(),
        dummy_sk,
        dummy_encryption_key,
        braid::native::board::NoOpStorage::new(),
        None,
    );
    let board_params = HttpB3BoardParams::new(&args.server_url).await;
    let board: HttpB3 = board_params.create_board(&args.board, None);
    let mut session: Verifier<RistrettoCtx, HttpB3, braid::native::board::NoOpStorage> =
        Verifier::new(trustee, board, &args.board);
    let _result = session.run().await?;

    Ok(())
}
