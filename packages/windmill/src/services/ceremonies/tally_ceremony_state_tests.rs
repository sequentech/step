// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::adapters::memory::tally_ceremony::{InMemoryTallyCeremony, TallyAuditEntry, TallyCall};
use crate::domain::tally_ceremony::TallyExecuter;
use sequent_core::types::ceremonies::TallyExecutionStatus::{
    AWAITING_INPUT, CANCELLED, CONNECTED, IN_PROGRESS, STARTED, SUCCESS,
};

const TENANT: &str = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
const EVENT: &str = "33f18502-a67c-4853-8333-a58630663559";
/// `get_event_board(TENANT, EVENT, "dev")`: the slug, 17 characters of the
/// tenant id and the event id, without dashes.
const SLUG_EVENT_BOARD: &str = "devtenant90505c8a23a94cdfaevent33f18502a67c48538333a58630663559";
/// The board in the election event's bulletin board reference.
const EVENT_BOARD: &str = "event-bulletin-board";
const SESSION: &str = "session";
const KEYS_CEREMONY: &str = "keys-ceremony";
const ELECTION: &str = "election";
const ADMIN_ID: &str = "admin-id";
const ADMIN: &str = "admin";
const TRUSTEE_USER_ID: &str = "trustee-user-id";
const TRUSTEE_LOGIN: &str = "trustee-login";
const STORED_KEY: &str = "stored-encrypted-key";
const LAST_MESSAGE_ID: i32 = 7;
const ALL_STATUSES: [TallyExecutionStatus; 6] = [
    STARTED,
    CONNECTED,
    IN_PROGRESS,
    AWAITING_INPUT,
    SUCCESS,
    CANCELLED,
];

fn tally_session(execution_status: &TallyExecutionStatus) -> TallySession {
    TallySession {
        id: SESSION.into(),
        tenant_id: TENANT.into(),
        election_event_id: EVENT.into(),
        created_at: None,
        last_updated_at: None,
        labels: None,
        annotations: Some(json!({"executer_user_id": ADMIN_ID, "executer_username": ADMIN})),
        election_ids: Some(vec![ELECTION.into()]),
        area_ids: None,
        is_execution_completed: false,
        keys_ceremony_id: KEYS_CEREMONY.into(),
        execution_status: Some(execution_status.to_string()),
        threshold: 2,
        configuration: None,
        tally_type: None,
        permission_label: None,
    }
}

fn ceremony_status(trustees: &[(&str, TallyTrusteeStatus)]) -> TallyCeremonyStatus {
    TallyCeremonyStatus {
        stop_date: None,
        logs: vec![Log {
            created_date: "2026-01-01T00:00:00+00:00".into(),
            log_text: "Created Tally Ceremony".into(),
        }],
        trustees: trustees
            .iter()
            .map(|(name, status)| TallyTrustee {
                name: name.to_string(),
                status: status.clone(),
            })
            .collect(),
        elections_status: vec![TallyElection {
            election_id: ELECTION.into(),
            status: TallyElectionStatus::SUCCESS,
            progress: 100.0,
        }],
    }
}

fn execution(status: Option<&TallyCeremonyStatus>) -> TallySessionExecution {
    TallySessionExecution {
        id: "first-execution".into(),
        tenant_id: TENANT.into(),
        election_event_id: EVENT.into(),
        created_at: None,
        last_updated_at: None,
        labels: None,
        annotations: None,
        current_message_id: LAST_MESSAGE_ID,
        tally_session_id: SESSION.into(),
        session_ids: None,
        status: status.map(|status| serde_json::to_value(status).unwrap()),
        results_event_id: None,
        documents: None,
        run_reason: Some(TallyRunReason::NORMAL.to_string()),
    }
}

fn keys_ceremony(threshold: i64) -> KeysCeremony {
    KeysCeremony {
        id: KEYS_CEREMONY.into(),
        created_at: None,
        last_updated_at: None,
        tenant_id: TENANT.into(),
        election_event_id: EVENT.into(),
        trustee_ids: vec![],
        status: None,
        execution_status: Some(KeysCeremonyExecutionStatus::SUCCESS.to_string()),
        labels: None,
        annotations: None,
        threshold,
        name: None,
        settings: None,
        is_default: Some(true),
        permission_label: None,
    }
}

fn election_event(bulletin_board_reference: Option<Value>) -> ElectionEvent {
    ElectionEvent {
        id: EVENT.into(),
        created_at: None,
        updated_at: None,
        labels: None,
        annotations: None,
        tenant_id: TENANT.into(),
        description: None,
        presentation: None,
        bulletin_board_reference,
        is_archived: false,
        voting_channels: None,
        status: None,
        user_boards: None,
        encryption_protocol: "RistrettoCtx".into(),
        is_audit: None,
        audit_election_event_id: None,
        public_key: None,
        statistics: None,
        external_id: None,
    }
}

