// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Per-account sending capacity: the rules of the token bucket every
//! process shares through Postgres (`windmill::postgres::rate_bucket`).
//!
//! - The bucket holds up to `messages_per_second` tokens and refills at
//!   that rate. Each message handed to the provider takes one.
//! - Codes take from the whole bucket ([`BucketShare::WHOLE`]).
//! - Notices take only above `otp_reserved_per_second`
//!   ([`BucketShare::ABOVE_OTP_RESERVE`]): that many tokens always stay
//!   for codes, and while the bucket is below that line only codes go.
//!   A saturating bulk send therefore never delays a code by more than
//!   the reserve allows.
//! - Without `messages_per_second` the account has no bucket and nothing
//!   is throttled ([`Throttling::UNTHROTTLED`]). There is no global limit.

use sequent_core::types::messaging::{AccountLimits, MessagePurpose};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use strum_macros::{Display, EnumString};

/// Which tokens of the bucket a message may take.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString, Serialize, Deserialize)]
pub enum BucketShare {
    /// Any token: codes.
    WHOLE,
    /// Only tokens above the account's OTP reserve: notices, including
    /// bulk sends.
    ABOVE_OTP_RESERVE,
}

impl BucketShare {
    pub fn of(purpose: MessagePurpose) -> BucketShare {
        match purpose {
            MessagePurpose::OTP => BucketShare::WHOLE,
            MessagePurpose::NOTICE => BucketShare::ABOVE_OTP_RESERVE,
        }
    }
}

/// Whether an account's sends are throttled, and why. What the account's
/// limits show and what the dispatcher logs.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString, Serialize, Deserialize)]
pub enum Throttling {
    /// No `messages_per_second`: every message goes at once.
    #[strum(serialize = "No sending limit: messages are not throttled")]
    UNTHROTTLED,
    /// The account's own `messages_per_second`, with its OTP reserve.
    #[strum(serialize = "Throttled to the account's messages per second")]
    THROTTLED,
}

impl Throttling {
    pub fn of(limits: &AccountLimits) -> Throttling {
        match BucketLimits::of(limits) {
            Some(_) => Throttling::THROTTLED,
            None => Throttling::UNTHROTTLED,
        }
    }
}

/// The bucket of an account that sets `messages_per_second`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BucketLimits {
    /// Most tokens the bucket holds: one second of sending.
    pub capacity: f64,
    /// Tokens added per second.
    pub per_second: f64,
    /// Tokens notices leave for codes. Always below the capacity, so
    /// notices can still send: an account whose reserve is at least its
    /// rate keeps one token per second for notices.
    pub otp_reserve: f64,
}

impl BucketLimits {
    /// `None` without `messages_per_second` (or with 0): not throttled.
    pub fn of(limits: &AccountLimits) -> Option<BucketLimits> {
        let rate = limits.messages_per_second.filter(|rate| *rate > 0)?;
        let reserve = limits
            .otp_reserved_per_second
            .unwrap_or(0)
            .min(rate.saturating_sub(1));
        Some(BucketLimits {
            capacity: f64::from(rate),
            per_second: f64::from(rate),
            otp_reserve: f64::from(reserve),
        })
    }

    /// Tokens that must remain after a message of `share` takes one.
    pub fn floor(&self, share: BucketShare) -> f64 {
        match share {
            BucketShare::WHOLE => 0.0,
            BucketShare::ABOVE_OTP_RESERVE => self.otp_reserve,
        }
    }

    /// The bucket's tokens `elapsed_seconds` after it held `tokens`.
    pub fn refilled(&self, tokens: f64, elapsed_seconds: f64) -> f64 {
        (tokens + elapsed_seconds.max(0.0) * self.per_second).min(self.capacity)
    }

