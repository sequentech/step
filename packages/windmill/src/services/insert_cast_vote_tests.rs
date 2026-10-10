// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use sequent_core::ballot::TYPES_VERSION;
use sequent_core::encrypt::encrypt_decoded_contest;
use sequent_core::fixtures::ballot_codec::{get_writein_ballot_style, get_writein_plaintext};
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

#[test]
fn kiosk_and_telephone_use_their_own_status_and_ignore_online_schedule() {
    let now = ISO8601::to_date("2026-01-01T12:01:00Z").unwrap();
    for channel in [VotingStatusChannel::KIOSK, VotingStatusChannel::TELEPHONE] {
        for state in [
            VotingStatus::OPEN,
            VotingStatus::NOT_STARTED,
            VotingStatus::PAUSED,
            VotingStatus::CLOSED,
        ] {
            let status = ElectionStatus {
                voting_status: VotingStatus::CLOSED,
                kiosk_voting_status: state.clone(),
                telephone_voting_status: state.clone(),
                ..Default::default()
            };
            let result = check_status_with_loaded_election(
                now,
                now - Duration::minutes(2),
                channel,
                false,
                VotingPeriodDates {
                    start_date: None,
                    end_date: Some("2026-01-01T12:00:00Z".into()),
                },
                &status,
                &ElectionPresentation::default(),
                "election",
            );
            if state == VotingStatus::OPEN {
                assert_eq!(result.unwrap(), channel);
            } else {
                assert!(result.is_err(), "channel={channel:?}, status={state:?}");
            }
        }
    }
}

#[test]
fn scheduled_grace_preserves_deadline_and_authentication_boundaries() {
    let close = ISO8601::to_date("2026-01-01T12:00:00Z").unwrap();
    let presentation = ElectionPresentation {
        grace_period_policy: Some(EGracePeriodPolicy::GRACE_PERIOD_WITHOUT_ALERT),
        grace_period_secs: Some(120),
        ..Default::default()
    };
    for seconds_after_close in [-1, 0, 1, 119, 120, 121] {
        for auth_offset in [-1, 0, 1] {
            let status = ElectionStatus {
                voting_status: if seconds_after_close <= 0 {
                    VotingStatus::OPEN
                } else {
                    VotingStatus::CLOSED
                },
                ..Default::default()
            };
            let result = check_status_with_loaded_election(
                close + Duration::seconds(seconds_after_close),
                close + Duration::seconds(auth_offset),
                VotingStatusChannel::ONLINE,
                false,
                VotingPeriodDates {
                    start_date: None,
                    end_date: Some("2026-01-01T12:00:00Z".into()),
                },
                &status,
                &presentation,
                "election",
            );
            assert_eq!(
                result.is_ok(),
                seconds_after_close <= 120 && auth_offset <= 0,
                "submission={seconds_after_close}, authentication={auth_offset}"
            );
        }
    }
}

#[test]
fn manual_grace_preserves_strict_stop_and_authentication_boundaries() {
    let close = ISO8601::to_date("2026-01-01T12:00:00Z").unwrap();
    let mut status = ElectionStatus {
        voting_status: VotingStatus::CLOSED,
        ..Default::default()
    };
    status.voting_period_dates.last_stopped_at = Some(close.into());
    let presentation = ElectionPresentation {
        grace_period_policy: Some(EGracePeriodPolicy::GRACE_PERIOD_WITHOUT_ALERT),
        grace_period_secs: Some(120),
        ..Default::default()
    };
    for offset in [0, 1, 119, 120, 121] {
        for auth_offset in [-1, 0, 1] {
            let result = check_status_with_loaded_election(
                close + Duration::seconds(offset),
                close + Duration::seconds(auth_offset),
                VotingStatusChannel::ONLINE,
                false,
                VotingPeriodDates::default(),
                &status,
                &presentation,
                "election",
            );
            assert_eq!(
                result.is_ok(),
                offset < 120 && auth_offset < 0,
                "submission={offset}, authentication={auth_offset}"
            );
        }
    }
}