fn election(voting_status: &str) -> Election {
    serde_json::from_value(json!({
        "id": ELECTION, "tenant_id": TENANT, "election_event_id": EVENT,
        "status": {"is_published": true, "voting_status": voting_status,
                   "allow_tally": "requires-voting-period-end"}
    }))
    .unwrap()
}

fn trustee_claims(trustee: Option<&str>) -> JwtClaims {
    serde_json::from_value(json!({
        "exp": 1, "iat": 0, "jti": "jti", "iss": "iss", "sub": TRUSTEE_USER_ID, "typ": "Bearer",
        "azp": "admin-portal", "acr": "1", "allowed-origins": [], "scope": "openid",
        "email_verified": true, "preferred_username": TRUSTEE_LOGIN, "trustee": trustee,
        "https://hasura.io/jwt/claims": {
            "x-hasura-default-role": "trustee", "x-hasura-tenant-id": TENANT,
            "x-hasura-user-id": TRUSTEE_USER_ID, "x-hasura-allowed-roles": ["trustee"]
        }
    }))
    .unwrap()
}

/// A session in `execution_status` with one execution holding `trustees`,
/// whose elections have closed, on an event with a bulletin board.
fn ceremony(
    execution_status: &TallyExecutionStatus,
    trustees: &[(&str, TallyTrusteeStatus)],
) -> InMemoryTallyCeremony {
    let ceremony = InMemoryTallyCeremony::default();
    ceremony.add_session(tally_session(execution_status));
    ceremony.add_execution(execution(Some(&ceremony_status(trustees))));
    ceremony.add_keys_ceremony(keys_ceremony(2));
    ceremony.add_private_key(KEYS_CEREMONY, "alice", STORED_KEY);
    ceremony.add_private_key(KEYS_CEREMONY, "bob", STORED_KEY);
    ceremony.add_election(election("CLOSED"));
    ceremony.add_election_event(election_event(Some(
        json!({"id": 1, "database_name": EVENT_BOARD, "is_archived": false}),
    )));
    ceremony.set_env_slug("dev");
    ceremony
}

