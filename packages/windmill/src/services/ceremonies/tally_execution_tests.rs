// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::*;
use crate::adapters::memory::tally_execution::*;
use crate::domain::tally_execution::*;
use crate::types::error::Error;
use sequent_core::types::ceremonies::{
    Log, TallySessionDocuments, TallyTrustee, TallyTrusteeStatus,
};
use serde_json::json;

fn ceremony(threshold: i64, automated: bool) -> KeysCeremony {
    serde_json::from_value(json!({"id":"keys-a", "tenant_id":"tenant-a", "election_event_id":"event-a", "trustee_ids":["a","b","c"], "threshold":threshold, "settings":{"policy":if automated {"automated-ceremonies"} else {"manual-ceremonies"}}})).unwrap()
}
fn status() -> TallyCeremonyStatus {
    TallyCeremonyStatus {
        trustees: vec![
            TallyTrustee {
                name: "a".into(),
                status: TallyTrusteeStatus::KEY_RESTORED,
            },
            TallyTrustee {
                name: "b".into(),
                status: TallyTrusteeStatus::WAITING,
            },
            TallyTrustee {
                name: "c".into(),
                status: TallyTrusteeStatus::KEY_RESTORED,
            },
        ],
        ..Default::default()
    }
}
async fn select(threshold: i64, automated: bool) -> TrusteeSelection {
    let store = MemoryTallyExecution::default();
    let keys = ceremony(threshold, automated);
    store.0.lock().unwrap().ceremonies.push(keys.clone());
    select_execution_trustees_with(
        &store,
        &ReverseTrusteeOrder::default(),
        "tenant-a",
        "event-a",
        &keys,
        status(),
    )
    .await
    .unwrap()
}
#[tokio::test]
async fn manual_execution_selects_only_restored_trustees_at_the_exact_threshold() {
    assert_eq!(
        select(2, false).await,
        TrusteeSelection::Ready(vec!["c".into(), "a".into()])
    );
}
#[tokio::test]
async fn automated_execution_can_select_waiting_trustees() {
    assert_eq!(
        select(2, true).await,
        TrusteeSelection::Ready(vec!["c".into(), "b".into()])
    );
}
#[tokio::test]
async fn insufficient_restored_trustees_prevent_execution() {
    assert_eq!(
        select(3, false).await,
        TrusteeSelection::Insufficient {
            available: 2,
            threshold: 3
        }
    );
}
#[tokio::test]
async fn trustee_selection_shuffles_the_eligible_set_before_truncation() {
    let store = MemoryTallyExecution::default();
    let order = ReverseTrusteeOrder::default();
    let keys = ceremony(1, false);
    store.0.lock().unwrap().ceremonies.push(keys.clone());
    assert_eq!(
        select_execution_trustees_with(&store, &order, "tenant-a", "event-a", &keys, status())
            .await
            .unwrap(),
        TrusteeSelection::Ready(vec!["c".into()])
    );
    assert_eq!(
        *order.0.lock().unwrap(),
        vec![vec!["a".to_string(), "c".into()]]
    );
    assert_eq!(
        store.0.lock().unwrap().reads,
        vec![("tenant-a".into(), "event-a".into())]
    );
}
#[tokio::test]
async fn zero_threshold_preserves_an_empty_ready_selection() {
    assert_eq!(select(0, false).await, TrusteeSelection::Ready(vec![]));
}
#[tokio::test]
async fn missing_ceremonies_do_not_shuffle_trustees() {
    let store = MemoryTallyExecution::default();
    let order = ReverseTrusteeOrder::default();
    assert_eq!(
        select_execution_trustees_with(
            &store,
            &order,
            "tenant-a",
            "event-a",
            &ceremony(2, false),
            status()
        )
        .await
        .unwrap(),
        TrusteeSelection::NoCeremony
    );
    assert!(order.0.lock().unwrap().is_empty());
}
#[tokio::test]
async fn ceremony_read_errors_keep_the_original_context() {
    let store = MemoryTallyExecution::default();
    store.0.lock().unwrap().read_failure = Some("connection unavailable");
    let err = select_execution_trustees_with(
        &store,
        &ReverseTrusteeOrder::default(),
        "tenant-a",
        "event-a",
        &ceremony(2, false),
        status(),
    )
    .await
    .unwrap_err();
    assert_eq!(
        format!("{err:#}"),
        "error listing existing keys ceremonies: connection unavailable"
    );
}
#[test]
fn new_board_selection_uses_first_unprocessed_message_in_board_order() {
    assert_eq!(
        board_message_plan(&[2, 8, 6, 12], 5, false),
        BoardMessagePlan::New(1)
    );
}
#[test]
fn an_equal_message_id_is_already_processed() {
    assert_eq!(
        board_message_plan(&[2, 5], 5, false),
        BoardMessagePlan::Wait
    );
}
#[test]
fn a_replay_still_prefers_new_messages() {
    assert_eq!(
        board_message_plan(&[2, 8, 12], 5, true),
        BoardMessagePlan::New(1)
    );
}
#[test]
fn processed_board_messages_request_replay_when_enabled() {
    assert_eq!(
        board_message_plan(&[8, 2, 5], 9, true),
        BoardMessagePlan::Replay
    );
}
#[test]
fn an_empty_board_without_replay_waits() {
    assert_eq!(board_message_plan(&[], -1, false), BoardMessagePlan::Wait);
}
#[test]
fn an_empty_replay_is_still_attempted() {
    assert_eq!(board_message_plan(&[], -1, true), BoardMessagePlan::Replay);
}

