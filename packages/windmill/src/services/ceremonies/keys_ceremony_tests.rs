// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::adapters::memory::clock::{FixedClock, SequentialIds};
use crate::adapters::memory::keys_ceremony::{
    BoardCall, BoardConfiguration, MemoryKeysBoard, MemoryKeysCeremonyAudit,
    MemoryKeysCeremonyStore, MemoryKeysCeremonyTasks, PostedMessage, QueuedTask, StoreCall,
};
use crate::domain::keys_ceremony::KeysBoardStatement;
use chrono::{DateTime, Local, TimeZone};
use sequent_core::types::ceremonies::Log;
use sequent_core::types::hasura::core::{Election, ElectionEvent};
use serde_json::json;

const TENANT: &str = "tenant";
const EVENT: &str = "election-event";
const CEREMONY: &str = "keys-ceremony";
const NEW_CEREMONY: &str = "00000000-0000-0000-0000-000000000001";
const CEREMONY_NAME: &str = "Keys";
const ELECTION: &str = "election-1";
const OTHER_ELECTION: &str = "election-2";
const TRUSTEE: &str = "trustee1";
const OTHER_TRUSTEE: &str = "trustee2";
const THIRD_TRUSTEE: &str = "trustee3";
const EVENT_BOARD: &str = "event-board";
const ELECTION_PUBLIC_KEY: &str = "election-public-key";
const USER_ID: &str = "admin-id";
const USERNAME: &str = "admin";
const LABEL: &str = "label-a";
const OTHER_LABEL: &str = "label-b";

fn now() -> DateTime<Local> {
    Local
        .timestamp_opt(1_700_000_000, 0)
        .single()
        .expect("valid time")
}

fn last_updated() -> DateTime<Local> {
    Local
        .timestamp_opt(1_699_990_000, 0)
        .single()
        .expect("valid time")
}

fn public_key_of(trustee_name: &str) -> String {
    format!("{trustee_name}-public-key")
}

fn private_key_of(trustee_name: &str) -> String {
    format!("{trustee_name}-private-key")
}

fn trustee(name: &str) -> TrusteeRecord {
    serde_json::from_value(json!({
        "id": format!("{name}-id"),
        "name": name,
        "public_key": public_key_of(name),
        "tenant_id": TENANT,
    }))
    .expect("trustee")
}

fn election_event(bulletin_board_reference: Option<Value>) -> ElectionEvent {
    serde_json::from_value(json!({
        "id": EVENT,
        "tenant_id": TENANT,
        "bulletin_board_reference": bulletin_board_reference,
        "is_archived": false,
        "encryption_protocol": "RSA256",
    }))
    .expect("election event")
}

fn event_board_reference() -> Option<Value> {
    Some(json!({"id": 1, "database_name": EVENT_BOARD, "is_archived": false}))
}

fn election(id: &str, keys_ceremony_id: Option<&str>, permission_label: Option<&str>) -> Election {
    serde_json::from_value(json!({
        "id": id,
        "tenant_id": TENANT,
        "election_event_id": EVENT,
        "keys_ceremony_id": keys_ceremony_id,
        "permission_label": permission_label,
    }))
    .expect("election")
}

/// A default ceremony of the given trustees, last updated at `last_updated`.
fn keys_ceremony(
    execution_status: KeysCeremonyExecutionStatus,
    trustees: &[(&str, TrusteeStatus)],
) -> KeysCeremony {
    let status = KeysCeremonyStatus {
        stop_date: Some("1699990000000".to_string()),
        public_key: None,
        logs: vec![Log {
            created_date: "2023-11-14T19:00:00+00:00".to_string(),
            log_text: "Created Keys Ceremony".to_string(),
        }],
        trustees: trustees
            .iter()
            .map(|(name, status)| Trustee {
                name: name.to_string(),
                status: status.clone(),
            })
            .collect(),
    };
    KeysCeremony {
        id: CEREMONY.to_string(),
        created_at: None,
        last_updated_at: Some(last_updated()),
        tenant_id: TENANT.to_string(),
        election_event_id: EVENT.to_string(),
        trustee_ids: trustees
            .iter()
            .map(|(name, _)| format!("{name}-id"))
            .collect(),
        status: Some(serde_json::to_value(status).expect("serializable status")),
        execution_status: Some(execution_status.to_string()),
        labels: None,
        annotations: None,
        threshold: 2,
        name: None,
        settings: None,
        is_default: Some(true),
        permission_label: None,
    }
}

fn in_progress(trustee_status: TrusteeStatus, other_trustee_status: TrusteeStatus) -> KeysCeremony {
    keys_ceremony(
        KeysCeremonyExecutionStatus::IN_PROGRESS,
        &[
            (TRUSTEE, trustee_status),
            (OTHER_TRUSTEE, other_trustee_status),
        ],
    )
}

fn with_status(
    mut keys_ceremony: KeysCeremony,
    change: impl FnOnce(&mut KeysCeremonyStatus),
) -> KeysCeremony {
    let mut status = keys_ceremony.status().expect("readable status");
    change(&mut status);
    keys_ceremony.status = Some(serde_json::to_value(status).expect("serializable status"));
    keys_ceremony
}

fn with_policy(keys_ceremony: KeysCeremony, policy: CeremoniesPolicy) -> KeysCeremony {
    KeysCeremony {
        settings: Some(json!({"policy": policy.to_string()})),
        ..keys_ceremony
    }
}

fn statuses(status: &KeysCeremonyStatus) -> Vec<(String, TrusteeStatus)> {
    status
        .trustees
        .iter()
        .map(|trustee| (trustee.name.clone(), trustee.status.clone()))
        .collect()
}

fn logs(status: &KeysCeremonyStatus) -> Vec<(String, String)> {
    status
        .logs
        .iter()
        .map(|log| (log.created_date.clone(), log.log_text.clone()))
        .collect()
}

fn is_download_unavailable(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<PrivateKeyDownloadUnavailable>()
        .is_some()
}

fn request(trustee: Option<&str>) -> TrusteeKeyRequest<'static> {
    TrusteeKeyRequest {
        trustee: trustee.map(str::to_string),
        tenant_id: TENANT,
        election_event_id: EVENT,
        keys_ceremony_id: CEREMONY,
    }
}

fn creation(
    election_id: Option<&str>,
    threshold: usize,
    trustee_names: &[&str],
) -> KeysCeremonyRequest<'static> {
    KeysCeremonyRequest {
        tenant_id: TENANT,
        user_id: USER_ID,
        username: USERNAME,
        election_event_id: EVENT,
        threshold,
        trustee_names: trustee_names.iter().map(|name| name.to_string()).collect(),
        election_id: election_id.map(str::to_string),
        name: Some(CEREMONY_NAME.to_string()),
        policy: CeremoniesPolicy::MANUAL_CEREMONIES,
    }
}

struct Fixture {
    store: MemoryKeysCeremonyStore,
    board: MemoryKeysBoard,
    audit: MemoryKeysCeremonyAudit,
    tasks: MemoryKeysCeremonyTasks,
    clock: FixedClock,
}

