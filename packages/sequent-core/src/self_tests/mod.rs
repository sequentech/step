// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! The six required TSF checks, their bounded schedule and persisted results.
use serde::{Deserialize, Serialize};
pub mod policy;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SelfTestTrigger {
    Startup,
    ElectionStartTime,
    BeforeStart,
    Requested,
    BeforeResume,
    Periodic,
    AfterMaintenance,
    AfterImport,
    AfterPause,
    BeforeCounting,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SelfTestOutcome {
    Passed,
    RecoverableFailure,
    UnrecoverableFailure,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SelfTestCheck {
    BallotBox,
    ElectionData,
    AuditGeneration,
    TrustedChannel,
    AccessControl,
    Clock,
}
impl SelfTestCheck {
    pub const ALL: [Self; 6] = [
        Self::BallotBox,
        Self::ElectionData,
        Self::AuditGeneration,
        Self::TrustedChannel,
        Self::AccessControl,
        Self::Clock,
    ];
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelfTestCheckResult {
    pub check: SelfTestCheck,
    pub outcome: SelfTestOutcome,
    pub detail: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelfTestRun {
    pub run_id: String,
    pub generation: i64,
    pub trigger: SelfTestTrigger,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub outcome: SelfTestOutcome,
    pub checks: Vec<SelfTestCheckResult>,
}
impl SelfTestRun {
    pub fn aggregate(&self) -> SelfTestOutcome {
        if self
            .checks
            .iter()
            .any(|c| c.outcome == SelfTestOutcome::UnrecoverableFailure)
        {
            return SelfTestOutcome::UnrecoverableFailure;
        }
        if self.checks.len() != 6
            || SelfTestCheck::ALL.iter().any(|check| {
                self.checks
                    .iter()
                    .filter(|r| {
                        &r.check == check
                            && r.outcome == SelfTestOutcome::Passed
                    })
                    .count()
                    != 1
            })
        {
            return SelfTestOutcome::RecoverableFailure;
        }
        SelfTestOutcome::Passed
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SelfTestSettings {
    pub interval_minutes: u16,
    pub timeout_seconds: u16,
    pub additional_conditions: Vec<SelfTestTrigger>,
}
impl Default for SelfTestSettings {
    fn default() -> Self {
        Self {
            interval_minutes: 15,
            timeout_seconds: 60,
            additional_conditions: vec![],
        }
    }
}
impl SelfTestSettings {
    pub fn validate(&self) -> Result<(), &'static str> {
        policy::validate_schedule(self.interval_minutes, self.timeout_seconds)?;
        for (i, trigger) in self.additional_conditions.iter().enumerate() {
            if !matches!(
                trigger,
                SelfTestTrigger::AfterImport
                    | SelfTestTrigger::AfterPause
                    | SelfTestTrigger::BeforeCounting
            ) || self.additional_conditions[..i].contains(trigger)
            {
                return Err("Additional conditions must be distinct after-import, after-pause or before-counting");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn run() -> SelfTestRun {
        SelfTestRun {
            run_id: "run".into(),
            generation: 1,
            trigger: SelfTestTrigger::Requested,
            started_at: String::new(),
            finished_at: Some(String::new()),
            outcome: SelfTestOutcome::RecoverableFailure,
            checks: SelfTestCheck::ALL
                .into_iter()
                .map(|check| SelfTestCheckResult {
                    check,
                    outcome: SelfTestOutcome::Passed,
                    detail: String::new(),
                })
                .collect(),
        }
    }
    #[test]
    fn missing_duplicate_and_failed_checks_never_pass() {
        assert_eq!(run().aggregate(), SelfTestOutcome::Passed);
        let mut r = run();
        r.checks.pop();
        assert_eq!(r.aggregate(), SelfTestOutcome::RecoverableFailure);
        r.checks.push(r.checks[0].clone());
        assert_eq!(r.aggregate(), SelfTestOutcome::RecoverableFailure);
        let mut r = run();
        r.checks[0].outcome = SelfTestOutcome::UnrecoverableFailure;
        assert_eq!(r.aggregate(), SelfTestOutcome::UnrecoverableFailure);
    }
    #[test]
    fn mandatory_triggers_cannot_be_disabled_in_additional_conditions() {
        let s = SelfTestSettings {
            additional_conditions: vec![SelfTestTrigger::Periodic],
            ..Default::default()
        };
        assert!(s.validate().is_err());
    }
}
