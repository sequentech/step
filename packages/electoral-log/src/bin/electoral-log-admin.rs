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
    Audit,
    BackfillNodes,
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
    let journal = store.journal();
    // Without a board, back-fill every log created before subtrees were stored.
    if matches!(args.action, Action::BackfillNodes) && args.board.is_none() {
        let mut rebuilt = Vec::new();
        for board in journal.unbuilt_logs().await? {
            rebuilt.push(journal.rebuild(&board).await?);
        }
        return output(&rebuilt);
    }
    let board = args.board.context("--board is required")?;
    // Optional trusted checkpoint for inclusion (anchor) and audit (extra history check).
    let checkpoint: Option<Checkpoint> = match args.checkpoint.as_ref() {
        Some(path) => Some(read_json(Some(path), "checkpoint")?),
        None => None,
    };
    if let Some(checkpoint) = &checkpoint {
        anyhow::ensure!(
            checkpoint.log_name == board,
            "Checkpoint belongs to another board"
        );
    }
    match args.action {
        Action::Checkpoint => output(&journal.checkpoint(&board).await?),
        Action::Inclusion => {
            let id = args.record_id.context("--record-id is required")?;
            output(
                &store
                    .record_proof(&journal, &board, id, checkpoint.as_ref())
                    .await?,
            )
        }
        Action::Consistency => {
            let old = checkpoint.context("--checkpoint is required")?;
            output(&journal.consistency(&old).await?)
        }
        Action::Audit => {
            let report = store.audit(&board, checkpoint.as_slice()).await?;
            output(&report)?;
            anyhow::ensure!(report.is_clean(), "Audit found inconsistencies");
            Ok(())
        }
        Action::BackfillNodes => output(&journal.rebuild(&board).await?),
        Action::CreateBoard => BoardClient::new(Arc::new(store)).create_board(&board).await,
        Action::DeleteBoard => BoardClient::new(Arc::new(store)).delete_board(&board).await,
        _ => unreachable!(),
    }
}
