// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! When a ballot box is sealed (VOTE-FREEZE): once voting at its election
//! has finished and the online grace period, if any, has ended.
//!
//! - **Finished:** every enabled channel is finished and at least one is
//!   CLOSED. A channel is finished when it is CLOSED, or when it never
//!   started and can no longer open under the platform's rules. Today that
//!   is only EARLY_VOTING once ONLINE has started (`EarlyVotingAfterOnline`).
//!   A never-started ONLINE, KIOSK or TELEPHONE channel holds the seal
//!   ([`holding_channel`] names it). Once an election has seals, opening any
//!   of its channels is refused.
//!   With the policy on, a never-started channel may be closed directly
//!   (NOT_STARTED → CLOSED), so it stops holding the seal.
//! - **Close time:** the latest `last_stopped_at` of the CLOSED channels that
//!   ran (a `first_started_at`); a channel closed without ever starting took
//!   no votes, so it doesn't move the close. When no closed channel ran, the
//!   latest `last_stopped_at` of the CLOSED channels.
//! - **Deadline:** the close time plus the [`grace_period`] when the ONLINE
//!   channel is enabled and actually ran (an ONLINE closed without ever
//!   starting took no online votes, so it has no grace).
//!
//! The cast check takes its grace period from [`grace_period`] too, but it
//! anchors it on the election's end date when there is one. With the policy
//! on, the cast check therefore also refuses once the box's seal deadline
//! (from its seal row, else from [`seal_deadline`]) has passed, so no
//! ballot after the signed deadline is accepted.

use chrono::{DateTime, Duration, Utc};
use sequent_core::ballot::{
    EGracePeriodPolicy, ElectionPresentation, ElectionStatus, VotingStatus, VotingStatusChannel,
};
use sequent_core::types::hasura::core::VotingChannels;

/// Every voting channel an election can enable.
const CHANNELS: [VotingStatusChannel; 4] = [
    VotingStatusChannel::ONLINE,
    VotingStatusChannel::KIOSK,
    VotingStatusChannel::EARLY_VOTING,
    VotingStatusChannel::TELEPHONE,
];

/// The election's grace period after an online close, or `None` when its
/// grace period policy is `NO_GRACE_PERIOD` (the default). It only ever
/// applies to the ONLINE channel. A period too long to represent is the
/// longest one.
pub fn grace_period(presentation: &ElectionPresentation) -> Option<Duration> {
    let policy = presentation
        .grace_period_policy
        .clone()
        .unwrap_or(EGracePeriodPolicy::NO_GRACE_PERIOD);
    if policy == EGracePeriodPolicy::NO_GRACE_PERIOD {
        return None;
    }
    let seconds = presentation.grace_period_secs.unwrap_or(0);
    Some(
        i64::try_from(seconds)
            .ok()
            .and_then(Duration::try_seconds)
            .unwrap_or(Duration::MAX),
    )
}

fn enabled_channels(channels: &VotingChannels) -> Vec<VotingStatusChannel> {
    CHANNELS
        .into_iter()
        .filter(|channel| channel.channel_from(channels) == Some(true))
        .collect()
}

/// Whether ONLINE voting has ever started at the election.
fn online_started(status: &ElectionStatus) -> bool {
    status.status_by_channel(VotingStatusChannel::ONLINE) != VotingStatus::NOT_STARTED
        || status
            .dates_by_channel(VotingStatusChannel::ONLINE)
            .first_started_at
            .is_some()
}

/// Whether `channel` is finished: CLOSED, or never started and unable to
/// open any more (EARLY_VOTING once ONLINE has started).
fn finished(status: &ElectionStatus, channel: VotingStatusChannel) -> bool {
    match status.status_by_channel(channel) {
        VotingStatus::CLOSED => true,
        VotingStatus::NOT_STARTED => {
            channel == VotingStatusChannel::EARLY_VOTING
                && status.dates_by_channel(channel).first_started_at.is_none()
                && online_started(status)
        }
        _ => false,
    }
}

/// The first enabled channel that holds the seal (not finished), if any.
pub fn holding_channel(
    status: &ElectionStatus,
    channels: &VotingChannels,
) -> Option<VotingStatusChannel> {
    enabled_channels(channels)
        .into_iter()
        .find(|channel| !finished(status, *channel))
}

