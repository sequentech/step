// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Election event databases against a real server: `ELECTORAL_LOG_PG_*` name the base
//! database, with a role that may create databases.

use anyhow::Result;
use electoral_log::{
    adapters::{
        events::{ActivityMark, EventDatabases},
        postgres::{LogScope, PostgresStore},
        transfer::{ExportManifest, ExportedRecord, SignedCheckpoint, EXPORT_FORMAT_V1},
    },
    messages::{
        message::{Message, SigningData},
        newtypes::{
            ElectoralLogCheckpointReason, ElectoralLogCheckpointV2, ElectoralLogContinuation,
            EventIdString,
        },
    },
    ports::ElectoralLogStore,
    proofs::{checkpoint_signing_bytes, LogState},
    service::BoardClient,
    ElectoralLogMessage, LogEntry, LogQuery,
};
use std::sync::Arc;
use uuid::Uuid;
use voting_crypto::signing::{SigningKey, VerificationKey};

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
    // Nor can it be created in the event's database.
    assert!(client.create_board(&foreign).await.is_err());
    assert!(!client.has_board(&foreign).await?);
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

    // A database a failed creation left unregistered is dropped too.
    let unregistered = Uuid::new_v4().to_string();
    let name = databases.database_name(&unregistered)?;
    let catalog = databases.catalog().client().await?;
    catalog
        .batch_execute(&format!("CREATE DATABASE \"{name}\""))
        .await?;
    databases
        .drop_event(&tenant.to_string(), &unregistered)
        .await?;
    assert!(catalog
        .query_opt("SELECT 1 FROM pg_database WHERE datname = $1", &[&name])
        .await?
        .is_none());
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
    key: &SigningKey,
    checkpoint: &electoral_log::proofs::Checkpoint,
) -> Result<SignedCheckpoint> {
    let reason = ElectoralLogCheckpointReason::Periodic;
    Ok(SignedCheckpoint {
        checkpoint: checkpoint.clone(),
        reason,
        signer_pk: VerificationKey::from_sk(key)?.to_b64_string()?,
        signature: key
            .sign(&checkpoint_signing_bytes(checkpoint, reason)?)?
            .to_b64_string()?,
    })
}

async fn import(
    store: &PostgresStore,
    manifest: &ExportManifest,
    records: &[ExportedRecord],
    anchors: &[electoral_log::proofs::Checkpoint],
) -> Result<Vec<electoral_log::adapters::transfer::ImportedLog>> {
    store
        .import_logs(
            manifest,
            &mut records.iter().cloned().map(anyhow::Ok),
            anchors,
        )
        .await
}

/// A signed record, as Windmill appends it.
fn signed_entry(delivery: &str, message: &Message) -> Result<LogEntry> {
    Ok(LogEntry {
        delivery_id: delivery.into(),
        message: ElectoralLogMessage::try_from(message)?,
    })
}

/// The record of a checkpoint published of the log, as Windmill appends it.
fn checkpoint_entry(
    sd: &SigningData,
    checkpoint: &electoral_log::proofs::Checkpoint,
) -> Result<LogEntry> {
    let message = Message::electoral_log_checkpoint_message(
        EventIdString("event".into()),
        ElectoralLogCheckpointV2 {
            log_uid: checkpoint.log_uid.hyphenated().to_string(),
            tree_size: checkpoint.tree_size,
            root: hex::encode(&checkpoint.root),
            reason: ElectoralLogCheckpointReason::Periodic,
        },
        sd,
    )?;
    signed_entry(&format!("checkpoint-{}", checkpoint.tree_size), &message)
}