impl Fixture {
    /// An election event with its own board, two elections and two trustees,
    /// whose private keys are on the event board.
    fn new() -> Self {
        let fixture = Fixture {
            store: MemoryKeysCeremonyStore::default(),
            board: MemoryKeysBoard::default(),
            audit: MemoryKeysCeremonyAudit::default(),
            tasks: MemoryKeysCeremonyTasks::default(),
            clock: FixedClock::at(now()),
        };
        {
            let mut tables = fixture.store.tables();
            tables
                .election_events
                .push(election_event(event_board_reference()));
            tables.trustees = vec![trustee(TRUSTEE), trustee(OTHER_TRUSTEE)];
            tables.elections = vec![
                election(ELECTION, None, Some(LABEL)),
                election(OTHER_ELECTION, None, Some(LABEL)),
            ];
        }
        for name in [TRUSTEE, OTHER_TRUSTEE] {
            fixture.board.boards().private_keys.insert(
                (EVENT_BOARD.to_string(), public_key_of(name)),
                private_key_of(name),
            );
        }
        fixture
    }

    fn with_ceremony(self, keys_ceremony: KeysCeremony) -> Self {
        self.store.tables().keys_ceremonies.push(keys_ceremony);
        self
    }

    fn ceremony(&self, id: &str) -> KeysCeremony {
        self.store
            .tables()
            .keys_ceremonies
            .iter()
            .find(|keys_ceremony| keys_ceremony.id == id)
            .cloned()
            .expect("stored ceremony")
    }

    fn status(&self) -> KeysCeremonyStatus {
        self.ceremony(CEREMONY).status().expect("readable status")
    }

    fn execution_status(&self) -> String {
        self.ceremony(CEREMONY)
            .execution_status
            .expect("execution status")
    }

    fn nothing_written(&self) -> bool {
        self.store.tables().status_updates.is_empty()
    }

    fn nothing_created(&self) -> bool {
        let tables = self.store.tables();
        tables
            .keys_ceremonies
            .iter()
            .all(|keys_ceremony| keys_ceremony.id != NEW_CEREMONY)
            && tables
                .elections
                .iter()
                .all(|election| election.keys_ceremony_id.is_none())
    }

    async fn board_of(&self, keys_ceremony: &KeysCeremony) -> Result<(String, Option<String>)> {
        keys_ceremony_board(&self.store, &self.board, TENANT, EVENT, keys_ceremony).await
    }

    async fn download(&self, trustee: Option<&str>) -> Result<String> {
        download_private_key(&self.store, &self.board, &self.clock, request(trustee)).await
    }

    async fn check(&self, trustee: Option<&str>, private_key: &str) -> Result<bool> {
        check_trustee_private_key(
            &self.store,
            &self.board,
            &self.clock,
            request(trustee),
            private_key,
        )
        .await
    }

    async fn create(&self, request: KeysCeremonyRequest<'_>) -> Result<String> {
        start_keys_ceremony(
            &self.store,
            &self.audit,
            &self.clock,
            &SequentialIds::default(),
            request,
        )
        .await
    }

    async fn allows(
        &self,
        election_id: Option<&str>,
        user_permission_labels: Option<&str>,
    ) -> Result<bool> {
        has_election_permission_labels(
            &self.store,
            TENANT,
            EVENT,
            election_id.map(str::to_string),
            user_permission_labels.map(str::to_string),
        )
        .await
    }

    async fn generate_keys(&self) -> Result<KeysCeremonyUpdate> {
        generate_keys(&self.store, &self.board, TENANT, EVENT, CEREMONY).await
    }

    async fn record_public_key(&self) -> Result<KeysCeremonyUpdate> {
        record_public_key(
            &self.store,
            &self.board,
            &self.clock,
            TENANT,
            EVENT,
            CEREMONY,
        )
        .await
    }

    async fn dispatch(&self) -> Result<()> {
        dispatch_keys_ceremony_tasks(&self.store, &self.tasks, TENANT, EVENT).await
    }
}

fn not_default(keys_ceremony: KeysCeremony) -> KeysCeremony {
    KeysCeremony {
        is_default: Some(false),
        ..keys_ceremony
    }
}

fn assign(fixture: &Fixture, election_id: &str, keys_ceremony_id: &str) {
    for election in fixture.store.tables().elections.iter_mut() {
        if election.id == election_id {
            election.keys_ceremony_id = Some(keys_ceremony_id.to_string());
        }
    }
}

#[tokio::test]
async fn the_default_ceremony_uses_the_election_event_board() {
    let fixture = Fixture::new();

    let board = fixture
        .board_of(&in_progress(
            TrusteeStatus::KEY_GENERATED,
            TrusteeStatus::KEY_GENERATED,
        ))
        .await
        .unwrap();

    assert_eq!(board, (EVENT_BOARD.to_string(), None));
}

#[tokio::test]
async fn a_ceremony_that_does_not_say_whether_it_is_the_default_uses_the_election_event_board() {
    let fixture = Fixture::new();
    let keys_ceremony = KeysCeremony {
        is_default: None,
        ..in_progress(TrusteeStatus::KEY_GENERATED, TrusteeStatus::KEY_GENERATED)
    };

    let board = fixture.board_of(&keys_ceremony).await.unwrap();

    assert_eq!(board, (EVENT_BOARD.to_string(), None));
}

#[tokio::test]
async fn an_election_ceremony_uses_the_board_of_its_first_election() {
    let fixture = Fixture::new();
    assign(&fixture, OTHER_ELECTION, CEREMONY);
    assign(&fixture, ELECTION, CEREMONY);
    let keys_ceremony = not_default(in_progress(
        TrusteeStatus::KEY_GENERATED,
        TrusteeStatus::KEY_GENERATED,
    ));

    let board = fixture.board_of(&keys_ceremony).await.unwrap();

    assert_eq!(
        board,
        (
            MemoryKeysBoard::election_board_name(ELECTION),
            Some(ELECTION.to_string())
        )
    );
}

#[tokio::test]
async fn an_election_ceremony_without_elections_has_no_board() {
    let fixture = Fixture::new();
    let keys_ceremony = not_default(in_progress(
        TrusteeStatus::KEY_GENERATED,
        TrusteeStatus::KEY_GENERATED,
    ));

    let error = fixture.board_of(&keys_ceremony).await.unwrap_err();

    assert_eq!(
        error.to_string(),
        "Can't find election with keys ceremony keys-ceremony"
    );
}

#[tokio::test]
async fn the_default_ceremony_needs_an_election_event_board() {
    let fixture = Fixture::new();
    fixture.store.tables().election_events = vec![election_event(None)];

    let error = fixture
        .board_of(&in_progress(
            TrusteeStatus::KEY_GENERATED,
            TrusteeStatus::KEY_GENERATED,
        ))
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "missing bulletin board");
}

