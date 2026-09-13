// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Channel transitions and displayed dates are part of the election contract.
//! Use bounded clock comparisons; no test depends on sleeping or wall-clock equality.

use chrono::{TimeZone, Utc};
use sequent_core::ballot::*;
use sequent_core::types::hasura::core::VotingChannels;

const CHANNELS: [VotingStatusChannel; 4] = [
    VotingStatusChannel::ONLINE,
    VotingStatusChannel::KIOSK,
    VotingStatusChannel::EARLY_VOTING,
    VotingStatusChannel::TELEPHONE,
];

#[test]
fn status_predicates_distinguish_paused_from_closed() {
    // "Open" includes a temporarily paused voting period; it is not permission
    // to cast a ballot. Callers also consult is_paused when deciding that.
    for (status, started, open, paused, closed) in [
        (VotingStatus::NOT_STARTED, false, false, false, false),
        (VotingStatus::OPEN, true, true, false, false),
        (VotingStatus::PAUSED, true, true, true, false),
        (VotingStatus::CLOSED, true, false, false, true),
    ] {
        assert_eq!(status.is_not_started(), !started);
        assert_eq!(status.is_started(), started);
        assert_eq!(status.is_open(), open);
        assert_eq!(status.is_paused(), paused);
        assert_eq!(status.is_closed(), closed);
        assert_eq!(status.is_closed_or_never_started(), closed || !started);
    }
}

#[test]
fn changing_one_channel_preserves_the_others_and_records_its_transition_dates()
{
    for selected in CHANNELS {
        let mut election = ElectionStatus::default();
        let mut event = ElectionEventStatus::default();
        let before = Utc::now();
        for status in [
            VotingStatus::NOT_STARTED,
            VotingStatus::OPEN,
            VotingStatus::PAUSED,
            VotingStatus::CLOSED,
        ] {
            election.set_status_by_channel(selected, status);
            event.set_status_by_channel(selected, status);
            for channel in CHANNELS {
                let expected = if channel == selected {
                    status
                } else {
                    VotingStatus::NOT_STARTED
                };
                assert_eq!(election.status_by_channel(channel), expected);
                assert_eq!(event.status_by_channel(channel), expected);
            }
        }
        let dates = election.dates_by_channel(selected);
        let after = Utc::now();
        for instant in [
            dates.first_started_at,
            dates.last_started_at,
            dates.first_paused_at,
            dates.last_paused_at,
            dates.first_stopped_at,
            dates.last_stopped_at,
        ] {
            assert!((before..=after).contains(&instant.unwrap()));
        }
        for channel in CHANNELS {
            if channel != selected {
                assert_eq!(
                    election.dates_by_channel(channel),
                    PeriodDates::default()
                );
            }
        }

        // Reopening updates the most recent start, preserving the first start
        // that reports use for the original voting-period boundary.
        election.set_status_by_channel(selected, VotingStatus::OPEN);
        let reopened = election.dates_by_channel(selected);
        assert_eq!(reopened.first_started_at, dates.first_started_at);
        assert!(reopened.last_started_at >= dates.last_started_at);
        assert_eq!(reopened.first_stopped_at, dates.first_stopped_at);
    }
}

#[test]
fn online_open_or_close_ends_started_early_voting_but_other_changes_do_not() {
    for prior in [
        VotingStatus::NOT_STARTED,
        VotingStatus::OPEN,
        VotingStatus::PAUSED,
        VotingStatus::CLOSED,
    ] {
        for channel in CHANNELS {
            for next in [
                VotingStatus::NOT_STARTED,
                VotingStatus::OPEN,
                VotingStatus::PAUSED,
                VotingStatus::CLOSED,
            ] {
                let mut event = ElectionEventStatus {
                    early_voting_status: prior,
                    ..Default::default()
                };
                let mut election = ElectionStatus {
                    early_voting_status: prior,
                    ..Default::default()
                };
                let closes = channel == VotingStatusChannel::ONLINE
                    && matches!(
                        next,
                        VotingStatus::OPEN | VotingStatus::CLOSED
                    )
                    && prior != VotingStatus::NOT_STARTED;
                let expected =
                    if closes { VotingStatus::CLOSED } else { prior };
                event.close_early_voting_if_online_status_change(channel, next);
                election
                    .close_early_voting_if_online_status_change(channel, next);
                assert_eq!(event.early_voting_status, expected);
                assert_eq!(election.early_voting_status, expected,
                    "event and election must apply the same early-voting transition rule");
            }
        }
    }
}

#[test]
fn date_serialization_preserves_absent_values_and_utc_instants() {
    let instant = Utc.with_ymd_and_hms(2026, 10, 1, 9, 30, 0).unwrap();
    let dates = PeriodDates {
        first_started_at: Some(instant),
        last_started_at: Some(instant),
        first_paused_at: Some(instant),
        last_paused_at: Some(instant),
        first_stopped_at: Some(instant),
        last_stopped_at: Some(instant),
    }
    .to_string_fields();
    for value in [
        dates.first_started_at,
        dates.last_started_at,
        dates.first_paused_at,
        dates.last_paused_at,
        dates.first_stopped_at,
        dates.last_stopped_at,
    ] {
        assert_eq!(value.as_deref(), Some("2026-10-01T09:30:00+00:00"));
    }
    assert_eq!(
        PeriodDates::default().to_string_fields(),
        StringifiedPeriodDates::default()
    );
    assert_eq!(format_date(&None, "not scheduled"), "not scheduled");
    assert_eq!(
        format_date(&Some(instant), "unused"),
        "2026-10-01T09:30:00+00:00"
    );
}

#[test]
fn channel_settings_preserve_explicit_false_and_unspecified_values() {
    let settings = VotingChannels {
        online: Some(true),
        kiosk: Some(false),
        early_voting: None,
        telephone: Some(true),
        ..Default::default()
    };
    assert_eq!(
        VotingStatusChannel::ONLINE.channel_from(&settings),
        Some(true)
    );
    assert_eq!(
        VotingStatusChannel::KIOSK.channel_from(&settings),
        Some(false)
    );
    assert_eq!(
        VotingStatusChannel::EARLY_VOTING.channel_from(&settings),
        None
    );
    assert_eq!(
        VotingStatusChannel::TELEPHONE.channel_from(&settings),
        Some(true)
    );
}
