// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Outbox rows → electoral log messages, and matching them on the board.

use super::*;
use chrono::{TimeZone, Utc};
use electoral_log::messages::statement::{StatementBody, StatementType};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use strand::serialization::StrandDeserialize;
use strand::signature::{StrandSignaturePk, StrandSignatureSk};

const STEP: &str = "2b7c9e40-1f5d-4a8e-9c3b-6d2e1f0a9b87";
const POST: &str = "6f1d2c3b-4a5e-4f60-8a7b-9c0d1e2f3a4b";
const COUNTRY: &str = "0a1b2c3d-4e5f-4a6b-8c7d-8e9f0a1b2c3d";
/// 2028-05-08 11:16:05 UTC.
const OCCURRED_AT: u64 = 1_841_397_365;

fn row(event_type: StatementEventType, log_type: StatementLogType) -> SigningLogOutboxRow {
    let user = event_type == StatementEventType::USER;
    SigningLogOutboxRow {
        id: 7,
        tenant_id: Uuid::new_v4(),
        election_event_id: Uuid::parse_str("11111111-2222-4333-8444-555555555555").unwrap(),
        step_id: Uuid::parse_str(STEP).unwrap(),
        statement_kind: SigningStatementKind::SigningRequestSigned,
        event_type,
        log_type,
        user_id: user.then(|| "user-maria".to_string()),
        username: user.then(|| "sbei-madrid-1".to_string()),
        election_id: Some(Uuid::parse_str(POST).unwrap()),
        area_id: Some(Uuid::parse_str(COUNTRY).unwrap()),
        body: json!({
            "description": "Signed 7F3A-91C2 as MARIA L. SANTOS",
            "details": {"code": "7F3A-91C2", "count": 1, "required": 3},
        }),
        occurred_at: Utc.timestamp_opt(OCCURRED_AT as i64, 0).unwrap(),
        posted_at: None,
        attempts: 0,
        last_error: None,
    }
}

struct Keys {
    sender: StrandSignatureSk,
    system: StrandSignatureSk,
}

impl Keys {
    fn new() -> Self {
        Keys {
            sender: StrandSignatureSk::generate().unwrap(),
            system: StrandSignatureSk::generate().unwrap(),
        }
    }

    fn signing_data(&self) -> SigningData {
        SigningData::new(self.sender.clone(), "", self.system.clone())
    }
}

fn entry_of(message: &Message) -> &SigningLogEntry {
    match &message.statement.body {
        StatementBody::Signing(entry) => entry,
        other => panic!("not a signing body: {other:?}"),
    }
}

#[test]
fn a_user_row_is_a_user_entry_of_its_person() {
    let keys = Keys::new();
    let row = row(StatementEventType::USER, StatementLogType::INFO);
    let message = outbox_message(&row, &keys.signing_data()).unwrap();

    let head = &message.statement.head;
    assert!(matches!(head.kind, StatementType::SigningRequestSigned));
    assert_eq!(head.event_type, StatementEventType::USER);
    assert_eq!(head.log_type, StatementLogType::INFO);
    assert_eq!(head.description, "Signed 7F3A-91C2 as MARIA L. SANTOS");
    assert_eq!(head.event.0, "11111111-2222-4333-8444-555555555555");
    // When the step happened, not when the worker posts it.
    assert_eq!(head.timestamp, OCCURRED_AT);
    assert_eq!(message.user_id.as_deref(), Some("user-maria"));
    assert_eq!(message.username.as_deref(), Some("sbei-madrid-1"));
    assert_eq!(message.election_id.as_deref(), Some(POST));
    assert_eq!(message.area_id.as_deref(), Some(COUNTRY));
    // Signed by the sender key given (the admin's, for USER rows).
    assert_eq!(
        message.sender.pk,
        StrandSignaturePk::from_sk(&keys.sender).unwrap()
    );

    let entry = entry_of(&message);
    assert_eq!(entry.kind, SigningStatementKind::SigningRequestSigned);
    assert_eq!(entry.event_type, StatementEventType::USER);
    assert_eq!(entry.log_type, StatementLogType::INFO);
    assert_eq!(entry.step_id, STEP);
    assert_eq!(entry.description, "Signed 7F3A-91C2 as MARIA L. SANTOS");
    let details: Value = serde_json::from_str(&entry.details_json).unwrap();
    assert_eq!(
        details,
        json!({"code": "7F3A-91C2", "count": 1, "required": 3})
    );
}

