// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Decisions about delivery attempts of one logical message: whether to
//! send, wait or stop. Attempts are persisted before dispatch, so a
//! replayed job sees what an earlier run did. A message only ever goes to
//! its one channel: there is no fallback to another one.

use crate::sender::FailureKind;
use chrono::{DateTime, Duration, Utc};
use sequent_core::types::messaging::{
    MessageAttemptState, MessageChannel, MessagePurpose, LATE_FAILURE_RETRY_HOURS,
};

/// What the ledger holds about one earlier attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttemptRecord {
    pub attempt: i32,
    /// `None` for a message recorded without any channel ("No contact
    /// channel"), which is never retried.
    pub channel: Option<MessageChannel>,
    pub state: MessageAttemptState,
    /// For failed attempts.
    pub failure: Option<FailureKind>,
    /// For transient failures that will be retried: when the next attempt
    /// is due. `None` when no retry is scheduled.
    pub next_retry_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dispatch {
    /// Persist attempt `attempt` on `channel`, then send it.
    Send {
        attempt: i32,
        channel: MessageChannel,
    },
    /// An earlier attempt was accepted or delivered: never send again.
    AlreadySent,
    /// An earlier attempt may have reached the provider. Reconcile before
    /// anything else; never resend blindly.
    AwaitReconciliation,
    /// A retry on the same channel is scheduled for `at`; nothing is sent
    /// before then.
    RetryLater { at: DateTime<Utc> },
    /// Nothing more to try: the last attempt's failure is final.
    GiveUp,
}

/// A message is tried at most this many times, all on its one channel.
pub const MAX_ATTEMPTS: i32 = 3;

/// The wait after a transient failure of attempt `attempt`, before the next
/// one: 1 minute after the first, 5 minutes after the second. The third
/// failure is final.
pub fn backoff(attempt: i32) -> Option<Duration> {
    match attempt {
        1 => Some(Duration::minutes(1)),
        2 => Some(Duration::minutes(5)),
        _ => None,
    }
}

/// When the attempt after `attempt` is due, if a failure of `kind` at `now`
/// is to be retried. Codes are never retried: the voter asks for another
/// code, and Keycloak issues a new challenge.
pub fn next_retry_at(
    purpose: MessagePurpose,
    attempt: i32,
    kind: FailureKind,
    now: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    if purpose == MessagePurpose::OTP || kind != FailureKind::TRANSIENT {
        return None;
    }
    backoff(attempt).map(|wait| now + wait)
}

/// Whether a failure reported at `now` for an attempt made at
/// `attempted_at` came too late to be retried: sent again so late, the
/// message could reach the voter after it stopped being useful.
pub fn reported_too_late(attempted_at: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    now - attempted_at >= Duration::hours(LATE_FAILURE_RETRY_HOURS)
}

/// Decides the next step for a logical message on `channel`, given its
/// attempts so far, at `now`.
pub fn decide(
    purpose: MessagePurpose,
    channel: MessageChannel,
    attempts: &[AttemptRecord],
    now: DateTime<Utc>,
) -> Dispatch {
    if attempts.iter().any(|a| {
        matches!(
            a.state,
            MessageAttemptState::ACCEPTED | MessageAttemptState::DELIVERED
        )
    }) {
        return Dispatch::AlreadySent;
    }
    if attempts.iter().any(|a| {
        matches!(
            a.state,
            MessageAttemptState::QUEUED | MessageAttemptState::UNKNOWN
        )
    }) {
        return Dispatch::AwaitReconciliation;
    }
    let Some(last) = attempts.iter().max_by_key(|a| a.attempt) else {
        return Dispatch::Send {
            attempt: 1,
            channel,
        };
    };
    if purpose == MessagePurpose::OTP
        || last.failure != Some(FailureKind::TRANSIENT)
        || last.attempt >= MAX_ATTEMPTS
    {
        return Dispatch::GiveUp;
    }
    match (last.next_retry_at, last.channel) {
        // The same channel as the failed attempt, whatever was asked now.
        (Some(at), Some(channel)) if at <= now => Dispatch::Send {
            attempt: last.attempt + 1,
            channel,
        },
        // Recorded without a channel: nothing to send it on.
        (Some(_), None) => Dispatch::GiveUp,
        (Some(at), Some(_)) => Dispatch::RetryLater { at },
        // No retry was scheduled, such as for a message without a stored
        // template: its failure is final.
        (None, _) => Dispatch::GiveUp,
    }
}

/// The state an attempt moves to when a provider report or reconciliation
/// says `reported`. `None` when the report must be ignored, such as a late
/// failure after delivery.
pub fn apply_report(
    current: MessageAttemptState,
    reported: MessageAttemptState,
) -> Option<MessageAttemptState> {
    current.can_transition_to(reported).then_some(reported)
}

#[cfg(test)]
mod tests {
    use super::*;
    use MessageAttemptState::*;
    use MessageChannel::*;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-10-09T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn record(attempt: i32, channel: MessageChannel, state: MessageAttemptState) -> AttemptRecord {
        AttemptRecord {
            attempt,
            channel: Some(channel),
            state,
            failure: None,
            next_retry_at: None,
        }
    }

    fn failed(
        attempt: i32,
        channel: MessageChannel,
        kind: FailureKind,
        retry_in: Option<Duration>,
    ) -> AttemptRecord {
        AttemptRecord {
            attempt,
            channel: Some(channel),
            state: FAILED,
            failure: Some(kind),
            next_retry_at: retry_in.map(|wait| now() + wait),
        }
    }

