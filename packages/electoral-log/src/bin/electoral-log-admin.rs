// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use electoral_log::{
    adapters::postgres::PostgresStore,
    proofs::{Checkpoint, Consistency, RecordProof},
    BoardClient,
};
use std::{path::PathBuf, sync::Arc};

#[derive(Parser)]
#[command(about = "Administer and verify the PostgreSQL/Trellis electoral log")]
struct Cli {
    #[arg(value_enum)]
    action: Action,
    #[arg(long)]
    board: Option<String>,
    #[arg(long)]
    record_id: Option<i64>,
    #[arg(long)]
    checkpoint: Option<PathBuf>,
    #[arg(long)]
    proof: Option<PathBuf>,
}
#[derive(Clone, ValueEnum)]
enum Action {
    Init,
    CreateBoard,
    DeleteBoard,
    Checkpoint,
    Inclusion,
    Consistency,
    VerifyInclusion,
    VerifyConsistency,
}
fn read_json<T: serde::de::DeserializeOwned>(path: Option<&PathBuf>, name: &str) -> Result<T> {
    Ok(serde_json::from_slice(&std::fs::read(
        path.with_context(|| format!("--{name} is required"))?,
    )?)?)
}
fn output(value: &impl serde::Serialize) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    if matches!(
        args.action,
        Action::VerifyInclusion | Action::VerifyConsistency
    ) {
        let checkpoint: Checkpoint = read_json(args.checkpoint.as_ref(), "checkpoint")?;
        match args.action {
            Action::VerifyInclusion => {
                read_json::<RecordProof>(args.proof.as_ref(), "proof")?.verify(&checkpoint)?
            }
            Action::VerifyConsistency => {
                read_json::<Consistency>(args.proof.as_ref(), "proof")?.verify(&checkpoint)?
            }
            _ => unreachable!(),
        }
        println!("Verified");
        return Ok(());
    }
    let store = PostgresStore::from_env()?;
    if matches!(args.action, Action::Init) {
        return store.initialize().await;
    }
    let board = args.board.context("--board is required")?;
    let journal = store.journal();
    if matches!(
        args.action,
        Action::Checkpoint | Action::Inclusion | Action::Consistency
    ) {
        // Catch up to the committed size observed at invocation, without chasing new appends.
        let target = journal.checkpoint(&board).await?.1;
        while journal.checkpoint(&board).await?.0.tree_size < target {
            journal.process_once().await?;
        }
    }
    match args.action {
        Action::Checkpoint => output(&journal.checkpoint(&board).await?.0),
        Action::Inclusion => output(
            &store
                .record_proof(
                    &journal,
                    &board,
                    args.record_id.context("--record-id is required")?,
                )
                .await?
                .context("Proof is pending")?,
        ),
        Action::Consistency => {
            let old: Checkpoint = read_json(args.checkpoint.as_ref(), "checkpoint")?;
            anyhow::ensure!(old.log_name == board, "Checkpoint belongs to another board");
            output(&journal.consistency(&old).await?)
        }
        Action::CreateBoard => BoardClient::new(Arc::new(store)).create_board(&board).await,
        Action::DeleteBoard => BoardClient::new(Arc::new(store)).delete_board(&board).await,
        _ => unreachable!(),
    }
}