#[test]
fn a_system_row_names_no_person() {
    let keys = Keys::new();
    let mut row = row(StatementEventType::SYSTEM, StatementLogType::INFO);
    row.body["description"] = json!("Signature verified on 7F3A-91C2: 1 of 3");
    let message = outbox_message(&row, &keys.signing_data()).unwrap();

    let head = &message.statement.head;
    assert!(matches!(head.kind, StatementType::SigningRequestSigned));
    assert_eq!(head.event_type, StatementEventType::SYSTEM);
    assert_eq!(head.log_type, StatementLogType::INFO);
    assert_eq!(head.description, "Signature verified on 7F3A-91C2: 1 of 3");
    assert_eq!(head.timestamp, OCCURRED_AT);
    assert_eq!(message.user_id, None);
    assert_eq!(message.username, None);
    assert_eq!(message.election_id.as_deref(), Some(POST));
    assert_eq!(entry_of(&message).step_id, STEP);
}

/// A user and username on a SYSTEM row (the table refuses them) still
/// don't reach the entry.
#[test]
fn a_system_row_drops_a_stray_user() {
    let keys = Keys::new();
    let mut row = row(StatementEventType::SYSTEM, StatementLogType::INFO);
    row.user_id = Some("user-maria".to_string());
    row.username = Some("sbei-madrid-1".to_string());
    let message = outbox_message(&row, &keys.signing_data()).unwrap();
    assert_eq!(message.user_id, None);
    assert_eq!(message.username, None);
}

#[test]
fn an_error_row_is_an_error_entry() {
    let keys = Keys::new();
    let mut row = row(StatementEventType::SYSTEM, StatementLogType::ERROR);
    row.statement_kind = SigningStatementKind::SigningSignatureRefused;
    row.body["description"] = json!("Refused on 7F3A-91C2: issuer not trusted");
    let message = outbox_message(&row, &keys.signing_data()).unwrap();

    let head = &message.statement.head;
    assert!(matches!(head.kind, StatementType::SigningSignatureRefused));
    assert_eq!(head.event_type, StatementEventType::SYSTEM);
    assert_eq!(head.log_type, StatementLogType::ERROR);
    assert_eq!(head.description, "Refused on 7F3A-91C2: issuer not trusted");
    let entry = entry_of(&message);
    assert_eq!(entry.kind, SigningStatementKind::SigningSignatureRefused);
    assert_eq!(entry.log_type, StatementLogType::ERROR);
}

/// No Post or country: the message has none either.
#[test]
fn an_event_wide_row_has_no_post() {
    let keys = Keys::new();
    let mut row = row(StatementEventType::USER, StatementLogType::INFO);
    row.election_id = None;
    row.area_id = None;
    let message = outbox_message(&row, &keys.signing_data()).unwrap();
    assert_eq!(message.election_id, None);
    assert_eq!(message.area_id, None);
}

/// A malformed body is refused, never posted as an empty entry.
#[test]
fn a_body_without_description_or_details_is_refused() {
    let control = row(StatementEventType::USER, StatementLogType::INFO);
    assert!(outbox_entry(&control).is_ok());

    for body in [
        json!({"details": {}}),
        json!({"description": 3, "details": {}}),
        json!({"description": "Started signing request 7F3A-91C2"}),
        json!("Started signing request 7F3A-91C2"),
    ] {
        let mut row = control.clone();
        row.body = body.clone();
        assert!(outbox_entry(&row).is_err(), "{body}");
    }
}