#[test]
fn grace_cannot_override_pause_disabled_policy_or_other_channels() {
    let close = ISO8601::to_date("2026-01-01T12:00:00Z").unwrap();
    for scheduled in [false, true] {
        for channel in [
            VotingStatusChannel::ONLINE,
            VotingStatusChannel::KIOSK,
            VotingStatusChannel::TELEPHONE,
        ] {
            for paused in [false, true] {
                for enabled in [false, true] {
                    let state = if paused {
                        VotingStatus::PAUSED
                    } else {
                        VotingStatus::CLOSED
                    };
                    let mut status = ElectionStatus {
                        voting_status: state.clone(),
                        kiosk_voting_status: state.clone(),
                        telephone_voting_status: state,
                        ..Default::default()
                    };
                    status.voting_period_dates.last_stopped_at = Some(close.into());
                    status.kiosk_voting_period_dates.last_stopped_at = Some(close.into());
                    status.telephone_voting_period_dates.last_stopped_at = Some(close.into());
                    let presentation = ElectionPresentation {
                        grace_period_policy: Some(if enabled {
                            EGracePeriodPolicy::GRACE_PERIOD_WITHOUT_ALERT
                        } else {
                            EGracePeriodPolicy::NO_GRACE_PERIOD
                        }),
                        grace_period_secs: Some(120),
                        ..Default::default()
                    };
                    let result = check_status_with_loaded_election(
                        close + Duration::seconds(1),
                        close - Duration::seconds(1),
                        channel,
                        false,
                        VotingPeriodDates {
                            start_date: None,
                            end_date: scheduled.then(|| "2026-01-01T12:00:00Z".into()),
                        },
                        &status,
                        &presentation,
                        "election",
                    );
                    assert_eq!(result.is_ok(), enabled && !paused && channel == VotingStatusChannel::ONLINE,
                        "scheduled={scheduled}, channel={channel:?}, paused={paused}, enabled={enabled}");
                }
            }
        }
    }
}

#[test]
fn jwt_authentication_seconds_preserve_grace_eligibility() {
    let close = ISO8601::to_date("2026-01-01T12:00:00Z").unwrap();
    let presentation = ElectionPresentation {
        grace_period_policy: Some(EGracePeriodPolicy::GRACE_PERIOD_WITHOUT_ALERT),
        grace_period_secs: Some(120),
        ..Default::default()
    };
    let mut status = ElectionStatus {
        voting_status: VotingStatus::CLOSED,
        ..Default::default()
    };
    status.voting_period_dates.last_stopped_at = Some(close.into());
    for offset in [-1, 1] {
        let authenticated = close + Duration::seconds(offset);
        let parsed = voter_authentication_time(Some(authenticated.timestamp())).unwrap();
        assert_eq!(parsed, authenticated);
        for scheduled in [false, true] {
            let result = check_status_with_loaded_election(
                close + Duration::seconds(30),
                parsed,
                VotingStatusChannel::ONLINE,
                false,
                VotingPeriodDates {
                    start_date: None,
                    end_date: scheduled.then(|| "2026-01-01T12:00:00Z".into()),
                },
                &status,
                &presentation,
                "election",
            );
            assert_eq!(
                result.is_ok(),
                offset < 0,
                "scheduled={scheduled}, auth offset={offset}"
            );
        }
    }
    for invalid in [None, Some(i64::MAX), Some(i64::MIN)] {
        assert!(voter_authentication_time(invalid).is_err());
    }
}

const CEREMONY_PUBLIC_KEY: &str = "ceremony-public-key";
const OTHER_PUBLIC_KEY: &str = "other-public-key";

fn ballot_style(public_key: Option<&str>, is_demo: bool) -> BallotStyle {
    deserialize_value(json!({
        "id": Uuid::new_v4().to_string(),
        "tenant_id": Uuid::new_v4().to_string(),
        "election_event_id": Uuid::new_v4().to_string(),
        "election_id": Uuid::new_v4().to_string(),
        "area_id": Uuid::new_v4().to_string(),
        "public_key": public_key.map(|public_key| json!({
            "public_key": public_key,
            "is_demo": is_demo,
        })),
        "contests": [],
    }))
    .unwrap()
}

fn stored(ballot_style: &BallotStyle) -> StoredBallotStyle {
    StoredBallotStyle::from_ballot_eml(&serde_json::to_string(ballot_style).unwrap()).unwrap()
}

/// The reference a voter's ballot carries when it is encrypted with
/// `voter_style`, as the portal serializes it.
fn cast_reference(voter_style: &BallotStyle) -> BallotStyleReference {
    let content = serde_json::to_string(&SignedHashableBallot {
        version: TYPES_VERSION,
        issue_date: "2026-01-01".to_string(),
        contests: vec![],
        config: voter_style.id.clone(),
        ballot_style_hash: hash_ballot_style(voter_style).unwrap(),
        voter_signing_pk: None,
        voter_ballot_signature: None,
    })
    .unwrap();
    deserialize_str(&content).unwrap()
}

#[test]
fn accepts_a_ballot_made_with_the_stored_style_and_ceremony_key() {
    let style = ballot_style(Some(CEREMONY_PUBLIC_KEY), false);

    assert!(check_stored_ballot_style(
        &cast_reference(&style),
        &stored(&style),
        Some(CEREMONY_PUBLIC_KEY),
    )
    .is_ok());
}

