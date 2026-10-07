// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use electoral_log::{
    adapters::router::StoreRouter,
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
    /// Tenant whose database `provision-tenant` creates.
    #[arg(long)]
    tenant_id: Option<String>,
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
    ProvisionTenant,
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
    let router = StoreRouter::from_env()?;
    // `init` and `backfill-nodes` without a board cover the shared database and every
    // tenant database of the environment.
    let mut tenants = Vec::new();
    for database in router.tenant_databases().await? {
        tenants.push(router.database_store(&database).await?);
    }
    if matches!(args.action, Action::Init) {
        // The reader role may read tenant databases only: the shared one holds every
        // tenant's boards.
        router.shared().initialize().await?;
        for store in &tenants {
            router.initialize(store).await?;
        }
        return Ok(());
    }
    let databases: Vec<_> = std::iter::once(router.shared()).chain(tenants).collect();
    if matches!(args.action, Action::ProvisionTenant) {
        let tenant_id = args.tenant_id.context("--tenant-id is required")?;
        return router.provision_tenant(&tenant_id).await;
    }
    // Without a board, back-fill every log created before subtrees were stored.
    if matches!(args.action, Action::BackfillNodes) && args.board.is_none() {
        let mut rebuilt = Vec::new();
        for store in &databases {
            let journal = store.journal();
            for board in journal.unbuilt_logs().await? {
                rebuilt.push(journal.rebuild(&board).await?);
            }
        }
        return output(&rebuilt);
    }
    let board = args.board.context("--board is required")?;
    let store = router.store_for(&board).await?;
    let journal = store.journal();
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
        Action::CreateBoard => {
            BoardClient::new(Arc::new(router))
                .create_board(&board)
                .await
        }
        Action::DeleteBoard => {
            BoardClient::new(Arc::new(router))
                .delete_board(&board)
                .await
        }
        _ => unreachable!(),
    }
}
