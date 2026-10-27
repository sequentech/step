// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Pure FAU_SAA.3 policy; telemetry absence never means no attacks.
pub fn valid_rates(failed_logins: i64, requests: i64) -> bool {
    (1..=100_000).contains(&failed_logins) && (1..=10_000_000).contains(&requests)
}
pub fn fresh_source(updated: i64, now: i64) -> bool {
    updated <= now && now.saturating_sub(updated) <= 30_000
}
pub fn valid_bucket_time(minute_start: i64, sampled_at: i64, now: i64) -> bool {
    // Retention belongs to the sampled snapshot. A still-fresh snapshot can
    // cross a minute boundary before consumption without losing continuity.
    fresh_source(sampled_at, now)
        && minute_start % 60_000 == 0
        && minute_start <= sampled_at
        && sampled_at.saturating_sub(minute_start) <= 16 * 60_000
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zero_negative_and_unbounded_rates_cannot_disable_detection() {
        assert!(!valid_rates(0, 20));
        assert!(!valid_rates(-1, 20));
        assert!(!valid_rates(20, 10_000_001));
        assert!(valid_rates(20, 2000));
    }
    #[test]
    fn unknown_and_stale_activity_never_mean_zero_requests() {
        assert!(!fresh_source(30_001, 30_000));
        assert!(!fresh_source(0, 30_001));
        assert!(fresh_source(15_000, 30_000));
    }
    #[test]
    fn a_fresh_snapshot_keeps_its_complete_prior_minute_at_rollover() {
        // The collector samples 16:47:59 and retains the complete 16:32 bucket.
        // A worker may validly consume that same snapshot at 16:48:02.
        let minute = 16 * 60 * 60_000 + 32 * 60_000;
        let sampled = minute + 15 * 60_000 + 59_000;
        assert!(valid_bucket_time(minute, sampled, sampled + 3_000));
        assert!(valid_bucket_time(minute, sampled, sampled + 30_000));
        assert!(!valid_bucket_time(minute, sampled, sampled + 30_001));
        assert!(!valid_bucket_time(minute, sampled, sampled - 1));
    }
    #[test]
    fn buckets_cannot_come_from_after_the_snapshot_or_outside_its_window() {
        assert!(!valid_bucket_time(120_000, 119_999, 120_001));
        assert!(!valid_bucket_time(1, 120_000, 120_001));
        assert!(!valid_bucket_time(0, 960_001, 960_002));
        assert!(!valid_bucket_time(i64::MIN, i64::MAX - 1, i64::MAX));
        assert!(valid_bucket_time(120_000, 120_005, 120_010));
    }
}
