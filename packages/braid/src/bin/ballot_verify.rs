// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
// cargo run --bin ballot_verify -- --ballot-hash c1035c2d09c3a6c915b3273ef18798c74c2dca3d7beb34e379df675f33eb9b50

use anyhow::Result;
use braid::verify_ballot::ballot_verifier::{BallotVerifier, BallotVerifierError};
use clap::Parser;
use hex::FromHex;
use reedline_repl_rs::yansi::Paint;
use std::process;
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

    // Run the whole flow, mapping every failure into a BallotVerifierError
    if let Err(e) = run_app(&args).await {
        eprintln!("{}", e.red());
        process::exit(1);
    }

    Ok(())
}

fn validate_digest(s: &str) -> Result<String, String> {
    if Vec::from_hex(s).map(|b| b.len()) == Ok(32) {
        Ok(s.to_string())
    } else {
        Err("must be a valid SHA-256 hex digest".into())
    }
}

async fn run_app(args: &Cli) -> Result<(), BallotVerifierError> {
    validate_digest(&args.ballot_hash).map_err(|_| BallotVerifierError::InvalidHash)?;

    let mut session = BallotVerifier::new(&args.ballot_hash);
    session.run().await.map_err(|e| match e {
        BallotVerifierError::BallotNotFound => BallotVerifierError::BallotNotFound,
        BallotVerifierError::DataFileOpen => BallotVerifierError::DataFileOpen,
        BallotVerifierError::DataCorruption => BallotVerifierError::DataCorruption,
        BallotVerifierError::Unexpected => BallotVerifierError::Unexpected,
        _ => BallotVerifierError::Unexpected,
    })?;

    Ok(())
}