/// The election's close time and seal deadline once its voting has
/// finished (see the module documentation); `None` until then.
pub fn seal_deadline(
    status: &ElectionStatus,
    channels: &VotingChannels,
    presentation: &ElectionPresentation,
) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
    let enabled = enabled_channels(channels);
    if holding_channel(status, channels).is_some() {
        return None;
    }
    let mut ran: Option<DateTime<Utc>> = None;
    let mut any: Option<DateTime<Utc>> = None;
    for channel in &enabled {
        if status.status_by_channel(*channel) == VotingStatus::CLOSED {
            let dates = status.dates_by_channel(*channel);
            let stopped = dates.last_stopped_at?;
            any = Some(any.map_or(stopped, |latest| latest.max(stopped)));
            if dates.first_started_at.is_some() {
                ran = Some(ran.map_or(stopped, |latest| latest.max(stopped)));
            }
        }
    }
    let closed_at = ran.or(any)?;
    let online_ran = status
        .dates_by_channel(VotingStatusChannel::ONLINE)
        .first_started_at
        .is_some();
    let grace = if enabled.contains(&VotingStatusChannel::ONLINE) && online_ran {
        grace_period(presentation).unwrap_or_else(Duration::zero)
    } else {
        Duration::zero()
    };
    Some((
        closed_at,
        closed_at
            .checked_add_signed(grace)
            .unwrap_or(DateTime::<Utc>::MAX_UTC),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::ballot::PeriodDates;

    const QUARTER_HOUR_SECS: u64 = 15 * 60;

    fn presentation(grace_secs: Option<u64>) -> ElectionPresentation {
        ElectionPresentation {
            grace_period_policy: Some(match grace_secs {
                Some(_) => EGracePeriodPolicy::GRACE_PERIOD_WITHOUT_ALERT,
                None => EGracePeriodPolicy::NO_GRACE_PERIOD,
            }),
            grace_period_secs: grace_secs,
            ..Default::default()
        }
    }

    fn channels(online: bool, kiosk: bool, early: bool) -> VotingChannels {
        VotingChannels {
            online: Some(online),
            kiosk: Some(kiosk),
            telephone: None,
            paper: None,
            early_voting: Some(early),
        }
    }

    fn at(minute: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(1_800_000_000 + minute * 60, 0).unwrap()
    }

    fn set(
        status: &mut ElectionStatus,
        channel: VotingStatusChannel,
        value: VotingStatus,
        started: Option<DateTime<Utc>>,
        stopped: Option<DateTime<Utc>>,
    ) {
        status.set_status_by_channel(channel, value);
        let dates = PeriodDates {
            first_started_at: started,
            last_started_at: started,
            first_paused_at: None,
            last_paused_at: None,
            first_stopped_at: stopped,
            last_stopped_at: stopped,
        };
        match channel {
            VotingStatusChannel::ONLINE => status.voting_period_dates = dates,
            VotingStatusChannel::KIOSK => status.kiosk_voting_period_dates = dates,
            VotingStatusChannel::EARLY_VOTING => status.early_voting_period_dates = dates,
            VotingStatusChannel::TELEPHONE => status.telephone_voting_period_dates = dates,
        }
    }

    fn online_closed_at(minute: i64) -> ElectionStatus {
        let mut status = ElectionStatus::default();
        set(
            &mut status,
            VotingStatusChannel::ONLINE,
            VotingStatus::CLOSED,
            Some(at(0)),
            Some(at(minute)),
        );
        status
    }

    #[test]
    fn without_grace_the_deadline_is_the_close() {
        let status = online_closed_at(60);
        assert_eq!(
            seal_deadline(&status, &channels(true, false, false), &presentation(None)),
            Some((at(60), at(60)))
        );
        // A grace period of 0 under a grace policy is the same.
        assert_eq!(
            seal_deadline(
                &status,
                &channels(true, false, false),
                &presentation(Some(0))
            ),
            Some((at(60), at(60)))
        );
    }

    #[test]
    fn a_quarter_hour_grace_moves_the_deadline() {
        let status = online_closed_at(60);
        assert_eq!(
            seal_deadline(
                &status,
                &channels(true, false, false),
                &presentation(Some(QUARTER_HOUR_SECS))
            ),
            Some((at(60), at(75)))
        );
    }

    #[test]
    fn an_open_or_paused_channel_holds_the_seal() {
        let mut status = online_closed_at(60);
        set(
            &mut status,
            VotingStatusChannel::KIOSK,
            VotingStatus::OPEN,
            Some(at(0)),
            None,
        );
        assert_eq!(
            seal_deadline(&status, &channels(true, true, false), &presentation(None)),
            None
        );
        set(
            &mut status,
            VotingStatusChannel::KIOSK,
            VotingStatus::PAUSED,
            Some(at(0)),
            None,
        );
        assert_eq!(
            seal_deadline(&status, &channels(true, true, false), &presentation(None)),
            None
        );
    }

    #[test]
    fn the_latest_close_of_the_enabled_channels_counts() {
        let mut status = online_closed_at(60);
        set(
            &mut status,
            VotingStatusChannel::KIOSK,
            VotingStatus::CLOSED,
            Some(at(0)),
            Some(at(90)),
        );
        assert_eq!(
            seal_deadline(
                &status,
                &channels(true, true, false),
                &presentation(Some(QUARTER_HOUR_SECS))
            ),
            Some((at(90), at(105)))
        );
    }

    #[test]
    fn early_voting_that_can_no_longer_open_does_not_hold_the_seal() {
        // Early voting enabled but never opened: ONLINE has started, so it
        // never can.
        let status = online_closed_at(60);
        assert_eq!(
            seal_deadline(&status, &channels(true, false, true), &presentation(None)),
            Some((at(60), at(60)))
        );
        assert_eq!(holding_channel(&status, &channels(true, false, true)), None);
    }

    #[test]
    fn closed_early_voting_with_online_never_started_is_not_due() {
        // Between the early-voting period and election day.
        let mut status = ElectionStatus::default();
        set(
            &mut status,
            VotingStatusChannel::EARLY_VOTING,
            VotingStatus::CLOSED,
            Some(at(0)),
            Some(at(30)),
        );
        assert_eq!(
            seal_deadline(&status, &channels(true, false, true), &presentation(None)),
            None
        );
        assert_eq!(
            holding_channel(&status, &channels(true, false, true)),
            Some(VotingStatusChannel::ONLINE)
        );
    }

    #[test]
    fn a_never_started_kiosk_holds_the_seal() {
        let status = online_closed_at(60);
        assert_eq!(
            seal_deadline(&status, &channels(true, true, false), &presentation(None)),
            None
        );
        assert_eq!(
            holding_channel(&status, &channels(true, true, false)),
            Some(VotingStatusChannel::KIOSK)
        );
    }

    #[test]
    fn a_channel_closed_without_starting_finishes_but_does_not_move_the_close() {
        // ONLINE ran and closed at 60; KIOSK never started and was closed
        // directly at 90 (Seal at close).
        let mut status = online_closed_at(60);
        set(
            &mut status,
            VotingStatusChannel::KIOSK,
            VotingStatus::CLOSED,
            None,
            Some(at(90)),
        );
        assert_eq!(holding_channel(&status, &channels(true, true, false)), None);
        assert_eq!(
            seal_deadline(
                &status,
                &channels(true, true, false),
                &presentation(Some(QUARTER_HOUR_SECS))
            ),
            Some((at(60), at(75)))
        );
        // Nothing ran: the close is the latest stop.
        let mut status = ElectionStatus::default();
        set(
            &mut status,
            VotingStatusChannel::KIOSK,
            VotingStatus::CLOSED,
            None,
            Some(at(90)),
        );
        assert_eq!(
            seal_deadline(&status, &channels(false, true, false), &presentation(None)),
            Some((at(90), at(90)))
        );
    }

    #[test]
    fn no_grace_when_online_never_ran() {
        // ONLINE closed directly without starting, KIOSK ran.
        let mut status = ElectionStatus::default();
        set(
            &mut status,
            VotingStatusChannel::ONLINE,
            VotingStatus::CLOSED,
            None,
            Some(at(40)),
        );
        set(
            &mut status,
            VotingStatusChannel::KIOSK,
            VotingStatus::CLOSED,
            Some(at(0)),
            Some(at(30)),
        );
        assert_eq!(
            seal_deadline(
                &status,
                &channels(true, true, false),
                &presentation(Some(QUARTER_HOUR_SECS))
            ),
            Some((at(30), at(30)))
        );
    }

    #[test]
    fn nothing_closed_is_not_finished() {
        let status = ElectionStatus::default();
        assert_eq!(
            seal_deadline(&status, &channels(true, true, false), &presentation(None)),
            None
        );
    }

    #[test]
    fn grace_applies_only_with_online_enabled() {
        let mut status = ElectionStatus::default();
        set(
            &mut status,
            VotingStatusChannel::KIOSK,
            VotingStatus::CLOSED,
            Some(at(0)),
            Some(at(30)),
        );
        assert_eq!(
            seal_deadline(
                &status,
                &channels(false, true, false),
                &presentation(Some(QUARTER_HOUR_SECS))
            ),
            Some((at(30), at(30)))
        );
    }

    #[test]
    fn a_huge_grace_period_does_not_overflow() {
        let status = online_closed_at(60);
        let (_, deadline) = seal_deadline(
            &status,
            &channels(true, false, false),
            &presentation(Some(u64::MAX)),
        )
        .unwrap();
        assert!(deadline > at(60));
    }
}