#[test]
fn completion_retains_exact_count_equality_including_empty_and_duplicate_plaintexts() {
    for (plaintexts, batches, expected) in [
        (0, 0, true),
        (0, 1, false),
        (1, 1, true),
        (2, 1, false),
        (1, 2, false),
        (2, 2, true),
    ] {
        assert_eq!(execution_is_complete(plaintexts, batches), expected);
    }
}
#[test]
fn pending_ties_pause_even_a_complete_execution() {
    for complete in [false, true] {
        assert_eq!(
            execution_conclusion(true, complete),
            ExecutionConclusion::AwaitingInput
        );
    }
    assert_eq!(
        execution_conclusion(false, false),
        ExecutionConclusion::InProgress
    );
    assert_eq!(
        execution_conclusion(false, true),
        ExecutionConclusion::Completed
    );
}
fn scope() -> ExecutionScope {
    ExecutionScope {
        tenant_id: "tenant-a".into(),
        election_event_id: "event-a".into(),
        tally_session_id: "session-a".into(),
    }
}
fn record() -> ExecutionRecord {
    let mut status = status();
    status.logs = vec![Log {
        created_date: "2025-12-31T00:00:00.000Z".into(),
        log_text: "prior log".into(),
    }];
    ExecutionRecord {
        current_message_id: 42,
        status,
        results_event_id: Some("results-a".into()),
        session_ids: Some(vec![7, 8]),
        documents: Some(TallySessionDocuments {
            sqlite: Some("sqlite-a".into()),
            xlsx: Some("xlsx-a".into()),
        }),
        run_reason: TallyRunReason::RECOUNT,
    }
}
async fn persist(
    store: &MemoryTallyExecution,
    conclusion: ExecutionConclusion,
    tally_type: TallyType,
) -> Result<()> {
    persist_execution_with(
        store,
        &FixedExecutionLogs,
        &scope(),
        record(),
        conclusion,
        tally_type,
        vec!["election-a".into(), "election-b".into()],
    )
    .await
}
fn assert_record(state: &ExecutionState, conclusion: &str) {
    assert_eq!(state.records.len(), 1);
    assert_eq!(state.records[0].0, scope());
    let record = &state.records[0].1;
    assert_eq!(record.current_message_id, 42);
    assert_eq!(record.results_event_id.as_deref(), Some("results-a"));
    assert_eq!(record.session_ids, Some(vec![7, 8]));
    assert_eq!(
        record.documents,
        Some(TallySessionDocuments {
            sqlite: Some("sqlite-a".into()),
            xlsx: Some("xlsx-a".into())
        })
    );
    assert_eq!(record.run_reason, TallyRunReason::NORMAL);
    assert_eq!(record.status.trustees.len(), 3);
    assert_eq!(record.status.logs.len(), 2);
    assert_eq!(record.status.logs[0].log_text, "prior log");
    assert_eq!(
        record.status.logs[1].log_text,
        format!("{conclusion}: [\"election-a\", \"election-b\"]")
    );
}
#[tokio::test]
async fn pending_ties_store_partial_results_and_pause_without_completing() {
    let store = MemoryTallyExecution::default();
    persist(
        &store,
        ExecutionConclusion::AwaitingInput,
        TallyType::INITIALIZATION_REPORT,
    )
    .await
    .unwrap();
    let state = store.0.lock().unwrap();
    assert_record(&state, "AwaitingInput");
    assert_eq!(state.awaiting_input, vec![scope()]);
    assert!(state.completed.is_empty());
    assert!(state.refreshed_events.is_empty());
    assert!(state.initialized.is_empty());
}
#[tokio::test]
async fn incomplete_execution_only_appends_its_progress_snapshot() {
    let store = MemoryTallyExecution::default();
    persist(
        &store,
        ExecutionConclusion::InProgress,
        TallyType::INITIALIZATION_REPORT,
    )
    .await
    .unwrap();
    let state = store.0.lock().unwrap();
    assert_record(&state, "InProgress");
    assert!(state.awaiting_input.is_empty());
    assert!(state.completed.is_empty());
    assert!(state.refreshed_events.is_empty());
    assert!(state.initialized.is_empty());
}
#[tokio::test]
async fn completed_electoral_results_refresh_the_event_without_marking_initialization() {
    let store = MemoryTallyExecution::default();
    persist(
        &store,
        ExecutionConclusion::Completed,
        TallyType::ELECTORAL_RESULTS,
    )
    .await
    .unwrap();
    let state = store.0.lock().unwrap();
    assert_record(&state, "Completed");
    assert_eq!(state.completed, vec![scope()]);
    assert_eq!(state.refreshed_events, vec![scope()]);
    assert!(state.awaiting_input.is_empty());
    assert!(state.initialized.is_empty());
}
#[tokio::test]
async fn completed_initialization_marks_each_requested_election() {
    let store = MemoryTallyExecution::default();
    persist(
        &store,
        ExecutionConclusion::Completed,
        TallyType::INITIALIZATION_REPORT,
    )
    .await
    .unwrap();
    assert_eq!(
        store.0.lock().unwrap().initialized,
        vec![
            (scope(), "election-a".into()),
            (scope(), "election-b".into())
        ]
    );
}
#[tokio::test]
async fn absent_result_documents_and_batches_remain_absent_in_the_snapshot() {
    let store = MemoryTallyExecution::default();
    let mut row = record();
    row.results_event_id = None;
    row.documents = None;
    row.session_ids = None;
    persist_execution_with(
        &store,
        &FixedExecutionLogs,
        &scope(),
        row,
        ExecutionConclusion::InProgress,
        TallyType::ELECTORAL_RESULTS,
        vec![],
    )
    .await
    .unwrap();
    let state = store.0.lock().unwrap();
    let row = &state.records[0].1;
    assert!(row.results_event_id.is_none());
    assert!(row.documents.is_none());
    assert!(row.session_ids.is_none());
    assert_eq!(row.status.logs[1].log_text, "InProgress: []");
}
#[tokio::test]
async fn insert_failure_prevents_both_pause_and_completion_writes() {
    for conclusion in [
        ExecutionConclusion::AwaitingInput,
        ExecutionConclusion::Completed,
    ] {
        let store = MemoryTallyExecution::default();
        store.0.lock().unwrap().failure = Some((Write::Insert, 1));
        assert!(
            matches!(persist(&store,conclusion,TallyType::INITIALIZATION_REPORT).await,Err(Error::String(s)) if s=="injected Insert failure")
        );
        let state = store.0.lock().unwrap();
        assert!(state.records.is_empty());
        assert!(state.awaiting_input.is_empty());
        assert!(state.completed.is_empty());
        assert!(state.refreshed_events.is_empty());
        assert!(state.initialized.is_empty());
    }
}
#[tokio::test]
async fn pause_failure_keeps_the_inserted_partial_snapshot() {
    let store = MemoryTallyExecution::default();
    store.0.lock().unwrap().failure = Some((Write::AwaitInput, 1));
    assert!(
        matches!(persist(&store,ExecutionConclusion::AwaitingInput,TallyType::INITIALIZATION_REPORT).await,Err(Error::String(s)) if s=="injected AwaitInput failure")
    );
    let state = store.0.lock().unwrap();
    assert_record(&state, "AwaitingInput");
    assert!(state.awaiting_input.is_empty());
    assert!(state.completed.is_empty());
    assert!(state.refreshed_events.is_empty());
}
#[tokio::test]
async fn completion_failure_prevents_refreshing_the_event() {
    let store = MemoryTallyExecution::default();
    store.0.lock().unwrap().failure = Some((Write::Complete, 1));
    assert!(
        matches!(persist(&store,ExecutionConclusion::Completed,TallyType::INITIALIZATION_REPORT).await,Err(Error::String(s)) if s=="injected Complete failure")
    );
    let state = store.0.lock().unwrap();
    assert_record(&state, "Completed");
    assert!(state.completed.is_empty());
    assert!(state.refreshed_events.is_empty());
    assert!(state.initialized.is_empty());
}
#[tokio::test]
async fn event_refresh_failure_prevents_initialization_updates() {
    let store = MemoryTallyExecution::default();
    store.0.lock().unwrap().failure = Some((Write::RefreshEvent, 1));
    assert!(
        matches!(persist(&store,ExecutionConclusion::Completed,TallyType::INITIALIZATION_REPORT).await,Err(Error::String(s)) if s=="injected RefreshEvent failure")
    );
    let state = store.0.lock().unwrap();
    assert_eq!(state.completed, vec![scope()]);
    assert!(state.refreshed_events.is_empty());
    assert!(state.initialized.is_empty());
}
#[tokio::test]
async fn initialization_failure_stops_after_previously_marked_elections() {
    let store = MemoryTallyExecution::default();
    store.0.lock().unwrap().failure = Some((Write::Initialization, 2));
    assert!(
        matches!(persist(&store,ExecutionConclusion::Completed,TallyType::INITIALIZATION_REPORT).await,Err(Error::String(s)) if s=="injected Initialization failure")
    );
    let state = store.0.lock().unwrap();
    assert_eq!(state.completed, vec![scope()]);
    assert_eq!(state.refreshed_events, vec![scope()]);
    assert_eq!(state.initialized, vec![(scope(), "election-a".into())]);
}

#[tokio::test]
async fn trustee_threshold_comes_from_the_linked_ceremony() {
    for (other_threshold, linked_threshold, expected) in [
        (3, 2, TrusteeSelection::Ready(vec!["c".into(), "a".into()])),
        (1, 2, TrusteeSelection::Ready(vec!["c".into(), "a".into()])),
        (
            2,
            3,
            TrusteeSelection::Insufficient {
                available: 2,
                threshold: 3,
            },
        ),
    ] {
        let store = MemoryTallyExecution::default();
        let linked = ceremony(linked_threshold, false);
        let mut other = ceremony(other_threshold, false);
        other.id = "unrelated-ceremony".into();
        store.0.lock().unwrap().ceremonies = vec![other, linked.clone()];
        let selection = select_execution_trustees_with(
            &store,
            &ReverseTrusteeOrder::default(),
            "tenant-a",
            "event-a",
            &linked,
            status(),
        )
        .await
        .unwrap();
        assert_eq!(
            selection, expected,
            "unrelated threshold {other_threshold}, linked threshold {linked_threshold}"
        );
    }
}
