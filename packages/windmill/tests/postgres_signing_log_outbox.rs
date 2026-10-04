// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The signing log outbox worker against the real outbox table, with an
//! in-memory board that keeps delivery receipts as immudb does: entries are
//! posted once each, in order, a failure stops the event until a later
//! pass, and signing steps never wait for the board.

#[path = "support/schema.rs"]
mod schema;

use anyhow::{anyhow, Result};
use deadpool_postgres::{Client, Pool, Transaction};
use electoral_log::messages::message::{Message, SigningData};
use electoral_log::messages::newtypes::SigningStatementKind;
use electoral_log::messages::statement::{StatementBody, StatementEventType};
use electoral_log::ElectoralLogMessage;
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;
use strand::serialization::StrandDeserialize;
use strand::signature::StrandSignatureSk;
use tokio::sync::{oneshot, Mutex};
use uuid::Uuid;
use windmill::postgres::signing::{
    list_signing_log_outbox_events, SigningLogOutboxRow, SigningLogOutboxUser,
};
use windmill::services::signing::log::{
    delivery_id, outbox_snapshot, post_event_outbox, post_outbox_snapshot,
    post_signing_log_outboxes, stage, take_outbox_snapshot, Actor, LogScope, LogStep, OutboxBatch,
    OutboxPass, OutboxSnapshot, PassOutcome, SigningLogBoard, SigningLogDelivery, SigningLogKeys,
    SystemOutcome,
};

#[derive(Clone, Copy)]
struct Scope {
    tenant: Uuid,
    event: Uuid,
}

async fn scope(tx: &Transaction<'_>) -> Scope {
    let s = Scope {
        tenant: Uuid::new_v4(),
        event: Uuid::new_v4(),
    };
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&s.tenant, &format!("tenant-{}", s.tenant)],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
         VALUES ($1, $2, 'RSA256')",
        &[&s.event, &s.tenant],
    )
    .await
    .unwrap();
    s
}

/// Stages one step; returns its step id.
async fn step(tx: &Transaction<'_>, s: Scope, kind: SigningStatementKind, n: u32) -> Uuid {
    stage(
        tx,
        &LogStep {
            kind,
            user: Actor {
                user_id: format!("user-{n}"),
                username: format!("sbei-{n}"),
            },
            system: SystemOutcome::Info,
            scope: LogScope {
                tenant_id: s.tenant,
                election_event_id: s.event,
                election_id: Some(Uuid::new_v4()),
                area_id: None,
            },
            description: format!("Step {n}"),
            details: json!({ "code": "7F3A-91C2", "n": n }),
        },
    )
    .await
    .unwrap()
}

/// One key for every entry.
struct TestKeys {
    sd: SigningData,
    prepared: Vec<String>,
}

impl TestKeys {
    fn new() -> Self {
        let sk = StrandSignatureSk::generate().unwrap();
        TestKeys {
            sd: SigningData::new(sk.clone(), "", sk),
            prepared: vec![],
        }
    }
}

impl SigningLogKeys for TestKeys {
    async fn prepare(
        &mut self,
        _: &mut Client,
        _: Uuid,
        _: Uuid,
        users: &[SigningLogOutboxUser],
    ) -> Result<String> {
        self.prepared
            .extend(users.iter().map(|user| user.user_id.clone()));
        Ok("board".to_string())
    }

    fn signing_data(&self, _: &SigningLogOutboxRow) -> Result<&SigningData> {
        Ok(&self.sd)
    }
}

/// What the board shows, in the order it was posted.
type Posted = Vec<(Uuid, StatementEventType)>;

/// A board in memory with immudb's receipts: a delivery whose id it holds
/// with the same payload is not written again. A call is all or nothing.
/// It fails on request: before writing, or after (the write lands but its
/// answer is lost).
#[derive(Default)]
struct MemoryBoard {
    entries: Vec<ElectoralLogMessage>,
    receipts: HashMap<String, String>,
    inserts: usize,
    refuse: Option<String>,
    lose_answer: Option<String>,
    /// Holds the first call: says it started, then waits to be let go.
    gate: Option<(oneshot::Sender<()>, oneshot::Receiver<()>)>,
}

