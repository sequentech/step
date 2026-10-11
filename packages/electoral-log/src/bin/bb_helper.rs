// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use electoral_log::BoardClient;

#[derive(Parser)]
struct Cli {
    /// Private native audit PostgreSQL connection URL.
    #[arg(long)]
    database_url: Option<String>,
    #[arg(long)]
    board_name: String,
    #[arg(value_enum)]
    action: Action,
}
#[derive(Clone, ValueEnum)]
enum Action {
    UpsertBoard,
    ArchiveBoard,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    let url = args
        .database_url
        .map(Ok)
        .unwrap_or_else(|| std::env::var("WBRAID_AUDIT_DATABASE_URL"))
        .context("WBRAID_AUDIT_DATABASE_URL must be set")?;
    let mut client = BoardClient::new(&url).await?;
    match args.action {
        Action::UpsertBoard => client.upsert_electoral_log_db(&args.board_name).await,
        Action::ArchiveBoard => client.archive_board(&args.board_name).await,
    }
}
