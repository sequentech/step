// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
// cargo run --bin ballot_verify -- --ballot-hash c1035c2d09c3a6c915b3273ef18798c74c2dca3d7beb34e379df675f33eb9b50

use anyhow::{Context, Result};
use braid::{
    util::VERIFIABLE_BULLETIN_BOARD_FILE,
    verify_ballot::ballot_verifier::{BallotVerifier, BallotVerifierError},
};
use clap::Parser;
use colored::Colorize;
use rusqlite::{Connection, OpenFlags};
use std::io::{self, Write};
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
    run_app(&args).await?;
    Ok(())
}

async fn run_app(args: &Cli) -> Result<()> {
    let sqlite_connection = Connection::open_with_flags(
        VERIFIABLE_BULLETIN_BOARD_FILE,
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|_| {
        anyhow::anyhow!("Failed to open the data file for the verifiable bulletin board")
    })?;

    let mut session = BallotVerifier::new(&args.ballot_hash, &sqlite_connection);
    if let Err(err) = session.run().await {
        let _ = writeln!(io::stderr(), "Error: {}", err.to_string().red());
    }
    Ok(())
}
