// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The stage's verdict from its ledger. The last result of a check counts,
//! so a mistake is corrected by recording again and both results stay.
use super::{
    definition::{Check, Severity},
    ledger::{CheckResult, Entry, Ledger, Outcome},
};
use std::fmt::Write;
use strum_macros::Display;

#[derive(Clone, Copy, Debug, Display, PartialEq, Eq)]
#[strum(serialize_all = "kebab-case")]
pub enum Verdict {
    Passed,
    /// A critical check failed.
    Failed,
    /// No critical check failed, and at least one has no result.
    Incomplete,
}

pub struct Row<'a> {
    pub check: &'a Check,
    pub latest: Option<(&'a Entry, &'a CheckResult)>,
}

impl Row<'_> {
    fn outcome(&self) -> Option<Outcome> {
        self.latest.map(|(_, result)| result.outcome)
    }

    fn status(&self) -> String {
        self.outcome()
            .map_or("no result".into(), |outcome| outcome.to_string())
    }
}

pub struct Assessment<'a> {
    pub ledger: &'a Ledger,
    pub rows: Vec<Row<'a>>,
    pub verdict: Verdict,
}

pub fn assess(ledger: &Ledger) -> Assessment<'_> {
    let rows: Vec<Row> = ledger
        .opening
        .stage
        .checks
        .iter()
        .map(|check| Row {
            check,
            latest: ledger
                .results()
                .filter(|(_, result)| result.check_id == check.id)
                .last(),
        })
        .collect();
    let critical = || {
        rows.iter()
            .filter(|row| row.check.severity == Severity::Critical)
    };
    let verdict = if critical().any(|row| row.outcome() == Some(Outcome::Fail)) {
        Verdict::Failed
    } else if critical().any(|row| row.outcome().is_none()) {
        Verdict::Incomplete
    } else {
        Verdict::Passed
    };
    Assessment {
        ledger,
        rows,
        verdict,
    }
}

