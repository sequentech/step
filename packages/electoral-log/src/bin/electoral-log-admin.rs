// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use electoral_log::{
    adapters::{
        events::{event_of_board, EventDatabases},
        postgres::LogScope,
    },
    ports::ElectoralLogStore,
    proofs::{Checkpoint, Consistency, RecordProof},
};
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "Administer and verify the PostgreSQL/Trellis electoral log")]
struct Cli {
    #[arg(value_enum)]
    action: Action,
    #[arg(long)]
    board: Option<String>,
    /// Election event whose database holds the board; by default, the event the
    /// board's name ends with. Needed for the sealed logs an imported event's board
    /// continues, whose names are those of the source event's boards.
    #[arg(long)]
    election_event_id: Option<String>,
    #[arg(long)]
    tenant_id: Option<String>,
    #[arg(long)]
    record_id: Option<i64>,
    #[arg(long)]
    checkpoint: Option<PathBuf>,
    #[arg(long)]
    proof: Option<PathBuf>,
}
#[derive(Clone, ValueEnum)]
enum Action {
    /// Create the catalog of election event databases in the base database.
    Init,
    /// Create an election event's database (`--tenant-id`, `--election-event-id`).
    CreateEventDatabase,
    /// Drop an election event's database and every record and ballot in it
    /// (`--tenant-id`, `--election-event-id`).
    DropEventDatabase,
    /// Apply the schema to an election event's database again, or to every event's
    /// without `--election-event-id`, and let the reader and backup roles read it.
    UpgradeEventDatabases,
    /// List the election events with a database.
    ListEventDatabases,
    /// List the logs of an election event's database.
    Logs,
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
    let databases = EventDatabases::from_env()?;
    let event = match (&args.election_event_id, &args.board) {
        (Some(event), _) => Some(event.clone()),
        (None, Some(board)) => Some(event_of_board(board)?),
        (None, None) => None,
    };
    let event_required = || event.clone().context("--election-event-id is required");
    let tenant_required = || args.tenant_id.as_deref().context("--tenant-id is required");
    match args.action {
        Action::Init => return databases.initialize().await,
        Action::CreateEventDatabase => {
            databases
                .create_event(tenant_required()?, &event_required()?)
                .await?;
            return Ok(());
        }
        Action::DropEventDatabase => {
            return databases
                .drop_event(tenant_required()?, &event_required()?)
                .await
        }
        Action::UpgradeEventDatabases => {
            let events = match &event {
                Some(event) => vec![event.clone()],
                None => databases
                    .events()
                    .await?
                    .into_iter()
                    .map(|entry| entry.election_event_id)
                    .collect(),
            };
            let mut failed = Vec::new();
            for event in &events {
                if let Err(error) = databases.apply_schema(event).await {
                    eprintln!("Error upgrading the database of election event {event}: {error:#}");
                    failed.push(event.clone());
                }
            }
            output(
                &serde_json::json!({ "upgraded": events.len() - failed.len(), "failed": failed }),
            )?;
            anyhow::ensure!(failed.is_empty(), "Some event databases were not upgraded");
            return Ok(());
        }
        Action::ListEventDatabases => {
            let events: Vec<_> = databases
                .events()
                .await?
                .into_iter()
                .map(|event| {
                    serde_json::json!({
                        "election_event_id": event.election_event_id,
                        "tenant_id": event.tenant_id,
                        "database": event.database_name,
                    })
                })
                .collect();
            return output(&events);
        }
        _ => {}
    }
    let store = databases.store(&event_required()?).await?;
    let journal = store.journal();
    match args.action {
        Action::Logs => return output(&journal.logs().await?),
        // Without a board, back-fill every log created before subtrees were stored.
        Action::BackfillNodes if args.board.is_none() => {
            let mut rebuilt = Vec::new();
            for board in journal.unbuilt_logs().await? {
                rebuilt.push(journal.rebuild(&board).await?);
            }
            return output(&rebuilt);
        }
        _ => {}
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
                    .record_proof(&journal, LogScope::Board(&board), id, checkpoint.as_ref())
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
        Action::CreateBoard => store.create_board(&board).await,
        Action::DeleteBoard => store.delete_board(&board).await,
        _ => unreachable!(),
    }
}