#[tokio::test]
async fn downloading_a_private_key_returns_it_and_marks_the_trustee_retrieved() {
    let fixture = Fixture::new().with_ceremony(with_status(
        in_progress(TrusteeStatus::KEY_GENERATED, TrusteeStatus::KEY_GENERATED),
        |status| status.public_key = Some(ELECTION_PUBLIC_KEY.to_string()),
    ));

    let private_key = fixture.download(Some(TRUSTEE)).await.unwrap();

    assert_eq!(private_key, private_key_of(TRUSTEE));
    let status = fixture.status();
    assert_eq!(
        statuses(&status),
        vec![
            (TRUSTEE.to_string(), TrusteeStatus::KEY_RETRIEVED),
            (OTHER_TRUSTEE.to_string(), TrusteeStatus::KEY_GENERATED),
        ]
    );
    assert_eq!(status.public_key.as_deref(), Some(ELECTION_PUBLIC_KEY));
    assert_eq!(status.stop_date, None);
    assert_eq!(
        logs(&status),
        vec![
            (
                "2023-11-14T19:00:00+00:00".to_string(),
                "Created Keys Ceremony".to_string()
            ),
            (
                now().to_rfc3339(),
                "Downloaded private key for trustee trustee1".to_string()
            ),
        ]
    );
    assert_eq!(fixture.execution_status(), "IN_PROGRESS");
}

#[tokio::test]
async fn a_download_is_recorded_while_the_ceremony_is_locked() {
    let fixture = Fixture::new().with_ceremony(in_progress(
        TrusteeStatus::KEY_GENERATED,
        TrusteeStatus::KEY_GENERATED,
    ));

    fixture.download(Some(TRUSTEE)).await.unwrap();

    let updates = fixture.store.tables().status_updates.clone();
    assert_eq!(updates.len(), 1);
    assert!(updates[0].locked);
}

#[tokio::test]
async fn a_download_without_a_trustee_claim_is_rejected() {
    let fixture = Fixture::new().with_ceremony(in_progress(
        TrusteeStatus::KEY_GENERATED,
        TrusteeStatus::KEY_GENERATED,
    ));

    let error = fixture.download(None).await.unwrap_err();

    assert_eq!(error.to_string(), "trustee name not found");
    assert!(fixture.nothing_written());
}

#[tokio::test]
async fn a_download_refused_by_the_ceremony_state_is_unavailable_and_writes_nothing() {
    let cases = [
        keys_ceremony(
            KeysCeremonyExecutionStatus::SUCCESS,
            &[(TRUSTEE, TrusteeStatus::KEY_GENERATED)],
        ),
        in_progress(TrusteeStatus::KEY_CHECKED, TrusteeStatus::KEY_GENERATED),
    ];
    for keys_ceremony in cases {
        let fixture = Fixture::new().with_ceremony(keys_ceremony);

        let error = fixture.download(Some(TRUSTEE)).await.unwrap_err();

        assert!(is_download_unavailable(&error));
        assert_eq!(
            error.to_string(),
            "Private key download is no longer available"
        );
        assert!(fixture.nothing_written());
    }
}

#[tokio::test]
async fn a_trustee_missing_from_the_database_cannot_download() {
    let fixture = Fixture::new().with_ceremony(in_progress(
        TrusteeStatus::KEY_GENERATED,
        TrusteeStatus::KEY_GENERATED,
    ));
    fixture.store.tables().trustees = vec![trustee(OTHER_TRUSTEE)];

    let error = fixture.download(Some(TRUSTEE)).await.unwrap_err();

    assert_eq!(
        format!("{error:#}"),
        "can't find trustee in the database: Trustee trustee1 not found"
    );
    assert!(!is_download_unavailable(&error));
    assert!(fixture.nothing_written());
}

#[tokio::test]
async fn a_trustee_without_a_public_key_cannot_download() {
    let fixture = Fixture::new().with_ceremony(in_progress(
        TrusteeStatus::KEY_GENERATED,
        TrusteeStatus::KEY_GENERATED,
    ));
    fixture.store.tables().trustees[0].public_key = None;

    let error = fixture.download(Some(TRUSTEE)).await.unwrap_err();

    assert_eq!(error.to_string(), "can't get trustee's public key");
    assert!(fixture.nothing_written());
}

#[tokio::test]
async fn a_private_key_missing_from_the_board_is_not_recorded_as_downloaded() {
    let fixture = Fixture::new().with_ceremony(in_progress(
        TrusteeStatus::KEY_GENERATED,
        TrusteeStatus::KEY_GENERATED,
    ));
    fixture.board.boards().private_keys.clear();

    let error = fixture.download(Some(TRUSTEE)).await.unwrap_err();

    assert_eq!(error.to_string(), "Channel not found on board event-board");
    assert!(!is_download_unavailable(&error));
    assert!(fixture.nothing_written());
}

#[tokio::test]
async fn a_failure_to_record_the_download_fails_it() {
    let fixture = Fixture::new().with_ceremony(in_progress(
        TrusteeStatus::KEY_GENERATED,
        TrusteeStatus::KEY_GENERATED,
    ));
    fixture.store.fail(StoreCall::UpdateKeysCeremonyStatus);

    let error = fixture.download(Some(TRUSTEE)).await.unwrap_err();

    assert_eq!(
        format!("{error:#}"),
        "couldn't update keys ceremony: UpdateKeysCeremonyStatus failed"
    );
}

#[tokio::test]
async fn checking_the_right_private_key_marks_the_trustee_checked() {
    let fixture = Fixture::new().with_ceremony(in_progress(
        TrusteeStatus::KEY_RETRIEVED,
        TrusteeStatus::KEY_RETRIEVED,
    ));

    let is_valid = fixture
        .check(Some(TRUSTEE), &private_key_of(TRUSTEE))
        .await
        .unwrap();

    assert!(is_valid);
    let status = fixture.status();
    assert_eq!(
        statuses(&status),
        vec![
            (TRUSTEE.to_string(), TrusteeStatus::KEY_CHECKED),
            (OTHER_TRUSTEE.to_string(), TrusteeStatus::KEY_RETRIEVED),
        ]
    );
    assert_eq!(status.stop_date, None);
    assert_eq!(
        logs(&status).last(),
        Some(&(
            now().to_rfc3339(),
            "Checked private key for trustee trustee1".to_string()
        ))
    );
    assert_eq!(fixture.execution_status(), "IN_PROGRESS");
}

#[tokio::test]
async fn the_ceremony_succeeds_when_the_last_trustee_checks_its_private_key() {
    let fixture = Fixture::new().with_ceremony(in_progress(
        TrusteeStatus::KEY_RETRIEVED,
        TrusteeStatus::KEY_CHECKED,
    ));

    fixture
        .check(Some(TRUSTEE), &private_key_of(TRUSTEE))
        .await
        .unwrap();

    assert_eq!(fixture.execution_status(), "SUCCESS");
}