fn of(step: Uuid, event_type: StatementEventType) -> Option<String> {
    Some(delivery_id(step, &event_type))
}

impl MemoryBoard {
    fn posted(&self) -> Posted {
        self.entries
            .iter()
            .map(|entry| {
                let message = Message::strand_deserialize(&entry.message).unwrap();
                match message.statement.body {
                    StatementBody::Signing(entry) => {
                        (Uuid::parse_str(&entry.step_id).unwrap(), entry.event_type)
                    }
                    other => panic!("not a signing entry: {other:?}"),
                }
            })
            .collect()
    }

    fn has(&self, deliveries: &[SigningLogDelivery], which: &Option<String>) -> bool {
        deliveries
            .iter()
            .any(|delivery| Some(&delivery.delivery_id) == which.as_ref())
    }
}

impl SigningLogBoard for MemoryBoard {
    async fn deliver(&mut self, _: &str, deliveries: &[SigningLogDelivery]) -> Result<Vec<bool>> {
        if let Some((started, release)) = self.gate.take() {
            let _ = started.send(());
            let _ = release.await;
        }
        if self.has(deliveries, &self.refuse) {
            return Err(anyhow!("board unavailable"));
        }
        let mut written = vec![];
        for delivery in deliveries {
            match self.receipts.get(&delivery.delivery_id) {
                Some(hash) if hash == &delivery.payload_hash => written.push(false),
                Some(_) => return Err(anyhow!("delivery ID reused with different input")),
                None => written.push(true),
            }
        }
        for (delivery, new) in deliveries.iter().zip(&written) {
            if *new {
                self.receipts
                    .insert(delivery.delivery_id.clone(), delivery.payload_hash.clone());
                self.entries.push(delivery.message.clone());
                self.inserts += 1;
            }
        }
        if self.has(deliveries, &self.lose_answer) {
            return Err(anyhow!("connection reset"));
        }
        Ok(written)
    }
}

const USER: StatementEventType = StatementEventType::USER;
const SYSTEM: StatementEventType = StatementEventType::SYSTEM;

/// A pass inside the test's transaction: the snapshot, then the entries up
/// to it, one step (two entries) per board transaction.
async fn pass_in(
    tx: &Transaction<'_>,
    s: Scope,
    board: &mut MemoryBoard,
    rows: i64,
) -> Result<Option<OutboxPass>> {
    match outbox_snapshot(tx, s.tenant, s.event, Duration::from_secs(2)).await? {
        OutboxSnapshot::Ready(up_to_id) => {
            let batch = OutboxBatch {
                up_to_id,
                rows,
                per_transaction: 2,
            };
            post_outbox_snapshot(
                tx,
                s.tenant,
                s.event,
                &TestKeys::new(),
                board,
                "board",
                batch,
            )
            .await
        }
        OutboxSnapshot::Empty => Ok(Some(OutboxPass::default())),
        other => Err(anyhow!("snapshot: {other:?}")),
    }
}

/// Lets the event's failed entries be tried again at once.
async fn end_backoff(tx: &Transaction<'_>, s: Scope) {
    tx.execute(
        "UPDATE sequent_backend.signing_log_outbox
         SET last_attempt_at = last_attempt_at - interval '1 day'
         WHERE election_event_id = $1",
        &[&s.event],
    )
    .await
    .unwrap();
}

/// (step, event type, posted, attempts, has an error) per outbox row of
/// the event, in order.
async fn outbox(tx: &Transaction<'_>, s: Scope) -> Vec<(Uuid, String, bool, i32, bool)> {
    tx.query(
        "SELECT step_id, event_type, posted_at IS NOT NULL, attempts, last_error IS NOT NULL
         FROM sequent_backend.signing_log_outbox
         WHERE tenant_id = $1 AND election_event_id = $2 ORDER BY id",
        &[&s.tenant, &s.event],
    )
    .await
    .unwrap()
    .iter()
    .map(|row| (row.get(0), row.get(1), row.get(2), row.get(3), row.get(4)))
    .collect()
}

