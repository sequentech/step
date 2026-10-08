// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Election event databases against a real server: `ELECTORAL_LOG_PG_*` name the base
//! database, with a role that may create databases.

use anyhow::Result;
use electoral_log::{
    adapters::{
        ballot_box::{AcceptBallot, AcceptOutcome, BallotStatus},
        events::{ActivityMark, EventDatabases},
        migration::move_to_event_databases,
        postgres::{LogScope, PostgresStore},
        transfer::{ExportManifest, ExportedRecord, SignedCheckpoint, EXPORT_FORMAT_V1},
    },
    messages::newtypes::ElectoralLogCheckpointReason,
    ports::ElectoralLogStore,
    proofs::{checkpoint_signing_bytes, LogState},
    service::BoardClient,
    ElectoralLogMessage, LogEntry, LogQuery,
};
use std::sync::Arc;
use strand::signature::{StrandSignaturePk, StrandSignatureSk};
use uuid::Uuid;

fn databases() -> Result<EventDatabases> {
    EventDatabases::from_env()
}

/// The board name of an event, as Windmill names it.
fn board(tenant: &Uuid, event: &Uuid) -> String {
    format!(
        "testtenant{}event{}",
        &tenant.simple().to_string()[..17],
        event.simple()
    )
}

fn entry(delivery: &str, created: i64) -> LogEntry {
    LogEntry {
        delivery_id: delivery.into(),
        message: ElectoralLogMessage {
            id: 0,
            created,
            sender_pk: "sender".into(),
            statement_timestamp: created,
            statement_kind: "KeycloakUserEvent".into(),
            message: vec![1, 2, 3, created as u8],
            version: "2".into(),
            user_id: Some(format!("user-{created}")),
            username: Some("O'Brien".into()),
            election_id: None,
            area_id: None,
            ballot_id: None,
        },
    }
}

