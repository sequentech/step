// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What the protected actions sign, without a database.

use super::configuration::{ballots_change, canonical_text, CHANGED, FIRST_VERSION, NO_CHANGES};
use super::initialize::is_initialization;
use super::voter::{choose_post, registry_line};
use super::voting::{action_of, plan, transitioned, VotingSubject};
use super::{refuse, EffectProgress, EffectRefused, SignedActionTask};
use crate::services::signing::{InvalidReason, SigningError};
use sequent_core::ballot::{ElectionStatus, VotingStatus, VotingStatusChannel};
use sequent_core::signing::SigningAction;
use sequent_core::types::hasura::core::Election;
use serde_json::json;
use uuid::Uuid;

fn reason<T: std::fmt::Debug>(result: Result<T, SigningError>) -> InvalidReason {
    match result {
        Err(SigningError::Invalid { reason, .. }) => reason,
        other => panic!("expected an invalid input, got {other:?}"),
    }
}

#[test]
fn only_opening_and_closing_are_protected_status_changes() {
    assert_eq!(
        action_of(&VotingStatus::OPEN),
        Some(SigningAction::OpenVoting)
    );
    assert_eq!(
        action_of(&VotingStatus::CLOSED),
        Some(SigningAction::CloseVoting)
    );
    assert_eq!(action_of(&VotingStatus::PAUSED), None);
    assert_eq!(action_of(&VotingStatus::NOT_STARTED), None);
}

#[test]
fn a_status_change_signs_its_channels_sorted_with_their_status_before() {
    let subject = VotingSubject::new(&[
        (VotingStatusChannel::TELEPHONE, VotingStatus::OPEN),
        (VotingStatusChannel::ONLINE, VotingStatus::PAUSED),
        (VotingStatusChannel::TELEPHONE, VotingStatus::OPEN),
    ]);
    assert_eq!(
        serde_json::to_value(&subject).unwrap(),
        json!({"channels": ["ONLINE", "TELEPHONE"], "from": ["ONLINE=PAUSED", "TELEPHONE=OPEN"]})
    );
    assert_eq!(subject.key(), "ONLINE,TELEPHONE");
    assert_eq!(
        subject.pairs().unwrap(),
        vec![
            (VotingStatusChannel::ONLINE, VotingStatus::PAUSED),
            (VotingStatusChannel::TELEPHONE, VotingStatus::OPEN),
        ]
    );
    let malformed = VotingSubject {
        channels: vec!["ONLINE".into()],
        from: vec!["ONLINE".into()],
    };
    assert!(malformed.pairs().is_err());
    assert!(transitioned(&json!({"channels": ["ONLINE"]})));
    assert!(!transitioned(&json!({"channels": []})));
}

#[test]
fn a_status_change_is_planned_only_when_every_channel_may_change() {
    let election: Election = serde_json::from_value(
        json!({"id": "p", "tenant_id": "t", "election_event_id": "e",
            "presentation": {"voting_period_end": "allowed"}}),
    )
    .unwrap();
    let status = ElectionStatus {
        voting_status: VotingStatus::OPEN,
        ..Default::default()
    };
    assert_eq!(
        plan(
            &election,
            &status,
            &[VotingStatusChannel::ONLINE],
            &VotingStatus::CLOSED
        )
        .unwrap(),
        vec![(VotingStatusChannel::ONLINE, VotingStatus::OPEN)]
    );
    for (channels, target) in [
        (vec![], VotingStatus::CLOSED),
        // Already open.
        (vec![VotingStatusChannel::ONLINE], VotingStatus::OPEN),
        // Kiosk never started: it can't close.
        (vec![VotingStatusChannel::KIOSK], VotingStatus::CLOSED),
    ] {
        assert_eq!(
            reason(plan(&election, &status, &channels, &target)),
            InvalidReason::Transition,
            "{channels:?} {target:?}"
        );
    }
}

#[test]
fn canonical_text_orders_keys_and_keeps_array_order() {
    let value = json!({"b": [{"z": 1, "a": 2}, {"c": 0}], "a": null});
    assert_eq!(
        canonical_text(&value),
        r#"{"a":null,"b":[{"a":2,"z":1},{"c":0}]}"#
    );
    // Candidate order matters: arrays are not sorted.
    assert_ne!(
        canonical_text(&json!(["second", "first"])),
        canonical_text(&json!(["first", "second"]))
    );
}

#[test]
fn the_ballots_change_compares_the_two_sides_of_the_diff() {
    let side = |styles| json!({"ballot_publication_id": "x", "ballot_styles": styles});
    assert_eq!(
        ballots_change(&json!({"current": side(json!([1])), "previous": null})),
        FIRST_VERSION
    );
    assert_eq!(
        ballots_change(
            &json!({"current": side(json!([{"a": 1, "b": 2}])), "previous": side(json!([{"b": 2, "a": 1}]))})
        ),
        NO_CHANGES
    );
    assert_eq!(
        ballots_change(
            &json!({"current": side(json!([{"a": 2}])), "previous": side(json!([{"a": 1}]))})
        ),
        CHANGED
    );
}

#[test]
fn a_registry_record_reads_last_first_and_username() {
    assert_eq!(
        registry_line(Some("Garcia"), Some("Ramon"), Some("rgarcia"), "id"),
        "Garcia, Ramon (rgarcia)"
    );
    assert_eq!(registry_line(Some("Garcia"), None, None, "id"), "Garcia");
    assert_eq!(
        registry_line(Some(" "), None, Some("rgarcia"), "id"),
        "rgarcia"
    );
    assert_eq!(registry_line(None, None, Some(""), "id"), "id");
}

#[test]
fn an_application_votes_in_one_post_or_its_label_says_which() {
    let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
    assert_eq!(choose_post(&[(a, false)]).unwrap(), a);
    assert_eq!(choose_post(&[(a, false), (b, true)]).unwrap(), b);
    assert_eq!(
        reason(choose_post(&[(a, false), (b, false)])),
        InvalidReason::AmbiguousPost
    );
    assert_eq!(
        reason(choose_post(&[(a, true), (b, true)])),
        InvalidReason::AmbiguousPost
    );
    assert_eq!(reason(choose_post(&[])), InvalidReason::Input);
}

#[test]
fn only_initialization_report_tallies_initialize_voting() {
    assert!(is_initialization("INITIALIZATION_REPORT"));
    assert!(!is_initialization("ELECTORAL_RESULTS"));
    assert!(!is_initialization("initialization-report"));
}

#[test]
fn a_refusal_keeps_its_code_and_the_point_of_no_return_is_marked_once_passed() {
    let error = refuse("state-changed", "The Post changed.");
    assert_eq!(
        error
            .downcast_ref::<EffectRefused>()
            .map(|r| r.code.as_str()),
        Some("state-changed")
    );
    assert_eq!(error.to_string(), "state-changed: The Post changed.");
    let progress = EffectProgress::default();
    assert!(!progress.reached_outside());
    progress.reach_outside();
    assert!(progress.reached_outside());
}

#[test]
fn a_task_reads_its_celery_arguments() {
    let ids: Vec<String> = (0..4).map(|_| Uuid::new_v4().to_string()).collect();
    let task = SignedActionTask::parse(&ids[0], &ids[1], &ids[2], &ids[3]).unwrap();
    assert_eq!(task.request_id.to_string(), ids[2]);
    assert!(SignedActionTask::parse(&ids[0], &ids[1], "not-a-uuid", &ids[3]).is_err());
}
