// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::adapters::memory::clock::{FixedClock, SequentialIds};
use crate::adapters::memory::keys_ceremony::{
    MemoryKeysCeremonyAudit, MemoryKeysCeremonyStore, MemoryKeysCeremonyTasks, QueuedTask,
    StoreCall,
};
use chrono::{DateTime, Local, TimeZone};
use sequent_core::types::ceremonies::Log;
use sequent_core::types::ceremonies::{Trustee, TrusteeStatus};
use sequent_core::types::hasura::core::Trustee as TrusteeRecord;
use sequent_core::types::hasura::core::{Election, ElectionEvent};
use serde_json::{json, Value};

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

fn trustee(name: &str) -> TrusteeRecord {
    let keys = protocol_board::TrusteeSecrets::generate()
        .public_keys()
        .unwrap();
    serde_json::from_value(json!({
        "id": format!("{name}-id"),
        "name": name,
        "public_key": keys.signing_public_key,
        "share_encryption_public_key": keys.share_encryption_public_key,
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
        public_key_hash: None,
        failure: None,
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
    audit: MemoryKeysCeremonyAudit,
    tasks: MemoryKeysCeremonyTasks,
    clock: FixedClock,
}
impl Fixture {
    fn new() -> Self {
        let fixture = Self {
            store: Default::default(),
            audit: Default::default(),
            tasks: Default::default(),
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
        fixture
    }
    fn with_ceremony(self, ceremony: KeysCeremony) -> Self {
        self.store.tables().keys_ceremonies.push(ceremony);
        self
    }
    fn ceremony(&self, id: &str) -> KeysCeremony {
        self.store
            .tables()
            .keys_ceremonies
            .iter()
            .find(|c| c.id == id)
            .cloned()
            .unwrap()
    }
    fn nothing_created(&self) -> bool {
        let tables = self.store.tables();
        tables.keys_ceremonies.iter().all(|c| c.id != NEW_CEREMONY)
            && tables
                .elections
                .iter()
                .all(|e| e.keys_ceremony_id.is_none())
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
    async fn allows(&self, election_id: Option<&str>, labels: Option<&str>) -> Result<bool> {
        has_election_permission_labels(
            &self.store,
            TENANT,
            EVENT,
            election_id.map(str::to_owned),
            labels.map(str::to_owned),
        )
        .await
    }
    async fn dispatch(&self) -> Result<()> {
        dispatch_keys_ceremony_tasks(&self.store, &self.tasks, TENANT, EVENT).await
    }
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
            "public_key_hash": null,
            "failure": null,
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

#[test]
fn reported_public_key_must_match_the_verified_board() {
    let mut ceremony = keys_ceremony(KeysCeremonyExecutionStatus::SUCCESS, &[]);
    ceremony.status = Some(json!({"public_key": ELECTION_PUBLIC_KEY}));
    assert_eq!(
        ceremony_public_key_state(&ceremony, Some(ELECTION_PUBLIC_KEY)),
        CeremonyPublicKeyState::OnBoard(ELECTION_PUBLIC_KEY.into())
    );
    for board_key in [None, Some("another-key")] {
        assert_eq!(
            ceremony_public_key_state(&ceremony, board_key),
            CeremonyPublicKeyState::NotOnBoard
        );
    }
}

#[test]
fn missing_reported_public_key_requires_no_board_comparison() {
    let mut ceremony = keys_ceremony(KeysCeremonyExecutionStatus::IN_PROGRESS, &[]);
    for status in [
        None,
        Some(json!({"public_key": null})),
        Some(json!({"trustees": 7})),
    ] {
        ceremony.status = status;
        for board_key in [None, Some(ELECTION_PUBLIC_KEY)] {
            assert_eq!(
                ceremony_public_key_state(&ceremony, board_key),
                CeremonyPublicKeyState::NoPublicKey
            );
        }
    }
}

#[test]
fn malformed_ceremony_status_cannot_hide_a_reported_public_key() {
    let mut ceremony = keys_ceremony(KeysCeremonyExecutionStatus::SUCCESS, &[]);
    ceremony.status = Some(json!({"public_key": ELECTION_PUBLIC_KEY, "trustees": "invalid"}));
    assert!(ceremony.status().is_err());
    assert_eq!(
        ceremony_public_key_state(&ceremony, Some(ELECTION_PUBLIC_KEY)),
        CeremonyPublicKeyState::OnBoard(ELECTION_PUBLIC_KEY.into())
    );
    assert_eq!(
        ceremony_public_key_state(&ceremony, None),
        CeremonyPublicKeyState::NotOnBoard
    );
    ceremony.status = Some(json!({"public_key": 7}));
    assert_eq!(
        ceremony_public_key_state(&ceremony, Some(ELECTION_PUBLIC_KEY)),
        CeremonyPublicKeyState::NotOnBoard
    );
}