#[tokio::test]
async fn a_check_is_recorded_while_the_ceremony_is_locked() {
    let fixture = Fixture::new().with_ceremony(in_progress(
        TrusteeStatus::KEY_RETRIEVED,
        TrusteeStatus::KEY_RETRIEVED,
    ));

    fixture
        .check(Some(TRUSTEE), &private_key_of(TRUSTEE))
        .await
        .unwrap();

    let updates = fixture.store.tables().status_updates.clone();
    assert_eq!(updates.len(), 1);
    assert!(updates[0].locked);
}

#[tokio::test]
async fn checking_a_wrong_private_key_fails_the_check_and_records_nothing() {
    let fixture = Fixture::new().with_ceremony(in_progress(
        TrusteeStatus::KEY_RETRIEVED,
        TrusteeStatus::KEY_RETRIEVED,
    ));

    let is_valid = fixture
        .check(Some(TRUSTEE), &private_key_of(OTHER_TRUSTEE))
        .await
        .unwrap();

    assert!(!is_valid);
    assert!(fixture.nothing_written());
}

#[tokio::test]
async fn a_check_without_a_trustee_claim_is_rejected() {
    let fixture = Fixture::new().with_ceremony(in_progress(
        TrusteeStatus::KEY_RETRIEVED,
        TrusteeStatus::KEY_RETRIEVED,
    ));

    let error = fixture
        .check(None, &private_key_of(TRUSTEE))
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "trustee name not found");
    assert!(fixture.nothing_written());
}

#[tokio::test]
async fn a_check_refused_by_the_ceremony_state_writes_nothing() {
    let cases = [
        (
            keys_ceremony(
                KeysCeremonyExecutionStatus::STARTED,
                &[(TRUSTEE, TrusteeStatus::KEY_RETRIEVED)],
            ),
            "Keys ceremony not in ExecutionStatus::IN_PROCESS or  ExecutionStatus::SUCCESS",
        ),
        (
            in_progress(TrusteeStatus::WAITING, TrusteeStatus::KEY_RETRIEVED),
            "Trustee not part of the keys ceremony or has invalid state",
        ),
    ];
    for (keys_ceremony, message) in cases {
        let fixture = Fixture::new().with_ceremony(keys_ceremony);

        let error = fixture
            .check(Some(TRUSTEE), &private_key_of(TRUSTEE))
            .await
            .unwrap_err();

        assert_eq!(error.to_string(), message);
        assert!(fixture.nothing_written());
    }
}

#[tokio::test]
async fn a_trustee_without_a_public_key_cannot_check_its_private_key() {
    let fixture = Fixture::new().with_ceremony(in_progress(
        TrusteeStatus::KEY_RETRIEVED,
        TrusteeStatus::KEY_RETRIEVED,
    ));
    fixture.store.tables().trustees[0].public_key = None;

    let error = fixture
        .check(Some(TRUSTEE), &private_key_of(TRUSTEE))
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "can't get trustee public key");
    assert!(fixture.nothing_written());
}

#[tokio::test]
async fn a_successful_ceremony_stays_successful_when_a_trustee_checks_again() {
    let fixture = Fixture::new().with_ceremony(keys_ceremony(
        KeysCeremonyExecutionStatus::SUCCESS,
        &[
            (TRUSTEE, TrusteeStatus::KEY_CHECKED),
            (OTHER_TRUSTEE, TrusteeStatus::KEY_CHECKED),
        ],
    ));

    let is_valid = fixture
        .check(Some(TRUSTEE), &private_key_of(TRUSTEE))
        .await
        .unwrap();

    assert!(is_valid);
    assert_eq!(fixture.execution_status(), "SUCCESS");
}

// An automated ceremony succeeds without the trustees checking their keys,
// so a later check moves it back in progress.
#[tokio::test]
async fn checking_a_private_key_of_a_successful_automated_ceremony_moves_it_back_in_progress() {
    let fixture = Fixture::new().with_ceremony(with_policy(
        keys_ceremony(
            KeysCeremonyExecutionStatus::SUCCESS,
            &[
                (TRUSTEE, TrusteeStatus::KEY_GENERATED),
                (OTHER_TRUSTEE, TrusteeStatus::KEY_GENERATED),
            ],
        ),
        CeremoniesPolicy::AUTOMATED_CEREMONIES,
    ));

    fixture
        .check(Some(TRUSTEE), &private_key_of(TRUSTEE))
        .await
        .unwrap();

    assert_eq!(fixture.execution_status(), "IN_PROGRESS");
}

#[tokio::test]
async fn creating_a_ceremony_stores_it_waiting_for_its_trustees() {
    let fixture = Fixture::new();

    let id = fixture
        .create(creation(Some(ELECTION), 2, &[OTHER_TRUSTEE, TRUSTEE]))
        .await
        .unwrap();

    assert_eq!(id, NEW_CEREMONY);
    let stored = fixture.ceremony(NEW_CEREMONY);
    assert_eq!(stored.tenant_id, TENANT);
    assert_eq!(stored.election_event_id, EVENT);
    assert_eq!(stored.trustee_ids, vec!["trustee1-id", "trustee2-id"]);
    assert_eq!(stored.threshold, 2);
    assert_eq!(stored.name.as_deref(), Some(CEREMONY_NAME));
    assert_eq!(stored.execution_status.as_deref(), Some("STARTED"));
    assert_eq!(
        stored.settings,
        Some(json!({"policy": "manual-ceremonies"}))
    );
    // The log names the trustees as requested; the trustees keep the stored
    // order.
    assert_eq!(
        stored.status,
        Some(json!({
            "stop_date": null,
            "public_key": null,
            "logs": [{
                "created_date": now().to_rfc3339(),
                "log_text": "Created Keys Ceremony with trustees: [\"trustee2\", \"trustee1\"]",
            }],
            "trustees": [
                {"name": "trustee1", "status": "WAITING"},
                {"name": "trustee2", "status": "WAITING"},
            ],
        }))
    );
}

#[tokio::test]
async fn an_election_ceremony_is_assigned_only_to_its_election() {
    let fixture = Fixture::new();

    fixture
        .create(creation(Some(ELECTION), 2, &[TRUSTEE, OTHER_TRUSTEE]))
        .await
        .unwrap();

    let stored = fixture.ceremony(NEW_CEREMONY);
    assert_eq!(stored.is_default, Some(false));
    assert_eq!(stored.permission_label, Some(vec![LABEL.to_string()]));
    let assigned: Vec<Option<String>> = fixture
        .store
        .tables()
        .elections
        .iter()
        .map(|election| election.keys_ceremony_id.clone())
        .collect();
    assert_eq!(assigned, vec![Some(NEW_CEREMONY.to_string()), None]);
}

#[tokio::test]
async fn an_event_ceremony_is_the_default_one_and_is_assigned_to_every_election() {
    let fixture = Fixture::new();

    fixture
        .create(creation(None, 2, &[TRUSTEE, OTHER_TRUSTEE]))
        .await
        .unwrap();

    assert_eq!(fixture.ceremony(NEW_CEREMONY).is_default, Some(true));
    assert!(fixture
        .store
        .tables()
        .elections
        .iter()
        .all(|election| election.keys_ceremony_id.as_deref() == Some(NEW_CEREMONY)));
}

