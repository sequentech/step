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

mod ballot_style_contests {
    use super::*;
    use sequent_core::ballot::{BallotStyle, Contest};
    use sequent_core::encrypt::{encrypt_decoded_contest, encrypt_decoded_multi_contest};
    use sequent_core::fixtures::ballot_codec::{
        get_test_contest, get_test_decoded_vote_contest, get_writein_ballot_style,
    };
    use std::collections::HashSet;

    const VOTER_ID: &str = "voter";
    const OTHER_CONTEST_ID: &str = "8d5b1c4e-2a3f-4e6b-9c7d-0e1f2a3b4c5d";

    fn ballot_style() -> BallotStyle {
        BallotStyle {
            contests: vec![get_test_contest()],
            ..get_writein_ballot_style()
        }
    }

    fn contest_ids(ids: &[&str]) -> HashSet<String> {
        ids.iter().map(|id| id.to_string()).collect()
    }

    fn style_contest_ids() -> HashSet<String> {
        contest_ids(&[get_test_contest().id.as_str()])
    }

    fn published_styles() -> Vec<HashSet<String>> {
        vec![style_contest_ids()]
    }

    fn single_input(signed: &SignedHashableBallot) -> InsertCastVoteInput {
        let hashable = HashableBallot::try_from(signed).unwrap();
        InsertCastVoteInput {
            ballot_id: hash_ballot(&hashable).unwrap(),
            election_id: Uuid::new_v4(),
            content: serde_json::to_string(signed).unwrap(),
        }
    }

    fn signed_single_ballot() -> SignedHashableBallot {
        let auditable = encrypt_decoded_contest::<RistrettoCtx>(
            &RistrettoCtx,
            &vec![get_test_decoded_vote_contest()],
            &ballot_style(),
        )
        .unwrap();
        SignedHashableBallot::try_from(&auditable).unwrap()
    }

    /// A ballot that carries one copy of the encrypted contest for each of
    /// the given ids.
    fn single_input_with_contest_ids(ids: &[&str]) -> InsertCastVoteInput {
        let signed = signed_single_ballot();
        let contest = signed
            .deserialize_contests::<RistrettoCtx>()
            .unwrap()
            .remove(0);
        let contests: Vec<HashableBallotContest<RistrettoCtx>> = ids
            .iter()
            .map(|id| HashableBallotContest {
                contest_id: id.to_string(),
                ..contest.clone()
            })
            .collect();
        single_input(&SignedHashableBallot {
            contests: SignedHashableBallot::serialize_contests::<RistrettoCtx>(&contests).unwrap(),
            ..signed
        })
    }

    fn signed_multi_ballot() -> SignedHashableMultiBallot {
        let auditable = encrypt_decoded_multi_contest::<RistrettoCtx>(
            &RistrettoCtx,
            &vec![get_test_decoded_vote_contest()],
            &ballot_style(),
        )
        .unwrap();
        SignedHashableMultiBallot::try_from(&auditable).unwrap()
    }

    fn multi_input(signed: &SignedHashableMultiBallot) -> InsertCastVoteInput {
        let hashable = HashableMultiBallot::try_from(signed).unwrap();
        InsertCastVoteInput {
            ballot_id: hash_multi_ballot(&hashable).unwrap(),
            election_id: Uuid::new_v4(),
            content: serde_json::to_string(signed).unwrap(),
        }
    }

    /// A multi ballot that lists exactly the given contest ids.
    fn multi_input_with_contest_ids(ids: &[&str]) -> InsertCastVoteInput {
        let signed = signed_multi_ballot();
        let mut contests = signed.deserialize_contests::<RistrettoCtx>().unwrap();
        contests.contest_ids = ids.iter().map(|id| id.to_string()).collect();
        multi_input(&SignedHashableMultiBallot {
            contests: SignedHashableMultiBallot::serialize_contests::<RistrettoCtx>(&contests)
                .unwrap(),
            ..signed
        })
    }

    #[test]
    fn ballot_with_every_style_contest_is_accepted() {
        let input = single_input(&signed_single_ballot());

        assert!(deserialize_and_check_ballot(&input, VOTER_ID, &published_styles()).is_ok());
    }

    #[test]
    fn ballot_missing_a_style_contest_is_rejected() {
        let input = single_input(&signed_single_ballot());
        let published = vec![contest_ids(&[&get_test_contest().id, OTHER_CONTEST_ID])];

        assert!(matches!(
            deserialize_and_check_ballot(&input, VOTER_ID, &published),
            Err(CastVoteError::BallotStyleMismatch(message)) if message.contains(OTHER_CONTEST_ID)
        ));
    }

    #[test]
    fn ballot_without_contests_is_rejected() {
        let input = single_input_with_contest_ids(&[]);

        assert!(matches!(
            deserialize_and_check_ballot(&input, VOTER_ID, &published_styles()),
            Err(CastVoteError::BallotStyleMismatch(_))
        ));
    }

