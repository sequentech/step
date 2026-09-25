// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::adapters::memory::clock::{FixedClock, SequentialIds};
use crate::adapters::memory::datafix_cast_vote::{
    ConcurrentChange, DatafixAuditEntry, InMemoryDatafixAudit, InMemoryDatafixElectionEvents,
    InMemoryDatafixVoterDirectory, InMemoryDatafixVoterLocks, InMemoryDatafixVotes,
    InMemoryVoterView, LockAcquisition, VoterViewReply, ANOTHER_OPERATION, LOCK_HELD_ELSEWHERE,
    SET_VOTED_TEMPLATE_SHA256,
};
use crate::services::external::datafix_types::SoapRequestResponse;
use chrono::{DateTime, Local, TimeZone, Utc};
use sequent_core::types::keycloak::{User, VOTED_CHANNEL};
use serde_json::{json, Value};
use std::collections::HashMap;

const TENANT_ID: &str = "10000000-0000-4000-8000-000000000001";
const ELECTION_EVENT_ID: &str = "10000000-0000-4000-8000-000000000002";
const CAST_VOTE_ID: &str = "10000000-0000-4000-8000-000000000003";
const EARLIER_CAST_VOTE_ID: &str = "10000000-0000-4000-8000-000000000004";
const VOTER_ID: &str = "10000000-0000-4000-8000-000000000005";
const USERNAME: &str = "voter-username";

type TestProcessor = DatafixCastVoteProcessor<
    InMemoryDatafixVotes,
    InMemoryDatafixElectionEvents,
    InMemoryDatafixVoterDirectory,
    InMemoryVoterView,
    InMemoryDatafixVoterLocks,
    InMemoryDatafixAudit,
    FixedClock,
    SequentialIds,
>;

fn now() -> DateTime<Local> {
    Utc.with_ymd_and_hms(2026, 3, 2, 9, 30, 0)
        .unwrap()
        .with_timezone(&Local)
}

fn realm() -> String {
    format!("tenant-{TENANT_ID}-event-{ELECTION_EVENT_ID}")
}

fn lock_key() -> String {
    format!("datafix-voter-{TENANT_ID}-{ELECTION_EVENT_ID}-{VOTER_ID}")
}

fn cast_vote(id: &str, status: CastVoteStatus) -> CastVote {
    CastVote {
        id: id.to_string(),
        tenant_id: TENANT_ID.to_string(),
        election_id: None,
        area_id: None,
        created_at: None,
        last_updated_at: None,
        content: None,
        voter_id_string: Some(VOTER_ID.to_string()),
        election_event_id: ELECTION_EVENT_ID.to_string(),
        ballot_id: None,
        cast_ballot_signature: None,
        status,
    }
}

fn election_event(annotations: Option<Value>) -> ElectionEvent {
    ElectionEvent {
        id: ELECTION_EVENT_ID.to_string(),
        created_at: None,
        updated_at: None,
        labels: None,
        annotations,
        tenant_id: TENANT_ID.to_string(),
        description: None,
        presentation: None,
        bulletin_board_reference: None,
        is_archived: false,
        voting_channels: None,
        status: None,
        user_boards: None,
        encryption_protocol: "protocol".to_string(),
        is_audit: None,
        audit_election_event_id: None,
        public_key: None,
        statistics: None,
        external_id: None,
    }
}

fn datafix_configuration() -> Value {
    json!({
        "datafix:id": "external-event",
        "datafix:password_policy": r#"{"base":"password-only","size":6,"characters":"numeric"}"#,
        "datafix:voterview_request": r#"{"url":"https://voterview.invalid","usr":"user","psw":"secret","county_mun":"county"}"#
    })
}

fn voter(enabled: Option<bool>, voted_channel: Option<&str>) -> User {
    User {
        id: Some(VOTER_ID.to_string()),
        username: Some(USERNAME.to_string()),
        enabled,
        attributes: voted_channel
            .map(|channel| HashMap::from([(VOTED_CHANNEL.to_string(), vec![channel.to_string()])])),
        ..Default::default()
    }
}