fn restored_pair() -> [(&'static str, TallyTrusteeStatus); 2] {
    [
        ("alice", TallyTrusteeStatus::KEY_RESTORED),
        ("bob", TallyTrusteeStatus::KEY_RESTORED),
    ]
}

fn waiting_pair() -> [(&'static str, TallyTrusteeStatus); 2] {
    [
        ("alice", TallyTrusteeStatus::WAITING),
        ("bob", TallyTrusteeStatus::WAITING),
    ]
}

async fn change_status(
    ceremony: &InMemoryTallyCeremony,
    tally_session: TallySession,
    new_execution_status: TallyExecutionStatus,
) -> Result<()> {
    update_tally_ceremony_with(
        ceremony,
        ceremony,
        ceremony,
        ceremony,
        TallyStatusChange {
            tenant_id: TENANT.into(),
            election_event_id: EVENT.into(),
            tally_session,
            new_execution_status,
            user_id: ADMIN_ID.into(),
            username: ADMIN.into(),
        },
    )
    .await
}

async fn restore_key(
    ceremony: &InMemoryTallyCeremony,
    claims: &JwtClaims,
    private_key_base64: &str,
) -> Result<bool> {
    set_private_key_with(
        ceremony,
        ceremony,
        ceremony,
        ceremony,
        ceremony,
        TrusteeKeyRestore {
            claims,
            tenant_id: TENANT,
            election_event_id: EVENT,
            tally_session_id: SESSION,
            private_key_base64,
        },
    )
    .await
}

async fn complete(ceremony: &InMemoryTallyCeremony) -> Result<()> {
    set_tally_session_completed_with(ceremony, ceremony, ceremony, TENANT, EVENT, SESSION).await
}

async fn recount(ceremony: &InMemoryTallyCeremony, election_ids: &[&str]) -> Result<bool> {
    let election_ids: Vec<String> = election_ids.iter().map(|id| id.to_string()).collect();
    begin_tally_session_recount_with(ceremony, TENANT, EVENT, SESSION, &election_ids).await
}

fn validation_message(error: anyhow::Error) -> String {
    error
        .downcast_ref::<TallyValidationError>()
        .unwrap_or_else(|| panic!("expected a TallyValidationError, got {error:?}"))
        .to_string()
}

fn stored_status(ceremony: &InMemoryTallyCeremony) -> Option<String> {
    ceremony.session(SESSION).execution_status
}

fn last_ceremony_status(ceremony: &InMemoryTallyCeremony) -> TallyCeremonyStatus {
    let executions = ceremony.executions(SESSION);
    get_tally_ceremony_status(executions.last().unwrap().status.clone()).unwrap()
}

fn trustee_statuses(status: &TallyCeremonyStatus) -> Vec<(String, TallyTrusteeStatus)> {
    status
        .trustees
        .iter()
        .map(|trustee| (trustee.name.clone(), trustee.status.clone()))
        .collect()
}

fn log_texts(status: &TallyCeremonyStatus) -> Vec<String> {
    status.logs.iter().map(|log| log.log_text.clone()).collect()
}

/// Only the execution each test starts from is stored, the session keeps
/// `status` and the electoral log is empty.
fn assert_nothing_written(ceremony: &InMemoryTallyCeremony, status: &TallyExecutionStatus) {
    assert_eq!(ceremony.executions(SESSION).len(), 1);
    assert_eq!(stored_status(ceremony), Some(status.to_string()));
    assert!(!ceremony.session(SESSION).is_execution_completed);
    assert_eq!(ceremony.audit_entries(), vec![]);
}

#[tokio::test]
async fn allowed_status_changes_are_stored_and_others_are_refused() {
    let allowed = [
        (STARTED, CANCELLED),
        (CONNECTED, IN_PROGRESS),
        (CONNECTED, CANCELLED),
        (IN_PROGRESS, CANCELLED),
        (AWAITING_INPUT, IN_PROGRESS),
        (AWAITING_INPUT, CANCELLED),
    ];
    for current in ALL_STATUSES {
        for new in ALL_STATUSES {
            let ceremony = ceremony(&current, &restored_pair());
            let result = change_status(&ceremony, tally_session(&current), new.clone()).await;
            if allowed.contains(&(current.clone(), new.clone())) {
                assert!(result.is_ok(), "{current} -> {new}: {result:?}");
                assert_eq!(stored_status(&ceremony), Some(new.to_string()));
                assert!(!ceremony.session(SESSION).is_execution_completed);
            } else {
                assert_eq!(
                    validation_message(result.unwrap_err()),
                    format!("Cannot change tally status from {current} to {new}.")
                );
                assert_nothing_written(&ceremony, &current);
            }
        }
    }
}

#[tokio::test]
async fn a_missing_or_unknown_status_changes_like_started() {
    for execution_status in [None, Some("PAUSED".to_string())] {
        let ceremony = ceremony(&STARTED, &restored_pair());
        let mut tally_session = tally_session(&STARTED);
        tally_session.execution_status = execution_status;
        let error = change_status(&ceremony, tally_session.clone(), IN_PROGRESS)
            .await
            .unwrap_err();
        assert_eq!(
            validation_message(error),
            "Cannot change tally status from STARTED to IN_PROGRESS."
        );
        change_status(&ceremony, tally_session, CANCELLED)
            .await
            .unwrap();
        assert_eq!(stored_status(&ceremony), Some(CANCELLED.to_string()));
    }
}

#[tokio::test]
async fn starting_the_tally_revalidates_the_elections() {
    let ceremony = ceremony(&CONNECTED, &restored_pair());
    let open = InMemoryTallyCeremony::default();
    open.add_election(election("OPEN"));
    let error = update_tally_ceremony_with(
        &ceremony,
        &open,
        &ceremony,
        &ceremony,
        TallyStatusChange {
            tenant_id: TENANT.into(),
            election_event_id: EVENT.into(),
            tally_session: tally_session(&CONNECTED),
            new_execution_status: IN_PROGRESS,
            user_id: ADMIN_ID.into(),
            username: ADMIN.into(),
        },
    )
    .await
    .unwrap_err();
    assert_eq!(
        validation_message(error),
        format!(
            "Election {ELECTION}: end its voting period and stop all active voting channels before tallying."
        )
    );
    assert_nothing_written(&ceremony, &CONNECTED);
}

#[tokio::test]
async fn cancelling_does_not_read_or_validate_the_elections() {
    let ceremony = ceremony(&CONNECTED, &restored_pair());
    ceremony.fail(TallyCall::GetElections, "elections are unavailable");
    change_status(&ceremony, tally_session(&CONNECTED), CANCELLED)
        .await
        .unwrap();
    assert_eq!(stored_status(&ceremony), Some(CANCELLED.to_string()));
}

#[tokio::test]
async fn an_invalid_tally_type_cannot_be_started() {
    let ceremony = ceremony(&CONNECTED, &restored_pair());
    let mut tally_session = tally_session(&CONNECTED);
    tally_session.tally_type = Some("RESULTS".into());
    let error = change_status(&ceremony, tally_session, IN_PROGRESS)
        .await
        .unwrap_err();
    assert_eq!(validation_message(error), "Invalid tally type");
    assert_nothing_written(&ceremony, &CONNECTED);
}

#[tokio::test]
async fn a_session_without_a_tally_type_is_validated_as_electoral_results() {
    // An open election can produce an initialization report but not results,
    // so only the electoral results rules refuse it.
    let ceremony = ceremony(&CONNECTED, &restored_pair());
    let open = InMemoryTallyCeremony::default();
    open.add_election(election("OPEN"));
    let start = |tally_type: Option<&str>| {
        let mut tally_session = tally_session(&CONNECTED);
        tally_session.tally_type = tally_type.map(str::to_string);
        update_tally_ceremony_with(
            &ceremony,
            &open,
            &ceremony,
            &ceremony,
            TallyStatusChange {
                tenant_id: TENANT.into(),
                election_event_id: EVENT.into(),
                tally_session,
                new_execution_status: IN_PROGRESS,
                user_id: ADMIN_ID.into(),
                username: ADMIN.into(),
            },
        )
    };
    let error = start(None).await.unwrap_err();
    assert!(validation_message(error).contains("end its voting period"));
    assert_eq!(stored_status(&ceremony), Some(CONNECTED.to_string()));
    start(Some("INITIALIZATION_REPORT")).await.unwrap();
    assert_eq!(stored_status(&ceremony), Some(IN_PROGRESS.to_string()));
}

#[tokio::test]
async fn a_session_without_executions_is_left_unchanged() {
    for new in [IN_PROGRESS, CANCELLED] {
        let ceremony = InMemoryTallyCeremony::default();
        ceremony.add_session(tally_session(&CONNECTED));
        ceremony.add_election(election("CLOSED"));
        ceremony.set_env_slug("dev");
        change_status(&ceremony, tally_session(&CONNECTED), new)
            .await
            .unwrap();
        assert_eq!(stored_status(&ceremony), Some(CONNECTED.to_string()));
        assert_eq!(ceremony.audit_entries(), vec![]);
    }
}

#[tokio::test]
async fn starting_the_tally_needs_the_session_threshold_of_restored_trustees() {
    let trustees = [
        ("alice", TallyTrusteeStatus::KEY_RESTORED),
        ("bob", TallyTrusteeStatus::KEY_RESTORED),
        ("carol", TallyTrusteeStatus::WAITING),
    ];
    let ceremony = ceremony(&CONNECTED, &trustees);
    let mut tally_session = tally_session(&CONNECTED);
    tally_session.threshold = 3;
    let error = change_status(&ceremony, tally_session.clone(), IN_PROGRESS)
        .await
        .unwrap_err();
    assert_eq!(
        validation_message(error),
        "Insufficient number of connected trustees 2. Required threshold 3."
    );
    assert_nothing_written(&ceremony, &CONNECTED);

    tally_session.threshold = 2;
    change_status(&ceremony, tally_session, IN_PROGRESS)
        .await
        .unwrap();
    assert_eq!(stored_status(&ceremony), Some(IN_PROGRESS.to_string()));
}

#[tokio::test]
async fn a_session_can_be_cancelled_before_any_trustee_restores_a_key() {
    let ceremony = ceremony(&CONNECTED, &waiting_pair());
    change_status(&ceremony, tally_session(&CONNECTED), CANCELLED)
        .await
        .unwrap();
    assert_eq!(stored_status(&ceremony), Some(CANCELLED.to_string()));
    assert_eq!(ceremony.audit_entries(), vec![]);
}

#[tokio::test]
async fn starting_the_tally_posts_a_tally_open_entry_on_the_event_board() {
    let ceremony = ceremony(&AWAITING_INPUT, &restored_pair());
    change_status(&ceremony, tally_session(&AWAITING_INPUT), IN_PROGRESS)
        .await
        .unwrap();
    assert_eq!(
        ceremony.audit_entries(),
        vec![TallyAuditEntry::TallyOpened {
            board_name: SLUG_EVENT_BOARD.into(),
            tenant_id: TENANT.into(),
            election_event_id: EVENT.into(),
            election_ids: Some(vec![ELECTION.into()]),
            user_id: ADMIN_ID.into(),
            username: ADMIN.into(),
        }]
    );
}

#[tokio::test]
async fn the_status_is_written_before_env_slug_is_read() {
    let ceremony = ceremony(&CONNECTED, &restored_pair());
    ceremony.fail(TallyCall::EnvSlug, "missing env var ENV_SLUG");
    let error = change_status(&ceremony, tally_session(&CONNECTED), IN_PROGRESS)
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "missing env var ENV_SLUG");
    assert_eq!(stored_status(&ceremony), Some(IN_PROGRESS.to_string()));
    assert_eq!(ceremony.audit_entries(), vec![]);
}

#[tokio::test]
async fn a_tally_open_entry_that_cannot_be_posted_fails_the_change() {
    let ceremony = ceremony(&CONNECTED, &restored_pair());
    ceremony.fail(TallyCall::TallyOpened, "electoral log unavailable");
    let error = change_status(&ceremony, tally_session(&CONNECTED), IN_PROGRESS)
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "electoral log unavailable");
}

