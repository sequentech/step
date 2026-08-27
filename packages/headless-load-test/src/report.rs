// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Aggregates `VoteOutcome`s from one election event's run into counts and
//! latency percentiles, and renders every election event's report to the
//! summary printed at the end of a run.

use std::fmt;
use std::time::Duration;

use crate::vote::{CastOutcome, VoteOutcome};

#[derive(Debug, Default, Clone)]
pub struct ElectionEventReport {
    pub tenant_slug: String,
    pub election_event_id: String,
    pub succeeded: u64,
    pub login_failed: u64,
    pub ballot_style_unavailable: u64,
    pub ballot_preparation_failed: u64,
    pub voter_state_locked: u64,
    pub revote_limit_exceeded: u64,
    pub rejected: u64,
    pub transport_failed: u64,
    latencies_millis: Vec<u64>,
}

impl ElectionEventReport {
    pub fn new(tenant_slug: impl Into<String>, election_event_id: impl Into<String>) -> Self {
        Self {
            tenant_slug: tenant_slug.into(),
            election_event_id: election_event_id.into(),
            ..Default::default()
        }
    }

    /// Folds one vote attempt's outcome and how long it took into the
    /// running counts.
    pub fn record(&mut self, outcome: &VoteOutcome, elapsed: Duration) {
        self.latencies_millis.push(elapsed.as_millis() as u64);
        match outcome {
            VoteOutcome::Cast(CastOutcome::Success { .. }) => self.succeeded += 1,
            VoteOutcome::Cast(CastOutcome::VoterStateLocked) => self.voter_state_locked += 1,
            VoteOutcome::Cast(CastOutcome::RevoteLimitExceeded) => self.revote_limit_exceeded += 1,
            VoteOutcome::Cast(CastOutcome::Rejected { .. }) => self.rejected += 1,
            VoteOutcome::Cast(CastOutcome::Transport(_)) => self.transport_failed += 1,
            VoteOutcome::LoginFailed(_) => self.login_failed += 1,
            VoteOutcome::BallotStyleUnavailable(_) => self.ballot_style_unavailable += 1,
            VoteOutcome::BallotPreparationFailed(_) => self.ballot_preparation_failed += 1,
        }
    }

    pub fn attempted(&self) -> u64 {
        self.latencies_millis.len() as u64
    }

    pub fn failed(&self) -> u64 {
        self.attempted() - self.succeeded
    }

    /// `(p50, p95, p99)` latency in milliseconds. `None` if nothing was
    /// recorded.
    pub fn latency_percentiles(&self) -> Option<(u64, u64, u64)> {
        if self.latencies_millis.is_empty() {
            return None;
        }
        let mut sorted = self.latencies_millis.clone();
        sorted.sort_unstable();
        let percentile = |p: f64| -> u64 {
            let rank = ((p * sorted.len() as f64).ceil() as usize)
                .saturating_sub(1)
                .min(sorted.len() - 1);
            sorted[rank]
        };
        Some((percentile(0.50), percentile(0.95), percentile(0.99)))
    }
}