/// An in-progress vote in a Datafix event, cast by an enabled voter with no
/// voted channel and no other votes, and a VoterView that accepts `SetVoted`.
fn processor() -> TestProcessor {
    let processor = DatafixCastVoteProcessor {
        votes: InMemoryDatafixVotes::default(),
        election_events: InMemoryDatafixElectionEvents::default(),
        voters: InMemoryDatafixVoterDirectory::default(),
        voter_view: InMemoryVoterView::default(),
        locks: InMemoryDatafixVoterLocks::default(),
        audit: InMemoryDatafixAudit::default(),
        clock: FixedClock::at(now()),
        ids: SequentialIds::default(),
    };
    processor
        .votes
        .insert(cast_vote(CAST_VOTE_ID, CastVoteStatus::InProgress));
    processor
        .election_events
        .insert(election_event(Some(datafix_configuration())));
    processor.voters.insert(&realm(), voter(Some(true), None));
    processor
}

async fn process(processor: &TestProcessor) -> Result<()> {
    processor
        .process(TENANT_ID, ELECTION_EVENT_ID, CAST_VOTE_ID)
        .await
}

/// Celery reports these errors by their message, so they must stay plain
/// messages.
fn error_message(result: Result<()>) -> String {
    match result {
        Err(Error::String(message)) => message,
        other => panic!("expected a plain error message, got {other:?}"),
    }
}

fn status(processor: &TestProcessor) -> Option<CastVoteStatus> {
    processor.votes.status(CAST_VOTE_ID)
}

fn audited(operation: &str) -> String {
    format!("cast_vote_id={CAST_VOTE_ID}; {operation}")
}

fn audit_entry(operation: String) -> DatafixAuditEntry {
    DatafixAuditEntry {
        tenant_id: TENANT_ID.to_string(),
        election_event_id: ELECTION_EVENT_ID.to_string(),
        voter_id: VOTER_ID.to_string(),
        username: USERNAME.to_string(),
        operation,
    }
}

fn audit_operations(processor: &TestProcessor) -> Vec<String> {
    let entries = processor.audit.entries();
    entries.into_iter().map(|entry| entry.operation).collect()
}

fn set_voted_operation(outcome: &str) -> String {
    audited(&format!(
        "{outcome} (template_sha256={SET_VOTED_TEMPLATE_SHA256})"
    ))
}

#[tokio::test]
async fn a_vote_that_no_longer_exists_is_skipped() {
    let mut processor = processor();
    processor.votes = InMemoryDatafixVotes::default();

    process(&processor).await.unwrap();

    assert!(processor.locks.acquisitions().is_empty());
}

#[tokio::test]
async fn a_vote_that_is_no_longer_in_progress_is_skipped_without_taking_the_voter_lock() {
    for resolved in [CastVoteStatus::Valid, CastVoteStatus::Discarded] {
        let processor = processor();
        processor.votes.insert(cast_vote(CAST_VOTE_ID, resolved));

        process(&processor).await.unwrap();

        assert_eq!(status(&processor), Some(resolved));
        assert!(processor.locks.acquisitions().is_empty(), "{resolved}");
    }
}

#[tokio::test]
async fn a_vote_resolved_while_waiting_for_the_voter_lock_is_left_alone() {
    let processor = processor();
    processor
        .votes
        .change_concurrently(ConcurrentChange::ResolvedAfterNextLoad(
            CastVoteStatus::Discarded,
        ));

    process(&processor).await.unwrap();

    assert_eq!(status(&processor), Some(CastVoteStatus::Discarded));
    assert!(processor.voter_view.sent().is_empty());
    assert!(processor.audit.entries().is_empty());
    assert_eq!(processor.locks.holder(&lock_key()), None);
}

#[tokio::test]
async fn a_vote_deleted_while_waiting_for_the_voter_lock_is_skipped() {
    let processor = processor();
    processor
        .votes
        .change_concurrently(ConcurrentChange::DeletedAfterNextLoad);

    process(&processor).await.unwrap();

    assert!(processor.voter_view.sent().is_empty());
    assert_eq!(processor.locks.holder(&lock_key()), None);
}

#[tokio::test]
async fn a_malformed_cast_vote_id_is_rejected() {
    let processor = processor();
    let parse_error = Uuid::parse_str("cast-vote").unwrap_err();

    let result = processor
        .process(TENANT_ID, ELECTION_EVENT_ID, "cast-vote")
        .await;

    assert_eq!(
        error_message(result),
        format!("Invalid cast_vote_id: {parse_error}")
    );
}

#[tokio::test]
async fn a_vote_without_a_voter_id_is_rejected_before_taking_the_voter_lock() {
    let processor = processor();
    processor.votes.insert(CastVote {
        voter_id_string: None,
        ..cast_vote(CAST_VOTE_ID, CastVoteStatus::InProgress)
    });

    assert_eq!(
        error_message(process(&processor).await),
        "Voter id not found"
    );
    assert!(processor.locks.acquisitions().is_empty());
}