#[tokio::test]
async fn restoring_the_stored_key_marks_the_trustee_restored() {
    let ceremony = ceremony(&STARTED, &waiting_pair());
    let restored = restore_key(&ceremony, &trustee_claims(Some("alice")), STORED_KEY)
        .await
        .unwrap();
    assert!(restored);

    let executions = ceremony.executions(SESSION);
    assert_eq!(executions.len(), 2);
    let appended = executions.last().unwrap();
    assert_eq!(appended.current_message_id, LAST_MESSAGE_ID);
    assert_eq!(appended.run_reason.as_deref(), Some("NORMAL"));
    let status = last_ceremony_status(&ceremony);
    assert_eq!(
        trustee_statuses(&status),
        vec![
            ("alice".to_string(), TallyTrusteeStatus::KEY_RESTORED),
            ("bob".to_string(), TallyTrusteeStatus::WAITING),
        ]
    );
    assert_eq!(
        log_texts(&status),
        vec![
            "Created Tally Ceremony",
            "Restored private key for trustee alice"
        ]
    );
    assert_eq!(status.elections_status.len(), 1);
    assert_eq!(
        status.elections_status[0].status,
        TallyElectionStatus::SUCCESS
    );
    // One of the two keys the keys ceremony requires is not enough.
    assert_eq!(stored_status(&ceremony), Some(STARTED.to_string()));
}