#[tokio::test]
async fn a_pass_posts_every_entry_once_in_order() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let s = scope(&tx).await;
    let other = scope(&tx).await;
    let first = step(&tx, s, SigningStatementKind::SigningRequestCreated, 1).await;
    step(&tx, other, SigningStatementKind::SigningRuleChanged, 9).await;
    let second = step(&tx, s, SigningStatementKind::SigningRequestSigned, 2).await;

    // Both entries of a step share its time.
    let times: Vec<(Uuid, i64)> = tx
        .query(
            "SELECT step_id, count(DISTINCT occurred_at) FROM sequent_backend.signing_log_outbox
             WHERE election_event_id = $1 GROUP BY step_id",
            &[&s.event],
        )
        .await
        .unwrap()
        .iter()
        .map(|row| (row.get(0), row.get(1)))
        .collect();
    assert!(times.iter().all(|(_, times)| *times == 1), "{times:?}");

    let mut board = MemoryBoard::default();
    let pass = pass_in(&tx, s, &mut board, 200).await.unwrap();
    assert_eq!(
        pass,
        Some(OutboxPass {
            posted: 4,
            already_on_board: 0,
            failed: None,
        })
    );
    assert_eq!(
        board.posted(),
        vec![
            (first, USER),
            (first, SYSTEM),
            (second, USER),
            (second, SYSTEM)
        ]
    );
    assert!(outbox(&tx, s).await.iter().all(|row| row.2 && row.3 == 0));
    // Another event's entries wait for its own pass.
    assert!(outbox(&tx, other).await.iter().all(|row| !row.2));
    let events = list_signing_log_outbox_events(&tx).await.unwrap();
    assert!(events.contains(&(other.tenant, other.event)));
    assert!(!events.contains(&(s.tenant, s.event)));

    // Nothing left to post.
    let again = pass_in(&tx, s, &mut board, 200).await.unwrap();
    assert_eq!(again, Some(OutboxPass::default()));
    assert_eq!(board.inserts, 4);
}

#[tokio::test]
async fn a_pass_posts_at_most_its_limit() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let s = scope(&tx).await;
    let first = step(&tx, s, SigningStatementKind::SigningRequestCreated, 1).await;
    let second = step(&tx, s, SigningStatementKind::SigningRequestSigned, 2).await;

    let mut board = MemoryBoard::default();
    let pass = pass_in(&tx, s, &mut board, 3).await.unwrap().unwrap();
    assert_eq!(pass.posted, 3);
    assert_eq!(
        board.posted(),
        vec![(first, USER), (first, SYSTEM), (second, USER)]
    );
    let pass = pass_in(&tx, s, &mut board, 3).await.unwrap().unwrap();
    assert_eq!(pass.posted, 1);
    assert_eq!(board.posted().last(), Some(&(second, SYSTEM)));
}

/// Entries queued after the snapshot wait for the next pass, even if they
/// are committed before the pass reaches them.
#[tokio::test]
async fn a_pass_posts_up_to_its_snapshot() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let s = scope(&tx).await;
    let first = step(&tx, s, SigningStatementKind::SigningRequestCreated, 1).await;
    let OutboxSnapshot::Ready(up_to_id) =
        outbox_snapshot(&tx, s.tenant, s.event, Duration::from_secs(2))
            .await
            .unwrap()
    else {
        panic!("no snapshot");
    };
    let later = step(&tx, s, SigningStatementKind::SigningRequestSigned, 2).await;

    let mut board = MemoryBoard::default();
    let batch = OutboxBatch {
        up_to_id,
        rows: 200,
        per_transaction: 16,
    };
    let keys = TestKeys::new();
    let pass = post_outbox_snapshot(&tx, s.tenant, s.event, &keys, &mut board, "b", batch)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pass.posted, 2);
    assert_eq!(board.posted(), vec![(first, USER), (first, SYSTEM)]);
    pass_in(&tx, s, &mut board, 200).await.unwrap();
    assert_eq!(
        board.posted(),
        vec![
            (first, USER),
            (first, SYSTEM),
            (later, USER),
            (later, SYSTEM)
        ]
    );
}

