// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Decisions about delivery attempts of one logical message: whether to
//! send, wait, retry or fall back. Attempts are persisted before dispatch,
//! so a replayed job sees what an earlier run did.

use crate::sender::FailureKind;
use chrono::Duration;
use sequent_core::types::messaging::{MessageAttemptState, MessageChannel, MessagePurpose};

/// What the ledger holds about one earlier attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttemptRecord {
    pub attempt: i32,
    pub channel: MessageChannel,
    pub state: MessageAttemptState,
    /// For failed attempts.
    pub failure: Option<FailureKind>,
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
    /// Retry the same channel after `after`.
    RetryLater { after: Duration },
    /// Nothing more to try.
    GiveUp,
}

/// Transient failures on one channel are retried this many times in total.
pub const MAX_TRANSIENT_ATTEMPTS_PER_CHANNEL: usize = 3;

fn backoff(failures: usize) -> Duration {
    match failures {
        0 | 1 => Duration::seconds(30),
        2 => Duration::minutes(2),
        _ => Duration::minutes(10),
    }
}

/// Where a notice may go next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoticeRoute<'a> {
    /// The voter's channel, or the single channel of the send.
    pub first: MessageChannel,
    /// The event's notice fallback order.
    pub fallback: &'a [MessageChannel],
    /// Channels the voter verified.
    pub verified: &'a [MessageChannel],
    /// Channels the event and the voter's election offer for notices.
    pub eligible: &'a [MessageChannel],
}

/// Decides the next step for a logical message given its attempts so far.
///
/// Codes are never retried or moved to another channel here: the voter
/// asks for another code explicitly, and Keycloak issues a new challenge.
pub fn decide(
    purpose: MessagePurpose,
    route: &NoticeRoute<'_>,
    attempts: &[AttemptRecord],
) -> Dispatch {
    let next_attempt = attempts.iter().map(|a| a.attempt).max().unwrap_or(0) + 1;
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
        return if route.eligible.contains(&route.first) {
            Dispatch::Send {
                attempt: next_attempt,
                channel: route.first,
            }
        } else {
            next_fallback(purpose, route, attempts, next_attempt)
        };
    };
    if purpose == MessagePurpose::OTP {
        return Dispatch::GiveUp;
    }
    if last.failure == Some(FailureKind::TRANSIENT) {
        let failures_on_channel = attempts
            .iter()
            .filter(|a| a.channel == last.channel)
            .count();
        if failures_on_channel < MAX_TRANSIENT_ATTEMPTS_PER_CHANNEL {
            return Dispatch::RetryLater {
                after: backoff(failures_on_channel),
            };
        }
    }
    next_fallback(purpose, route, attempts, next_attempt)
}

