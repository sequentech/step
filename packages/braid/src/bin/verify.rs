// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// cargo run --bin verify -- --server-url http://immudb:3322 --board defaultboard
use anyhow::Result;
use clap::Parser;
use tracing::info;
use tracing::instrument;

use braid::protocol::board::immudb::ImmudbBoard;
use braid::protocol::trustee::Trustee;
use braid::verify::verifier::Verifier;
use sequent_core::util::init_log::init_log;
use strand::backend::ristretto::RistrettoCtx;
use strand::signature::StrandSignatureSk;

const IMMUDB_USER: &str = "immudb";
const IMMUDB_PW: &str = "immudb";

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
    #[arg(long)]
    ballot_hash: Option<String>,
}

#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    // generate dummy values, these are not important
    let dummy_sk = StrandSignatureSk::gen().unwrap();
    let dummy_encryption_key = strand::symm::gen_key();

    init_log(true);
    let args = Cli::parse();

    let _store_root = std::env::current_dir().unwrap().join("message_store");

    info!("Connecting to board '{}'..", args.board);
    let trustee: Trustee<RistrettoCtx> =
        Trustee::new("Verifier".to_string(), dummy_sk, dummy_encryption_key);
    let board =
        ImmudbBoard::new(&args.server_url, IMMUDB_USER, IMMUDB_PW, args.board, None).await?;
    let mut session = Verifier::new(trustee, board);
    session.run().await?;

    Ok(())
}