/// A failed board transaction stops the event: the entries after it wait,
/// so the board keeps the order. Its first entry is marked failed, and the
/// event waits out a backoff before the next pass goes on from there.
#[tokio::test]
async fn a_failure_stops_the_event_until_the_next_pass() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let s = scope(&tx).await;
    let first = step(&tx, s, SigningStatementKind::SigningRequestCreated, 1).await;
    let second = step(&tx, s, SigningStatementKind::SigningRequestSigned, 2).await;

    let mut board = MemoryBoard {
        refuse: of(second, SYSTEM),
        ..Default::default()
    };
    let pass = pass_in(&tx, s, &mut board, 200).await.unwrap().unwrap();
    assert_eq!(pass.posted, 2);
    assert!(pass.failed.is_some());
    assert_eq!(board.posted(), vec![(first, USER), (first, SYSTEM)]);
    let rows = outbox(&tx, s).await;
    assert_eq!(
        rows.iter()
            .map(|row| (row.2, row.3, row.4))
            .collect::<Vec<_>>(),
        vec![
            (true, 0, false),
            (true, 0, false),
            (false, 1, true),
            (false, 0, false)
        ]
    );

    // Too soon: the event waits.
    let snapshot = outbox_snapshot(&tx, s.tenant, s.event, Duration::from_secs(2))
        .await
        .unwrap();
    assert!(
        matches!(snapshot, OutboxSnapshot::Backoff(_)),
        "{snapshot:?}"
    );

    // Still refused: nothing after it goes.
    end_backoff(&tx, s).await;
    pass_in(&tx, s, &mut board, 200).await.unwrap();
    assert_eq!(board.posted().len(), 2);
    assert_eq!(outbox(&tx, s).await[2].3, 2);

    board.refuse = None;
    end_backoff(&tx, s).await;
    let pass = pass_in(&tx, s, &mut board, 200).await.unwrap().unwrap();
    assert_eq!(pass.posted, 2);
    assert_eq!(pass.failed, None);
    assert_eq!(
        board.posted(),
        vec![
            (first, USER),
            (first, SYSTEM),
            (second, USER),
            (second, SYSTEM)
        ]
    );
    // Posted entries keep no error.
    assert!(outbox(&tx, s).await.iter().all(|row| row.2 && !row.4));
}

/// The write landed but its answer was lost: the retry finds the receipts
/// and marks the entries posted without writing them again.
#[tokio::test]
async fn a_retry_after_a_lost_answer_posts_no_duplicate() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let s = scope(&tx).await;
    let first = step(&tx, s, SigningStatementKind::SigningHandover, 1).await;
    let second = step(&tx, s, SigningStatementKind::SigningRequestSigned, 2).await;

    let mut board = MemoryBoard {
        lose_answer: of(first, SYSTEM),
        ..Default::default()
    };
    let pass = pass_in(&tx, s, &mut board, 200).await.unwrap().unwrap();
    assert_eq!(pass.posted, 0);
    assert!(pass.failed.is_some());
    assert_eq!(board.posted(), vec![(first, USER), (first, SYSTEM)]);

    board.lose_answer = None;
    end_backoff(&tx, s).await;
    let pass = pass_in(&tx, s, &mut board, 200).await.unwrap().unwrap();
    assert_eq!(
        pass,
        OutboxPass {
            posted: 4,
            already_on_board: 2,
            failed: None,
        }
    );
    assert_eq!(
        board.posted(),
        vec![
            (first, USER),
            (first, SYSTEM),
            (second, USER),
            (second, SYSTEM)
        ]
    );
    assert_eq!(board.inserts, 4);
    assert!(outbox(&tx, s).await.iter().all(|row| row.2));
}

/// The pass posted but its transaction never committed (the worker died):
/// the rows are unposted again, and the next pass writes nothing twice.
#[tokio::test]
async fn a_retry_after_a_lost_commit_posts_no_duplicate() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let s = scope(&tx).await;
    let first = step(&tx, s, SigningStatementKind::SigningRequestCreated, 1).await;

    let mut board = MemoryBoard::default();
    {
        let lost = tx.transaction().await.unwrap();
        pass_in(&lost, s, &mut board, 200).await.unwrap();
        lost.rollback().await.unwrap();
    }
    assert!(outbox(&tx, s).await.iter().all(|row| !row.2));
    let second = step(&tx, s, SigningStatementKind::SigningRequestSigned, 2).await;

    let pass = pass_in(&tx, s, &mut board, 200).await.unwrap().unwrap();
    assert_eq!(pass.already_on_board, 2);
    assert_eq!(pass.posted, 4);
    assert_eq!(
        board.posted(),
        vec![
            (first, USER),
            (first, SYSTEM),
            (second, USER),
            (second, SYSTEM)
        ]
    );
    assert!(outbox(&tx, s).await.iter().all(|row| row.2));
}

