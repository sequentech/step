// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Resource limits for the periodic electoral-log audit, not the complete
//! FPT_TST.1 suite. Invalid settings prevent the scheduler from starting.

pub const AUDIT_INTERVAL_ENV: &str = "ELECTORAL_LOG_AUDIT_INTERVAL_SECS";
pub const AUDIT_TIMEOUT_ENV: &str = "ELECTORAL_LOG_AUDIT_TIMEOUT_SECS";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuditPolicy {
    pub interval_secs: u64,
    pub timeout_secs: u64,
}

impl AuditPolicy {
    pub fn parse(interval: Option<&str>, timeout: Option<&str>) -> Result<Self, String> {
        fn seconds(
            value: Option<&str>,
            default: u64,
            name: &str,
            min: u64,
            max: u64,
        ) -> Result<u64, String> {
            match value {
                None => Ok(default),
                Some(value) => value
                    .trim()
                    .parse::<u64>()
                    .ok()
                    .filter(|seconds| (min..=max).contains(seconds))
                    .ok_or_else(|| {
                        format!("{name} must be an integer from {min} to {max} seconds")
                    }),
            }
        }
        let interval_secs = seconds(interval, 900, AUDIT_INTERVAL_ENV, 60, 3600)?;
        let timeout_secs = seconds(timeout, 60, AUDIT_TIMEOUT_ENV, 10, interval_secs)?;
        Ok(Self {
            interval_secs,
            timeout_secs,
        })
    }

    pub fn from_env() -> Result<Self, String> {
        Self::parse(
            std::env::var(AUDIT_INTERVAL_ENV).ok().as_deref(),
            std::env::var(AUDIT_TIMEOUT_ENV).ok().as_deref(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_schedule_is_fifteen_minutes_with_a_bounded_run() {
        assert_eq!(
            AuditPolicy::parse(None, None),
            Ok(AuditPolicy {
                interval_secs: 900,
                timeout_secs: 60,
            })
        );
    }

    #[test]
    fn accepts_configured_one_minute_and_one_hour_schedules() {
        assert_eq!(
            AuditPolicy::parse(Some("60"), Some("30")),
            Ok(AuditPolicy {
                interval_secs: 60,
                timeout_secs: 30,
            })
        );
        assert_eq!(
            AuditPolicy::parse(Some("3600"), Some("600")),
            Ok(AuditPolicy {
                interval_secs: 3600,
                timeout_secs: 600,
            })
        );
    }

    #[test]
    fn rejects_disabled_malformed_and_excessively_sparse_schedules() {
        for interval in [
            "",
            "0",
            "59",
            "3601",
            "-1",
            "1.5",
            "never",
            "18446744073709551616",
        ] {
            let error = AuditPolicy::parse(Some(interval), Some("30")).unwrap_err();
            assert!(error.contains(AUDIT_INTERVAL_ENV), "{error}");
        }
    }

    #[test]
    fn a_shorter_interval_bounds_the_default_timeout() {
        assert_eq!(
            AuditPolicy::parse(Some("60"), None),
            Ok(AuditPolicy {
                interval_secs: 60,
                timeout_secs: 60,
            })
        );
    }

    #[test]
    fn rejects_unbounded_and_overlapping_timeout_configuration() {
        for timeout in ["", "0", "9", "61", "-1", "slow"] {
            let error = AuditPolicy::parse(Some("60"), Some(timeout)).unwrap_err();
            assert!(error.contains(AUDIT_TIMEOUT_ENV), "{error}");
        }
    }
}
