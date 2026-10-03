// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Append-only evidence of one stage run, one JSON entry per line.
//!
//! Every entry carries the hash of the previous one, so an edited, removed or
//! reordered entry is detected when the ledger is read. Removing entries from
//! the end is not: the head hash printed by every command is what witnesses
//! note down.
use super::definition::{Method, Stage};
use anyhow::{bail, ensure, Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::Path,
};
use strum_macros::Display;

const GENESIS: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Clone, Copy, Debug, Display, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Outcome {
    Pass,
    Fail,
}

/// What a finding is about. A finding names a voter or a ballot, never both.
#[derive(Clone, Copy, Debug, Display, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Subject {
    Stage,
    Voter,
    Area,
    Ballot,
    File,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub subject: Subject,
    pub name: String,
    pub outcome: Outcome,
    pub detail: String,
}

impl Finding {
    pub fn pass(subject: Subject, name: &str, detail: impl Into<String>) -> Self {
        Self {
            subject,
            name: name.into(),
            outcome: Outcome::Pass,
            detail: detail.into(),
        }
    }

    pub fn fail(subject: Subject, name: &str, detail: impl Into<String>) -> Self {
        Self {
            outcome: Outcome::Fail,
            ..Self::pass(subject, name, detail)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Opening {
    pub stage: Stage,
    /// SHA-256 of the definition file the stage was read from.
    pub definition_sha256: String,
    pub tenant_id: String,
    pub election_event_id: String,
    /// Records older than this are not evidence of this run.
    pub started_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckResult {
    pub check_id: String,
    pub method: Method,
    pub outcome: Outcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub witness: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub findings: Vec<Finding>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Body {
    Opened(Opening),
    Checked(CheckResult),
}

/// The hashed part of an entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Content {
    pub sequence: u64,
    pub recorded_at: String,
    pub recorder: String,
    pub previous: String,
    pub body: Body,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    #[serde(flatten)]
    pub content: Content,
    pub hash: String,
}

impl Content {
    fn seal(self) -> Result<Entry> {
        let hash = hex::encode(Sha256::digest(serde_json::to_vec(&self)?));
        Ok(Entry {
            content: self,
            hash,
        })
    }
}

#[derive(Debug)]
pub struct Ledger {
    pub opening: Opening,
    pub entries: Vec<Entry>,
}

fn timestamp(now: DateTime<Utc>) -> String {
    now.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn write_line(file: &mut fs::File, entry: &Entry) -> Result<()> {
    let mut line = serde_json::to_vec(entry)?;
    line.push(b'\n');
    file.write_all(&line)?;
    file.sync_all()?;
    Ok(())
}

impl Ledger {
    /// Start a ledger; an existing file is never reused for another run.
    pub fn open(path: &Path, opening: Opening, recorder: &str, now: DateTime<Utc>) -> Result<Self> {
        opening.stage.validate()?;
        ensure!(!recorder.trim().is_empty(), "The recorder needs a name");
        let entry = Content {
            sequence: 0,
            recorded_at: timestamp(now),
            recorder: recorder.into(),
            previous: GENESIS.into(),
            body: Body::Opened(opening.clone()),
        }
        .seal()?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
            .with_context(|| format!("Cannot create ledger {}", path.display()))?;
        write_line(&mut file, &entry)?;
        Ok(Self {
            opening,
            entries: vec![entry],
        })
    }

    /// Read a ledger, refusing one whose entries were altered.
    pub fn read(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)
            .with_context(|| format!("Cannot read ledger {}", path.display()))?;
        let mut entries: Vec<Entry> = Vec::new();
        let mut previous = GENESIS.to_string();
        for (index, line) in text.lines().enumerate() {
            let entry: Entry = serde_json::from_str(line)
                .with_context(|| format!("Ledger entry {index} cannot be read"))?;
            ensure!(
                entry.content.sequence == index as u64,
                "Ledger entry {index} is out of order: an entry was removed or moved"
            );
            ensure!(
                entry.content.previous == previous,
                "Ledger entry {index} does not follow the entry before it"
            );
            ensure!(
                entry.content.clone().seal()?.hash == entry.hash,
                "Ledger entry {index} was altered"
            );
            previous = entry.hash.clone();
            entries.push(entry);
        }
        let opening = match entries.first().map(|entry| &entry.content.body) {
            Some(Body::Opened(opening)) => opening.clone(),
            _ => bail!("Ledger {} does not start with an opening", path.display()),
        };
        opening.stage.validate()?;
        for entry in &entries[1..] {
            match &entry.content.body {
                Body::Opened(_) => bail!(
                    "Ledger entry {} opens a second stage",
                    entry.content.sequence
                ),
                Body::Checked(result) => {
                    opening.stage.check(&result.check_id)?;
                }
            }
        }
        Ok(Self { opening, entries })
    }

    /// Hash of the last entry: it changes with anything recorded before it.
    pub fn head(&self) -> &str {
        self.entries.last().map_or(GENESIS, |entry| &entry.hash)
    }

    pub fn results(&self) -> impl Iterator<Item = (&Entry, &CheckResult)> {
        self.entries
            .iter()
            .filter_map(|entry| match &entry.content.body {
                Body::Checked(result) => Some((entry, result)),
                Body::Opened(_) => None,
            })
    }

    /// Record one result of a check of this stage, with the method the stage gives it.
    pub fn append(
        &mut self,
        path: &Path,
        result: CheckResult,
        recorder: &str,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let check = self.opening.stage.check(&result.check_id)?;
        ensure!(
            check.method == result.method,
            "Check {} is {}: it cannot be recorded as {}",
            check.id,
            check.method,
            result.method
        );
        ensure!(!recorder.trim().is_empty(), "The recorder needs a name");
        let entry = Content {
            sequence: self.entries.len() as u64,
            recorded_at: timestamp(now),
            recorder: recorder.into(),
            previous: self.head().into(),
            body: Body::Checked(result),
        }
        .seal()?;
        let mut file = OpenOptions::new().append(true).open(path)?;
        write_line(&mut file, &entry)?;
        self.entries.push(entry);
        Ok(())
    }
}

/// SHA-256 of a file's bytes, for definitions and evidence files.
pub fn file_sha256(path: &Path) -> Result<String> {
    crate::load::files::digest(path).with_context(|| format!("Cannot read {}", path.display()))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::acceptance::definition::Template;
    use chrono::TimeZone;

    pub fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2028, 1, 10, 9, 0, 0).unwrap()
    }

    pub fn opening(template: Template) -> Opening {
        Opening {
            stage: Stage::parse(template.text()).unwrap(),
            definition_sha256: "ab".repeat(32),
            tenant_id: "tenant".into(),
            election_event_id: "event".into(),
            started_at: now(),
            release: Some("v1".into()),
        }
    }

    pub fn witnessed(check: &str, outcome: Outcome) -> CheckResult {
        CheckResult {
            check_id: check.into(),
            method: Method::Witnessed,
            outcome,
            witness: Some("Witness".into()),
            note: None,
            findings: vec![],
        }
    }

    fn sample(path: &Path) -> Ledger {
        let mut ledger = Ledger::open(path, opening(Template::Voting), "operator", now()).unwrap();
        for outcome in [Outcome::Fail, Outcome::Pass] {
            ledger
                .append(
                    path,
                    witnessed("voting.ballot-displayed", outcome),
                    "operator",
                    now(),
                )
                .unwrap();
        }
        ledger
    }

    #[test]
    fn a_ledger_reads_back_as_written() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("ledger.jsonl");
        let written = sample(&path);
        let read = Ledger::read(&path).unwrap();
        assert_eq!(read.entries, written.entries);
        assert_eq!(read.head(), written.head());
        assert_eq!(read.results().count(), 2);
        assert_eq!(read.entries[0].content.recorded_at, "2028-01-10T09:00:00Z");
    }

    #[test]
    fn a_ledger_is_never_reopened_over_an_existing_one() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("ledger.jsonl");
        sample(&path);
        assert!(Ledger::open(&path, opening(Template::Voting), "operator", now()).is_err());
    }