impl Assessment<'_> {
    /// One line per check for the terminal.
    pub fn text(&self) -> String {
        let mut text = String::new();
        for row in &self.rows {
            let _ = writeln!(
                text,
                "{:<10} {:<9} {}  {}",
                row.status(),
                row.check.severity.to_string(),
                row.check.id,
                row.check.title
            );
            if let Some((_, result)) = row.latest {
                for finding in result
                    .findings
                    .iter()
                    .filter(|finding| finding.outcome == Outcome::Fail)
                {
                    let _ = writeln!(
                        text,
                        "           {} {}: {}",
                        finding.subject, finding.name, finding.detail
                    );
                }
            }
        }
        let _ = writeln!(
            text,
            "Stage {}: {}",
            self.ledger.opening.stage.id, self.verdict
        );
        let _ = writeln!(text, "Ledger head: {}", self.ledger.head());
        text
    }

    /// A report to hand to witnesses; table cells never break the table.
    pub fn markdown(&self) -> String {
        let cell = |value: &str| value.replace('|', "\\|").replace('\n', " ");
        let opening = &self.ledger.opening;
        let mut text = String::new();
        let _ = writeln!(text, "# {}: {}\n", cell(&opening.stage.title), self.verdict);
        let _ = writeln!(text, "- Tenant: `{}`", opening.tenant_id);
        let _ = writeln!(text, "- Election event: `{}`", opening.election_event_id);
        let _ = writeln!(text, "- Started: {}", opening.started_at.to_rfc3339());
        if let Some(release) = &opening.release {
            let _ = writeln!(text, "- Release: {}", cell(release));
        }
        let _ = writeln!(
            text,
            "- Stage definition SHA-256: `{}`",
            opening.definition_sha256
        );
        let _ = writeln!(text, "- Ledger entries: {}", self.ledger.entries.len());
        let _ = writeln!(text, "- Ledger head: `{}`\n", self.ledger.head());
        let _ = writeln!(
            text,
            "| Check | Source | Severity | Method | Result | Recorded | By | Entry |"
        );
        let _ = writeln!(text, "| --- | --- | --- | --- | --- | --- | --- | --- |");
        for row in &self.rows {
            let (recorded, by, entry) =
                row.latest.map_or_else(Default::default, |(entry, result)| {
                    let by = match &result.witness {
                        Some(witness) => format!("{}, witness {witness}", entry.content.recorder),
                        None => entry.content.recorder.clone(),
                    };
                    (
                        entry.content.recorded_at.clone(),
                        by,
                        entry.content.sequence.to_string(),
                    )
                });
            let _ = writeln!(
                text,
                "| `{}` {} | {} | {} | {} | **{}** | {} | {} | {} |",
                row.check.id,
                cell(&row.check.title),
                cell(row.check.source.as_deref().unwrap_or("")),
                row.check.severity,
                row.check.method,
                row.status(),
                recorded,
                cell(&by),
                entry
            );
        }
        for row in &self.rows {
            let Some((_, result)) = row.latest else {
                continue;
            };
            if result.findings.is_empty() && result.note.is_none() {
                continue;
            }
            let _ = writeln!(text, "\n## `{}`: {}\n", row.check.id, result.outcome);
            if let Some(note) = &result.note {
                let _ = writeln!(text, "{note}\n");
            }
            for finding in &result.findings {
                let _ = writeln!(
                    text,
                    "- {}: {} `{}`: {}",
                    finding.outcome, finding.subject, finding.name, finding.detail
                );
            }
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acceptance::{
        definition::{Method, Stage},
        ledger::tests::{now, opening, witnessed},
        ledger::{Finding, Opening, Subject},
        Template,
    };
    use std::path::Path;

    const STAGE: &str = "id: s\ntitle: S\nchecks:\n  - id: a\n    title: A\n    method: witnessed\n  - id: b\n    title: B|b\n    method: witnessed\n  - id: c\n    title: C\n    severity: advisory\n    method: witnessed\n";

    fn ledger(path: &Path, results: &[(&str, Outcome)]) -> Ledger {
        let opening = Opening {
            stage: Stage::parse(STAGE).unwrap(),
            ..opening(Template::Voting)
        };
        let mut ledger = Ledger::open(path, opening, "operator", now()).unwrap();
        for (check, outcome) in results {
            ledger
                .append(path, witnessed(check, *outcome), "operator", now())
                .unwrap();
        }
        ledger
    }

    fn verdict(results: &[(&str, Outcome)]) -> Verdict {
        let directory = tempfile::tempdir().unwrap();
        let ledger = ledger(&directory.path().join("ledger.jsonl"), results);
        assess(&ledger).verdict
    }

    #[test]
    fn every_critical_check_must_pass() {
        use Outcome::{Fail, Pass};
        assert_eq!(verdict(&[]), Verdict::Incomplete);
        assert_eq!(verdict(&[("a", Pass)]), Verdict::Incomplete);
        assert_eq!(verdict(&[("a", Pass), ("b", Pass)]), Verdict::Passed);
        assert_eq!(verdict(&[("a", Pass), ("b", Fail)]), Verdict::Failed);
        assert_eq!(verdict(&[("a", Fail)]), Verdict::Failed);
    }

    #[test]
    fn an_advisory_check_never_changes_the_verdict() {
        use Outcome::{Fail, Pass};
        assert_eq!(
            verdict(&[("a", Pass), ("b", Pass), ("c", Fail)]),
            Verdict::Passed
        );
        assert_eq!(verdict(&[("c", Pass)]), Verdict::Incomplete);
    }

    #[test]
    fn the_last_result_of_a_check_counts() {
        use Outcome::{Fail, Pass};
        assert_eq!(
            verdict(&[("a", Fail), ("b", Pass), ("a", Pass)]),
            Verdict::Passed
        );
        assert_eq!(
            verdict(&[("a", Pass), ("b", Pass), ("a", Fail)]),
            Verdict::Failed
        );
    }

    #[test]
    fn reports_show_each_check_and_the_failed_findings() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("ledger.jsonl");
        let mut ledger = ledger(&path, &[("a", Outcome::Pass)]);
        ledger
            .append(
                &path,
                CheckResult {
                    check_id: "b".into(),
                    method: Method::Witnessed,
                    outcome: Outcome::Fail,
                    witness: Some("W".into()),
                    note: Some("Blank screen".into()),
                    findings: vec![
                        Finding::fail(Subject::Voter, "ana", "no ballot"),
                        Finding::pass(Subject::Voter, "ben", "ballot shown"),
                    ],
                },
                "operator",
                now(),
            )
            .unwrap();
        let assessment = assess(&ledger);
        let text = assessment.text();
        assert!(text.contains("fail       critical  b  B|b"));
        assert!(text.contains("voter ana: no ballot"));
        assert!(!text.contains("ben"));
        assert!(text.contains("no result  advisory  c  C"));
        assert!(text.contains("Stage s: failed"));
        assert!(text.contains(ledger.head()));

        let markdown = assessment.markdown();
        assert!(markdown.starts_with("# Voting: failed") || markdown.starts_with("# S: failed"));
        assert!(markdown.contains("| `b` B\\|b |  | critical | witnessed | **fail** | 2028-01-10T09:00:00Z | operator, witness W | 2 |"));
        assert!(markdown.contains("| `c` C |  | advisory | witnessed | **no result** |  |  |  |"));
        assert!(markdown.contains("- pass: voter `ben`: ballot shown"));
        assert!(markdown.contains("Blank screen"));
    }
}