#[tokio::test]
async fn a_vote_with_a_malformed_voter_id_is_rejected_before_taking_the_voter_lock() {
    let processor = processor();
    processor.votes.insert(CastVote {
        voter_id_string: Some(USERNAME.to_string()),
        ..cast_vote(CAST_VOTE_ID, CastVoteStatus::InProgress)
    });
    let parse_error = Uuid::parse_str(USERNAME).unwrap_err();

    assert_eq!(
        error_message(process(&processor).await),
        format!("Invalid voter id: {parse_error}")
    );
    assert!(processor.locks.acquisitions().is_empty());
}

#[tokio::test]
async fn a_vote_that_cannot_be_loaded_fails_the_task_with_the_store_error() {
    let processor = processor();
    processor
        .votes
        .fail_load("Error loading cast vote: connection reset");

    assert_eq!(
        error_message(process(&processor).await),
        "Error loading cast vote: connection reset"
    );
}

#[tokio::test]
async fn the_voter_lock_is_taken_per_event_voter_for_300_seconds_and_released() {
    let processor = processor();

    process(&processor).await.unwrap();

    assert_eq!(
        processor.locks.acquisitions(),
        vec![LockAcquisition {
            key: lock_key(),
            value: Uuid::from_u128(1).to_string(),
            expiry_date: now() + Duration::seconds(300),
        }]
    );
    assert_eq!(processor.locks.holder(&lock_key()), None);
}

#[tokio::test]
async fn a_voter_locked_by_another_operation_leaves_the_vote_for_a_later_beat() {
    let processor = processor();
    processor.locks.hold_for_another_operation(&lock_key());

    process(&processor).await.unwrap();

    assert_eq!(status(&processor), Some(CastVoteStatus::InProgress));
    assert!(processor.voter_view.sent().is_empty());
    assert_eq!(
        processor.locks.holder(&lock_key()).as_deref(),
        Some(ANOTHER_OPERATION)
    );
}

#[tokio::test]
async fn the_voter_lock_is_released_when_processing_fails() {
    let processor = processor();
    processor
        .voter_view
        .reply_with(VoterViewReply::NotDispatched(
            "connection refused".to_string(),
        ));

    assert!(process(&processor).await.is_err());
    assert_eq!(processor.locks.holder(&lock_key()), None);
}

#[tokio::test]
async fn a_lock_release_failure_is_reported_after_the_vote_is_resolved() {
    let processor = processor();
    processor.locks.fail_release("lock table unavailable");

    assert_eq!(
        error_message(process(&processor).await),
        "Error releasing Datafix voter lock: lock table unavailable"
    );
    assert_eq!(status(&processor), Some(CastVoteStatus::Valid));
}

#[tokio::test]
async fn a_processing_error_is_reported_instead_of_a_lock_release_failure() {
    let processor = processor();
    processor
        .voter_view
        .reply_with(VoterViewReply::Ambiguous("read timed out".to_string()));
    processor.locks.fail_release("lock table unavailable");

    assert_eq!(
        error_message(process(&processor).await),
        "VoterView SetVoted outcome is ambiguous; the vote stays in-progress: read timed out"
    );
}

#[tokio::test]
async fn losing_the_voter_lock_after_the_keycloak_lookup_leaves_the_vote_in_progress() {
    let processor = processor();
    // Had the lock been renewed, this disabled voter's vote would be discarded.
    processor.voters.insert(&realm(), voter(Some(false), None));
    processor.locks.lose_before_renewal(1);

    assert_eq!(
        error_message(process(&processor).await),
        format!("Datafix voter lock was lost after Keycloak lookup: {LOCK_HELD_ELSEWHERE}")
    );
    assert_eq!(status(&processor), Some(CastVoteStatus::InProgress));
    assert!(processor.audit.entries().is_empty());
    assert_eq!(
        processor.locks.holder(&lock_key()).as_deref(),
        Some(ANOTHER_OPERATION)
    );
}

#[tokio::test]
async fn losing_the_voter_lock_before_set_voted_leaves_the_vote_in_progress_and_unsent() {
    let processor = processor();
    processor.locks.lose_before_renewal(2);

    assert_eq!(
        error_message(process(&processor).await),
        format!("Datafix voter lock was lost before SetVoted: {LOCK_HELD_ELSEWHERE}")
    );
    assert_eq!(status(&processor), Some(CastVoteStatus::InProgress));
    assert!(processor.voter_view.sent().is_empty());
    assert!(processor.audit.entries().is_empty());
}

