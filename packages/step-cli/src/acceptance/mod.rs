// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Acceptance stages checked against a live election event, with an evidence
//! ledger and a pass/fail verdict.
//!
//! `init` writes a stage definition to edit. `open` starts a ledger for one run
//! of the stage. `run` decides the automatic checks from the event's records and
//! `record` stores what a witness observed. `verdict` reads the ledger back:
//! one failed critical check fails the stage.
mod definition;
mod hasura;
mod ledger;
mod verdict;
mod voting;

use crate::{adapters::graphql::HasuraGraphql, utils::read_config};
use anyhow::{anyhow, bail, ensure, Context, Result};
use chrono::{DateTime, Utc};
use clap::Subcommand;
pub use definition::Template;
use definition::{Method, Stage};
use ledger::{CheckResult, Finding, Ledger, Opening, Outcome, Subject};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};
use verdict::Verdict;
use voting::{Facts, VotingRecords};

/// Acceptance stage lifecycle. Every operation exits nonzero on failure.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Write a bundled stage definition to edit.
    Init {
        /// Stage definition to start from.
        #[arg(long, value_enum, default_value = "voting")]
        template: Template,
        /// YAML destination; existing files are never overwritten.
        #[arg(long, default_value = "acceptance-stage.yaml")]
        output: PathBuf,
    },
    /// Start the evidence ledger of one run of a stage.
    Open {
        /// Stage definition written by init.
        definition: PathBuf,
        /// Ledger destination; existing files are never overwritten.
        #[arg(long)]
        ledger: PathBuf,
        /// Election event the stage is checked against.
        #[arg(long)]
        election_event_id: String,
        /// Start of the run (RFC 3339); older records are not evidence. Defaults to now.
        #[arg(long)]
        started_at: Option<DateTime<Utc>>,
        /// Release or build identifier of the system under test.
        #[arg(long)]
        release: Option<String>,
        /// Who records; defaults to the configured administrator.
        #[arg(long)]
        recorder: Option<String>,
    },
    /// Decide the automatic checks from the election event's records.
    Run {
        /// Ledger written by open.
        ledger: PathBuf,
        /// Username of a voter who took part; repeat for each voter.
        #[arg(long = "voter")]
        voters: Vec<String>,
        /// File with one voter username per line.
        #[arg(long)]
        voters_file: Option<PathBuf>,
        /// Ballot ID read from a voter's confirmation screen or receipt; repeatable.
        #[arg(long = "ballot-id")]
        ballot_ids: Vec<String>,
        /// File with one presented Ballot ID per line.
        #[arg(long)]
        ballot_ids_file: Option<PathBuf>,
        /// Run only this check; repeatable. Defaults to every automatic check.
        #[arg(long = "check")]
        checks: Vec<String>,
    },
    /// Record what a witness observed for a witnessed check.
    Record {
        /// Ledger written by open.
        ledger: PathBuf,
        /// Check id from the stage definition.
        #[arg(long)]
        check: String,
        #[arg(long, value_enum)]
        outcome: Outcome,
        /// Name of the person who observed it.
        #[arg(long)]
        witness: String,
        /// What was observed.
        #[arg(long)]
        note: Option<String>,
        /// File kept as evidence; its SHA-256 is recorded. Repeatable.
        #[arg(long = "evidence")]
        evidence: Vec<PathBuf>,
        /// Who records; defaults to the configured administrator.
        #[arg(long)]
        recorder: Option<String>,
    },
    /// Verify the ledger and print the stage's verdict; nonzero unless it passed.
    Verdict {
        /// Ledger written by open.
        ledger: PathBuf,
        /// Also write a Markdown report.
        #[arg(long)]
        report: Option<PathBuf>,
    },
}

fn configured() -> Result<crate::types::config::ConfigData> {
    read_config::read_config().map_err(|error| anyhow!(error.to_string()))
}

fn recorder(given: &Option<String>) -> Result<String> {
    match given {
        Some(name) => Ok(name.clone()),
        None => configured()
            .map(|config| config.username)
            .ok()
            .filter(|name| !name.trim().is_empty())
            .context("Pass --recorder, or configure the CLI first"),
    }
}