/// The board row the worker inserts: the statement timestamp is the
/// step's time, and the columns the board filters on are set.
#[test]
fn the_board_row_carries_the_step_time_and_columns() {
    let keys = Keys::new();
    let row = row(StatementEventType::USER, StatementLogType::INFO);
    let board_message = outbox_board_message(&row, &keys.signing_data()).unwrap();
    assert_eq!(board_message.statement_timestamp, OCCURRED_AT as i64);
    assert_eq!(board_message.statement_kind, "SigningRequestSigned");
    assert_eq!(board_message.user_id.as_deref(), Some("user-maria"));
    assert_eq!(board_message.username.as_deref(), Some("sbei-madrid-1"));
    assert_eq!(board_message.election_id.as_deref(), Some(POST));
    assert_eq!(board_message.area_id.as_deref(), Some(COUNTRY));
    // The stored bytes are the signed message.
    let decoded = Message::strand_deserialize(&board_message.message).unwrap();
    assert_eq!(entry_of(&decoded).step_id, STEP);
}

/// The delivery id is the step's entry, whatever the attempt; the payload
/// hash is what the entry says, not its signed bytes.
#[test]
fn a_delivery_is_the_same_for_every_attempt() {
    let keys = Keys::new();
    let user = row(StatementEventType::USER, StatementLogType::INFO);
    let first = outbox_delivery(&user, &keys.signing_data()).unwrap();
    // Expected: SHA-256 of the literal id text.
    assert_eq!(
        first.delivery_id,
        hex::encode(Sha256::digest(
            b"signing:2b7c9e40-1f5d-4a8e-9c3b-6d2e1f0a9b87:USER"
        ))
    );
    assert_eq!(first.row_id, 7);
    // The message is the row's board message.
    let board_message = outbox_board_message(&user, &keys.signing_data()).unwrap();
    assert_eq!(first.message.statement_kind, board_message.statement_kind);
    assert_eq!(
        first.message.statement_timestamp,
        board_message.statement_timestamp
    );
    assert_eq!(first.message.user_id, board_message.user_id);

    // Signed again, with other keys: other bytes, the same delivery.
    let again = outbox_delivery(&user, &Keys::new().signing_data()).unwrap();
    assert_ne!(again.message.message, first.message.message);
    assert_eq!(again.delivery_id, first.delivery_id);
    assert_eq!(again.payload_hash, first.payload_hash);

    // The same details read in another key order.
    let mut reordered = user.clone();
    reordered.body = json!({
        "details": {"required": 3, "count": 1, "code": "7F3A-91C2"},
        "description": "Signed 7F3A-91C2 as MARIA L. SANTOS",
    });
    assert_eq!(payload_hash(&reordered).unwrap(), first.payload_hash);
}

#[test]
fn a_delivery_differs_per_entry_and_content() {
    let user = row(StatementEventType::USER, StatementLogType::INFO);
    let system = row(StatementEventType::SYSTEM, StatementLogType::INFO);
    assert_ne!(
        delivery_id(user.step_id, &user.event_type),
        delivery_id(system.step_id, &system.event_type)
    );
    assert_ne!(
        delivery_id(user.step_id, &user.event_type),
        delivery_id(Uuid::new_v4(), &user.event_type)
    );

    let hash = payload_hash(&user).unwrap();
    let changes: Vec<fn(&mut SigningLogOutboxRow)> = vec![
        |row| row.body["description"] = json!("Signed 7F3A-91C2 as JOSE"),
        |row| row.body["details"]["count"] = json!(2),
        |row| row.log_type = StatementLogType::ERROR,
        |row| row.statement_kind = SigningStatementKind::SigningHandover,
        |row| row.username = Some("sbei-madrid-2".to_string()),
        |row| row.election_id = None,
        |row| row.occurred_at += chrono::Duration::microseconds(1),
    ];
    for change in changes {
        let mut changed = user.clone();
        change(&mut changed);
        assert_ne!(payload_hash(&changed).unwrap(), hash);
    }
}

#[test]
fn a_failing_entry_waits_longer_after_each_failure() {
    let at = Utc.timestamp_opt(OCCURRED_AT as i64, 0).unwrap();
    let wait = |attempts| (retry_after(attempts, at) - at).num_seconds();
    assert_eq!(wait(0), 1);
    assert_eq!(wait(1), 2);
    assert_eq!(wait(5), 32);
    assert_eq!(wait(9), 512);
    assert_eq!(wait(10), 600);
    assert_eq!(wait(1000), 600);
    assert_eq!(wait(-1), 1);
}