    #[test]
    fn ballot_with_a_contest_outside_the_style_is_rejected() {
        let input = single_input_with_contest_ids(&[&get_test_contest().id, OTHER_CONTEST_ID]);

        assert!(matches!(
            deserialize_and_check_ballot(&input, VOTER_ID, &published_styles()),
            Err(CastVoteError::BallotStyleMismatch(message)) if message.contains(OTHER_CONTEST_ID)
        ));
    }

    #[test]
    fn ballot_repeating_a_style_contest_is_rejected() {
        let contest_id = get_test_contest().id;
        let input = single_input_with_contest_ids(&[&contest_id, &contest_id]);

        assert!(matches!(
            deserialize_and_check_ballot(&input, VOTER_ID, &published_styles()),
            Err(CastVoteError::BallotStyleMismatch(message)) if message.contains(&contest_id)
        ));
    }

    #[test]
    fn ballot_is_rejected_when_no_ballot_style_is_published() {
        let input = single_input(&signed_single_ballot());

        assert!(matches!(
            deserialize_and_check_ballot(&input, VOTER_ID, &[]),
            Err(CastVoteError::BallotStyleMismatch(_))
        ));
    }

    #[test]
    fn ballot_without_contests_is_rejected_when_no_ballot_style_is_published() {
        let input = single_input_with_contest_ids(&[]);

        assert!(matches!(
            deserialize_and_check_ballot(&input, VOTER_ID, &[]),
            Err(CastVoteError::BallotStyleMismatch(_))
        ));
    }

    #[test]
    fn ballot_without_contests_is_accepted_for_a_style_without_votable_contests() {
        let input = single_input_with_contest_ids(&[]);

        assert!(deserialize_and_check_ballot(&input, VOTER_ID, &[HashSet::new()]).is_ok());
    }

    #[test]
    fn ballot_matching_any_published_style_is_accepted() {
        let input = single_input(&signed_single_ballot());
        let published = vec![contest_ids(&[OTHER_CONTEST_ID]), style_contest_ids()];

        assert!(deserialize_and_check_ballot(&input, VOTER_ID, &published).is_ok());
    }

    #[test]
    fn ballot_cannot_combine_the_contests_of_different_published_styles() {
        let input = single_input_with_contest_ids(&[&get_test_contest().id, OTHER_CONTEST_ID]);
        let published = vec![style_contest_ids(), contest_ids(&[OTHER_CONTEST_ID])];

        assert!(matches!(
            deserialize_and_check_ballot(&input, VOTER_ID, &published),
            Err(CastVoteError::BallotStyleMismatch(_))
        ));
    }

    #[test]
    fn multi_ballot_with_every_style_contest_is_accepted() {
        let input = multi_input(&signed_multi_ballot());

        assert!(deserialize_and_check_multi_ballot(&input, VOTER_ID, &published_styles()).is_ok());
    }

    #[test]
    fn multi_ballot_missing_a_style_contest_is_rejected() {
        let input = multi_input(&signed_multi_ballot());
        let published = vec![contest_ids(&[&get_test_contest().id, OTHER_CONTEST_ID])];

        assert!(matches!(
            deserialize_and_check_multi_ballot(&input, VOTER_ID, &published),
            Err(CastVoteError::BallotStyleMismatch(message)) if message.contains(OTHER_CONTEST_ID)
        ));
    }

    #[test]
    fn multi_ballot_with_a_contest_outside_the_style_is_rejected() {
        let input = multi_input_with_contest_ids(&[&get_test_contest().id, OTHER_CONTEST_ID]);

        assert!(matches!(
            deserialize_and_check_multi_ballot(&input, VOTER_ID, &published_styles()),
            Err(CastVoteError::BallotStyleMismatch(message)) if message.contains(OTHER_CONTEST_ID)
        ));
    }

    #[test]
    fn multi_ballot_repeating_a_style_contest_is_rejected() {
        let contest_id = get_test_contest().id;
        let input = multi_input_with_contest_ids(&[&contest_id, &contest_id]);

        assert!(matches!(
            deserialize_and_check_multi_ballot(&input, VOTER_ID, &published_styles()),
            Err(CastVoteError::BallotStyleMismatch(message)) if message.contains(&contest_id)
        ));
    }

    #[test]
    fn multi_ballot_is_rejected_when_no_ballot_style_is_published() {
        let input = multi_input(&signed_multi_ballot());

        assert!(matches!(
            deserialize_and_check_multi_ballot(&input, VOTER_ID, &[]),
            Err(CastVoteError::BallotStyleMismatch(_))
        ));
    }

    #[test]
    fn acclaimed_contests_are_not_required_from_the_ballot() {
        let acclaimed = Contest {
            id: OTHER_CONTEST_ID.to_string(),
            is_acclaimed: Some(true),
            ..get_test_contest()
        };
        let style = BallotStyle {
            contests: vec![get_test_contest(), acclaimed],
            ..ballot_style()
        };

        assert_eq!(
            votable_contest_ids(&serde_json::to_string(&style).unwrap()).unwrap(),
            style_contest_ids()
        );
    }

    #[test]
    fn malformed_published_ballot_style_is_an_internal_error() {
        assert!(matches!(
            votable_contest_ids("{\"contests\": 1}"),
            Err(CastVoteError::CheckStatusInternalFailed(_))
        ));
    }
}
