// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{Context, Result};
use ballot_audit::{audit, decode_message, CastVote};
use clap::Parser;
use std::{fs, path::PathBuf};

/// Offline receipt-to-pre-shuffle-input audit. Never connects to a database.
#[derive(Parser)]
struct Args {
    #[arg(long)]
    manifest: PathBuf,
    /// Independently authenticated SHA-512 of the manifest's exact bytes.
    #[arg(long)]
    manifest_sha512: String,
    /// JSON array of cast_vote_id, ballot_id, voter_id, voting_channel and content.
    #[arg(long)]
    cast_votes: PathBuf,
    /// Raw Borsh Configuration<RistrettoCtx> bytes.
    #[arg(long)]
    configuration: PathBuf,
    /// Independently authenticated SHA-512 of the configuration bytes.
    #[arg(long)]
    configuration_sha512: String,
    /// JSON array of unpadded-base64 signed B3 Ballots messages, exact scope only.
    #[arg(long)]
    messages: PathBuf,
    /// JSON array of unpadded-base64 signed electoral-log CastVote messages.
    #[arg(long)]
    electoral_log: PathBuf,
    /// Filter reported positions only. The entire manifest is always checked.
    #[arg(long)]
    ballot_id: Option<String>,
}
fn main() {
    if let Err(error) = run() {
        // Keep parser/source chains private: they can contain malformed voter data.
        eprintln!("Audit failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Args::parse();
    let rows: Vec<CastVote> =
        serde_json::from_slice(&fs::read(args.cast_votes)?).context("invalid cast vote export")?;
    let encoded: Vec<String> =
        serde_json::from_slice(&fs::read(args.messages)?).context("invalid messages export")?;
    let messages = encoded
        .iter()
        .map(|s| decode_message(s))
        .collect::<Result<Vec<_>>>()?;
    let log_encoded: Vec<String> = serde_json::from_slice(&fs::read(args.electoral_log)?)
        .context("invalid electoral-log export")?;
    let logs = log_encoded
        .iter()
        .map(|s| decode_message(s))
        .collect::<Result<Vec<_>>>()?;
    let report = audit(
        &fs::read(args.manifest)?,
        &args.manifest_sha512,
        &rows,
        &fs::read(args.configuration)?,
        &args.configuration_sha512,
        &messages,
        args.ballot_id.as_deref(),
        &logs,
    )?;
    serde_json::to_writer_pretty(std::io::stdout().lock(), &report)?;
    println!();
    Ok(())
}