/// A connection that logs its calls to `log`.
struct FakeConnection {
    log: Arc<Mutex<Vec<String>>>,
    receipts: Arc<Mutex<HashMap<String, String>>>,
    fail_insert: Option<String>,
}

impl FakeConnection {
    fn note(&self, call: String) {
        self.log.lock().unwrap().push(call);
    }
}

impl BoardConnection for FakeConnection {
    async fn open_session(&mut self, board: &str) -> Result<()> {
        self.note(format!("open {board}"));
        Ok(())
    }

    async fn ensure_receipts(&mut self) -> Result<()> {
        self.note("receipts".to_string());
        Ok(())
    }

    async fn begin(&mut self) -> Result<String> {
        self.note("begin".to_string());
        Ok("tx".to_string())
    }

    async fn insert_delivery(
        &mut self,
        transaction_id: &String,
        delivery: &SigningLogDelivery,
    ) -> Result<bool> {
        self.note(format!("insert {transaction_id} {}", delivery.row_id));
        if self.fail_insert.as_deref() == Some(delivery.delivery_id.as_str()) {
            return Err(anyhow!("board unavailable"));
        }
        let mut receipts = self.receipts.lock().unwrap();
        match receipts.get(&delivery.delivery_id) {
            Some(hash) if hash == &delivery.payload_hash => Ok(false),
            Some(_) => Err(anyhow!("delivery ID reused with different input")),
            None => {
                receipts.insert(delivery.delivery_id.clone(), delivery.payload_hash.clone());
                Ok(true)
            }
        }
    }

    async fn commit(&mut self, transaction_id: &String) -> Result<()> {
        self.note(format!("commit {transaction_id}"));
        Ok(())
    }

    async fn close_session(&mut self) -> Result<()> {
        self.note("close".to_string());
        Ok(())
    }
}

struct FakeBoard {
    board: BoardSigningLog<FakeConnection>,
    log: Arc<Mutex<Vec<String>>>,
    fail_insert: Arc<Mutex<Option<String>>>,
    fail_connect: Arc<AtomicBool>,
}

fn fake_board() -> FakeBoard {
    let log = Arc::new(Mutex::new(vec![]));
    let receipts = Arc::new(Mutex::new(HashMap::new()));
    let fail_insert = Arc::new(Mutex::new(None));
    let fail_connect = Arc::new(AtomicBool::new(false));
    let (l, r, f, c) = (
        log.clone(),
        receipts.clone(),
        fail_insert.clone(),
        fail_connect.clone(),
    );
    let connect: Connect<FakeConnection> = Box::new(move || {
        let (log, receipts, fail_insert, fail_connect) =
            (l.clone(), r.clone(), f.clone(), c.clone());
        Box::pin(async move {
            if fail_connect.load(Ordering::SeqCst) {
                return Err(anyhow!("immudb unreachable"));
            }
            log.lock().unwrap().push("connect".to_string());
            let fail_insert = fail_insert.lock().unwrap().clone();
            Ok(FakeConnection {
                log,
                receipts,
                fail_insert,
            })
        })
    });
    FakeBoard {
        board: BoardSigningLog::new(connect),
        log,
        fail_insert,
        fail_connect,
    }
}

fn deliveries(keys: &Keys) -> Vec<SigningLogDelivery> {
    let mut user = row(StatementEventType::USER, StatementLogType::INFO);
    user.id = 1;
    let mut system = row(StatementEventType::SYSTEM, StatementLogType::INFO);
    system.id = 2;
    [user, system]
        .iter()
        .map(|row| outbox_delivery(row, &keys.signing_data()).unwrap())
        .collect()
}

