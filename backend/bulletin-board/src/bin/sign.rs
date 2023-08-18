// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use bulletin_board::signature::Signable;
use bulletin_board::util::KeyPairConfig;
use bulletin_board::{AddEntriesRequest, CreateBoardRequest, NewDataEntry};
use clap::{Parser, ValueEnum};
use std::fs;
use std::path::PathBuf;
use strand::signature::StrandSignatureSk as SecretKey;
use tracing::instrument;

/// Struct used to receive the command line arguments of this command
#[derive(Parser)]
struct Cli {
    #[arg(short, long, value_enum)]
    kind: Kind,

    #[arg(short, long)]
    data: String,

    #[arg(short = 'p', long, value_name = "FILE_PATH")]
    key_pair_path: PathBuf,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Kind {
    CreateBoardRequest,
    AddEntriesRequest,
}

/// Simple command that receives a request in json and prints it out to stdin
/// signed
#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    let args = Cli::parse();

    // Get the secret_key from cli args
    let keys_pair_str: String = fs::read_to_string(args.key_pair_path)?;
    let key_pair: KeyPairConfig = toml::from_str(&keys_pair_str)?;
    let secret_key: SecretKey = key_pair.secret_key.try_into()?;

    let output = match args.kind {
        Kind::CreateBoardRequest => {
            let request: CreateBoardRequest = serde_json::from_str(&args.data)?;
            let signed_request = request.sign(&secret_key)?;
            serde_json::to_string_pretty(&signed_request)?
        }
        Kind::AddEntriesRequest => {
            let request: AddEntriesRequest = serde_json::from_str(&args.data)?;
            let signed_request = AddEntriesRequest {
                entries: request
                    .entries
                    .into_iter()
                    .map(|entry| Ok(entry.sign(&secret_key)?))
                    .collect::<Result<Vec<NewDataEntry>>>()?,
                ..request
            };
            serde_json::to_string_pretty(&signed_request)?
        }
    };
    println!("{}", output);
    Ok(())
}