#[tokio::test]
async fn a_new_ceremony_keeps_each_permission_label_of_its_elections_once() {
    let fixture = Fixture::new();
    fixture.store.tables().elections = vec![
        election(ELECTION, None, Some(LABEL)),
        election(OTHER_ELECTION, None, Some(OTHER_LABEL)),
        election("election-3", None, Some(LABEL)),
        election("election-4", None, None),
    ];

    fixture
        .create(creation(None, 2, &[TRUSTEE, OTHER_TRUSTEE]))
        .await
        .unwrap();

    let mut labels = fixture
        .ceremony(NEW_CEREMONY)
        .permission_label
        .expect("permission labels");
    labels.sort();
    assert_eq!(labels, vec![LABEL, OTHER_LABEL]);
}

#[tokio::test]
async fn creating_a_ceremony_records_the_keygen_in_the_electoral_log() {
    let fixture = Fixture::new();

    fixture
        .create(creation(Some(ELECTION), 2, &[TRUSTEE, OTHER_TRUSTEE]))
        .await
        .unwrap();

    assert_eq!(
        fixture.audit.entries(),
        vec![KeygenAuditEntry {
            board_name: EVENT_BOARD.to_string(),
            tenant_id: TENANT.to_string(),
            stored_election_event_id: EVENT.to_string(),
            election_event_id: EVENT.to_string(),
            user_id: USER_ID.to_string(),
            username: USERNAME.to_string(),
            election_id: Some(ELECTION.to_string()),
        }]
    );
}

#[tokio::test]
async fn an_automatic_ceremony_is_stored_with_the_automated_policy() {
    let fixture = Fixture::new();

    fixture
        .create(KeysCeremonyRequest {
            policy: CeremoniesPolicy::AUTOMATED_CEREMONIES,
            ..creation(Some(ELECTION), 2, &[TRUSTEE, OTHER_TRUSTEE])
        })
        .await
        .unwrap();

    assert_eq!(
        fixture.ceremony(NEW_CEREMONY).settings,
        Some(json!({"policy": "automated-ceremonies"}))
    );
}

#[tokio::test]
async fn a_ceremony_accepts_thresholds_from_two_to_its_number_of_trustees() {
    for threshold in [2, 3] {
        let fixture = Fixture::new();
        fixture.store.tables().trustees.push(trustee(THIRD_TRUSTEE));

        fixture
            .create(creation(
                Some(ELECTION),
                threshold,
                &[TRUSTEE, OTHER_TRUSTEE, THIRD_TRUSTEE],
            ))
            .await
            .unwrap();

        assert_eq!(fixture.ceremony(NEW_CEREMONY).threshold, threshold as i64);
    }
}

#[tokio::test]
async fn a_ceremony_rejects_thresholds_below_two_or_above_its_number_of_trustees() {
    for threshold in [1, 4] {
        let fixture = Fixture::new();
        fixture.store.tables().trustees.push(trustee(THIRD_TRUSTEE));

        let error = fixture
            .create(creation(
                Some(ELECTION),
                threshold,
                &[TRUSTEE, OTHER_TRUSTEE, THIRD_TRUSTEE],
            ))
            .await
            .unwrap_err();

        assert_eq!(error.to_string(), "invalid threshold, minimum is 2");
        assert!(fixture.nothing_created());
    }
}

#[tokio::test]
async fn a_ceremony_with_an_unknown_or_repeated_trustee_is_rejected() {
    for trustee_names in [[TRUSTEE, THIRD_TRUSTEE], [TRUSTEE, TRUSTEE]] {
        let fixture = Fixture::new();

        let error = fixture
            .create(creation(Some(ELECTION), 2, &trustee_names))
            .await
            .unwrap_err();

        assert_eq!(error.to_string(), "can't find trustees");
        assert!(fixture.nothing_created());
    }
}

#[tokio::test]
async fn no_ceremony_can_start_while_a_ceremony_covers_all_elections() {
    for is_default in [Some(true), None] {
        for election_id in [Some(ELECTION), None] {
            let fixture = Fixture::new().with_ceremony(KeysCeremony {
                is_default,
                ..in_progress(TrusteeStatus::KEY_GENERATED, TrusteeStatus::KEY_GENERATED)
            });

            let error = fixture
                .create(creation(election_id, 2, &[TRUSTEE, OTHER_TRUSTEE]))
                .await
                .unwrap_err();

            assert_eq!(
                error.to_string(),
                "there's already an existing running ceremony for all elections"
            );
            assert!(fixture.nothing_created());
        }
    }
}

#[tokio::test]
async fn an_election_with_a_ceremony_cannot_get_another() {
    let fixture = Fixture::new().with_ceremony(not_default(in_progress(
        TrusteeStatus::KEY_GENERATED,
        TrusteeStatus::KEY_GENERATED,
    )));
    assign(&fixture, ELECTION, CEREMONY);

    let error = fixture
        .create(creation(Some(ELECTION), 2, &[TRUSTEE, OTHER_TRUSTEE]))
        .await
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        "there's already an existing running ceremony for election id 'election-1'"
    );
    assert_eq!(fixture.store.tables().keys_ceremonies.len(), 1);
    assert!(fixture.audit.entries().is_empty());
}

#[tokio::test]
async fn an_election_can_get_a_ceremony_while_other_elections_have_theirs() {
    let fixture = Fixture::new().with_ceremony(not_default(in_progress(
        TrusteeStatus::KEY_GENERATED,
        TrusteeStatus::KEY_GENERATED,
    )));
    assign(&fixture, OTHER_ELECTION, CEREMONY);

    let id = fixture
        .create(creation(Some(ELECTION), 2, &[TRUSTEE, OTHER_TRUSTEE]))
        .await
        .unwrap();

    assert_eq!(id, NEW_CEREMONY);
}

#[tokio::test]
async fn a_ceremony_for_an_unknown_election_is_rejected() {
    let fixture = Fixture::new();

    let error = fixture
        .create(creation(Some("election-3"), 2, &[TRUSTEE, OTHER_TRUSTEE]))
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "Can't find election");
    assert!(fixture.nothing_created());
}

#[tokio::test]
async fn an_event_ceremony_cannot_start_while_any_election_has_a_ceremony() {
    let fixture = Fixture::new().with_ceremony(not_default(in_progress(
        TrusteeStatus::KEY_GENERATED,
        TrusteeStatus::KEY_GENERATED,
    )));

    let error = fixture
        .create(creation(None, 2, &[TRUSTEE, OTHER_TRUSTEE]))
        .await
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        "Can't create an election event keys ceremony when there are already existing keys ceremonies."
    );
    assert!(fixture.nothing_created());
}