/// Non-empty trimmed lines of the arguments and the optional file, without repeats.
fn names(given: &[String], file: &Option<PathBuf>) -> Result<Vec<String>> {
    let mut all: Vec<String> = given.to_vec();
    if let Some(path) = file {
        let text =
            fs::read_to_string(path).with_context(|| format!("Cannot read {}", path.display()))?;
        all.extend(text.lines().map(String::from));
    }
    let mut seen = BTreeSet::new();
    Ok(all
        .iter()
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty() && seen.insert(name.clone()))
        .collect())
}

/// The system and moment a run is about.
struct Target {
    tenant_id: String,
    election_event_id: String,
    started_at: DateTime<Utc>,
    release: Option<String>,
}

fn open(
    definition: &Path,
    path: &Path,
    target: Target,
    recorder: &str,
    now: DateTime<Utc>,
) -> Result<Ledger> {
    let text = fs::read_to_string(definition)
        .with_context(|| format!("Cannot read {}", definition.display()))?;
    ensure!(target.started_at <= now, "--started-at is in the future");
    Ledger::open(
        path,
        Opening {
            stage: Stage::parse(&text)?,
            definition_sha256: ledger::file_sha256(definition)?,
            tenant_id: target.tenant_id,
            election_event_id: target.election_event_id,
            started_at: target.started_at,
            release: target.release,
        },
        recorder,
        now,
    )
}

/// What `run` did with one automatic check.
#[derive(Debug, PartialEq, Eq)]
enum Ran {
    Recorded(Outcome),
    NotRun(String),
}

/// Decide the selected automatic checks and append their results.
fn run(
    path: &Path,
    records: &impl VotingRecords,
    voters: &[String],
    ballot_ids: Vec<String>,
    only: &[String],
    recorder: &str,
    now: DateTime<Utc>,
) -> Result<Vec<(String, Ran)>> {
    let mut ledger = Ledger::read(path)?;
    let stage = ledger.opening.stage.clone();
    for id in only {
        ensure!(
            stage.check(id)?.method == Method::Automatic,
            "Check {id} is witnessed; use acceptance record"
        );
    }
    ensure!(
        !voters.is_empty(),
        "Name the voters with --voter or --voters-file"
    );
    let facts = Facts::gather(
        records,
        &ledger.opening.election_event_id,
        voters,
        ballot_ids,
        ledger.opening.started_at,
    )?;
    let mut ran = Vec::new();
    for check in &stage.checks {
        let Some(probe) = check.probe else {
            continue;
        };
        if !only.is_empty() && !only.contains(&check.id) {
            continue;
        }
        let outcome = match voting::probe(probe, &facts) {
            Ok(findings) => {
                let failed = findings.iter().any(|f| f.outcome == Outcome::Fail);
                let outcome = if failed { Outcome::Fail } else { Outcome::Pass };
                ledger.append(
                    path,
                    CheckResult {
                        check_id: check.id.clone(),
                        method: Method::Automatic,
                        outcome,
                        witness: None,
                        note: None,
                        findings,
                    },
                    recorder,
                    now,
                )?;
                Ran::Recorded(outcome)
            }
            Err(reason) => Ran::NotRun(reason),
        };
        ran.push((check.id.clone(), outcome));
    }
    Ok(ran)
}

/// What a witness observed for one check.
struct Observation<'a> {
    check: &'a str,
    outcome: Outcome,
    witness: &'a str,
    note: &'a Option<String>,
    evidence: &'a [PathBuf],
}