fn next_fallback(
    purpose: MessagePurpose,
    route: &NoticeRoute<'_>,
    attempts: &[AttemptRecord],
    next_attempt: i32,
) -> Dispatch {
    if purpose == MessagePurpose::OTP {
        return Dispatch::GiveUp;
    }
    route
        .fallback
        .iter()
        .find(|channel| {
            **channel != route.first
                && route.verified.contains(channel)
                && route.eligible.contains(channel)
                && !attempts.iter().any(|a| a.channel == **channel)
        })
        .map(|channel| Dispatch::Send {
            attempt: next_attempt,
            channel: *channel,
        })
        .unwrap_or(Dispatch::GiveUp)
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

    fn record(attempt: i32, channel: MessageChannel, state: MessageAttemptState) -> AttemptRecord {
        AttemptRecord {
            attempt,
            channel,
            state,
            failure: None,
        }
    }

    fn failed(attempt: i32, channel: MessageChannel, kind: FailureKind) -> AttemptRecord {
        AttemptRecord {
            attempt,
            channel,
            state: FAILED,
            failure: Some(kind),
        }
    }

    const ALL: &[MessageChannel] = &[EMAIL, SMS, WHATSAPP, VIBER, MESSENGER];

    fn route(first: MessageChannel) -> NoticeRoute<'static> {
        NoticeRoute {
            first,
            fallback: &[VIBER, SMS, EMAIL],
            verified: &[WHATSAPP, SMS, EMAIL],
            eligible: ALL,
        }
    }

    #[test]
    fn a_new_message_is_sent_on_its_first_channel() {
        assert_eq!(
            decide(MessagePurpose::NOTICE, &route(WHATSAPP), &[]),
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
                    &route(WHATSAPP),
                    &[record(1, WHATSAPP, state)]
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
                    decide(purpose, &route(WHATSAPP), &[record(1, WHATSAPP, state)]),
                    Dispatch::AwaitReconciliation
                );
            }
        }
    }

    #[test]
    fn transient_failures_are_retried_with_bounded_backoff() {
        let one = [failed(1, WHATSAPP, FailureKind::TRANSIENT)];
        assert_eq!(
            decide(MessagePurpose::NOTICE, &route(WHATSAPP), &one),
            Dispatch::RetryLater {
                after: Duration::seconds(30)
            }
        );
        let two = [
            failed(1, WHATSAPP, FailureKind::TRANSIENT),
            failed(2, WHATSAPP, FailureKind::TRANSIENT),
        ];
        assert_eq!(
            decide(MessagePurpose::NOTICE, &route(WHATSAPP), &two),
            Dispatch::RetryLater {
                after: Duration::minutes(2)
            }
        );
        let three = [
            failed(1, WHATSAPP, FailureKind::TRANSIENT),
            failed(2, WHATSAPP, FailureKind::TRANSIENT),
            failed(3, WHATSAPP, FailureKind::TRANSIENT),
        ];
        assert_eq!(
            decide(MessagePurpose::NOTICE, &route(WHATSAPP), &three),
            Dispatch::Send {
                attempt: 4,
                channel: SMS
            }
        );
    }

    #[test]
    fn confirmed_failures_fall_back_to_verified_eligible_channels_in_order() {
        // VIBER is first in the fallback order but not verified.
        let attempts = [failed(1, WHATSAPP, FailureKind::PERMANENT)];
        assert_eq!(
            decide(MessagePurpose::NOTICE, &route(WHATSAPP), &attempts),
            Dispatch::Send {
                attempt: 2,
                channel: SMS
            }
        );
        let attempts = [
            failed(1, WHATSAPP, FailureKind::PERMANENT),
            failed(2, SMS, FailureKind::PERMANENT),
        ];
        assert_eq!(
            decide(MessagePurpose::NOTICE, &route(WHATSAPP), &attempts),
            Dispatch::Send {
                attempt: 3,
                channel: EMAIL
            }
        );
        let attempts = [
            failed(1, WHATSAPP, FailureKind::PERMANENT),
            failed(2, SMS, FailureKind::PERMANENT),
            failed(3, EMAIL, FailureKind::PERMANENT),
        ];
        assert_eq!(
            decide(MessagePurpose::NOTICE, &route(WHATSAPP), &attempts),
            Dispatch::GiveUp
        );
    }

    #[test]
    fn fallback_skips_channels_the_voters_election_does_not_offer() {
        let route = NoticeRoute {
            eligible: &[WHATSAPP, EMAIL],
            ..route(WHATSAPP)
        };
        let attempts = [failed(1, WHATSAPP, FailureKind::PERMANENT)];
        assert_eq!(
            decide(MessagePurpose::NOTICE, &route, &attempts),
            Dispatch::Send {
                attempt: 2,
                channel: EMAIL
            }
        );
    }

    #[test]
    fn an_ineligible_first_channel_starts_with_the_fallback() {
        let route = NoticeRoute {
            eligible: &[SMS, EMAIL],
            ..route(WHATSAPP)
        };
        assert_eq!(
            decide(MessagePurpose::NOTICE, &route, &[]),
            Dispatch::Send {
                attempt: 1,
                channel: SMS
            }
        );
    }

    #[test]
    fn codes_never_retry_or_fall_back_on_their_own() {
        for kind in [FailureKind::TRANSIENT, FailureKind::PERMANENT] {
            assert_eq!(
                decide(
                    MessagePurpose::OTP,
                    &route(WHATSAPP),
                    &[failed(1, WHATSAPP, kind)]
                ),
                Dispatch::GiveUp
            );
        }
        let route = NoticeRoute {
            eligible: &[SMS],
            ..route(WHATSAPP)
        };
        assert_eq!(decide(MessagePurpose::OTP, &route, &[]), Dispatch::GiveUp);
    }

    #[test]
    fn late_reports_cannot_regress_delivery() {
        assert_eq!(apply_report(DELIVERED, FAILED), None);
        assert_eq!(apply_report(DELIVERED, ACCEPTED), None);
        assert_eq!(apply_report(ACCEPTED, DELIVERED), Some(DELIVERED));
        assert_eq!(apply_report(UNKNOWN, ACCEPTED), Some(ACCEPTED));
        assert_eq!(apply_report(FAILED, DELIVERED), Some(DELIVERED));
    }
}
