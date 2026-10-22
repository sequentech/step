// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use ballot_audit::{
    audit_with_selection, decode_message, invalid_input, validate_sha512_pin, AuditReport,
    CastVote, ExitStatus, Selection,
};
use clap::Parser;
use serde::de::DeserializeOwned;
use std::{fs, path::Path, path::PathBuf};

/// Offline receipt-to-pre-shuffle-input audit. Never connects to a database.
#[derive(Parser)]
struct Args {
    #[arg(long)]
    manifest: PathBuf,
    /// SHA-512 of the manifest's exact bytes, authenticated outside the bundle.
    #[arg(long)]
    manifest_sha512: String,
    /// JSON array of cast_vote_id, ballot_id, voter_id, voting_channel and content.
    #[arg(long)]
    cast_votes: PathBuf,
    /// Raw Borsh Configuration<RistrettoCtx> bytes.
    #[arg(long)]
    configuration: PathBuf,
    /// SHA-512 of the configuration bytes, authenticated outside the bundle.
    #[arg(long)]
    configuration_sha512: String,
    /// JSON array of unpadded-base64 signed B3 Ballots messages, exact scope only.
    #[arg(long)]
    messages: PathBuf,
    /// JSON array of unpadded-base64 signed electoral-log CastVote messages.
    #[arg(long)]
    electoral_log: PathBuf,
    /// Look up one receipt. The entire manifest is always checked.
    #[arg(long)]
    ballot_id: Option<String>,
    /// prepare-ballot-audit's selection.json, to explain receipts that are not selected.
    #[arg(long)]
    selection: Option<PathBuf>,
}

fn main() {
    match run() {
        Ok(report) => {
            if serde_json::to_writer_pretty(std::io::stdout().lock(), &report).is_err() {
                eprintln!("Audit failed: cannot write report");
                std::process::exit(ExitStatus::VerificationFailed.code());
            }
            println!();
            std::process::exit(report.status().code());
        }
        Err(error) => {
            // Keep parser/source chains private: they can contain malformed voter data.
            eprintln!("Audit failed: {error}");
            std::process::exit(ExitStatus::of(&error).code());
        }
    }
}

fn read(path: &Path, label: &str) -> Result<Vec<u8>> {
    fs::read(path).map_err(|_| invalid_input(format!("cannot read {label}")))
}

fn read_json<T: DeserializeOwned>(path: &Path, label: &str) -> Result<T> {
    serde_json::from_slice(&read(path, label)?)
        .map_err(|_| invalid_input(format!("invalid {label}")))
}

fn read_messages(path: &Path, label: &str) -> Result<Vec<Vec<u8>>> {
    read_json::<Vec<String>>(path, label)?
        .iter()
        .map(|value| {
            decode_message(value).map_err(|_| invalid_input(format!("invalid {label} encoding")))
        })
        .collect()
}

fn run() -> Result<AuditReport> {
    let args = Args::parse();
    validate_sha512_pin(&args.manifest_sha512, "manifest")?;
    validate_sha512_pin(&args.configuration_sha512, "configuration")?;
    let rows: Vec<CastVote> = read_json(&args.cast_votes, "cast vote export")?;
    let messages = read_messages(&args.messages, "messages export")?;
    let logs = read_messages(&args.electoral_log, "electoral-log export")?;
    let selection: Option<Selection> = args
        .selection
        .as_deref()
        .map(|path| read_json(path, "selection"))
        .transpose()?;
    audit_with_selection(
        &read(&args.manifest, "manifest")?,
        &args.manifest_sha512,
        &rows,
        &read(&args.configuration, "configuration")?,
        &args.configuration_sha512,
        &messages,
        args.ballot_id.as_deref(),
        &logs,
        selection.as_ref(),
    )
}