/// The record that a board continues the log whose last checkpoint is `previous`.
fn continuation_entry(
    sd: &SigningData,
    previous: &electoral_log::proofs::Checkpoint,
) -> Result<LogEntry> {
    let message = Message::electoral_log_continuation_message(
        EventIdString("event".into()),
        ElectoralLogContinuation {
            previous_log_name: previous.log_name.clone(),
            previous_log_uid: previous.log_uid.hyphenated().to_string(),
            tree_size: previous.tree_size,
            root: hex::encode(&previous.root),
        },
        sd,
    )?;
    signed_entry(&format!("continuation-{}", previous.log_uid), &message)
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_PG_* with a role that may create databases"]
async fn exported_logs_import_with_their_roots_and_tampering_is_refused() -> Result<()> {
    let databases = databases()?;
    let tenant = Uuid::new_v4();
    let [source_event, target_event, tampered_event, chained_event, unlinked_event] =
        [(); 5].map(|_| Uuid::new_v4());
    let key = SigningKey::generate()?;
    let sd = SigningData::new(key.clone(), "", key.clone());
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
    // The publication of `early` is recorded in the log.
    source
        .append(
            &source_board,
            &mut std::iter::once(checkpoint_entry(&sd, &early))
                .chain(entries(7..11).into_iter().map(anyhow::Ok)),
        )
        .await?;
    let late = journal.checkpoint(&source_board).await?;
    let (records, mut manifest) = export(&source).await?;
    assert_eq!(records.len(), 12);
    assert_eq!(manifest.logs.len(), 1);
    assert_eq!(manifest.logs[0].checkpoint, late);
    assert_eq!(manifest.logs[0].state, LogState::Open);
    manifest.logs[0].published = vec![sign(&key, &early)?, sign(&key, &late)?];

    let target = databases
        .create_event(&tenant.to_string(), &target_event.to_string())
        .await?;
    // `early` is a checkpoint this environment published itself.
    let imported = import(&target, &manifest, &records, std::slice::from_ref(&early)).await?;
    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0].checkpoint, late);
    assert_eq!(imported[0].verified_checkpoints, 2);
    assert_eq!(imported[0].anchored_checkpoints, 1);
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
    assert!(import(&target, &manifest, &records, &[]).await.is_err());
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
    // The log records the publication of `early`, so the export cannot leave it out.
    let mut unpublished = manifest.clone();
    unpublished.logs[0].published.remove(0);
    let other_anchor = electoral_log::proofs::Checkpoint {
        root: vec![7; 32],
        ..early.clone()
    };
    for (manifest, records, anchors) in [
        (&manifest, &changed, vec![]),
        (&manifest, &reordered, vec![]),
        (&manifest, &missing, vec![]),
        (&forged, &records, vec![]),
        (&other_root, &records, vec![]),
        (&unpublished, &records, vec![]),
        (&manifest, &records, vec![other_anchor]),
    ] {
        assert!(import(&tampered, manifest, records, &anchors)
            .await
            .is_err());
        assert!(
            tampered.logs().await?.is_empty(),
            "a refused import stored a log"
        );
    }

    // An export of the imported event holds the sealed log, then the event's board,
    // which records that it continues the sealed log.
    let target_board = board(&tenant, &target_event);
    target.create_board(&target_board).await?;
    target
        .append(
            &target_board,
            &mut std::iter::once(continuation_entry(&sd, &late))
                .chain(entries(20..22).into_iter().map(anyhow::Ok)),
        )
        .await?;
    let (chain_records, mut chain_manifest) = export(&target).await?;
    assert_eq!(chain_manifest.logs.len(), 2);
    chain_manifest.logs[0].published = manifest.logs[0].published.clone();
    let chained = databases
        .create_event(&tenant.to_string(), &chained_event.to_string())
        .await?;
    let imported = import(
        &chained,
        &chain_manifest,
        &chain_records,
        std::slice::from_ref(&early),
    )
    .await?;
    assert_eq!(imported.len(), 2);
    assert_eq!(imported[0].anchored_checkpoints, 1);

    // A board that does not record the log before it is refused.
    let unlinked = databases
        .create_event(&tenant.to_string(), &unlinked_event.to_string())
        .await?;
    import(&unlinked, &manifest, &records, &[]).await?;
    let unlinked_board = board(&tenant, &unlinked_event);
    unlinked.create_board(&unlinked_board).await?;
    unlinked
        .append(
            &unlinked_board,
            &mut entries(30..32).into_iter().map(anyhow::Ok),
        )
        .await?;
    let (unlinked_records, mut unlinked_manifest) = export(&unlinked).await?;
    unlinked_manifest.logs[0].published = manifest.logs[0].published.clone();
    let error = import(&tampered, &unlinked_manifest, &unlinked_records, &[])
        .await
        .expect_err("a log that continues nothing was imported");
    assert!(
        error
            .to_string()
            .contains("does not record that it continues"),
        "{error:#}"
    );
    assert!(tampered.logs().await?.is_empty());

    for event in [
        source_event,
        target_event,
        tampered_event,
        chained_event,
        unlinked_event,
    ] {
        databases
            .drop_event(&tenant.to_string(), &event.to_string())
            .await?;
    }
    Ok(())
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_PG_* with a role that may create databases"]
async fn upgrading_an_event_database_completes_its_ballot_box() -> Result<()> {
    use electoral_log::adapters::ballot_box_received::{
        ReceiveBallot, ReceiveOutcome, ReceivedSignature,
    };
    let databases = databases()?;
    let (tenant, event, election, area) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4().to_string(),
        Uuid::new_v4().to_string(),
    );
    let event_id = event.to_string();
    let store = databases
        .create_event(&tenant.to_string(), &event_id)
        .await?;
    store.create_ballot_box(&event_id).await?;
    // The ballot box as it was before received ballots and seals.
    let suffix = event.simple();
    store
        .client()
        .await?
        .batch_execute(&format!(
            "ALTER TABLE ballot_box_received DETACH PARTITION ballot_box_received_{suffix};
             DROP TABLE ballot_box_received_{suffix};
             DROP TRIGGER ballot_box_seal_guard_truncate ON ballot_box_ballot_{suffix};
             DROP TRIGGER ballot_box_seal_guard_truncate ON ballot_box_ballot;
             DROP TRIGGER ballot_box_seal_guard ON ballot_box_ballot;
             DROP TABLE ballot_box_seal;"
        ))
        .await?;

    databases.apply_schema(&event_id).await?;
    databases.apply_schema(&event_id).await?;
    let triggers: Vec<String> = store
        .client()
        .await?
        .query(
            "SELECT tgname::text FROM pg_trigger WHERE tgrelid = $1::text::regclass ORDER BY 1",
            &[&format!("ballot_box_ballot_{suffix}")],
        )
        .await?
        .iter()
        .map(|row| row.get(0))
        .collect();
    assert_eq!(
        triggers,
        ["ballot_box_seal_guard", "ballot_box_seal_guard_truncate"]
    );
    assert!(!store.has_ballot_box_seals(&event_id).await?);
    let received = store
        .receive_ballot(
            &ReceiveBallot {
                election_event_id: &event_id,
                election_id: &election,
                area_id: &area,
                voter_id: "voter",
                ballot_hash: "hash",
                content: "content",
                voter_signing_pk: "key",
                voter_ballot_signature: "signature",
            },
            chrono::DateTime::from_timestamp_millis(1_841_396_472_000).unwrap(),
            |_| {
                Ok(ReceivedSignature {
                    ballot_id: "ABCD-EF01".into(),
                    key_id: "key".into(),
                    received_signature: "signed".into(),
                })
            },
        )
        .await?;
    assert!(matches!(received, ReceiveOutcome::Received(_)));
    databases.drop_event(&tenant.to_string(), &event_id).await?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_PG_* with a role that may create databases"]
