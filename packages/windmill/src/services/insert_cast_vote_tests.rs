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

mod ballot_style_binding {
    use super::*;
    use sequent_core::ballot::{AuditableBallot, PublicKeyConfig};
    use sequent_core::encrypt::{encrypt_decoded_contest, encrypt_decoded_multi_contest};
    use sequent_core::fixtures::ballot_codec::{
        get_test_contest, get_test_decoded_vote_contest, get_writein_ballot_style,
    };
    use sequent_core::multi_ballot::AuditableMultiBallot;
    use sequent_core::serialization::base64::Base64Serialize;
    use strand::elgamal::PrivateKey;

    const VOTER_ID: &str = "voter";

    fn ballot_style() -> BallotStyle {
        BallotStyle {
            contests: vec![get_test_contest()],
            ..get_writein_ballot_style()
        }
    }

    fn style_with_other_public_key() -> BallotStyle {
        let private_key = PrivateKey::<RistrettoCtx>::gen(&RistrettoCtx);
        BallotStyle {
            public_key: Some(PublicKeyConfig {
                public_key: private_key.pk_element().serialize().unwrap(),
                is_demo: false,
            }),
            ..ballot_style()
        }
    }

    fn style_with_reordered_candidates() -> BallotStyle {
        let mut ballot_style = ballot_style();
        ballot_style.contests[0].candidates.reverse();
        ballot_style
    }

    fn published_style(ballot_style: &BallotStyle) -> PublishedBallotStyleHash {
        PublishedBallotStyleHash::from_ballot_eml(&serde_json::to_string(ballot_style).unwrap())
            .unwrap()
    }

    fn published_styles(ballot_style: &BallotStyle) -> Vec<PublishedBallotStyleHash> {
        vec![published_style(ballot_style)]
    }

    fn single_ballot(ballot_style: &BallotStyle) -> AuditableBallot {
        encrypt_decoded_contest::<RistrettoCtx>(
            &RistrettoCtx,
            &vec![get_test_decoded_vote_contest()],
            ballot_style,
        )
        .unwrap()
    }

    fn multi_ballot(ballot_style: &BallotStyle) -> AuditableMultiBallot {
        encrypt_decoded_multi_contest::<RistrettoCtx>(
            &RistrettoCtx,
            &vec![get_test_decoded_vote_contest()],
            ballot_style,
        )
        .unwrap()
    }

    fn single_input(ballot: &AuditableBallot) -> InsertCastVoteInput {
        let signed = SignedHashableBallot::try_from(ballot).unwrap();
        let hashable = HashableBallot::try_from(&signed).unwrap();
        InsertCastVoteInput {
            ballot_id: hash_ballot(&hashable).unwrap(),
            election_id: Uuid::new_v4(),
            content: serde_json::to_string(&signed).unwrap(),
        }
    }

    fn multi_input(ballot: &AuditableMultiBallot) -> InsertCastVoteInput {
        let signed = SignedHashableMultiBallot::try_from(ballot).unwrap();
        let hashable = HashableMultiBallot::try_from(&signed).unwrap();
        InsertCastVoteInput {
            ballot_id: hash_multi_ballot(&hashable).unwrap(),
            election_id: Uuid::new_v4(),
            content: serde_json::to_string(&signed).unwrap(),
        }
    }

    fn legacy_single_input() -> InsertCastVoteInput {
        let mut ballot = single_ballot(&ballot_style());
        ballot.version = LEGACY_TYPES_VERSION;
        single_input(&ballot)
    }

    fn check_single(
        input: &InsertCastVoteInput,
        policy: &BallotTrackerPolicy,
        published_ballot_styles: &[PublishedBallotStyleHash],
    ) -> Result<(), CastVoteError> {
        deserialize_and_check_ballot(input, VOTER_ID, policy, published_ballot_styles).map(|_| ())
    }

    fn check_multi(
        input: &InsertCastVoteInput,
        published_ballot_styles: &[PublishedBallotStyleHash],
    ) -> Result<(), CastVoteError> {
        deserialize_and_check_multi_ballot(
            input,
            VOTER_ID,
            &BallotTrackerPolicy::default(),
            published_ballot_styles,
        )
        .map(|_| ())
    }

    fn assert_style_mismatch(result: Result<(), CastVoteError>) {
        assert!(
            matches!(result, Err(CastVoteError::BallotStyleMismatch(_))),
            "expected a ballot style mismatch, got {result:?}"
        );
    }

    #[test]
    fn ballot_tracker_policy_defaults_to_style_bound() {
        assert_eq!(
            BallotTrackerPolicy::default(),
            BallotTrackerPolicy::STYLE_BOUND
        );
    }

    #[test]
    fn ballot_encrypted_with_the_published_style_is_accepted() {
        let ballot_style = ballot_style();
        let published = published_styles(&ballot_style);

        assert!(check_single(
            &single_input(&single_ballot(&ballot_style)),
            &BallotTrackerPolicy::default(),
            &published,
        )
        .is_ok());
        assert!(check_multi(&multi_input(&multi_ballot(&ballot_style)), &published).is_ok());
    }

    #[test]
    fn ballot_matching_any_published_style_is_accepted() {
        let ballot_style = ballot_style();
        let published = vec![
            published_style(&style_with_other_public_key()),
            published_style(&ballot_style),
        ];

        assert!(check_single(
            &single_input(&single_ballot(&ballot_style)),
            &BallotTrackerPolicy::default(),
            &published,
        )
        .is_ok());
    }

    #[test]
    fn ballot_encrypted_with_another_public_key_is_rejected() {
        let published = published_styles(&ballot_style());
        let other_style = style_with_other_public_key();

        assert_style_mismatch(check_single(
            &single_input(&single_ballot(&other_style)),
            &BallotTrackerPolicy::default(),
            &published,
        ));
        assert_style_mismatch(check_multi(
            &multi_input(&multi_ballot(&other_style)),
            &published,
        ));
    }

    #[test]
    fn ballot_encrypted_with_reordered_candidates_is_rejected() {
        let published = published_styles(&ballot_style());
        let other_style = style_with_reordered_candidates();

        assert_style_mismatch(check_single(
            &single_input(&single_ballot(&other_style)),
            &BallotTrackerPolicy::default(),
            &published,
        ));
        assert_style_mismatch(check_multi(
            &multi_input(&multi_ballot(&other_style)),
            &published,
        ));
    }

    #[test]
    fn ballot_without_a_published_style_is_rejected() {
        let ballot_style = ballot_style();

        assert_style_mismatch(check_single(
            &single_input(&single_ballot(&ballot_style)),
            &BallotTrackerPolicy::default(),
            &[],
        ));
        assert_style_mismatch(check_multi(&multi_input(&multi_ballot(&ballot_style)), &[]));
    }

    #[test]
    fn legacy_ballot_is_rejected_by_default() {
        let published = published_styles(&ballot_style());

        assert_style_mismatch(check_single(
            &legacy_single_input(),
            &BallotTrackerPolicy::default(),
            &published,
        ));
    }

    #[test]
    fn legacy_ballot_is_accepted_when_the_policy_allows_it() {
        assert!(check_single(
            &legacy_single_input(),
            &BallotTrackerPolicy::ALLOW_LEGACY,
            &[],
        )
        .is_ok());
    }

    #[test]
    fn current_ballot_is_checked_when_the_policy_allows_legacy() {
        let published = published_styles(&ballot_style());

        assert_style_mismatch(check_single(
            &single_input(&single_ballot(&style_with_other_public_key())),
            &BallotTrackerPolicy::ALLOW_LEGACY,
            &published,
        ));
    }
}
