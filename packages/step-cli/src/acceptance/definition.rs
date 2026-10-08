// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The checks of one acceptance stage. A definition is data: an organization
//! describes its own stage in YAML and no check is tied to one client.
//! Unknown keys are rejected so a misspelled severity cannot relax a check.
use anyhow::{bail, ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs::OpenOptions, io::Write, path::Path};
use strum_macros::Display;

/// What a failed check means for the stage.
#[derive(Clone, Copy, Debug, Default, Display, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Severity {
    /// A failure, or a missing result, keeps the stage from passing.
    #[default]
    Critical,
    /// A failure is reported and does not change the stage's verdict.
    Advisory,
}

/// How a check gets its result.
#[derive(Clone, Copy, Debug, Display, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Method {
    /// `acceptance run` reads the election event's records.
    Automatic,
    /// A person observes it and `acceptance record` stores what they saw.
    Witnessed,
}

/// What an automatic check reads from the live election event.
#[derive(Clone, Copy, Debug, Display, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Probe {
    /// Each voter is enabled and has a successful sign-in in the event's log.
    VoterAuthenticated,
    /// A ballot style is published for each voter's area.
    BallotPublished,
    /// Each voter has a valid cast vote.
    BallotCast,
    /// The Ballot IDs voters were shown are those of their stored ballots.
    ReceiptProduced,
    /// Each stored ballot matches the hash recorded in the event's log.
    BallotStored,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Check {
    /// Stable name used on the command line and in the ledger.
    pub id: String,
    pub title: String,
    /// Clause or document the check comes from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default)]
    pub severity: Severity,
    pub method: Method,
    /// Required for automatic checks, not allowed for witnessed ones.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub probe: Option<Probe>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stage {
    pub id: String,
    pub title: String,
    pub checks: Vec<Check>,
}

/// Stage definitions shipped with the CLI as a starting point.
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum Template {
    /// The voting conditions, for any organization.
    Voting,
    /// The voting conditions plus one witnessed check per PQRI Annex A D.4 condition.
    PqriVoting,
}

impl Template {
    pub fn text(self) -> &'static str {
        match self {
            Self::Voting => include_str!("../../data/acceptance/voting.yaml"),
            Self::PqriVoting => include_str!("../../data/acceptance/pqri-voting.yaml"),
        }
    }

    /// Write the template for editing; an existing file is never overwritten.
    pub fn create(self, path: &Path) -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .with_context(|| format!("Cannot create {}", path.display()))?;
        file.write_all(self.text().as_bytes())?;
        Ok(())
    }
}

fn is_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

impl Stage {
    pub fn parse(text: &str) -> Result<Self> {
        let stage: Self = serde_yaml::from_str(text).context("Invalid stage definition")?;
        stage.validate()?;
        Ok(stage)
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(
            is_name(&self.id),
            "Stage id must use letters, digits, '.', '-' or '_'"
        );
        ensure!(!self.checks.is_empty(), "A stage needs at least one check");
        let mut ids = BTreeSet::new();
        for check in &self.checks {
            ensure!(
                is_name(&check.id),
                "Check id {:?} must use letters, digits, '.', '-' or '_'",
                check.id
            );
            ensure!(ids.insert(&check.id), "Check id {} is repeated", check.id);
            match (check.method, check.probe) {
                (Method::Automatic, None) => bail!("Automatic check {} needs a probe", check.id),
                (Method::Witnessed, Some(_)) => {
                    bail!("Witnessed check {} cannot have a probe", check.id)
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn check(&self, id: &str) -> Result<&Check> {
        self.checks
            .iter()
            .find(|check| check.id == id)
            .with_context(|| format!("Stage {} has no check {id}", self.id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = "id: s\ntitle: S\nchecks:\n";

    #[test]
    fn bundled_templates_are_valid() {
        let voting = Stage::parse(Template::Voting.text()).unwrap();
        let probes: BTreeSet<_> = voting.checks.iter().filter_map(|c| c.probe).collect();
        assert_eq!(probes.len(), 5);
        assert!(voting
            .checks
            .iter()
            .all(|check| check.severity == Severity::Critical));

        let pqri = Stage::parse(Template::PqriVoting.text()).unwrap();
        for check in &voting.checks {
            let same = pqri.check(&check.id).unwrap();
            assert_eq!((same.method, same.probe), (check.method, check.probe));
        }
        let annex = pqri
            .checks
            .iter()
            .filter(|check| check.id.starts_with("annex-a."))
            .count();
        assert_eq!(annex, 20);
    }

    #[test]
    fn severity_defaults_to_critical() {
        let stage = Stage::parse(&format!(
            "{MINIMAL}  - id: a\n    title: A\n    method: witnessed\n"
        ))
        .unwrap();
        assert_eq!(stage.checks[0].severity, Severity::Critical);
    }

    #[test]
    fn rejects_definitions_that_would_relax_or_confuse_checks() {
        for (checks, message) in [
            ("  []\n", "at least one check"),
            (
                "  - id: a\n    title: A\n    method: automatic\n",
                "needs a probe",
            ),
            (
                "  - id: a\n    title: A\n    method: witnessed\n    probe: ballot-cast\n",
                "cannot have a probe",
            ),
            (
                "  - id: a\n    title: A\n    method: witnessed\n  - id: a\n    title: B\n    method: witnessed\n",
                "repeated",
            ),
            (
                "  - id: a b\n    title: A\n    method: witnessed\n",
                "must use letters",
            ),
        ] {
            let error = Stage::parse(&format!("{MINIMAL}{checks}")).unwrap_err();
            assert!(format!("{error:#}").contains(message), "{error:#}");
        }
        for checks in [
            "  - id: a\n    title: A\n    method: witnessed\n    severty: advisory\n",
            "  - id: a\n    title: A\n    method: witnessed\n    severity: optional\n",
            "  - id: a\n    title: A\n    method: automatic\n    probe: unknown\n",
        ] {
            assert!(Stage::parse(&format!("{MINIMAL}{checks}")).is_err());
        }
    }

    #[test]
    fn a_template_never_overwrites_a_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("stage.yaml");
        Template::Voting.create(&path).unwrap();
        assert!(Template::PqriVoting.create(&path).is_err());
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            Template::Voting.text()
        );
    }
}