#[test]
fn rejects_a_ballot_made_with_a_style_carrying_another_public_key() {
    let style = ballot_style(Some(CEREMONY_PUBLIC_KEY), false);
    let mut voter_style = style.clone();
    voter_style.public_key = Some(PublicKeyConfig {
        public_key: OTHER_PUBLIC_KEY.to_string(),
        is_demo: false,
    });

    assert!(matches!(
        check_stored_ballot_style(
            &cast_reference(&voter_style),
            &stored(&style),
            Some(CEREMONY_PUBLIC_KEY),
        ),
        Err(CastVoteError::BallotStyleMismatch(_))
    ));
}

#[test]
fn rejects_a_ballot_whose_style_hash_differs_from_the_stored_style() {
    let style = ballot_style(Some(CEREMONY_PUBLIC_KEY), false);
    let mut voter_style = style.clone();
    voter_style.description = Some("changed".to_string());

    assert!(matches!(
        check_stored_ballot_style(
            &cast_reference(&voter_style),
            &stored(&style),
            Some(CEREMONY_PUBLIC_KEY),
        ),
        Err(CastVoteError::BallotStyleMismatch(_))
    ));
}

#[test]
fn rejects_a_stored_style_that_does_not_use_the_keys_ceremony_public_key() {
    let style = ballot_style(Some(OTHER_PUBLIC_KEY), false);

    assert!(matches!(
        check_stored_ballot_style(
            &cast_reference(&style),
            &stored(&style),
            Some(CEREMONY_PUBLIC_KEY),
        ),
        Err(CastVoteError::BallotStyleMismatch(_))
    ));
}

#[test]
fn rejects_a_non_demo_key_for_an_election_without_a_keys_ceremony_public_key() {
    let style = ballot_style(Some(OTHER_PUBLIC_KEY), false);

    assert!(matches!(
        check_stored_ballot_style(&cast_reference(&style), &stored(&style), None),
        Err(CastVoteError::BallotStyleMismatch(_))
    ));
}

#[test]
fn rejects_a_stored_style_without_a_public_key() {
    let style = ballot_style(None, false);

    for keys_ceremony_public_key in [Some(CEREMONY_PUBLIC_KEY), None] {
        assert!(matches!(
            check_stored_ballot_style(
                &cast_reference(&style),
                &stored(&style),
                keys_ceremony_public_key,
            ),
            Err(CastVoteError::BallotStyleMismatch(_))
        ));
    }
}

#[test]
fn accepts_a_demo_key_for_an_election_without_a_keys_ceremony_public_key() {
    let style = ballot_style(Some(OTHER_PUBLIC_KEY), true);

    assert!(check_stored_ballot_style(&cast_reference(&style), &stored(&style), None).is_ok());
}

#[test]
fn reads_the_ballot_style_reference_of_multi_contest_ballots() {
    let style = ballot_style(Some(CEREMONY_PUBLIC_KEY), false);
    let content = serde_json::to_string(&SignedHashableMultiBallot {
        version: TYPES_VERSION,
        issue_date: "2026-01-01".to_string(),
        contests: String::new(),
        config: style.id.clone(),
        ballot_style_hash: hash_ballot_style(&style).unwrap(),
        voter_signing_pk: None,
        voter_ballot_signature: None,
    })
    .unwrap();
    let reference: BallotStyleReference = deserialize_str(&content).unwrap();

    assert_eq!(reference.config, style.id);
    assert!(
        check_stored_ballot_style(&reference, &stored(&style), Some(CEREMONY_PUBLIC_KEY)).is_ok()
    );
}

#[test]
fn ballot_style_mismatch_is_not_retried() {
    assert!(matches!(
        skip_or_propagate(CastVoteError::BallotStyleMismatch(String::new())),
        Ok(InsertCastVoteResult::SkipRetryFailure(
            CastVoteError::BallotStyleMismatch(_)
        ))
    ));
}

#[test]
fn accepts_a_ballot_encrypted_from_the_stored_ballot_eml() {
    let style = get_writein_ballot_style();
    let public_key = style.public_key.clone().unwrap().public_key;
    let ballot_eml = serde_json::to_string(&style).unwrap();
    let portal_style: BallotStyle =
        deserialize_value(serde_json::from_str::<serde_json::Value>(&ballot_eml).unwrap()).unwrap();
    let auditable_ballot =
        encrypt_decoded_contest(&RistrettoCtx, &vec![get_writein_plaintext()], &portal_style)
            .unwrap();
    let content =
        serde_json::to_string(&SignedHashableBallot::try_from(&auditable_ballot).unwrap()).unwrap();

    assert!(check_stored_ballot_style(
        &deserialize_str(&content).unwrap(),
        &StoredBallotStyle::from_ballot_eml(&ballot_eml).unwrap(),
        Some(&public_key),
    )
    .is_ok());
}