#[tokio::test]
async fn a_vote_in_an_event_without_datafix_configuration_stays_in_progress() {
    let processor = processor();
    processor.election_events.insert(election_event(None));

    assert_eq!(
        error_message(process(&processor).await),
        "Cast vote is pending but the election event is not configured for Datafix"
    );
    assert_eq!(status(&processor), Some(CastVoteStatus::InProgress));
}

#[tokio::test]
async fn a_vote_in_an_event_with_incomplete_datafix_configuration_stays_in_progress() {
    let processor = processor();
    processor.election_events.insert(election_event(Some(
        json!({"datafix:id": "external-event"}),
    )));

    assert_eq!(
        error_message(process(&processor).await),
        "Invalid Datafix configuration: Invalid Datafix election event configuration: \
         datafix:password_policy not found"
    );
    assert_eq!(status(&processor), Some(CastVoteStatus::InProgress));
}

#[tokio::test]
async fn a_vote_whose_election_event_is_missing_stays_in_progress() {
    let mut processor = processor();
    processor.election_events = InMemoryDatafixElectionEvents::default();

    assert_eq!(
        error_message(process(&processor).await),
        format!("Election event {ELECTION_EVENT_ID} not found")
    );
    assert_eq!(status(&processor), Some(CastVoteStatus::InProgress));
}

#[tokio::test]
async fn a_voter_missing_from_the_event_realm_leaves_the_vote_in_progress() {
    let mut processor = processor();
    processor.voters = InMemoryDatafixVoterDirectory::default();
    processor
        .voters
        .insert("tenant-other-event-other", voter(Some(true), None));

    assert_eq!(
        error_message(process(&processor).await),
        format!("Voter {VOTER_ID} not found in realm {}", realm())
    );
    assert_eq!(status(&processor), Some(CastVoteStatus::InProgress));
}

#[tokio::test]
async fn a_voter_without_a_username_leaves_the_vote_in_progress() {
    let processor = processor();
    // Disabled, so the vote would be discarded if the username were not needed first.
    processor.voters.insert(
        &realm(),
        User {
            username: None,
            ..voter(Some(false), None)
        },
    );

    assert_eq!(error_message(process(&processor).await), "Username is None");
    assert_eq!(status(&processor), Some(CastVoteStatus::InProgress));
}

#[tokio::test]
async fn a_disabled_voter_has_the_vote_discarded_without_voterview() {
    let processor = processor();
    processor.voters.insert(&realm(), voter(Some(false), None));

    process(&processor).await.unwrap();

    assert_eq!(status(&processor), Some(CastVoteStatus::Discarded));
    assert!(processor.voter_view.sent().is_empty());
    assert!(processor.voters.marked_via_internet().is_empty());
    assert_eq!(
        processor.audit.entries(),
        vec![audit_entry(audited(
            "SetVoted Skipped: voter is disabled or marked via another channel"
        ))]
    );
}

#[tokio::test]
async fn a_voter_marked_via_another_channel_has_the_vote_discarded() {
    let processor = processor();
    processor
        .voters
        .insert(&realm(), voter(Some(true), Some("Paper")));

    process(&processor).await.unwrap();

    assert_eq!(status(&processor), Some(CastVoteStatus::Discarded));
    assert!(processor.voter_view.sent().is_empty());
}

#[tokio::test]
async fn a_discard_after_a_concurrent_resolution_is_audited_as_ignored() {
    let processor = processor();
    processor.voters.insert(&realm(), voter(Some(false), None));
    processor
        .votes
        .change_concurrently(ConcurrentChange::ResolvedBeforeNextCompareAndSet(
            CastVoteStatus::Valid,
        ));

    process(&processor).await.unwrap();

    assert_eq!(status(&processor), Some(CastVoteStatus::Valid));
    assert_eq!(
        audit_operations(&processor),
        vec![audited("SetVoted skip ignored after concurrent resolution")]
    );
}

#[tokio::test]
async fn a_failed_status_update_fails_the_task_without_an_audit_entry() {
    let processor = processor();
    processor.voters.insert(&realm(), voter(Some(false), None));
    processor
        .votes
        .fail_compare_and_set("Error transitioning cast vote status: deadlock detected");

    assert_eq!(
        error_message(process(&processor).await),
        "Error transitioning cast vote status: deadlock detected"
    );
    assert!(processor.audit.entries().is_empty());
}

