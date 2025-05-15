// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
// cargo run --bin verify -- --ballot-hash c1035c2d09c3a6c915b3273ef18798c74c2dca3d7beb34e379df675f33eb9b50

use anyhow::{Context, Result};
use braid::protocol::trustee2::Trustee;
use braid::verify::verifier::Verifier;
use braid::verify_ballot::ballot_verifier;
use clap::Parser;
use colored::Colorize;
use rusqlite::OpenFlags;
use std::io::{self, Write};
use tracing::instrument;

use strand::backend::ristretto::RistrettoCtx;
use strand::signature::StrandSignatureSk;

/// Verifies election data on a bulletin board
#[derive(Parser)]
struct Cli {
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
    braid::util::init_log(true);

    // generate dummy values, these are not important
    let dummy_sk = StrandSignatureSk::gen().unwrap();
    let dummy_encryption_key = strand::symm::gen_key();

    let args = Cli::parse();

    let _store_root = std::env::current_dir().unwrap().join("message_store");

    let trustee: Trustee<RistrettoCtx> = Trustee::new(
        "Verifier".to_string(),
        "bulletin_board".to_string(),
        dummy_sk,
        dummy_encryption_key,
        None,
        None,
    );

    let sqlite_connection: rusqlite::Connection = rusqlite::Connection::open_with_flags(
        braid::util::VERIFIABLE_BULLETIN_BOARD_FILE,
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .context("Could not open db file")?;

    let mut session = Verifier::new(trustee, "bulletin_board", &sqlite_connection);
    session.run().await?;

    let ballot_hash = args.ballot_hash;
    if let Some(hash) = ballot_hash {
        let mut ballot_verifier = ballot_verifier::BallotVerifier::new(&hash, &sqlite_connection);

        if let Err(err) = ballot_verifier.run().await {
            let _ = writeln!(io::stderr(), "Error: {}", err.to_string().red());
        }
    }

    Ok(())
}