#[tokio::test]
async fn an_event_without_elections_cannot_get_a_ceremony() {
    let fixture = Fixture::new();
    fixture.store.tables().elections.clear();

    let error = fixture
        .create(creation(None, 2, &[TRUSTEE, OTHER_TRUSTEE]))
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "No election found");
    assert!(fixture.nothing_created());
}

#[tokio::test]
async fn failures_reading_trustees_or_ceremonies_are_reported_with_their_step() {
    let cases = [
        (
            StoreCall::TrusteesByName,
            "can't find trustees: TrusteesByName failed",
        ),
        (
            StoreCall::KeysCeremonies,
            "error listing existing keys ceremonies: KeysCeremonies failed",
        ),
    ];
    for (call, message) in cases {
        let fixture = Fixture::new();
        fixture.store.fail(call);

        let error = fixture
            .create(creation(Some(ELECTION), 2, &[TRUSTEE, OTHER_TRUSTEE]))
            .await
            .unwrap_err();

        assert_eq!(format!("{error:#}"), message);
        assert!(fixture.nothing_created());
    }
}

#[tokio::test]
async fn a_ceremony_that_cannot_be_stored_is_not_logged() {
    let fixture = Fixture::new();
    fixture.store.fail(StoreCall::InsertKeysCeremony);

    let error = fixture
        .create(creation(Some(ELECTION), 2, &[TRUSTEE, OTHER_TRUSTEE]))
        .await
        .unwrap_err();

    assert_eq!(
        format!("{error:#}"),
        "couldn't insert keys ceremony: InsertKeysCeremony failed"
    );
    assert!(fixture.audit.entries().is_empty());
}

#[tokio::test]
async fn a_ceremony_of_an_event_without_a_board_is_not_logged() {
    let fixture = Fixture::new();
    fixture.store.tables().election_events = vec![election_event(None)];

    let error = fixture
        .create(creation(Some(ELECTION), 2, &[TRUSTEE, OTHER_TRUSTEE]))
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "missing bulletin board");
    assert!(fixture.audit.entries().is_empty());
}

#[tokio::test]
async fn a_failure_to_log_the_keygen_fails_the_creation() {
    let fixture = Fixture::new();
    fixture.audit.fail();

    let error = fixture
        .create(creation(Some(ELECTION), 2, &[TRUSTEE, OTHER_TRUSTEE]))
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "keygen failed");
}