    #[test]
    fn altered_removed_and_reordered_entries_are_detected() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("ledger.jsonl");
        sample(&path);
        let text = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        let rewrite = |lines: &[&str]| fs::write(&path, lines.join("\n")).unwrap();
        let error = |path: &Path| format!("{:#}", Ledger::read(path).unwrap_err());

        fs::write(&path, text.replacen("\"fail\"", "\"pass\"", 1)).unwrap();
        assert!(error(&path).contains("entry 1 was altered"));

        rewrite(&[lines[0], lines[2]]);
        assert!(error(&path).contains("entry 1 is out of order"));

        rewrite(&[lines[0], lines[2], lines[1]]);
        assert!(error(&path).contains("entry 1 is out of order"));

        rewrite(&lines[1..]);
        assert!(error(&path).contains("entry 0 is out of order"));

        fs::write(&path, text.replacen("\"v1\"", "\"v2\"", 1)).unwrap();
        assert!(error(&path).contains("entry 0 was altered"));
    }

    #[test]
    fn a_resealed_entry_breaks_the_entries_after_it() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("ledger.jsonl");
        let ledger = sample(&path);
        let mut forged = ledger.entries[1].content.clone();
        forged.body = Body::Checked(witnessed("voting.ballot-displayed", Outcome::Pass));
        let forged = serde_json::to_string(&forged.seal().unwrap()).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        fs::write(&path, [lines[0], &forged, lines[2]].join("\n")).unwrap();
        let error = format!("{:#}", Ledger::read(&path).unwrap_err());
        assert!(error.contains("entry 2 does not follow"), "{error}");
    }

    #[test]
    fn only_checks_of_the_stage_are_recorded_with_their_method() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("ledger.jsonl");
        let mut ledger = sample(&path);
        let before = fs::read_to_string(&path).unwrap();
        assert!(ledger
            .append(
                &path,
                witnessed("unknown", Outcome::Pass),
                "operator",
                now()
            )
            .is_err());
        assert!(ledger
            .append(
                &path,
                witnessed("voting.ballot-cast", Outcome::Pass),
                "operator",
                now()
            )
            .is_err());
        assert!(ledger
            .append(
                &path,
                witnessed("voting.ballot-displayed", Outcome::Pass),
                " ",
                now()
            )
            .is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), before);
    }
}