fn entries(range: std::ops::Range<i64>) -> Vec<LogEntry> {
    range.map(|n| entry(&format!("delivery-{n}"), n)).collect()
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_PG_* with a role that may create databases"]
async fn event_databases_are_created_routed_and_dropped() -> Result<()> {
    let databases = databases()?;
    let (tenant, other_tenant, event) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let board = board(&tenant, &event);
    let event_id = event.to_string();
    assert!(!databases.has_event(&event_id).await?);
    databases
        .create_event(&tenant.to_string(), &event_id)
        .await?;
    // Creating it again is harmless; another tenant cannot claim it.
    databases
        .create_event(&tenant.to_string(), &event_id)
        .await?;
    assert!(databases
        .create_event(&other_tenant.to_string(), &event_id)
        .await
        .is_err());
    let entry = databases.event(&event_id).await?.unwrap();
    assert_eq!(entry.tenant_id, tenant.to_string());
    assert_eq!(entry.database_name, databases.database_name(&event_id)?);

    let client = BoardClient::new(Arc::new(databases.clone()));
    client.create_board(&board).await?;
    client.append(&board, &entries(0..5)).await?;
    assert_eq!(client.count(&board, &LogQuery::default()).await?, 5);
    assert_eq!(client.query(&board, &LogQuery::default()).await?.len(), 5);
    // Another tenant's board name for the same event reads nothing.
    let foreign = self::board(&other_tenant, &event);
    assert!(!client.has_board(&foreign).await?);
    assert_eq!(client.count(&foreign, &LogQuery::default()).await?, 0);
    assert!(client
        .query(&foreign, &LogQuery::default())
        .await?
        .is_empty());
    assert!(client.append(&foreign, &entries(5..6)).await.is_err());
    // A board of an event without a database neither.
    let unknown = self::board(&tenant, &Uuid::new_v4());
    assert!(!client.has_board(&unknown).await?);
    assert_eq!(client.count(&unknown, &LogQuery::default()).await?, 0);

    // Another tenant cannot drop it.
    assert!(databases
        .drop_event(&other_tenant.to_string(), &event_id)
        .await
        .is_err());
    assert!(databases.has_event(&event_id).await?);
    databases.apply_schema(&event_id).await?;
    assert!(client.has_board(&board).await?);

    databases.drop_event(&tenant.to_string(), &event_id).await?;
    assert!(!databases.has_event(&event_id).await?);
    assert!(!client.has_board(&board).await?);
    databases.drop_event(&tenant.to_string(), &event_id).await?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_PG_* with a role that may create databases"]
async fn ballot_activity_marks_clear_only_when_older_than_the_margin() -> Result<()> {
    let databases = databases()?;
    let (tenant, event) = (Uuid::new_v4().to_string(), Uuid::new_v4().to_string());
    databases.create_event(&tenant, &event).await?;
    let marked = |activity: &[electoral_log::adapters::events::BallotActivity]| {
        activity
            .iter()
            .find(|activity| activity.election_event_id == event)
            .cloned()
    };
    assert!(marked(&databases.events_with_ballot_activity().await?).is_none());
    databases
        .mark_ballot_activity(&event, ActivityMark::Immediate)
        .await?;
    let activity = marked(&databases.events_with_ballot_activity().await?).unwrap();
    assert_eq!(activity.tenant_id, tenant);
    // Too young: a vote accepted while the event was checked may not be marked yet.
    assert!(!databases.clear_ballot_activity(&activity).await?);
    databases
        .catalog()
        .client()
        .await?
        .execute(
            "UPDATE electoral_log_events SET ballots_accepted_at = now() - interval '2 minutes' \
             WHERE election_event_id = $1::text::uuid",
            &[&event],
        )
        .await?;
    let old = marked(&databases.events_with_ballot_activity().await?).unwrap();
    // A mark renewed since it was read stays.
    databases
        .mark_ballot_activity(&event, ActivityMark::Immediate)
        .await?;
    assert!(!databases.clear_ballot_activity(&old).await?);
    databases
        .catalog()
        .client()
        .await?
        .execute(
            "UPDATE electoral_log_events SET ballots_accepted_at = now() - interval '2 minutes' \
             WHERE election_event_id = $1::text::uuid",
            &[&event],
        )
        .await?;
    let old = marked(&databases.events_with_ballot_activity().await?).unwrap();
    // A mark young when it was read stays, however late the check ends.
    let young = electoral_log::adapters::events::BallotActivity {
        listed_at: old.marked_at + std::time::Duration::from_secs(30),
        ..old.clone()
    };
    assert!(!databases.clear_ballot_activity(&young).await?);
    assert!(databases.clear_ballot_activity(&old).await?);
    assert!(marked(&databases.events_with_ballot_activity().await?).is_none());
    databases.drop_event(&tenant, &event).await?;
    Ok(())
}

/// Export the logs of `store` with their records.
async fn export(store: &PostgresStore) -> Result<(Vec<ExportedRecord>, ExportManifest)> {
    let mut records = Vec::new();
    let logs = store
        .export_logs(|record| {
            records.push(record);
            Ok(())
        })
        .await?;
    Ok((
        records,
        ExportManifest {
            format: EXPORT_FORMAT_V1.into(),
            election_event_id: "source".into(),
            logs,
        },
    ))
}

fn sign(
    key: &StrandSignatureSk,
    checkpoint: &electoral_log::proofs::Checkpoint,
) -> Result<SignedCheckpoint> {
    let reason = ElectoralLogCheckpointReason::Periodic;
    Ok(SignedCheckpoint {
        checkpoint: checkpoint.clone(),
        reason,
        signer_pk: StrandSignaturePk::from_sk(key)?.to_der_b64_string()?,
        signature: key
            .sign(&checkpoint_signing_bytes(checkpoint, reason)?)?
            .to_b64_string()?,
    })
}

async fn import(
    store: &PostgresStore,
    manifest: &ExportManifest,
    records: &[ExportedRecord],
) -> Result<Vec<electoral_log::adapters::transfer::ImportedLog>> {
    store
        .import_logs(manifest, &mut records.iter().cloned().map(anyhow::Ok))
        .await
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_PG_* with a role that may create databases"]
async fn exported_logs_import_with_their_roots_and_tampering_is_refused() -> Result<()> {
    let databases = databases()?;
    let tenant = Uuid::new_v4();
    let [source_event, target_event, tampered_event] =
        [Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
    let source_board = board(&tenant, &source_event);
    let source = databases
        .create_event(&tenant.to_string(), &source_event.to_string())
        .await?;
    source.create_board(&source_board).await?;
    source
        .append(
            &source_board,
            &mut entries(0..7).into_iter().map(anyhow::Ok),
        )
        .await?;
    let journal = source.journal();
    let early = journal.checkpoint(&source_board).await?;
    source
        .append(
            &source_board,
            &mut entries(7..12).into_iter().map(anyhow::Ok),
        )
        .await?;
    let late = journal.checkpoint(&source_board).await?;
    let key = StrandSignatureSk::generate()?;
    let (records, mut manifest) = export(&source).await?;
    assert_eq!(records.len(), 12);
    assert_eq!(manifest.logs.len(), 1);
    assert_eq!(manifest.logs[0].checkpoint, late);
    assert_eq!(manifest.logs[0].state, LogState::Open);
    manifest.logs[0].published = vec![sign(&key, &early)?, sign(&key, &late)?];

    let target = databases
        .create_event(&tenant.to_string(), &target_event.to_string())
        .await?;
    let imported = import(&target, &manifest, &records).await?;
    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0].checkpoint, late);
    assert_eq!(imported[0].verified_checkpoints, 2);
    let logs = target.logs().await?;
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].state, LogState::Sealed);
    // The imported log is sealed and cannot be imported twice.
    assert!(target
        .append(
            &source_board,
            &mut entries(12..13).into_iter().map(anyhow::Ok)
        )
        .await
        .is_err());
    assert!(import(&target, &manifest, &records).await.is_err());
    // A proof from the imported copy verifies against a checkpoint of the source.
    let id = target
        .query_scope(LogScope::Database, &LogQuery::default())
        .await?
        .last()
        .unwrap()
        .id;
    let proof = target
        .record_proof(
            &target.journal(),
            LogScope::Board(&source_board),
            id,
            Some(&early),
        )
        .await?;
    proof.verify(&early)?;
    let report = target
        .audit(&source_board, &[early.clone(), late.clone()])
        .await?;
    assert!(report.is_clean(), "{:?}", report.findings());

    let tampered = databases
        .create_event(&tenant.to_string(), &tampered_event.to_string())
        .await?;
    let mut changed = records.clone();
    changed[3].username = Some("changed".into());
    let mut reordered = records.clone();
    reordered.swap(1, 2);
    let mut missing = records.clone();
    missing.remove(5);
    let mut forged = manifest.clone();
    forged.logs[0].published[0].signature = forged.logs[0].published[1].signature.clone();
    let mut other_root = manifest.clone();
    other_root.logs[0].published[0].checkpoint.root = vec![7; 32];
    other_root.logs[0].published[0] = sign(&key, &other_root.logs[0].published[0].checkpoint)?;
    for (manifest, records) in [
        (&manifest, &changed),
        (&manifest, &reordered),
        (&manifest, &missing),
        (&forged, &records),
        (&other_root, &records),
    ] {
        assert!(import(&tampered, manifest, records).await.is_err());
        assert!(
            tampered.logs().await?.is_empty(),
            "a refused import stored a log"
        );
    }
    for event in [source_event, target_event, tampered_event] {
        databases
            .drop_event(&tenant.to_string(), &event.to_string())
            .await?;
    }
    Ok(())
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_PG_* with a role that may create databases"]
async fn events_move_out_of_a_shared_database_with_their_ballots() -> Result<()> {
    let databases = databases()?;
    // A database holding several events, as the single electoral-log database did.
    let shared = format!(
        "{}_shared_{}",
        databases.connection().database(),
        &Uuid::new_v4().simple().to_string()[..8]
    );
    databases
        .catalog()
        .client()
        .await?
        .batch_execute(&format!("CREATE DATABASE {shared}"))
        .await?;
    let source = databases.connection().store_of(&shared, 2)?;
    source.initialize().await?;
    let tenant = Uuid::new_v4();
    let events = [Uuid::new_v4(), Uuid::new_v4()];
    for (n, event) in events.iter().enumerate() {
        let board = board(&tenant, event);
        source.create_board(&board).await?;
        source
            .append(
                &board,
                &mut entries(0..(3 + n as i64)).into_iter().map(anyhow::Ok),
            )
            .await?;
        source.create_ballot_box(&event.to_string()).await?;
        let event = event.to_string();
        let (election, area) = (Uuid::new_v4().to_string(), Uuid::new_v4().to_string());
        let ballot_id = format!("{n:064x}");
        let outcome = source
            .accept_ballot(&AcceptBallot {
                election_event_id: &event,
                election_id: &election,
                area_id: &area,
                voter_id: "voter",
                ballot_id: &ballot_id,
                format: "hashable-ballot",
                content: "ciphertext",
                voter_signature: None,
                pseudonym_hash: &[1; 64],
                ballot_hash: &[2; 64],
                voting_channel: "ONLINE",
                status: BallotStatus::Valid,
                voter_ip: None,
                voter_country: None,
                username: Some("voter"),
                allowed_votes: 1,
            })
            .await?;
        assert!(matches!(outcome, AcceptOutcome::Accepted { .. }));
    }
    let unknown_tenant = Uuid::new_v4();
    let stranded = Uuid::new_v4();
    source
        .create_board(&board(&unknown_tenant, &stranded))
        .await?;

    // An event's database is refused as the source.
    let event_database = databases.database_name(&events[0].to_string())?;
    assert!(
        move_to_event_databases(&databases, &event_database, &[tenant.to_string()])
            .await
            .is_err()
    );
    let report = move_to_event_databases(&databases, &shared, &[tenant.to_string()]).await?;
    assert_eq!(report.moved.len(), 2, "{report:?}");
    assert_eq!(report.skipped.len(), 1, "{report:?}");
    let activity = databases.events_with_ballot_activity().await?;
    for (n, event) in events.iter().enumerate() {
        // The moved ballots waiting for the sequencer are found by the mark.
        assert!(activity
            .iter()
            .any(|activity| activity.election_event_id == event.to_string()));
        let board = board(&tenant, event);
        let target = databases.store(&event.to_string()).await?;
        assert_eq!(
            target.count(&board, &LogQuery::default()).await?,
            3 + n as i64
        );
        assert!(target.audit(&board, &[]).await?.is_clean());
        assert!(target.has_ballot_box(&event.to_string()).await?);
        assert_eq!(target.pending_count(&event.to_string()).await?, 1);
        assert!(!source.has_board(&board).await?);
        assert!(!source.has_ballot_box(&event.to_string()).await?);
        databases
            .drop_event(&tenant.to_string(), &event.to_string())
            .await?;
    }
    // Running it again moves nothing more.
    let again = move_to_event_databases(&databases, &shared, &[tenant.to_string()]).await?;
    assert!(again.moved.is_empty());
    source.close();
    drop(source);
    databases
        .catalog()
        .client()
        .await?
        .batch_execute(&format!("DROP DATABASE {shared} WITH (FORCE)"))
        .await?;
    Ok(())
}
