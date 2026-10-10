// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use sequent_core::ballot::TYPES_VERSION;
use serde_json::json;
use strand::context::Ctx;
use strand::elgamal::{PrivateKey, PublicKey};

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

fn encrypted_contest(contest_id: &str) -> HashableBallotContest<RistrettoCtx> {
    let ctx = RistrettoCtx;
    let mut rng = ctx.get_rng();
    let secret_key = PrivateKey::gen(&ctx);
    let public_key = PublicKey::from_element(secret_key.pk_element(), &ctx);
    let (ciphertext, proof, _) = public_key
        .encrypt_and_pok(&ctx.rnd(&mut rng), &DEFAULT_PLAINTEXT_LABEL)
        .unwrap();
    HashableBallotContest {
        contest_id: contest_id.to_string(),
        ciphertext,
        proof,
    }
}

fn cast_input(
    election_id: Uuid,
    issue_date: &str,
    contests: Vec<HashableBallotContest<RistrettoCtx>>,
) -> InsertCastVoteInput {
    let hashable_ballot = HashableBallot {
        version: TYPES_VERSION,
        issue_date: issue_date.to_string(),
        contests: HashableBallot::serialize_contests(&contests).unwrap(),
        config: "style".to_string(),
        ballot_style_hash: "style-hash".to_string(),
    };
    let signed_hashable_ballot = SignedHashableBallot {
        version: hashable_ballot.version,
        issue_date: hashable_ballot.issue_date.clone(),
        contests: hashable_ballot.contests.clone(),
        config: hashable_ballot.config.clone(),
        ballot_style_hash: hashable_ballot.ballot_style_hash.clone(),
        voter_signing_pk: None,
        voter_ballot_signature: None,
    };
    InsertCastVoteInput {
        ballot_id: hash_ballot(&hashable_ballot).unwrap(),
        election_id,
        content: serde_json::to_string(&signed_hashable_ballot).unwrap(),
    }
}

#[test]
fn contest_ciphertext_moved_into_another_ballot_keeps_its_fingerprint() {
    let election_id = Uuid::new_v4();
    let original = encrypted_contest("contest");
    let moved = HashableBallotContest {
        contest_id: "other-contest".to_string(),
        ..original.clone()
    };
    let first = cast_input(election_id, "2026-01-01", vec![original]);
    let second = cast_input(
        election_id,
        "2026-01-02",
        vec![encrypted_contest("contest"), moved],
    );
    assert_ne!(first.ballot_id, second.ballot_id);

    let (_, _, _, first_fingerprints) = deserialize_and_check_ballot(&first, "first").unwrap();
    let (_, _, _, second_fingerprints) = deserialize_and_check_ballot(&second, "second").unwrap();

    assert_eq!(first_fingerprints.len(), 1);
    assert_eq!(second_fingerprints.len(), 2);
    assert_ne!(second_fingerprints[0], first_fingerprints[0]);
    assert_eq!(second_fingerprints[1], first_fingerprints[0]);
}

#[test]
fn ciphertext_already_cast_is_not_retried() {
    assert!(matches!(
        skip_or_propagate(CastVoteError::CiphertextAlreadyCast),
        Ok(InsertCastVoteResult::SkipRetryFailure(
            CastVoteError::CiphertextAlreadyCast
        ))
    ));
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
