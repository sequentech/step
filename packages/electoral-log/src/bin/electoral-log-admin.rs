// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use electoral_log::{adapters::postgres::PostgresStore, BoardClient};
use std::sync::Arc;

#[derive(Parser)]
#[command(about = "Initialize or administer the PostgreSQL electoral log")]
struct Cli {
    #[arg(value_enum)]
    action: Action,
    #[arg(long)]
    board: Option<String>,
}

#[derive(Clone, ValueEnum)]
enum Action {
    Init,
    CreateBoard,
    DeleteBoard,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    let store = PostgresStore::from_env()?;
    if matches!(args.action, Action::Init) {
        return store.initialize().await;
    }
    let board = args
        .board
        .context("--board is required for board operations")?;
    let client = BoardClient::new(Arc::new(store));
    match args.action {
        Action::CreateBoard => client.create_board(&board).await,
        Action::DeleteBoard => client.delete_board(&board).await,
        Action::Init => unreachable!(),
    }
}