#[tokio::test]
async fn an_internet_voter_has_the_vote_validated_without_voterview_or_a_new_mark() {
    let processor = processor();
    processor
        .voters
        .insert(&realm(), voter(Some(true), Some("Internet")));

    process(&processor).await.unwrap();

    assert_eq!(status(&processor), Some(CastVoteStatus::Valid));
    assert!(processor.voter_view.sent().is_empty());
    assert!(processor.voters.marked_via_internet().is_empty());
    assert!(processor.audit.entries().is_empty());
}

#[tokio::test]
async fn a_re_vote_is_validated_without_voterview_and_marks_the_voter() {
    let processor = processor();
    processor
        .votes
        .insert(cast_vote(EARLIER_CAST_VOTE_ID, CastVoteStatus::Valid));

    process(&processor).await.unwrap();

    assert_eq!(status(&processor), Some(CastVoteStatus::Valid));
    assert!(processor.voter_view.sent().is_empty());
    assert_eq!(processor.voters.marked_via_internet(), vec![VOTER_ID]);
    assert!(processor.audit.entries().is_empty());
}

#[tokio::test]
async fn a_re_vote_resolved_concurrently_does_not_mark_the_voter() {
    let processor = processor();
    processor
        .votes
        .insert(cast_vote(EARLIER_CAST_VOTE_ID, CastVoteStatus::Valid));
    processor
        .votes
        .change_concurrently(ConcurrentChange::ResolvedBeforeNextCompareAndSet(
            CastVoteStatus::Discarded,
        ));

    process(&processor).await.unwrap();

    assert_eq!(status(&processor), Some(CastVoteStatus::Discarded));
    assert!(processor.voters.marked_via_internet().is_empty());
}

#[tokio::test]
async fn a_failed_internet_mark_does_not_fail_the_vote() {
    let processor = processor();
    processor
        .votes
        .insert(cast_vote(EARLIER_CAST_VOTE_ID, CastVoteStatus::Valid));
    processor.voters.fail_marking("Keycloak unavailable");

    process(&processor).await.unwrap();

    assert_eq!(status(&processor), Some(CastVoteStatus::Valid));
}

#[tokio::test]
async fn a_prior_vote_lookup_failure_fails_the_task_even_for_an_internet_voter() {
    let processor = processor();
    processor
        .voters
        .insert(&realm(), voter(Some(true), Some("Internet")));
    processor
        .votes
        .fail_has_valid_vote("Error checking prior valid votes: statement timeout");

    assert_eq!(
        error_message(process(&processor).await),
        "Error checking prior valid votes: statement timeout"
    );
    assert_eq!(status(&processor), Some(CastVoteStatus::InProgress));
}

#[tokio::test]
async fn set_voted_accepted_by_voterview_validates_the_vote_and_marks_the_voter() {
    let processor = processor();

    process(&processor).await.unwrap();

    assert_eq!(status(&processor), Some(CastVoteStatus::Valid));
    assert_eq!(processor.voter_view.sent(), vec![USERNAME]);
    assert_eq!(processor.voters.marked_via_internet(), vec![VOTER_ID]);
    assert_eq!(
        processor.audit.entries(),
        vec![audit_entry(set_voted_operation("SetVoted Succeeded"))]
    );
}

#[tokio::test]
async fn a_voter_voterview_already_counts_as_voted_has_the_vote_validated_and_marked() {
    let processor = processor();
    processor
        .voter_view
        .reply_with(VoterViewReply::Response(SoapRequestResponse::AlreadyVoted));

    process(&processor).await.unwrap();

    assert_eq!(status(&processor), Some(CastVoteStatus::Valid));
    assert_eq!(processor.voters.marked_via_internet(), vec![VOTER_ID]);
    assert_eq!(
        audit_operations(&processor),
        vec![set_voted_operation("SetVoted Failed: voter already voted")]
    );
}

