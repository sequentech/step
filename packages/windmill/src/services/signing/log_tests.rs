// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Outbox rows → electoral log messages, and matching them on the board.

use super::*;
use chrono::{TimeZone, Utc};
use electoral_log::messages::statement::{StatementBody, StatementType};
use std::sync::{Arc, Mutex};
use voting_crypto::signing::{SigningKey, VerificationKey};
use voting_crypto::RecordDeserialize;

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
    sender: SigningKey,
    system: SigningKey,
}

impl Keys {
    fn new() -> Self {
        Keys {
            sender: SigningKey::generate().unwrap(),
            system: SigningKey::generate().unwrap(),
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
        VerificationKey::from_sk(&keys.sender).unwrap()
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
    let decoded = Message::record_deserialize(&board_message.message).unwrap();
    assert_eq!(entry_of(&decoded).step_id, STEP);
}

/// The delivery id is the step's entry, whatever the attempt; the payload
/// hash is what the entry says, not its signed bytes.
#[test]
fn a_delivery_is_the_same_for_every_attempt() {
    let keys = Keys::new();
    let user = row(StatementEventType::USER, StatementLogType::INFO);
    let first = outbox_delivery(&user, &keys.signing_data()).unwrap();
    assert_eq!(
        first.delivery_id,
        "signing:2b7c9e40-1f5d-4a8e-9c3b-6d2e1f0a9b87:USER"
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
        |row| row.occurred_at += chrono::Duration::seconds(1),
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

/// What the board holds says what its row says, whatever key signed it; a
/// different row's entry doesn't.
#[test]
fn the_board_entry_of_a_row_says_what_the_row_says() {
    for event_type in [StatementEventType::USER, StatementEventType::SYSTEM] {
        let row = row(event_type, StatementLogType::INFO);
        let held = outbox_board_message(&row, &Keys::new().signing_data()).unwrap();
        assert_eq!(
            stored_payload_hash(&held).unwrap(),
            payload_hash(&row).unwrap()
        );
    }
    let user = row(StatementEventType::USER, StatementLogType::INFO);
    let mut other = user.clone();
    other.body["details"]["count"] = json!(2);
    let held = outbox_board_message(&other, &Keys::new().signing_data()).unwrap();
    assert_ne!(
        stored_payload_hash(&held).unwrap(),
        payload_hash(&user).unwrap()
    );
}

/// A store that logs its calls and keeps the entries it appended.
#[derive(Default)]
struct FakeStore {
    log: Arc<Mutex<Vec<String>>>,
    entries: Arc<Mutex<Vec<LogEntry>>>,
    unreachable: bool,
}

impl SigningLogStore for FakeStore {
    async fn append(&self, board: &str, entries: &[LogEntry]) -> Result<Vec<StoredRecord>> {
        self.log
            .lock()
            .unwrap()
            .push(format!("append {board} {}", entries.len()));
        if self.unreachable {
            return Err(anyhow!("board unreachable"));
        }
        let mut held = self.entries.lock().unwrap();
        let mut stored = vec![];
        for entry in entries {
            if held.iter().any(|old| old.delivery_id == entry.delivery_id) {
                continue;
            }
            held.push(entry.clone());
            stored.push(StoredRecord {
                delivery_id: entry.delivery_id.clone(),
                id: held.len() as i64,
                log_uid: Uuid::nil(),
                leaf_index: held.len() as i64 - 1,
            });
        }
        Ok(stored)
    }

    async fn delivered(
        &self,
        board: &str,
        delivery_id: &str,
    ) -> Result<Option<ElectoralLogMessage>> {
        self.log
            .lock()
            .unwrap()
            .push(format!("delivered {board} {delivery_id}"));
        Ok(self
            .entries
            .lock()
            .unwrap()
            .iter()
            .find(|entry| entry.delivery_id == delivery_id)
            .map(|entry| entry.message.clone()))
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

/// One append per call; a delivery the board holds is not written again,
/// once it is found to say the same.
#[tokio::test]
async fn the_board_appends_a_call_at_once_and_checks_what_it_held() {
    let store = FakeStore::default();
    let (log, entries) = (store.log.clone(), store.entries.clone());
    let mut board = BoardSigningLog::new(store);
    let both = deliveries(&Keys::new());
    let written = board.deliver("event-board", &both).await.unwrap();
    assert_eq!(written, vec![true, true]);
    // Signed again with other keys, as after a lost answer.
    let again = deliveries(&Keys::new());
    let written = board.deliver("event-board", &again[1..]).await.unwrap();
    assert_eq!(written, vec![false]);
    assert_eq!(entries.lock().unwrap().len(), 2);
    assert_eq!(
        *log.lock().unwrap(),
        [
            "append event-board 2".to_string(),
            "append event-board 1".to_string(),
            format!("delivered event-board {}", both[1].delivery_id),
        ]
    );
}

/// The board holds the delivery id with other content: the call fails,
/// naming the row.
#[tokio::test]
async fn a_delivery_the_board_holds_with_other_content_fails() {
    let keys = Keys::new();
    let store = FakeStore::default();
    let entries = store.entries.clone();
    let mut board = BoardSigningLog::new(store);
    let both = deliveries(&keys);
    let mut changed = row(StatementEventType::SYSTEM, StatementLogType::ERROR);
    changed.id = 2;
    entries.lock().unwrap().push(LogEntry {
        delivery_id: both[1].delivery_id.clone(),
        message: outbox_board_message(&changed, &keys.signing_data()).unwrap(),
    });

    let error = board.deliver("event-board", &both).await.unwrap_err();
    assert_eq!(
        error.downcast_ref::<ReusedDelivery>(),
        Some(&ReusedDelivery {
            row_id: 2,
            delivery_id: both[1].delivery_id.clone(),
        })
    );
}

#[tokio::test]
async fn an_unreachable_board_fails_the_delivery() {
    let store = FakeStore {
        unreachable: true,
        ..Default::default()
    };
    let mut board = BoardSigningLog::new(store);
    let error = board
        .deliver("event-board", &deliveries(&Keys::new()))
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "board unreachable");
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
        VerificationKey::from_sk(&admin.sender).unwrap()
    );
    assert_eq!(
        signed_by(&system),
        VerificationKey::from_sk(&event.sender).unwrap()
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