#[tokio::test]
async fn restoring_a_key_that_does_not_match_the_board_writes_nothing() {
    let ceremony = ceremony(&STARTED, &waiting_pair());
    let restored = restore_key(&ceremony, &trustee_claims(Some("alice")), "another-key")
        .await
        .unwrap();
    assert!(!restored);
    assert_nothing_written(&ceremony, &STARTED);
}

#[tokio::test]
async fn the_session_connects_when_the_keys_ceremony_threshold_is_reached() {
    let trustees = [
        ("alice", TallyTrusteeStatus::KEY_RESTORED),
        ("bob", TallyTrusteeStatus::WAITING),
    ];
    let ceremony = ceremony(&STARTED, &trustees);
    restore_key(&ceremony, &trustee_claims(Some("bob")), STORED_KEY)
        .await
        .unwrap();
    assert_eq!(stored_status(&ceremony), Some(CONNECTED.to_string()));
    assert!(!ceremony.session(SESSION).is_execution_completed);
}

#[tokio::test]
async fn the_connection_threshold_is_the_keys_ceremony_threshold() {
    // The session asks for three trustees, the keys ceremony for one.
    let ceremony = InMemoryTallyCeremony::default();
    let mut tally_session = tally_session(&STARTED);
    tally_session.threshold = 3;
    ceremony.add_session(tally_session);
    ceremony.add_execution(execution(Some(&ceremony_status(&waiting_pair()))));
    ceremony.add_keys_ceremony(keys_ceremony(1));
    ceremony.add_private_key(KEYS_CEREMONY, "alice", STORED_KEY);
    ceremony.add_election_event(election_event(Some(
        json!({"id": 1, "database_name": EVENT_BOARD, "is_archived": false}),
    )));
    restore_key(&ceremony, &trustee_claims(Some("alice")), STORED_KEY)
        .await
        .unwrap();
    assert_eq!(stored_status(&ceremony), Some(CONNECTED.to_string()));
}

#[tokio::test]
async fn a_connected_session_still_accepts_the_remaining_keys() {
    let trustees = [
        ("alice", TallyTrusteeStatus::KEY_RESTORED),
        ("bob", TallyTrusteeStatus::KEY_RESTORED),
        ("carol", TallyTrusteeStatus::WAITING),
    ];
    let ceremony = ceremony(&CONNECTED, &trustees);
    ceremony.add_private_key(KEYS_CEREMONY, "carol", STORED_KEY);
    let restored = restore_key(&ceremony, &trustee_claims(Some("carol")), STORED_KEY)
        .await
        .unwrap();
    assert!(restored);
    assert_eq!(stored_status(&ceremony), Some(CONNECTED.to_string()));
    assert_eq!(restored_trustee_count(&last_ceremony_status(&ceremony)), 3);
}

#[tokio::test]
async fn restoring_a_key_posts_a_key_insertion_entry_on_the_event_bulletin_board() {
    let ceremony = ceremony(&STARTED, &waiting_pair());
    restore_key(&ceremony, &trustee_claims(Some("alice")), STORED_KEY)
        .await
        .unwrap();
    assert_eq!(
        ceremony.audit_entries(),
        vec![TallyAuditEntry::KeyRestored {
            board_name: EVENT_BOARD.into(),
            tenant_id: TENANT.into(),
            election_event_id: EVENT.into(),
            election_ids: Some(vec![ELECTION.into()]),
            trustee_name: "alice".into(),
            user_id: TRUSTEE_USER_ID.into(),
            username: Some(TRUSTEE_LOGIN.into()),
        }]
    );
}

#[tokio::test]
async fn a_request_without_a_trustee_claim_is_refused() {
    let ceremony = ceremony(&STARTED, &waiting_pair());
    let error = restore_key(&ceremony, &trustee_claims(None), STORED_KEY)
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "trustee name not found");
    assert_nothing_written(&ceremony, &STARTED);
}