/// One board transaction per call, through one connection kept across
/// calls; a delivery with a receipt is not written again.
#[tokio::test]
async fn the_board_writes_a_call_in_one_transaction_on_one_connection() {
    let keys = Keys::new();
    let mut fake = fake_board();
    let both = deliveries(&keys);
    let written = fake.board.deliver("event-board", &both).await.unwrap();
    assert_eq!(written, vec![true, true]);
    let written = fake.board.deliver("event-board", &both[1..]).await.unwrap();
    assert_eq!(written, vec![false]);
    assert_eq!(
        *fake.log.lock().unwrap(),
        [
            "connect",
            "open event-board",
            "receipts",
            "begin",
            "insert tx 1",
            "insert tx 2",
            "commit tx",
            "close",
            "open event-board",
            "receipts",
            "begin",
            "insert tx 2",
            "commit tx",
            "close",
        ]
    );
}

/// A failed transaction is not committed, and its connection is dropped:
/// the next call connects again.
#[tokio::test]
async fn a_failed_delivery_drops_the_connection() {
    let keys = Keys::new();
    let mut fake = fake_board();
    let both = deliveries(&keys);
    *fake.fail_insert.lock().unwrap() = Some(both[1].delivery_id.clone());
    let error = fake.board.deliver("event-board", &both).await.unwrap_err();
    assert_eq!(error.to_string(), "board unavailable");
    assert!(!fake
        .log
        .lock()
        .unwrap()
        .iter()
        .any(|call| call.starts_with("commit")));

    *fake.fail_insert.lock().unwrap() = None;
    fake.log.lock().unwrap().clear();
    // The receipt of the first delivery was written by the fake, but the
    // real board rolls it back with the transaction; either way it's one.
    let written = fake.board.deliver("event-board", &both[1..]).await.unwrap();
    assert_eq!(written, vec![true]);
    assert_eq!(fake.log.lock().unwrap()[0], "connect");

    fake.fail_connect.store(true, Ordering::SeqCst);
    // The kept connection is used; no new one is needed.
    assert!(fake.board.deliver("event-board", &both[..1]).await.is_ok());
}

#[tokio::test]
async fn an_unreachable_board_fails_the_delivery() {
    let keys = Keys::new();
    let mut fake = fake_board();
    fake.fail_connect.store(true, Ordering::SeqCst);
    let error = fake
        .board
        .deliver("event-board", &deliveries(&keys))
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "immudb unreachable");
}

/// Keys loaded for the event sign its SYSTEM entries and its people's
/// USER entries; an entry with no key loaded is refused.
#[test]
fn entries_are_signed_with_the_loaded_keys() {
    let user = row(StatementEventType::USER, StatementLogType::INFO);
    let system = row(StatementEventType::SYSTEM, StatementLogType::INFO);
    let mut keys = ElectoralLogKeys::new();
    assert!(keys.signing_data(&user).is_err());
    assert!(keys.signing_data(&system).is_err());

    let (admin, event) = (Keys::new(), Keys::new());
    let electoral_log = |keys: &Keys| ElectoralLog {
        sd: keys.signing_data(),
        elog_database: "board".to_string(),
    };
    keys.system = Some(electoral_log(&event));
    keys.admins
        .insert("user-maria".to_string(), electoral_log(&admin));
    let signed_by = |row: &SigningLogOutboxRow| {
        outbox_message(row, keys.signing_data(row).unwrap())
            .unwrap()
            .sender
            .pk
    };
    assert_eq!(
        signed_by(&user),
        StrandSignaturePk::from_sk(&admin.sender).unwrap()
    );
    assert_eq!(
        signed_by(&system),
        StrandSignaturePk::from_sk(&event.sender).unwrap()
    );
    let mut other = user.clone();
    other.user_id = Some("user-jose".to_string());
    assert!(keys.signing_data(&other).is_err());
}

#[test]
fn a_long_error_is_cut_on_a_character_boundary() {
    let short = anyhow!("board unavailable");
    assert_eq!(error_text(&short), "board unavailable");

    let long = anyhow!("é".repeat(MAX_ERROR_CHARS + 10)).context("insert failed");
    let text = error_text(&long);
    assert_eq!(text.chars().count(), MAX_ERROR_CHARS);
    assert!(text.starts_with("insert failed: é"));
}
