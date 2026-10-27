// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Pure FAU_SAA.3 policy; telemetry absence never means no attacks.
pub fn valid_rates(failed_logins: i64, requests: i64) -> bool {
    (1..=100_000).contains(&failed_logins) && (1..=10_000_000).contains(&requests)
}
pub fn fresh_source(updated: i64, now: i64) -> bool {
    updated <= now && now.saturating_sub(updated) <= 30_000
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
}