// Tests that commit. The full pass sees every committed event, so they take
// turns.

static COMMITTED: Mutex<()> = Mutex::const_new(());

async fn remove(pool: &Pool, scopes: &[Scope]) {
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    for s in scopes {
        tx.execute(
            "DELETE FROM sequent_backend.election_event WHERE id = $1",
            &[&s.event],
        )
        .await
        .unwrap();
        tx.execute(
            "DELETE FROM sequent_backend.tenant WHERE id = $1",
            &[&s.tenant],
        )
        .await
        .unwrap();
    }
    tx.commit().await.unwrap();
}

/// `n` events with one committed step each.
async fn committed_scopes(pool: &Pool, n: usize) -> (Vec<Scope>, Vec<Uuid>) {
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let mut scopes = vec![];
    let mut steps = vec![];
    for i in 0..n {
        let s = scope(&tx).await;
        steps.push(
            step(
                &tx,
                s,
                SigningStatementKind::SigningRequestCreated,
                i as u32,
            )
            .await,
        );
        scopes.push(s);
    }
    tx.commit().await.unwrap();
    (scopes, steps)
}

async fn committed_outbox(pool: &Pool, s: Scope) -> Vec<(Uuid, String, bool, i32, bool)> {
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    outbox(&tx, s).await
}

/// An event is skipped while another worker holds it, and the full pass
/// visits every event, each on its own, past an event that fails, through
/// one board.
#[tokio::test]
async fn one_worker_per_event_and_every_event_per_pass() {
    let _turn = COMMITTED.lock().await;
    let pool = schema::pool().await;
    let (scopes, steps) = committed_scopes(&pool, 3).await;
    let (busy, free, broken) = (scopes[0], scopes[1], scopes[2]);

    // A worker holds `busy` (its lock lasts until its transaction ends).
    let mut holder = pool.get().await.unwrap();
    let holding = holder.transaction().await.unwrap();
    let batch = OutboxBatch {
        up_to_id: i64::MAX,
        rows: 200,
        per_transaction: 16,
    };
    let keys = TestKeys::new();
    let mut first = MemoryBoard::default();
    post_outbox_snapshot(
        &holding,
        busy.tenant,
        busy.event,
        &keys,
        &mut first,
        "b",
        batch,
    )
    .await
    .unwrap()
    .unwrap();

    let mut worker = pool.get().await.unwrap();
    let mut board = MemoryBoard {
        refuse: of(steps[2], USER),
        ..Default::default()
    };
    let passes = post_signing_log_outboxes(
        &mut worker,
        &mut board,
        |_, _| TestKeys::new(),
        Duration::from_secs(30),
    )
    .await
    .unwrap();
    let outcome = |s: Scope| {
        passes
            .iter()
            .find(|(_, event, _)| *event == s.event)
            .map(|(_, _, outcome)| outcome.as_ref().unwrap().clone())
            .unwrap_or_else(|| panic!("{} not visited", s.event))
    };
    assert_eq!(outcome(busy), PassOutcome::Busy);
    let PassOutcome::Posted(posted) = outcome(free) else {
        panic!("free not posted");
    };
    assert_eq!(posted.posted, 2);
    let PassOutcome::Posted(failed) = outcome(broken) else {
        panic!("broken not tried");
    };
    assert_eq!((failed.posted, failed.failed.is_some()), (0, true));
    // One board for the whole pass.
    assert_eq!(board.posted(), vec![(steps[1], USER), (steps[1], SYSTEM)]);
    holding.rollback().await.unwrap();

    // Each event's pass committed on its own.
    assert!(committed_outbox(&pool, free).await.iter().all(|row| row.2));
    let broken_rows = committed_outbox(&pool, broken).await;
    assert_eq!((broken_rows[0].2, broken_rows[0].3), (false, 1));
    assert!(committed_outbox(&pool, busy)
        .await
        .iter()
        .all(|row| !row.2 && row.3 == 0));
    remove(&pool, &scopes).await;
}

