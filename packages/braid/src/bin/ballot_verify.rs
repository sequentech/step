// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// cargo run --bin ballot_verify -- --ballot-hash c1035c2d09c3a6c915b3273ef18798c74c2dca3d7beb34e379df675f33eb9b50
use anyhow::Result;
use braid::verify_ballot::ballot_verifier::BallotVerifier;
use clap::Parser;
use tracing::info;
use tracing::instrument;

/// Verifies election data on a bulletin board
#[derive(Parser)]
struct Cli {
    /// Checks inclusion of the given ballot
    ///
    #[arg(long)]
    ballot_hash: String,
}

/// Entry point for the braid ballot verifier.
///
/// Executes verification against sqlite db which holds a cipy of specific bulletin board and cast_vote table.
#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    braid::util::init_log(true);

    let args = Cli::parse();

    let mut session = BallotVerifier::new(&args.ballot_hash);
    session.run().await?;

    Ok(())
}
