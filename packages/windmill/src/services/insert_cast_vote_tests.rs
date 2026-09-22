// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use serde_json::json;

fn election_event(annotations: Option<serde_json::Value>) -> ElectionEvent {
    ElectionEvent {
        id: "event".to_string(),
        created_at: None,
        updated_at: None,
        labels: None,
        annotations,
        tenant_id: "tenant".to_string(),
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

#[test]
fn malformed_presentation_is_an_internal_error_instead_of_default_policy() {
    assert!(matches!(
        parse_election_presentation(Some(json!({"grace_period_secs": "invalid"}))),
        Err(CastVoteError::CheckStatusInternalFailed(message))
            if message.contains("Failed to deserialize election presentation")
    ));
}

#[test]
fn missing_and_valid_presentation_remain_supported() {
    assert!(parse_election_presentation(None).is_ok());
    let presentation =
        parse_election_presentation(Some(json!({"grace_period_secs": 120}))).unwrap();
    assert_eq!(presentation.grace_period_secs, Some(120));
}

#[test]
fn scheduled_close_is_checked_at_each_submission_without_stale_acceptance() {
    let close = ISO8601::to_date("2026-01-01T12:00:00Z").unwrap();
    let status = ElectionStatus {
        voting_status: VotingStatus::OPEN,
        ..Default::default()
    };
    for offset in [-30, -1, 0, 1, 30] {
        let result = check_status_with_loaded_election(
            close + Duration::seconds(offset),
            close - Duration::minutes(1),
            VotingStatusChannel::ONLINE,
            false,
            VotingPeriodDates {
                start_date: None,
                end_date: Some("2026-01-01T12:00:00Z".into()),
            },
            &status,
            &ElectionPresentation::default(),
            "election",
        );
        // Preserve existing boundary semantics: at the deadline is accepted.
        assert_eq!(result.is_ok(), offset <= 0, "offset={offset}");
    }
}

#[test]
fn administrative_pause_is_not_hidden_by_a_future_scheduled_close() {
    let now = ISO8601::to_date("2026-01-01T12:00:00Z").unwrap();
    for voting_status in [
        VotingStatus::NOT_STARTED,
        VotingStatus::PAUSED,
        VotingStatus::CLOSED,
    ] {
        let status = ElectionStatus {
            voting_status,
            ..Default::default()
        };
        assert!(check_status_with_loaded_election(
            now,
            now - Duration::minutes(1),
            VotingStatusChannel::ONLINE,
            false,
            VotingPeriodDates {
                start_date: None,
                end_date: Some("2026-01-02T12:00:00Z".into())
            },
            &status,
            &ElectionPresentation::default(),
            "election",
        )
        .is_err());
    }
}

#[test]
fn ordinary_events_insert_valid_votes_without_async_processing() {
    let status = initial_cast_vote_status(&election_event(None)).unwrap();
    assert_eq!(status, CastVoteStatus::Valid);
}

#[test]
fn configured_datafix_events_insert_pending_votes() {
    let annotations = json!({
        "datafix:id": "external-event",
        "datafix:password_policy": r#"{"base":"password-only","size":6,"characters":"numeric"}"#,
        "datafix:voterview_request": r#"{"url":"https://example.invalid","usr":"user","psw":"secret","county_mun":"county"}"#
    });
    let status = initial_cast_vote_status(&election_event(Some(annotations))).unwrap();
    assert_eq!(status, CastVoteStatus::InProgress);
}

#[test]
fn malformed_datafix_configuration_fails_closed() {
    let annotations = json!({"datafix:id": "external-event"});
    assert!(matches!(
        initial_cast_vote_status(&election_event(Some(annotations))),
        Err(CastVoteError::InvalidDatafixConfiguration(_))
    ));
}

#[test]
fn online_votes_in_open_early_voting_areas_use_early_voting_channel() {
    let election_status = ElectionStatus {
        voting_status: VotingStatus::NOT_STARTED,
        early_voting_status: VotingStatus::OPEN,
        ..Default::default()
    };

    assert_eq!(
        effective_voting_channel_for_status(VotingStatusChannel::ONLINE, true, &election_status,),
        VotingStatusChannel::EARLY_VOTING
    );
}

#[test]
fn online_close_date_keeps_existing_status_rejection_for_early_voting_area() {
    let election_status = ElectionStatus {
        voting_status: VotingStatus::NOT_STARTED,
        early_voting_status: VotingStatus::OPEN,
        ..Default::default()
    };
    let now = ISO8601::to_date("2026-01-01T12:00:00Z").unwrap();
    let auth_time = ISO8601::to_date("2026-01-01T11:00:00Z").unwrap();
    let dates = VotingPeriodDates {
        start_date: None,
        end_date: Some("2026-01-02T00:00:00Z".to_string()),
    };

    let result = check_status_with_loaded_election(
        now,
        auth_time,
        VotingStatusChannel::ONLINE,
        true,
        dates,
        &election_status,
        &ElectionPresentation::default(),
        "election-id",
    );

    assert!(matches!(result, Err(CastVoteError::CheckStatusFailed(_))));
}

#[test]
fn accepted_early_vote_without_online_close_date_is_labelled_early_voting() {
    let election_status = ElectionStatus {
        voting_status: VotingStatus::NOT_STARTED,
        early_voting_status: VotingStatus::OPEN,
        ..Default::default()
    };
    let now = ISO8601::to_date("2026-01-01T12:00:00Z").unwrap();
    let auth_time = ISO8601::to_date("2026-01-01T11:00:00Z").unwrap();

    let channel = check_status_with_loaded_election(
        now,
        auth_time,
        VotingStatusChannel::ONLINE,
        true,
        VotingPeriodDates::default(),
        &election_status,
        &ElectionPresentation::default(),
        "election-id",
    )
    .unwrap();

    assert_eq!(channel, VotingStatusChannel::EARLY_VOTING);
}

#[test]
fn early_voting_area_does_not_overwrite_transport_channels() {
    let election_status = ElectionStatus {
        voting_status: VotingStatus::NOT_STARTED,
        kiosk_voting_status: VotingStatus::NOT_STARTED,
        early_voting_status: VotingStatus::OPEN,
        telephone_voting_status: VotingStatus::NOT_STARTED,
        ..Default::default()
    };

    for channel in [VotingStatusChannel::KIOSK, VotingStatusChannel::TELEPHONE] {
        assert_eq!(
            effective_voting_channel_for_status(channel, true, &election_status,),
            channel
        );
    }
}

#[path = "insert_cast_vote_database_tests.rs"]
mod database;