#[tokio::test]
async fn voterview_error_replies_validate_the_vote_without_marking_the_voter() {
    let replies = [
        (SoapRequestResponse::AlreadyNotVoted, "already-not-voted"),
        (
            SoapRequestResponse::Fault("Server was unable to process request".to_string()),
            "soap-fault",
        ),
        (
            SoapRequestResponse::Rejected("Voter not found".to_string()),
            "rejected",
        ),
    ];
    for (response, classification) in replies {
        let processor = processor();
        processor
            .voter_view
            .reply_with(VoterViewReply::Response(response));

        process(&processor).await.unwrap();

        assert_eq!(
            status(&processor),
            Some(CastVoteStatus::Valid),
            "{classification}"
        );
        assert!(
            processor.voters.marked_via_internet().is_empty(),
            "{classification}"
        );
        assert_eq!(
            audit_operations(&processor),
            vec![set_voted_operation(&format!(
                "SetVoted Failed: {classification}"
            ))]
        );
    }
}

#[tokio::test]
async fn a_set_voted_confirmation_after_a_concurrent_resolution_is_audited_as_ignored() {
    let replies = [
        (
            SoapRequestResponse::Ok,
            "SetVoted result ignored after concurrent resolution",
        ),
        (
            SoapRequestResponse::AlreadyVoted,
            "SetVoted already-voted result ignored after concurrent resolution",
        ),
    ];
    for (response, outcome) in replies {
        let processor = processor();
        processor
            .voter_view
            .reply_with(VoterViewReply::Response(response));
        processor
            .votes
            .change_concurrently(ConcurrentChange::ResolvedBeforeNextCompareAndSet(
                CastVoteStatus::Discarded,
            ));

        process(&processor).await.unwrap();

        assert_eq!(
            status(&processor),
            Some(CastVoteStatus::Discarded),
            "{outcome}"
        );
        assert!(
            processor.voters.marked_via_internet().is_empty(),
            "{outcome}"
        );
        assert_eq!(
            audit_operations(&processor),
            vec![set_voted_operation(outcome)]
        );
    }
}

#[tokio::test]
async fn an_error_reply_after_a_concurrent_resolution_is_still_audited_as_a_failure() {
    let processor = processor();
    processor
        .voter_view
        .reply_with(VoterViewReply::Response(SoapRequestResponse::Rejected(
            "Voter not found".to_string(),
        )));
    processor
        .votes
        .change_concurrently(ConcurrentChange::ResolvedBeforeNextCompareAndSet(
            CastVoteStatus::Discarded,
        ));

    process(&processor).await.unwrap();

    assert_eq!(status(&processor), Some(CastVoteStatus::Discarded));
    assert_eq!(
        audit_operations(&processor),
        vec![set_voted_operation("SetVoted Failed: rejected")]
    );
}

#[tokio::test]
async fn an_undispatched_set_voted_is_audited_and_leaves_the_vote_in_progress() {
    let processor = processor();
    processor
        .voter_view
        .reply_with(VoterViewReply::NotDispatched(
            "connection refused".to_string(),
        ));

    assert_eq!(
        error_message(process(&processor).await),
        "VoterView SetVoted was not dispatched; the vote stays in-progress: connection refused"
    );
    assert_eq!(status(&processor), Some(CastVoteStatus::InProgress));
    assert!(processor.voters.marked_via_internet().is_empty());
    assert_eq!(
        audit_operations(&processor),
        vec![set_voted_operation(
            "SetVoted NotDispatched: connection-error"
        )]
    );
}

#[tokio::test]
async fn an_ambiguous_set_voted_outcome_is_audited_and_leaves_the_vote_in_progress() {
    let processor = processor();
    processor
        .voter_view
        .reply_with(VoterViewReply::Ambiguous("read timed out".to_string()));

    assert_eq!(
        error_message(process(&processor).await),
        "VoterView SetVoted outcome is ambiguous; the vote stays in-progress: read timed out"
    );
    assert_eq!(status(&processor), Some(CastVoteStatus::InProgress));
    assert!(processor.voters.marked_via_internet().is_empty());
    assert_eq!(
        audit_operations(&processor),
        vec![set_voted_operation(
            "SetVoted Failed: transport-or-response-error"
        )]
    );
}

#[tokio::test]
async fn a_set_voted_request_that_cannot_be_prepared_leaves_the_vote_in_progress_unaudited() {
    let processor = processor();
    processor
        .voter_view
        .fail_prepare("Invalid Datafix election event annotations");

    assert_eq!(
        error_message(process(&processor).await),
        "Unable to prepare SetVoted before dispatch: Invalid Datafix election event annotations"
    );
    assert_eq!(status(&processor), Some(CastVoteStatus::InProgress));
    assert!(processor.audit.entries().is_empty());
}