async fn tenant_logs_are_boards_of_the_base_database() -> Result<()> {
    let databases = databases()?;
    let (tenant, other_tenant) = (Uuid::new_v4(), Uuid::new_v4());
    let board = format!("testtenant{}", tenant.simple());
    let other_board = format!("testtenant{}", other_tenant.simple());
    // An event's board is routed to its event's database; a tenant's is not one.
    let events = BoardClient::new(Arc::new(databases.clone()));
    assert!(events.create_board(&board).await.is_err());

    let logs = BoardClient::new(Arc::new(databases.tenant_logs().await?));
    // Asking again does not create the tables again.
    databases.tenant_logs().await?;
    assert!(!logs.has_board(&board).await?);
    logs.create_board(&board).await?;
    logs.create_board(&board).await?;
    logs.create_board(&other_board).await?;
    logs.append(&board, &entries(0..3)).await?;
    logs.append(&other_board, &entries(0..2)).await?;
    // A delivery stored twice is one record.
    logs.append(&board, &entries(2..3)).await?;

    assert_eq!(logs.count(&board, &LogQuery::default()).await?, 3);
    assert_eq!(logs.count(&other_board, &LogQuery::default()).await?, 2);
    let listed = logs.query(&board, &LogQuery::default()).await?;
    assert_eq!(listed.len(), 3);
    assert_eq!(listed[0].user_id.as_deref(), Some("user-2"));
    // The catalog still lists no event for either tenant.
    assert!(databases.events().await?.iter().all(|event| {
        event.tenant_id != tenant.to_string() && event.tenant_id != other_tenant.to_string()
    }));

    logs.delete_board(&board).await?;
    logs.delete_board(&other_board).await?;
    assert!(!logs.has_board(&board).await?);
    Ok(())
}