impl fmt::Display for ElectionEventReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Tenant {} / event {}:",
            self.tenant_slug, self.election_event_id
        )?;
        writeln!(
            f,
            "  attempted: {}   succeeded: {}   failed: {}",
            self.attempted(),
            self.succeeded,
            self.failed()
        )?;
        if self.failed() > 0 {
            if self.login_failed > 0 {
                writeln!(f, "    login failures: {}", self.login_failed)?;
            }
            if self.ballot_style_unavailable > 0 {
                writeln!(
                    f,
                    "    ballot style unavailable: {}",
                    self.ballot_style_unavailable
                )?;
            }
            if self.ballot_preparation_failed > 0 {
                writeln!(
                    f,
                    "    ballot preparation failed: {}",
                    self.ballot_preparation_failed
                )?;
            }
            if self.voter_state_locked > 0 {
                writeln!(
                    f,
                    "    cast conflicts (409, concurrent same-voter write): {}",
                    self.voter_state_locked
                )?;
            }
            if self.revote_limit_exceeded > 0 {
                writeln!(
                    f,
                    "    revote limit exceeded: {}",
                    self.revote_limit_exceeded
                )?;
            }
            if self.rejected > 0 {
                writeln!(f, "    rejected by the server: {}", self.rejected)?;
            }
            if self.transport_failed > 0 {
                writeln!(f, "    transport failures: {}", self.transport_failed)?;
            }
        }
        if let Some((p50, p95, p99)) = self.latency_percentiles() {
            writeln!(f, "  p50: {p50}ms   p95: {p95}ms   p99: {p99}ms")?;
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct RunReport {
    pub election_events: Vec<ElectionEventReport>,
    /// One entry per election event that never made it to voting — tenant
    /// creation, import, publish, opening voting, or voter provisioning
    /// failed. These have no `ElectionEventReport`, since voting never
    /// started.
    pub provisioning_failures: Vec<String>,
}

impl RunReport {
    /// Non-zero if any election event had a failed cast, or never made it
    /// to voting at all.
    pub fn exit_code(&self) -> i32 {
        let any_cast_failures = self
            .election_events
            .iter()
            .any(|report| report.failed() > 0);
        if any_cast_failures || !self.provisioning_failures.is_empty() {
            1
        } else {
            0
        }
    }
}

impl fmt::Display for RunReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for report in &self.election_events {
            writeln!(f, "{report}")?;
        }
        for failure in &self.provisioning_failures {
            writeln!(f, "Provisioning failed: {failure}")?;
        }
        let with_failures = self
            .election_events
            .iter()
            .filter(|report| report.failed() > 0)
            .count();
        writeln!(
            f,
            "{} election event(s) voted, {} with cast failures, {} never provisioned. Exit code: {}",
            self.election_events.len(),
            with_failures,
            self.provisioning_failures.len(),
            self.exit_code()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn success() -> VoteOutcome {
        VoteOutcome::Cast(CastOutcome::Success {
            id: "vote-id".to_string(),
        })
    }

    #[test]
    fn attempted_and_failed_are_derived_from_recorded_outcomes() {
        let mut report = ElectionEventReport::new("acme", "event-1");
        report.record(&success(), Duration::from_millis(10));
        report.record(&success(), Duration::from_millis(20));
        report.record(
            &VoteOutcome::Cast(CastOutcome::VoterStateLocked),
            Duration::from_millis(5),
        );

        assert_eq!(report.attempted(), 3);
        assert_eq!(report.succeeded, 2);
        assert_eq!(report.failed(), 1);
        assert_eq!(report.voter_state_locked, 1);
    }

    #[test]
    fn every_outcome_variant_is_folded_into_a_distinct_counter() {
        let mut report = ElectionEventReport::new("acme", "event-1");
        let outcomes = [
            success(),
            VoteOutcome::Cast(CastOutcome::VoterStateLocked),
            VoteOutcome::Cast(CastOutcome::RevoteLimitExceeded),
            VoteOutcome::Cast(CastOutcome::Rejected {
                code: "AreaNotFound".to_string(),
                message: "Area not found".to_string(),
            }),
            VoteOutcome::Cast(CastOutcome::Transport("connection reset".to_string())),
            VoteOutcome::LoginFailed("invalid username or password".to_string()),
            VoteOutcome::BallotStyleUnavailable("no ballot style visible".to_string()),
            VoteOutcome::BallotPreparationFailed("encrypt failed".to_string()),
        ];
        for outcome in &outcomes {
            report.record(outcome, Duration::from_millis(1));
        }

        assert_eq!(report.attempted(), outcomes.len() as u64);
        assert_eq!(report.succeeded, 1);
        assert_eq!(report.voter_state_locked, 1);
        assert_eq!(report.revote_limit_exceeded, 1);
        assert_eq!(report.rejected, 1);
        assert_eq!(report.transport_failed, 1);
        assert_eq!(report.login_failed, 1);
        assert_eq!(report.ballot_style_unavailable, 1);
        assert_eq!(report.ballot_preparation_failed, 1);
        // Every non-success outcome above counts as failed, by construction.
        assert_eq!(report.failed(), outcomes.len() as u64 - 1);
    }

    #[test]
    fn latency_percentiles_are_none_with_nothing_recorded() {
        let report = ElectionEventReport::new("acme", "event-1");
        assert!(report.latency_percentiles().is_none());
    }

    #[test]
    fn latency_percentiles_match_a_known_distribution() {
        let mut report = ElectionEventReport::new("acme", "event-1");
        // 1..=100 ms, uniformly: p50=50, p95=95, p99=99.
        for millis in 1..=100u64 {
            report.record(&success(), Duration::from_millis(millis));
        }
        let (p50, p95, p99) = report.latency_percentiles().unwrap();
        assert_eq!((p50, p95, p99), (50, 95, 99));
    }

    #[test]
    fn run_report_exit_code_is_nonzero_only_with_a_failure() {
        let mut clean = RunReport::default();
        let mut clean_event = ElectionEventReport::new("acme", "event-1");
        clean_event.record(&success(), Duration::from_millis(1));
        clean.election_events.push(clean_event);
        assert_eq!(clean.exit_code(), 0);

        let mut dirty = RunReport::default();
        let mut dirty_event = ElectionEventReport::new("acme", "event-2");
        dirty_event.record(
            &VoteOutcome::Cast(CastOutcome::VoterStateLocked),
            Duration::from_millis(1),
        );
        dirty.election_events.push(dirty_event);
        assert_eq!(dirty.exit_code(), 1);
    }

    #[test]
    fn a_provisioning_failure_alone_still_fails_the_run() {
        // No election event ever ran, so there's nothing in
        // `election_events` — the failure only shows up as an entry in
        // `provisioning_failures`, and that alone must still be a nonzero
        // exit code.
        let mut report = RunReport::default();
        report
            .provisioning_failures
            .push("tenant `acme` creation failed: slug already in use".to_string());
        assert_eq!(report.exit_code(), 1);
    }
}