#[tokio::test]
async fn a_user_holding_the_election_labels_may_create_its_ceremony() {
    let fixture = Fixture::new();

    let allowed = fixture
        .allows(Some(ELECTION), Some(r#"{"label-a","label-b"}"#))
        .await
        .unwrap();

    assert!(allowed);
}

#[tokio::test]
async fn an_event_ceremony_needs_the_labels_of_every_election() {
    let fixture = Fixture::new();
    fixture.store.tables().elections[1].permission_label = Some(OTHER_LABEL.to_string());

    assert!(fixture
        .allows(Some(ELECTION), Some(r#"{"label-a"}"#))
        .await
        .unwrap());
    assert!(!fixture.allows(None, Some(r#"{"label-a"}"#)).await.unwrap());
}

#[tokio::test]
async fn a_user_without_permission_labels_is_rejected() {
    let fixture = Fixture::new();

    let error = fixture.allows(Some(ELECTION), None).await.unwrap_err();

    assert_eq!(error.to_string(), "user dont have permission labels");
}

#[tokio::test]
async fn a_failure_reading_the_election_labels_is_reported() {
    let fixture = Fixture::new();
    fixture.store.fail(StoreCall::ElectionPermissionLabels);

    let error = fixture
        .allows(Some(ELECTION), Some(r#"{"label-a"}"#))
        .await
        .unwrap_err();

    assert!(error
        .to_string()
        .starts_with("Error getting election permissionlabel ElectionPermissionLabels failed"));
}

fn started() -> KeysCeremony {
    keys_ceremony(
        KeysCeremonyExecutionStatus::STARTED,
        &[
            (TRUSTEE, TrusteeStatus::WAITING),
            (OTHER_TRUSTEE, TrusteeStatus::WAITING),
        ],
    )
}

#[tokio::test]
async fn a_started_ceremony_gets_its_board_configured_and_moves_in_progress() {
    let fixture = Fixture::new().with_ceremony(started());

    let update = fixture.generate_keys().await.unwrap();

    assert_eq!(update, KeysCeremonyUpdate::Updated);
    assert_eq!(
        fixture.board.boards().configurations,
        vec![BoardConfiguration {
            board_name: EVENT_BOARD.to_string(),
            tenant_id: TENANT.to_string(),
            election_event_id: EVENT.to_string(),
            trustee_public_keys: vec![public_key_of(TRUSTEE), public_key_of(OTHER_TRUSTEE)],
            threshold: 2,
        }]
    );
    assert_eq!(fixture.execution_status(), "IN_PROGRESS");
    assert_eq!(fixture.ceremony(CEREMONY).status, started().status);
}

#[tokio::test]
async fn trustees_without_a_public_key_are_left_out_of_the_board_configuration() {
    let fixture = Fixture::new().with_ceremony(started());
    fixture.store.tables().trustees[1].public_key = None;

    fixture.generate_keys().await.unwrap();

    assert_eq!(
        fixture.board.boards().configurations[0].trustee_public_keys,
        vec![public_key_of(TRUSTEE)]
    );
}

#[tokio::test]
async fn a_board_that_is_already_configured_is_not_configured_again() {
    let fixture = Fixture::new().with_ceremony(started());
    let existing = BoardConfiguration {
        board_name: EVENT_BOARD.to_string(),
        tenant_id: TENANT.to_string(),
        election_event_id: EVENT.to_string(),
        trustee_public_keys: vec![],
        threshold: 3,
    };
    fixture.board.boards().configurations.push(existing.clone());

    let update = fixture.generate_keys().await.unwrap();

    assert_eq!(update, KeysCeremonyUpdate::Updated);
    assert_eq!(fixture.board.boards().configurations, vec![existing]);
    assert_eq!(fixture.execution_status(), "IN_PROGRESS");
}

#[tokio::test]
async fn only_a_started_ceremony_without_a_public_key_generates_keys() {
    let cases = [
        in_progress(TrusteeStatus::WAITING, TrusteeStatus::WAITING),
        with_status(started(), |status| {
            status.public_key = Some(ELECTION_PUBLIC_KEY.to_string())
        }),
    ];
    for keys_ceremony in cases {
        let fixture = Fixture::new().with_ceremony(keys_ceremony);

        let update = fixture.generate_keys().await.unwrap();

        assert_eq!(update, KeysCeremonyUpdate::Skipped);
        assert!(fixture.board.boards().configurations.is_empty());
        assert!(fixture.nothing_written());
    }
}

// The board is looked up before the status is checked, so a ceremony that
// would be skipped still fails without a board.
#[tokio::test]
async fn a_ceremony_without_a_board_fails_to_generate_keys_even_when_not_started() {
    let fixture = Fixture::new().with_ceremony(not_default(in_progress(
        TrusteeStatus::KEY_GENERATED,
        TrusteeStatus::KEY_GENERATED,
    )));

    let error = fixture.generate_keys().await.unwrap_err();

    assert_eq!(
        error.to_string(),
        "Can't find election with keys ceremony keys-ceremony"
    );
}

#[tokio::test]
async fn an_unknown_ceremony_cannot_generate_keys() {
    let fixture = Fixture::new();

    let error = fixture.generate_keys().await.unwrap_err();

    assert_eq!(
        format!("{error:#}"),
        "error finding keys ceremony: Keys ceremony keys-ceremony not found"
    );
}

#[tokio::test]
async fn a_ceremony_whose_board_cannot_be_configured_stays_started() {
    let fixture = Fixture::new().with_ceremony(started());
    fixture.board.boards().failing.insert(BoardCall::CreateKeys);

    let error = fixture.generate_keys().await.unwrap_err();

    assert_eq!(error.to_string(), "CreateKeys failed");
    assert!(fixture.nothing_written());
}

fn awaiting_public_key(policy: CeremoniesPolicy) -> KeysCeremony {
    with_policy(
        in_progress(TrusteeStatus::WAITING, TrusteeStatus::WAITING),
        policy,
    )
}

fn post(
    fixture: &Fixture,
    statement: KeysBoardStatement,
    sender: &str,
    posted_at: u64,
    log: (&str, &str),
) {
    fixture
        .board
        .boards()
        .messages
        .entry(EVENT_BOARD.to_string())
        .or_default()
        .push(PostedMessage {
            message: KeysBoardMessage {
                statement,
                sender: sender.to_string(),
            },
            log: Log {
                created_date: log.0.to_string(),
                log_text: log.1.to_string(),
            },
            posted_at,
        });
}

fn publish_public_key(fixture: &Fixture) {
    fixture
        .board
        .boards()
        .public_keys
        .insert(EVENT_BOARD.to_string(), ELECTION_PUBLIC_KEY.to_string());
}

#[tokio::test]
async fn an_automated_ceremony_succeeds_once_the_board_has_the_public_key() {
    let fixture =
        Fixture::new().with_ceremony(awaiting_public_key(CeremoniesPolicy::AUTOMATED_CEREMONIES));
    publish_public_key(&fixture);

    let update = fixture.record_public_key().await.unwrap();

    assert_eq!(update, KeysCeremonyUpdate::Updated);
    assert_eq!(fixture.execution_status(), "SUCCESS");
    assert_eq!(
        fixture.status().public_key.as_deref(),
        Some(ELECTION_PUBLIC_KEY)
    );
}

#[tokio::test]
async fn a_manual_ceremony_stays_in_progress_with_the_public_key() {
    let fixture =
        Fixture::new().with_ceremony(awaiting_public_key(CeremoniesPolicy::MANUAL_CEREMONIES));
    publish_public_key(&fixture);

    fixture.record_public_key().await.unwrap();

    assert_eq!(fixture.execution_status(), "IN_PROGRESS");
    assert_eq!(
        fixture.status().public_key.as_deref(),
        Some(ELECTION_PUBLIC_KEY)
    );
}

#[tokio::test]
async fn a_ceremony_stays_in_progress_until_the_board_has_the_public_key() {
    let fixture =
        Fixture::new().with_ceremony(awaiting_public_key(CeremoniesPolicy::AUTOMATED_CEREMONIES));
    post(
        &fixture,
        KeysBoardStatement::PublicKey,
        &public_key_of(TRUSTEE),
        1_699_990_001,
        (
            "2023-11-14T19:46:41+00:00",
            "trustee1: Added message PublicKey",
        ),
    );

    let update = fixture.record_public_key().await.unwrap();

    assert_eq!(update, KeysCeremonyUpdate::Updated);
    assert_eq!(fixture.execution_status(), "IN_PROGRESS");
    let status = fixture.status();
    assert_eq!(status.public_key, None);
    assert_eq!(
        statuses(&status),
        vec![
            (TRUSTEE.to_string(), TrusteeStatus::KEY_GENERATED),
            (OTHER_TRUSTEE.to_string(), TrusteeStatus::WAITING),
        ]
    );
}

#[tokio::test]
async fn recording_the_public_key_sets_the_stop_date_to_the_current_time() {
    let fixture =
        Fixture::new().with_ceremony(awaiting_public_key(CeremoniesPolicy::MANUAL_CEREMONIES));

    fixture.record_public_key().await.unwrap();

    assert_eq!(fixture.status().stop_date.as_deref(), Some("1700000000000"));
}

#[tokio::test]
async fn a_trustee_generated_its_key_only_once_the_board_has_its_public_key_share() {
    let fixture = Fixture::new().with_ceremony(keys_ceremony(
        KeysCeremonyExecutionStatus::IN_PROGRESS,
        &[
            (TRUSTEE, TrusteeStatus::WAITING),
            (OTHER_TRUSTEE, TrusteeStatus::WAITING),
            (THIRD_TRUSTEE, TrusteeStatus::WAITING),
            ("trustee4", TrusteeStatus::WAITING),
        ],
    ));
    {
        let mut tables = fixture.store.tables();
        tables.trustees.push(trustee(THIRD_TRUSTEE));
        let mut without_public_key = trustee("trustee4");
        without_public_key.public_key = None;
        tables.trustees.push(without_public_key);
    }
    let log = ("2023-11-14T19:46:41+00:00", "message");
    post(
        &fixture,
        KeysBoardStatement::PublicKey,
        &public_key_of(TRUSTEE),
        0,
        log,
    );
    post(
        &fixture,
        KeysBoardStatement::PublicKeySigned,
        &public_key_of(OTHER_TRUSTEE),
        0,
        log,
    );
    post(
        &fixture,
        KeysBoardStatement::Other,
        &public_key_of(THIRD_TRUSTEE),
        0,
        log,
    );
    post(
        &fixture,
        KeysBoardStatement::PublicKey,
        "unknown-public-key",
        0,
        log,
    );

    fixture.record_public_key().await.unwrap();

    assert_eq!(
        statuses(&fixture.status()),
        vec![
            (TRUSTEE.to_string(), TrusteeStatus::KEY_GENERATED),
            (OTHER_TRUSTEE.to_string(), TrusteeStatus::KEY_GENERATED),
            (THIRD_TRUSTEE.to_string(), TrusteeStatus::WAITING),
            ("trustee4".to_string(), TrusteeStatus::WAITING),
        ]
    );
}

// The statuses come from the board alone, so downloads and checks recorded
// before the public key is set are lost.
#[tokio::test]
async fn recording_the_public_key_recomputes_every_trustee_status_from_the_board() {
    let fixture = Fixture::new().with_ceremony(in_progress(
        TrusteeStatus::KEY_RETRIEVED,
        TrusteeStatus::KEY_CHECKED,
    ));
    post(
        &fixture,
        KeysBoardStatement::PublicKey,
        &public_key_of(TRUSTEE),
        0,
        ("2023-11-14T19:46:41+00:00", "message"),
    );

    fixture.record_public_key().await.unwrap();

    assert_eq!(
        statuses(&fixture.status()),
        vec![
            (TRUSTEE.to_string(), TrusteeStatus::KEY_GENERATED),
            (OTHER_TRUSTEE.to_string(), TrusteeStatus::WAITING),
        ]
    );
}

#[tokio::test]
async fn only_messages_posted_since_the_last_update_are_logged_in_date_order() {
    let fixture =
        Fixture::new().with_ceremony(awaiting_public_key(CeremoniesPolicy::MANUAL_CEREMONIES));
    let since = last_updated().timestamp() as u64;
    post(
        &fixture,
        KeysBoardStatement::Other,
        "sender",
        since - 1,
        ("2023-11-14T18:00:00+00:00", "before the update"),
    );
    post(
        &fixture,
        KeysBoardStatement::Other,
        "sender",
        since,
        ("2023-11-14T18:30:00+00:00", "at the update"),
    );

    fixture.record_public_key().await.unwrap();

    assert_eq!(
        logs(&fixture.status()),
        vec![
            (
                "2023-11-14T18:30:00+00:00".to_string(),
                "at the update".to_string()
            ),
            (
                "2023-11-14T19:00:00+00:00".to_string(),
                "Created Keys Ceremony".to_string()
            ),
        ]
    );
}

#[tokio::test]
async fn a_ceremony_with_a_public_key_or_not_in_progress_is_left_alone() {
    let cases = [
        with_status(
            awaiting_public_key(CeremoniesPolicy::MANUAL_CEREMONIES),
            |status| status.public_key = Some(ELECTION_PUBLIC_KEY.to_string()),
        ),
        started(),
        keys_ceremony(
            KeysCeremonyExecutionStatus::SUCCESS,
            &[(TRUSTEE, TrusteeStatus::KEY_CHECKED)],
        ),
    ];
    for keys_ceremony in cases {
        let fixture = Fixture::new().with_ceremony(keys_ceremony);
        publish_public_key(&fixture);

        let update = fixture.record_public_key().await.unwrap();

        assert_eq!(update, KeysCeremonyUpdate::Skipped);
        assert!(fixture.nothing_written());
    }
}

#[tokio::test]
async fn trustees_that_do_not_match_the_stored_trustees_fail_the_public_key_update() {
    let fixture =
        Fixture::new().with_ceremony(awaiting_public_key(CeremoniesPolicy::MANUAL_CEREMONIES));
    fixture.store.tables().trustees.remove(1);

    let error = fixture.record_public_key().await.unwrap_err();

    assert_eq!(
        error.to_string(),
        "trustee_names don't correspond to trustees_by_name"
    );
    assert!(fixture.nothing_written());
}

#[tokio::test]
async fn an_unreadable_trustee_public_key_fails_the_public_key_update() {
    let fixture =
        Fixture::new().with_ceremony(awaiting_public_key(CeremoniesPolicy::MANUAL_CEREMONIES));
    fixture
        .board
        .boards()
        .invalid_keys
        .insert(public_key_of(OTHER_TRUSTEE));

    let error = fixture.record_public_key().await.unwrap_err();

    assert_eq!(error.to_string(), "invalid public key trustee2-public-key");
    assert!(fixture.nothing_written());
}

#[tokio::test]
async fn a_ceremony_without_an_update_time_fails_the_public_key_update() {
    let fixture = Fixture::new().with_ceremony(KeysCeremony {
        last_updated_at: None,
        ..awaiting_public_key(CeremoniesPolicy::MANUAL_CEREMONIES)
    });

    let error = fixture.record_public_key().await.unwrap_err();

    assert_eq!(error.to_string(), "empty last_updated_at");
    assert!(fixture.nothing_written());
}

fn with_id(id: &str, keys_ceremony: KeysCeremony) -> KeysCeremony {
    KeysCeremony {
        id: id.to_string(),
        ..keys_ceremony
    }
}

fn queued(step: KeysBoardStep, keys_ceremony_id: &str) -> QueuedTask {
    QueuedTask {
        step,
        tenant_id: TENANT.to_string(),
        election_event_id: EVENT.to_string(),
        keys_ceremony_id: keys_ceremony_id.to_string(),
    }
}

#[tokio::test]
async fn each_ceremony_gets_the_board_task_its_state_needs() {
    let with_public_key = |keys_ceremony| {
        with_status(keys_ceremony, |status| {
            status.public_key = Some(ELECTION_PUBLIC_KEY.to_string())
        })
    };
    let fixture = Fixture::new();
    fixture.store.tables().keys_ceremonies = vec![
        with_id("started", started()),
        with_id("started-with-key", with_public_key(started())),
        with_id(
            "in-progress",
            in_progress(TrusteeStatus::WAITING, TrusteeStatus::WAITING),
        ),
        with_id(
            "in-progress-with-key",
            with_public_key(in_progress(TrusteeStatus::WAITING, TrusteeStatus::WAITING)),
        ),
        with_id(
            "success",
            keys_ceremony(KeysCeremonyExecutionStatus::SUCCESS, &[]),
        ),
        with_id(
            "cancelled",
            keys_ceremony(KeysCeremonyExecutionStatus::CANCELLED, &[]),
        ),
        with_id(
            "user-configuration",
            keys_ceremony(KeysCeremonyExecutionStatus::USER_CONFIGURATION, &[]),
        ),
    ];

    fixture.dispatch().await.unwrap();

    assert_eq!(
        fixture.tasks.queued(),
        vec![
            queued(KeysBoardStep::CreateKeys, "started"),
            queued(KeysBoardStep::CreateKeys, "started-with-key"),
            queued(KeysBoardStep::SetPublicKey, "in-progress"),
        ]
    );
}

#[tokio::test]
async fn an_unreadable_ceremony_stops_the_dispatch() {
    let fixture = Fixture::new();
    fixture.store.tables().keys_ceremonies = vec![
        KeysCeremony {
            status: Some(json!({"trustees": "invalid"})),
            ..with_id(
                "cancelled",
                keys_ceremony(KeysCeremonyExecutionStatus::CANCELLED, &[]),
            )
        },
        with_id("started", started()),
    ];

    assert!(fixture.dispatch().await.is_err());
    assert!(fixture.tasks.queued().is_empty());
}

#[tokio::test]
async fn a_task_that_cannot_be_queued_stops_the_dispatch() {
    let fixture = Fixture::new();
    fixture.store.tables().keys_ceremonies = vec![
        with_id("started", started()),
        with_id(
            "in-progress",
            in_progress(TrusteeStatus::WAITING, TrusteeStatus::WAITING),
        ),
    ];
    fixture.tasks.fail(KeysBoardStep::CreateKeys);

    let error = fixture.dispatch().await.unwrap_err();

    assert_eq!(error.to_string(), "CreateKeys task failed");
    assert!(fixture.tasks.queued().is_empty());
}