#[tokio::test]
async fn the_session_is_read_before_the_trustee_claim_is_checked() {
    let ceremony = InMemoryTallyCeremony::default();
    let error = restore_key(&ceremony, &trustee_claims(None), STORED_KEY)
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        format!("Tally Session {SESSION} not found")
    );
}

#[tokio::test]
async fn keys_cannot_be_restored_into_a_session_without_executions() {
    let ceremony = InMemoryTallyCeremony::default();
    ceremony.add_session(tally_session(&STARTED));
    let error = restore_key(&ceremony, &trustee_claims(Some("alice")), STORED_KEY)
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Can't find tally session or tally session execution"
    );
}

#[tokio::test]
async fn keys_can_only_be_restored_before_the_tally_starts() {
    for status in [IN_PROGRESS, AWAITING_INPUT, SUCCESS, CANCELLED] {
        let ceremony = ceremony(&status, &waiting_pair());
        let error = restore_key(&ceremony, &trustee_claims(Some("alice")), STORED_KEY)
            .await
            .unwrap_err();
        assert_eq!(error.to_string(), format!("Unexpected status {status}"));
        assert_nothing_written(&ceremony, &status);
    }
}

#[tokio::test]
async fn a_session_with_an_unknown_status_accepts_keys_like_a_started_one() {
    let ceremony = InMemoryTallyCeremony::default();
    let mut tally_session = tally_session(&STARTED);
    tally_session.execution_status = Some("PAUSED".into());
    ceremony.add_session(tally_session);
    ceremony.add_execution(execution(Some(&ceremony_status(&waiting_pair()))));
    ceremony.add_keys_ceremony(keys_ceremony(2));
    ceremony.add_private_key(KEYS_CEREMONY, "alice", STORED_KEY);
    ceremony.add_election_event(election_event(Some(
        json!({"id": 1, "database_name": EVENT_BOARD, "is_archived": false}),
    )));
    assert!(
        restore_key(&ceremony, &trustee_claims(Some("alice")), STORED_KEY)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn the_status_is_checked_before_the_keys_ceremony_is_read() {
    let ceremony = ceremony(&IN_PROGRESS, &waiting_pair());
    ceremony.fail(TallyCall::GetKeysCeremony, "keys ceremony unavailable");
    let error = restore_key(&ceremony, &trustee_claims(Some("alice")), STORED_KEY)
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "Unexpected status IN_PROGRESS");
}

#[tokio::test]
async fn the_keys_ceremony_is_read_before_the_trustee_is_checked() {
    let ceremony = ceremony(&STARTED, &waiting_pair());
    ceremony.fail(TallyCall::GetKeysCeremony, "keys ceremony unavailable");
    let error = restore_key(&ceremony, &trustee_claims(Some("mallory")), STORED_KEY)
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "keys ceremony unavailable");
}

#[tokio::test]
async fn a_trustee_outside_the_ceremony_cannot_restore_a_key() {
    let ceremony = ceremony(&STARTED, &waiting_pair());
    let error = restore_key(&ceremony, &trustee_claims(Some("mallory")), STORED_KEY)
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Trustee not part of the keys ceremony or has invalid state"
    );
    assert_nothing_written(&ceremony, &STARTED);
}

#[tokio::test]
async fn a_trustee_cannot_restore_their_key_twice() {
    let ceremony = ceremony(&STARTED, &[("alice", TallyTrusteeStatus::KEY_RESTORED)]);
    let error = restore_key(&ceremony, &trustee_claims(Some("alice")), STORED_KEY)
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "Unexpected trustee status KEY_RESTORED");
    assert_nothing_written(&ceremony, &STARTED);
}

#[tokio::test]
async fn a_key_that_cannot_be_read_from_the_board_writes_nothing() {
    let ceremony = ceremony(&STARTED, &waiting_pair());
    ceremony.fail(TallyCall::GetPrivateKey, "board unavailable");
    let error = restore_key(&ceremony, &trustee_claims(Some("alice")), STORED_KEY)
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "board unavailable");
    assert_nothing_written(&ceremony, &STARTED);
}

#[tokio::test]
async fn a_restored_key_is_stored_before_the_bulletin_board_is_resolved() {
    let trustees = [
        ("alice", TallyTrusteeStatus::KEY_RESTORED),
        ("bob", TallyTrusteeStatus::WAITING),
    ];
    let ceremony = InMemoryTallyCeremony::default();
    ceremony.add_session(tally_session(&STARTED));
    ceremony.add_execution(execution(Some(&ceremony_status(&trustees))));
    ceremony.add_keys_ceremony(keys_ceremony(2));
    ceremony.add_private_key(KEYS_CEREMONY, "bob", STORED_KEY);
    ceremony.add_election_event(election_event(None));
    let error = restore_key(&ceremony, &trustee_claims(Some("bob")), STORED_KEY)
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "missing bulletin board");
    assert_eq!(ceremony.executions(SESSION).len(), 2);
    assert_eq!(stored_status(&ceremony), Some(CONNECTED.to_string()));
    assert_eq!(ceremony.audit_entries(), vec![]);
}