    #[test]
    fn a_new_message_is_sent_on_its_channel() {
        assert_eq!(
            decide(MessagePurpose::NOTICE, WHATSAPP, &[], now()),
            Dispatch::Send {
                attempt: 1,
                channel: WHATSAPP
            }
        );
    }

    #[test]
    fn replaying_a_job_never_resends_an_accepted_message() {
        for state in [ACCEPTED, DELIVERED] {
            assert_eq!(
                decide(
                    MessagePurpose::NOTICE,
                    WHATSAPP,
                    &[record(1, WHATSAPP, state)],
                    now()
                ),
                Dispatch::AlreadySent
            );
        }
    }

    #[test]
    fn unknown_or_unfinished_attempts_wait_for_reconciliation() {
        for state in [UNKNOWN, QUEUED] {
            for purpose in [MessagePurpose::NOTICE, MessagePurpose::OTP] {
                assert_eq!(
                    decide(purpose, WHATSAPP, &[record(1, WHATSAPP, state)], now()),
                    Dispatch::AwaitReconciliation
                );
            }
        }
    }

    #[test]
    fn the_backoff_is_one_then_five_minutes_and_the_third_failure_is_final() {
        assert_eq!(backoff(1), Some(Duration::minutes(1)));
        assert_eq!(backoff(2), Some(Duration::minutes(5)));
        assert_eq!(backoff(3), None);
        let at = |attempt| {
            next_retry_at(
                MessagePurpose::NOTICE,
                attempt,
                FailureKind::TRANSIENT,
                now(),
            )
        };
        assert_eq!(at(1), Some(now() + Duration::minutes(1)));
        assert_eq!(at(2), Some(now() + Duration::minutes(5)));
        assert_eq!(at(3), None);
    }

    #[test]
    fn a_transient_failure_is_retried_on_the_same_channel_only_once_due() {
        // Not due yet: a job running now sends nothing.
        let pending = [failed(
            1,
            VIBER,
            FailureKind::TRANSIENT,
            Some(Duration::seconds(30)),
        )];
        assert_eq!(
            decide(MessagePurpose::NOTICE, VIBER, &pending, now()),
            Dispatch::RetryLater {
                at: now() + Duration::seconds(30)
            }
        );
        // Due: the next attempt goes to the same channel, even when asked
        // for another one.
        let due = [failed(
            1,
            VIBER,
            FailureKind::TRANSIENT,
            Some(Duration::zero()),
        )];
        assert_eq!(
            decide(MessagePurpose::NOTICE, SMS, &due, now()),
            Dispatch::Send {
                attempt: 2,
                channel: VIBER
            }
        );
    }

    #[test]
    fn three_attempts_then_the_failure_is_final() {
        let attempts = [
            failed(1, VIBER, FailureKind::TRANSIENT, None),
            failed(2, VIBER, FailureKind::TRANSIENT, None),
            failed(3, VIBER, FailureKind::TRANSIENT, Some(Duration::zero())),
        ];
        assert_eq!(
            decide(MessagePurpose::NOTICE, VIBER, &attempts, now()),
            Dispatch::GiveUp
        );
    }

    #[test]
    fn permanent_failures_and_unscheduled_retries_are_final_and_never_change_channel() {
        for attempts in [
            vec![failed(1, WHATSAPP, FailureKind::PERMANENT, None)],
            vec![failed(1, WHATSAPP, FailureKind::TRANSIENT, None)],
        ] {
            assert_eq!(
                decide(MessagePurpose::NOTICE, WHATSAPP, &attempts, now()),
                Dispatch::GiveUp
            );
        }
    }

    #[test]
    fn codes_are_never_retried() {
        for kind in [FailureKind::TRANSIENT, FailureKind::PERMANENT] {
            assert_eq!(
                decide(
                    MessagePurpose::OTP,
                    WHATSAPP,
                    &[failed(1, WHATSAPP, kind, Some(Duration::zero()))],
                    now()
                ),
                Dispatch::GiveUp
            );
            assert_eq!(next_retry_at(MessagePurpose::OTP, 1, kind, now()), None);
        }
    }

    #[test]
    fn a_message_without_a_channel_is_never_sent_again() {
        let mut no_channel = failed(1, WHATSAPP, FailureKind::TRANSIENT, Some(Duration::zero()));
        no_channel.channel = None;
        assert_eq!(
            decide(MessagePurpose::NOTICE, SMS, &[no_channel], now()),
            Dispatch::GiveUp
        );
    }

    #[test]
    fn failures_reported_an_hour_after_the_attempt_are_too_late() {
        assert!(!reported_too_late(now() - Duration::minutes(59), now()));
        assert!(reported_too_late(now() - Duration::hours(1), now()));
        assert!(reported_too_late(now() - Duration::hours(48), now()));
    }

    #[test]
    fn late_reports_cannot_regress_delivery() {
        assert_eq!(apply_report(DELIVERED, FAILED), None);
        assert_eq!(apply_report(DELIVERED, ACCEPTED), None);
        assert_eq!(apply_report(ACCEPTED, DELIVERED), Some(DELIVERED));
        assert_eq!(apply_report(QUEUED, DELIVERED), Some(DELIVERED));
        assert_eq!(apply_report(UNKNOWN, ACCEPTED), Some(ACCEPTED));
        assert_eq!(apply_report(FAILED, DELIVERED), Some(DELIVERED));
    }
}
