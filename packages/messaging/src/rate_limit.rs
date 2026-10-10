// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Per-account sending rate with capacity kept for codes. Notices may use
//! the account rate minus the OTP reservation; codes may use all of it, so
//! a saturating bulk send never delays them.
//!
//! The limit applies within one process. Bulk workers each get their share
//! of the account rate through their configured concurrency.

use sequent_core::types::messaging::{AccountLimits, MessagePurpose};
use std::sync::Mutex;
use tokio::time::{sleep, Duration, Instant};

#[derive(Debug)]
struct Bucket {
    capacity: f64,
    tokens: f64,
    per_second: f64,
    updated: Instant,
}

impl Bucket {
    fn new(per_second: u32, now: Instant) -> Bucket {
        Bucket {
            capacity: f64::from(per_second.max(1)),
            tokens: f64::from(per_second.max(1)),
            per_second: f64::from(per_second.max(1)),
            updated: now,
        }
    }

    fn refill(&mut self, now: Instant) {
        let elapsed = now.saturating_duration_since(self.updated).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.per_second).min(self.capacity);
        self.updated = now;
    }

    /// Time until one token is available.
    fn wait(&self) -> Duration {
        if self.tokens >= 1.0 {
            Duration::ZERO
        } else {
            Duration::from_secs_f64((1.0 - self.tokens) / self.per_second)
        }
    }
}

#[derive(Debug)]
struct Buckets {
    all: Bucket,
    notices: Option<Bucket>,
}

#[derive(Debug)]
pub struct RateLimiter {
    buckets: Option<Mutex<Buckets>>,
}

impl RateLimiter {
    pub fn new(limits: &AccountLimits) -> RateLimiter {
        let now = Instant::now();
        let buckets = limits.messages_per_second.map(|rate| {
            let reserved = limits
                .otp_reserved_per_second
                .unwrap_or(0)
                .min(rate.saturating_sub(1));
            Mutex::new(Buckets {
                all: Bucket::new(rate, now),
                notices: (reserved > 0).then(|| Bucket::new(rate - reserved, now)),
            })
        });
        RateLimiter { buckets }
    }

    /// Waits until a message of `purpose` may be sent.
    pub async fn acquire(&self, purpose: MessagePurpose) {
        let Some(buckets) = &self.buckets else {
            return;
        };
        loop {
            let wait = {
                let Ok(mut buckets) = buckets.lock() else {
                    return;
                };
                let now = Instant::now();
                buckets.all.refill(now);
                let mut wait = buckets.all.wait();
                if purpose == MessagePurpose::NOTICE {
                    if let Some(notices) = buckets.notices.as_mut() {
                        notices.refill(now);
                        wait = wait.max(notices.wait());
                    }
                }
                if wait.is_zero() {
                    buckets.all.tokens -= 1.0;
                    if purpose == MessagePurpose::NOTICE {
                        if let Some(notices) = buckets.notices.as_mut() {
                            notices.tokens -= 1.0;
                        }
                    }
                }
                wait
            };
            if wait.is_zero() {
                return;
            }
            sleep(wait).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn sent_within(limiter: &RateLimiter, purpose: MessagePurpose, window: Duration) -> u32 {
        let start = Instant::now();
        let mut sent = 0;
        while Instant::now() - start < window {
            limiter.acquire(purpose).await;
            if Instant::now() - start < window {
                sent += 1;
            }
        }
        sent
    }

    #[tokio::test(start_paused = true)]
    async fn unlimited_accounts_never_wait() {
        let limiter = RateLimiter::new(&AccountLimits::default());
        let start = Instant::now();
        for _ in 0..1000 {
            limiter.acquire(MessagePurpose::NOTICE).await;
        }
        assert_eq!(Instant::now(), start);
    }

    #[tokio::test(start_paused = true)]
    async fn notices_leave_the_reserved_rate_to_codes() {
        let limiter = RateLimiter::new(&AccountLimits {
            messages_per_second: Some(10),
            otp_reserved_per_second: Some(4),
            allowed_calling_codes: vec![],
        });
        // Bulk notices saturate their share: about 6 per second.
        let notices = sent_within(&limiter, MessagePurpose::NOTICE, Duration::from_secs(10)).await;
        assert!((60..=66).contains(&notices), "{notices}");
        // Codes still get the reserved capacity right away.
        let start = Instant::now();
        for _ in 0..3 {
            limiter.acquire(MessagePurpose::OTP).await;
        }
        assert!(Instant::now() - start < Duration::from_millis(500));
    }

    #[tokio::test(start_paused = true)]
    async fn codes_alone_may_use_the_whole_rate() {
        let limiter = RateLimiter::new(&AccountLimits {
            messages_per_second: Some(10),
            otp_reserved_per_second: Some(4),
            allowed_calling_codes: vec![],
        });
        let codes = sent_within(&limiter, MessagePurpose::OTP, Duration::from_secs(10)).await;
        assert!((100..=110).contains(&codes), "{codes}");
    }
}