#[tokio::test]
async fn completing_a_session_marks_it_successful_and_completed() {
    let ceremony = ceremony(&IN_PROGRESS, &restored_pair());
    complete(&ceremony).await.unwrap();
    let tally_session = ceremony.session(SESSION);
    assert_eq!(tally_session.execution_status, Some(SUCCESS.to_string()));
    assert!(tally_session.is_execution_completed);
}

#[tokio::test]
async fn completion_marks_post_processing_pending_and_preserves_other_annotations() {
    for (before, expected) in [
        (None, None),
        (
            Some(json!({})),
            Some(json!({"is_post_task_completed": false})),
        ),
        (
            Some(json!({"is_post_task_completed": true, "operator": "test"})),
            Some(json!({"is_post_task_completed": false, "operator": "test"})),
        ),
        (
            Some(json!([1])),
            Some(json!([1, {"is_post_task_completed": false}])),
        ),
        (
            Some(json!(null)),
            Some(json!([null, {"is_post_task_completed": false}])),
        ),
        (
            Some(json!(true)),
            Some(json!([true, {"is_post_task_completed": false}])),
        ),
    ] {
        let ceremony = InMemoryTallyCeremony::default();
        let mut session = tally_session(&IN_PROGRESS);
        session.annotations = before;
        ceremony.add_session(session);
        ceremony.add_election_event(election_event(Some(
            json!({"id": 1, "database_name": EVENT_BOARD, "is_archived": false}),
        )));
        complete(&ceremony).await.unwrap();
        assert_eq!(ceremony.session(SESSION).annotations, expected);
    }
}

#[tokio::test]
async fn completing_a_session_posts_a_tally_close_entry_naming_its_executer() {
    let ceremony = ceremony(&IN_PROGRESS, &restored_pair());
    complete(&ceremony).await.unwrap();
    assert_eq!(
        ceremony.audit_entries(),
        vec![TallyAuditEntry::TallyClosed {
            board_name: EVENT_BOARD.into(),
            tenant_id: TENANT.into(),
            election_event_id: EVENT.into(),
            election_ids: Some(vec![ELECTION.into()]),
            executer: TallyExecuter {
                user_id: Some(ADMIN_ID.into()),
                username: Some(ADMIN.into()),
            },
        }]
    );
}

#[tokio::test]
async fn a_session_without_annotations_closes_without_an_executer() {
    let ceremony = InMemoryTallyCeremony::default();
    let mut tally_session = tally_session(&IN_PROGRESS);
    tally_session.annotations = None;
    ceremony.add_session(tally_session);
    ceremony.add_election_event(election_event(Some(
        json!({"id": 1, "database_name": EVENT_BOARD, "is_archived": false}),
    )));
    complete(&ceremony).await.unwrap();
    assert!(matches!(
        ceremony.audit_entries().as_slice(),
        [TallyAuditEntry::TallyClosed { executer, .. }] if *executer == TallyExecuter::default()
    ));
}

#[tokio::test]
async fn an_error_marking_the_session_completed_is_ignored() {
    // Pinned as found: the error is swallowed, so the caller carries on as if
    // the session had completed.
    let ceremony = ceremony(&IN_PROGRESS, &restored_pair());
    ceremony.fail(TallyCall::MarkCompleted, "database unavailable");
    complete(&ceremony).await.unwrap();
    assert_eq!(stored_status(&ceremony), Some(IN_PROGRESS.to_string()));
    assert_eq!(ceremony.audit_entries(), vec![]);
}

#[tokio::test]
async fn the_election_event_is_read_before_the_completed_session() {
    let ceremony = ceremony(&IN_PROGRESS, &restored_pair());
    ceremony.fail(TallyCall::GetElectionEvent, "election event unavailable");
    ceremony.fail(TallyCall::GetSession, "tally session unavailable");
    let error = complete(&ceremony).await.unwrap_err();
    assert_eq!(error.to_string(), "election event unavailable");
}

#[tokio::test]
async fn a_completed_session_without_a_bulletin_board_fails_after_it_is_marked() {
    let ceremony = InMemoryTallyCeremony::default();
    ceremony.add_session(tally_session(&IN_PROGRESS));
    ceremony.add_election_event(election_event(None));
    let error = complete(&ceremony).await.unwrap_err();
    assert_eq!(error.to_string(), "missing bulletin board");
    assert!(ceremony.session(SESSION).is_execution_completed);
    assert_eq!(ceremony.audit_entries(), vec![]);
}

