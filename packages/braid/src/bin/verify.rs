// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// cargo run --bin verify -- --b3-url http://[::1]:50051 --board testboard
use anyhow::Result;
use clap::Parser;
use tracing::info;
use tracing::instrument;

use braid::protocol::board::grpc_m::GrpcB3;
use braid::protocol::trustee2::Trustee;
use braid::verify::verifier::BallotForBatch;
use braid::verify::verifier::Verifier;

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
    #[arg(long)]
    ballot: Option<String>,
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
    let mut ballot_for_batch: Option<BallotForBatch> = None;
    if let Some(batch) = args.ballot.clone() {
        let mut parts = batch.split(",");

        let ballot = parts.next();
        let batch_str = parts.next();
        if let (Some(b), Some(bs)) = (ballot, batch_str) {
            let batch_number: usize = bs.parse().unwrap();

            ballot_for_batch = Some(BallotForBatch {
                ballot: b.to_string(),
                batch_number,
            });
        }
    }
    let mut session = Verifier::new(trustee, board, &args.board, ballot_for_batch);
    session.run().await?;

    Ok(())
}