/// A signing step of the event commits while the worker is posting (the
/// board holds the first call), without waiting for it. Its entries, queued
/// after the pass's snapshot, go in the next pass, after the earlier ones.
#[tokio::test]
async fn a_signing_step_does_not_wait_for_the_worker() {
    let _turn = COMMITTED.lock().await;
    let pool = schema::pool().await;
    let (scopes, steps) = committed_scopes(&pool, 1).await;
    let (s, first) = (scopes[0], steps[0]);

    let (started, started_rx) = oneshot::channel();
    let (release, release_rx) = oneshot::channel();
    let mut board = MemoryBoard {
        gate: Some((started, release_rx)),
        ..Default::default()
    };
    let mut keys = TestKeys::new();
    let mut worker = pool.get().await.unwrap();
    let mut stepper = pool.get().await.unwrap();

    let (pass, later) = tokio::join!(
        post_event_outbox(&mut worker, s.tenant, s.event, &mut keys, &mut board),
        async {
            started_rx.await.unwrap();
            let later = tokio::time::timeout(Duration::from_secs(10), async {
                let tx = stepper.transaction().await.unwrap();
                let later = step(&tx, s, SigningStatementKind::SigningRequestSigned, 2).await;
                tx.commit().await.unwrap();
                later
            })
            .await
            .expect("the signing step waited for the worker");
            release.send(()).unwrap();
            later
        }
    );
    let PassOutcome::Posted(pass) = pass.unwrap() else {
        panic!("not posted");
    };
    assert_eq!(pass.posted, 2);
    assert_eq!(board.posted(), vec![(first, USER), (first, SYSTEM)]);
    // The keys were loaded for the snapshot's people only.
    assert_eq!(keys.prepared, vec!["user-0".to_string()]);

    let PassOutcome::Posted(pass) =
        post_event_outbox(&mut worker, s.tenant, s.event, &mut keys, &mut board)
            .await
            .unwrap()
    else {
        panic!("not posted");
    };
    assert_eq!(pass.posted, 2);
    assert_eq!(
        board.posted(),
        vec![
            (first, USER),
            (first, SYSTEM),
            (later, USER),
            (later, SYSTEM)
        ]
    );
    assert_eq!(board.inserts, 4);
    assert_eq!(
        post_event_outbox(&mut worker, s.tenant, s.event, &mut keys, &mut board)
            .await
            .unwrap(),
        PassOutcome::Empty
    );
    remove(&pool, &scopes).await;
}

/// The snapshot waits for a signing step being written, and includes its
/// entries once it commits; past the lock timeout it leaves the event.
#[tokio::test]
async fn the_snapshot_waits_for_a_step_being_written() {
    let _turn = COMMITTED.lock().await;
    let pool = schema::pool().await;
    let (scopes, _) = committed_scopes(&pool, 1).await;
    let s = scopes[0];

    let mut writer = pool.get().await.unwrap();
    let writing = writer.transaction().await.unwrap();
    step(&writing, s, SigningStatementKind::SigningRequestSigned, 2).await;
    let last: i64 = writing
        .query_one(
            "SELECT max(id) FROM sequent_backend.signing_log_outbox WHERE election_event_id = $1",
            &[&s.event],
        )
        .await
        .unwrap()
        .get(0);

    let mut worker = pool.get().await.unwrap();
    // Past the lock timeout: busy.
    assert_eq!(
        take_outbox_snapshot(&mut worker, s.tenant, s.event, Duration::from_millis(100))
            .await
            .unwrap(),
        OutboxSnapshot::Busy
    );

    let snapshot = take_outbox_snapshot(&mut worker, s.tenant, s.event, Duration::from_secs(10));
    tokio::pin!(snapshot);
    assert!(
        tokio::time::timeout(Duration::from_millis(300), &mut snapshot)
            .await
            .is_err(),
        "the snapshot didn't wait for the step"
    );
    writing.commit().await.unwrap();
    assert_eq!(snapshot.await.unwrap(), OutboxSnapshot::Ready(last));
    remove(&pool, &scopes).await;
}