/// A session in `status` with one execution in which both trustees restored
/// their keys and the election was tallied.
fn finished_ceremony(
    status: &TallyExecutionStatus,
    is_execution_completed: bool,
) -> InMemoryTallyCeremony {
    let ceremony = InMemoryTallyCeremony::default();
    let mut tally_session = tally_session(status);
    tally_session.is_execution_completed = is_execution_completed;
    ceremony.add_session(tally_session);
    ceremony.add_execution(execution(Some(&ceremony_status(&restored_pair()))));
    ceremony
}

#[tokio::test]
async fn a_recount_appends_a_recount_execution_and_reopens_the_session() {
    let ceremony = finished_ceremony(&SUCCESS, true);
    let started = recount(&ceremony, &[ELECTION, "second"]).await.unwrap();
    assert!(started);

    let executions = ceremony.executions(SESSION);
    assert_eq!(executions.len(), 2);
    let appended = executions.last().unwrap();
    assert_eq!(appended.run_reason.as_deref(), Some("RECOUNT"));
    assert_eq!(appended.current_message_id, LAST_MESSAGE_ID);
    let status = last_ceremony_status(&ceremony);
    assert_eq!(
        status
            .elections_status
            .iter()
            .map(|election| (
                election.election_id.as_str(),
                election.status.clone(),
                election.progress
            ))
            .collect::<Vec<_>>(),
        vec![
            (ELECTION, TallyElectionStatus::WAITING, 0.0),
            ("second", TallyElectionStatus::WAITING, 0.0),
        ]
    );
    assert_eq!(trustee_statuses(&status).len(), 2);
    assert_eq!(restored_trustee_count(&status), 2);
    assert_eq!(
        log_texts(&status),
        vec![
            "Created Tally Ceremony",
            "Recount launched for election ids: [\"election\", \"second\"]",
        ]
    );
    let tally_session = ceremony.session(SESSION);
    assert_eq!(
        tally_session.execution_status,
        Some(IN_PROGRESS.to_string())
    );
    assert!(!tally_session.is_execution_completed);
}

#[tokio::test]
async fn only_a_successfully_completed_session_can_be_recounted() {
    for (status, is_execution_completed) in
        [(SUCCESS, false), (CANCELLED, true), (IN_PROGRESS, true)]
    {
        let ceremony = finished_ceremony(&status, is_execution_completed);
        assert!(!recount(&ceremony, &[ELECTION]).await.unwrap());
        assert_eq!(ceremony.executions(SESSION).len(), 1);
        assert_eq!(stored_status(&ceremony), Some(status.to_string()));
    }
}

#[tokio::test]
async fn a_recount_rechecks_the_session_after_taking_the_lock() {
    // Another recount reopened the session while this one waited for the lock.
    let ceremony = finished_ceremony(&SUCCESS, true);
    ceremony.on_lock(|tally_session| {
        tally_session.execution_status = Some(IN_PROGRESS.to_string());
        tally_session.is_execution_completed = false;
    });
    assert!(!recount(&ceremony, &[ELECTION]).await.unwrap());
    assert_eq!(ceremony.executions(SESSION).len(), 1);
    assert_eq!(stored_status(&ceremony), Some(IN_PROGRESS.to_string()));
}

#[tokio::test]
async fn a_session_without_executions_is_not_recounted() {
    let ceremony = InMemoryTallyCeremony::default();
    let mut tally_session = tally_session(&SUCCESS);
    tally_session.is_execution_completed = true;
    ceremony.add_session(tally_session);
    assert!(!recount(&ceremony, &[ELECTION]).await.unwrap());
    assert_eq!(ceremony.executions(SESSION), vec![]);
    assert_eq!(stored_status(&ceremony), Some(SUCCESS.to_string()));
}

#[tokio::test]
async fn a_session_that_cannot_be_locked_is_not_recounted() {
    let ceremony = finished_ceremony(&SUCCESS, true);
    ceremony.fail(TallyCall::LockSession, "lock timeout");
    let error = recount(&ceremony, &[ELECTION]).await.unwrap_err();
    assert_eq!(error.to_string(), "lock timeout");
    assert_eq!(ceremony.executions(SESSION).len(), 1);
    assert_eq!(stored_status(&ceremony), Some(SUCCESS.to_string()));
}

#[tokio::test]
async fn a_recount_of_an_execution_without_status_writes_nothing() {
    let ceremony = InMemoryTallyCeremony::default();
    let mut tally_session = tally_session(&SUCCESS);
    tally_session.is_execution_completed = true;
    ceremony.add_session(tally_session);
    ceremony.add_execution(execution(None));
    let error = recount(&ceremony, &[ELECTION]).await.unwrap_err();
    assert_eq!(error.to_string(), "Missing tally ceremony status");
    assert_eq!(ceremony.executions(SESSION).len(), 1);
    assert_eq!(stored_status(&ceremony), Some(SUCCESS.to_string()));
}