    /// The tokens left after a message of `share` takes one from a bucket
    /// holding `tokens` (already refilled), or `None` when it must wait.
    /// The reference for the single `UPDATE` in windmill.
    pub fn take(&self, tokens: f64, share: BucketShare) -> Option<f64> {
        (tokens >= self.floor(share) + 1.0).then(|| tokens - 1.0)
    }

    /// How long until a message of `share` may take a token from a bucket
    /// holding `tokens` (already refilled), if nobody else takes one.
    pub fn wait(&self, tokens: f64, share: BucketShare) -> Duration {
        let missing = self.floor(share) + 1.0 - tokens;
        if missing <= 0.0 {
            Duration::ZERO
        } else {
            Duration::from_secs_f64(missing / self.per_second)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits(rate: Option<u32>, reserved: Option<u32>) -> AccountLimits {
        AccountLimits {
            messages_per_second: rate,
            otp_reserved_per_second: reserved,
            allowed_calling_codes: vec![],
            otp_concurrency: None,
        }
    }

    #[test]
    fn accounts_without_a_rate_are_not_throttled() {
        assert_eq!(BucketLimits::of(&limits(None, Some(4))), None);
        assert_eq!(BucketLimits::of(&limits(Some(0), None)), None);
        assert_eq!(
            Throttling::of(&limits(None, None)).to_string(),
            "No sending limit: messages are not throttled"
        );
        assert_eq!(
            Throttling::of(&limits(Some(5), None)),
            Throttling::THROTTLED
        );
    }

    #[test]
    fn the_reserve_always_leaves_notices_a_token() {
        let bucket = BucketLimits::of(&limits(Some(10), Some(4))).unwrap();
        assert_eq!(bucket.capacity, 10.0);
        assert_eq!(bucket.otp_reserve, 4.0);
        let tight = BucketLimits::of(&limits(Some(3), Some(9))).unwrap();
        assert_eq!(tight.otp_reserve, 2.0);
        assert_eq!(tight.take(3.0, BucketShare::ABOVE_OTP_RESERVE), Some(2.0));
    }

    #[test]
    fn notices_stop_at_the_reserve_and_codes_go_on() {
        let bucket = BucketLimits::of(&limits(Some(10), Some(4))).unwrap();
        let mut tokens = bucket.capacity;
        let mut notices = 0;
        while let Some(left) = bucket.take(tokens, BucketShare::ABOVE_OTP_RESERVE) {
            tokens = left;
            notices += 1;
        }
        assert_eq!(notices, 6);
        assert_eq!(tokens, 4.0);
        assert!(bucket.wait(tokens, BucketShare::ABOVE_OTP_RESERVE) > Duration::ZERO);
        // The reserve is there for codes right away.
        let mut codes = 0;
        while let Some(left) = bucket.take(tokens, BucketShare::WHOLE) {
            tokens = left;
            codes += 1;
        }
        assert_eq!(codes, 4);
        assert_eq!(
            bucket.wait(tokens, BucketShare::WHOLE),
            Duration::from_millis(100)
        );
        // A notice waits until the reserve is full again plus one token.
        assert_eq!(
            bucket.wait(tokens, BucketShare::ABOVE_OTP_RESERVE),
            Duration::from_millis(500)
        );
    }

    #[test]
    fn refilling_is_by_elapsed_time_up_to_the_capacity() {
        let bucket = BucketLimits::of(&limits(Some(10), None)).unwrap();
        assert_eq!(bucket.refilled(0.0, 0.25), 2.5);
        assert_eq!(bucket.refilled(9.0, 5.0), 10.0);
        // A clock that went back never removes tokens.
        assert_eq!(bucket.refilled(3.0, -1.0), 3.0);
    }

    #[test]
    fn the_share_follows_the_purpose() {
        assert_eq!(BucketShare::of(MessagePurpose::OTP), BucketShare::WHOLE);
        assert_eq!(
            BucketShare::of(MessagePurpose::NOTICE),
            BucketShare::ABOVE_OTP_RESERVE
        );
    }
}