fn record(
    path: &Path,
    observation: Observation,
    recorder: &str,
    now: DateTime<Utc>,
) -> Result<Ledger> {
    let Observation {
        check,
        outcome,
        witness,
        note,
        evidence,
    } = observation;
    let mut ledger = Ledger::read(path)?;
    ensure!(!witness.trim().is_empty(), "The witness needs a name");
    let mut findings = Vec::new();
    for file in evidence {
        let name = file
            .file_name()
            .and_then(|name| name.to_str())
            .with_context(|| format!("Evidence {} has no file name", file.display()))?;
        findings.push(Finding {
            subject: Subject::File,
            name: name.into(),
            outcome,
            detail: format!("sha256 {}", ledger::file_sha256(file)?),
        });
    }
    ledger.append(
        path,
        CheckResult {
            check_id: check.into(),
            method: Method::Witnessed,
            outcome,
            witness: Some(witness.trim().into()),
            note: note.clone(),
            findings,
        },
        recorder,
        now,
    )?;
    Ok(ledger)
}

/// Print the verdict, optionally write the report, and fail unless the stage passed.
fn verdict(path: &Path, report: &Option<PathBuf>) -> Result<()> {
    let ledger = Ledger::read(path)?;
    let assessment = verdict::assess(&ledger);
    print!("{}", assessment.text());
    if let Some(report) = report {
        fs::write(report, assessment.markdown())
            .with_context(|| format!("Cannot write {}", report.display()))?;
    }
    match assessment.verdict {
        Verdict::Passed => Ok(()),
        other => bail!("Stage {} is {other}", ledger.opening.stage.id),
    }
}

impl Command {
    pub fn run(&self) -> Result<()> {
        let now = Utc::now();
        match self {
            Self::Init { template, output } => {
                template.create(output)?;
                println!(
                    "Created {}. Review its checks, then run step-cli acceptance open {} --ledger <ledger> --election-event-id <id>.",
                    output.display(),
                    output.display()
                );
                Ok(())
            }
            Self::Open {
                definition,
                ledger,
                election_event_id,
                started_at,
                release,
                recorder: given,
            } => {
                let opened = open(
                    definition,
                    ledger,
                    Target {
                        tenant_id: configured()?.tenant_id,
                        election_event_id: election_event_id.into(),
                        started_at: started_at.unwrap_or(now),
                        release: release.clone(),
                    },
                    &recorder(given)?,
                    now,
                )?;
                println!(
                    "Opened {} for stage {} with {} checks.",
                    ledger.display(),
                    opened.opening.stage.id,
                    opened.opening.stage.checks.len()
                );
                println!("Ledger head: {}", opened.head());
                Ok(())
            }
            Self::Run {
                ledger,
                voters,
                voters_file,
                ballot_ids,
                ballot_ids_file,
                checks,
            } => {
                let config = read_config::refresh_and_save_token()
                    .map_err(|error| anyhow!("Cannot refresh the session: {error}"))?;
                let records = hasura::HasuraRecords {
                    graphql: HasuraGraphql,
                    tenant_id: config.tenant_id,
                };
                let ran = run(
                    ledger,
                    &records,
                    &names(voters, voters_file)?,
                    names(ballot_ids, ballot_ids_file)?,
                    checks,
                    &config.username,
                    now,
                )?;
                for (check, outcome) in &ran {
                    match outcome {
                        Ran::Recorded(outcome) => println!("{outcome:<8} {check}"),
                        Ran::NotRun(reason) => println!("not run  {check}: {reason}"),
                    }
                }
                println!("Ledger head: {}", Ledger::read(ledger)?.head());
                ensure!(
                    !ran.iter()
                        .any(|(_, outcome)| *outcome == Ran::Recorded(Outcome::Fail)),
                    "A check failed; see step-cli acceptance verdict {}",
                    ledger.display()
                );
                Ok(())
            }
            Self::Record {
                ledger,
                check,
                outcome,
                witness,
                note,
                evidence,
                recorder: given,
            } => {
                let recorded = record(
                    ledger,
                    Observation {
                        check,
                        outcome: *outcome,
                        witness,
                        note,
                        evidence,
                    },
                    &recorder(given)?,
                    now,
                )?;
                println!("Recorded {outcome} for {check}.");
                println!("Ledger head: {}", recorded.head());
                Ok(())
            }
            Self::Verdict { ledger, report } => verdict(ledger, report),
        }
    }
}

#[cfg(test)]
mod tests;
