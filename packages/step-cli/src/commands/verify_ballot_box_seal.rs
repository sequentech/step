// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! `step-cli step verify-ballot-box-seal`: checks a public ballot box seal
//! record offline (VOTE-FREEZE), optionally looks up a Ballot ID in it and
//! compares it with an `export-cast-votes` CSV.
//!
//! Ballot ID forms: the stored `cast_vote.ballot_id` is `hash_ballot`, the hex
//! of the first 32 bytes of the ballot's SHA-512. The `CastVote` log entry
//! holds that full SHA-512, and `export-cast-votes` writes the hex of its
//! first 32 bytes (`shorten_hash`), so the CSV `ballot_id` is the stored ID.
//! The voting portal and the PDF receipt show the stored ID too. All three
//! are the same 64 hex digits; matching ignores case and surrounding spaces.

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use clap::Args;
use colored::Colorize;
use electoral_log::messages::statement::ballot_box_sealed_description;
use electoral_log::seal::{
    format_time, verify_record, BallotBoxSealManifest, ExpectedKey, SealCheck, SealCheckError,
    SealCheckStep, SealDisposition, SealEntry, SealRecord,
};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs::File;
use std::path::Path;

/// Width of the check name column.
const LABEL_WIDTH: usize = 18;
/// Characters of the close request id shown.
const REQUEST_ID_SHOWN: usize = 8;
/// Ballot IDs listed per CastVote difference.
const IDS_LISTED: usize = 5;
/// Header label column width.
const HEADER_WIDTH: usize = 14;

#[derive(Args)]
#[command(about = "Verify a ballot box seal record offline")]
pub struct VerifyBallotBoxSeal {
    /// The seal record (JSON).
    record: String,

    /// A Ballot ID to look up in the ballot box (the full ID, as on the
    /// receipt and in the export-cast-votes CSV).
    #[arg(long)]
    ballot_id: Option<String>,

    /// An export-cast-votes CSV to compare the ballot box with.
    #[arg(long)]
    cast_votes: Option<String>,

    /// The key you trust: its base64 public key or its 128-bit fingerprint
    /// (32 hex digits).
    #[arg(long)]
    expected_key: Option<String>,

    /// The election this seal must be for; another one fails.
    #[arg(long)]
    election_id: Option<String>,

    /// The area this seal must be for; another one fails.
    #[arg(long)]
    area_id: Option<String>,
}

/// A line's mark.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mark {
    Pass,
    Fail,
    Warn,
    NotChecked,
}

fn line(mark: Mark, label: &str, detail: &str) {
    let mark = match mark {
        Mark::Pass => "✓".green(),
        Mark::Fail => "✗".red(),
        Mark::Warn => "!".yellow(),
        Mark::NotChecked => "-".normal(),
    };
    println!("{mark} {label:<LABEL_WIDTH$} {detail}");
}

fn header(label: &str, detail: &str) {
    println!("{label:<HEADER_WIDTH$}{detail}");
}

/// The CSV columns `export-cast-votes` writes that the comparison needs.
#[derive(Deserialize)]
struct CastVoteRow {
    election_id: String,
    area_id: String,
    ballot_id: String,
}

impl VerifyBallotBoxSeal {
    /// Prints the report. Any failed check is an error, so the exit status
    /// is nonzero.
    pub fn run(&self) -> Result<()> {
        let text = std::fs::read_to_string(&self.record)
            .with_context(|| format!("cannot read {}", self.record))?;
        let record: SealRecord = serde_json::from_str(&text)
            .with_context(|| format!("{} is not a ballot box seal record", self.record))?;
        let expected_key = self
            .expected_key
            .as_deref()
            .map(ExpectedKey::parse)
            .transpose()
            .context("--expected-key")?;
        let key_was_expected = expected_key.is_some();

        let check = match verify_record(&record, expected_key.as_ref()) {
            Ok(check) => check,
            Err(error) => {
                header(
                    "Record",
                    &format!(
                        "{}: election {}, area {} (not verified)",
                        self.record, record.election.id, record.area.id
                    ),
                );
                println!();
                print_failure(&error);
                return self.not_valid(error.to_string());
            }
        };

        let names_signed = names_are_signed(&record, &check);
        print_header(&record, &check, names_signed);
        println!();
        print_checks(&record, &check, key_was_expected, names_signed);
        let mut box_ok = names_signed;
        box_ok &= self.print_expected_box(&check.manifest);
        if !box_ok {
            return self.not_valid(
                "the record's names or the ballot box you gave differ from the signed seal".into(),
            );
        }

        let mut passed = true;
        if let Some(ballot_id) = &self.ballot_id {
            passed &= print_ballot_id(&check.manifest.entries, ballot_id);
        }
        if let Some(path) = &self.cast_votes {
            passed &= match print_cast_votes(&check, path) {
                Ok(matches) => matches,
                Err(error) => {
                    line(Mark::Fail, "CastVote entries", &format!("{error:#}"));
                    false
                }
            };
        }

        println!();
        let verdict = if key_was_expected {
            "Seal valid".to_string()
        } else {
            format!(
                "Seal valid for key {} (not checked against a key you trust: pass --expected-key)",
                check.system_key_fingerprint
            )
        };
        if passed {
            println!("{}", verdict.green().bold());
            Ok(())
        } else {
            println!(
                "{}",
                format!("{verdict}, but a requested check failed (marked ✗ above)")
                    .red()
                    .bold()
            );
            Err(anyhow!("a requested check failed"))
        }
    }

    /// Marks the requested lookups as not run and prints the failed verdict.
    fn not_valid(&self, reason: String) -> Result<()> {
        if self.ballot_id.is_some() {
            line(
                Mark::NotChecked,
                "Ballot ID",
                "not checked: the seal is not valid",
            );
        }
        if self.cast_votes.is_some() {
            line(
                Mark::NotChecked,
                "CastVote entries",
                "not checked: the seal is not valid",
            );
        }
        println!();
        println!("{}", "Seal NOT valid".red().bold());
        Err(anyhow!("the seal is not valid: {reason}"))
    }

    /// With `--election-id` or `--area-id`, the signed ids must be those.
    /// Returns whether they are (true when none was given).
    fn print_expected_box(&self, manifest: &BallotBoxSealManifest) -> bool {
        if self.election_id.is_none() && self.area_id.is_none() {
            return true;
        }
        let mut differs = Vec::new();
        if let Some(election) = &self.election_id {
            if election.trim() != manifest.election_id {
                differs.push(format!("election {}", election.trim()));
            }
        }
        if let Some(area) = &self.area_id {
            if area.trim() != manifest.area_id {
                differs.push(format!("area {}", area.trim()));
            }
        }
        let signed = format!(
            "election {}, area {}",
            manifest.election_id, manifest.area_id
        );
        if differs.is_empty() {
            line(
                Mark::Pass,
                "Expected box",
                &format!("{signed}: the one you gave"),
            );
            true
        } else {
            line(
                Mark::Fail,
                "Expected box",
                &format!(
                    "the seal is for {signed}, not the expected {}",
                    differs.join(", ")
                ),
            );
            false
        }
    }
}

/// The record's election and area names are not signed on their own, but
/// the signed statement's description holds them: they are verified when
/// the record's names give back that exact description.
fn names_are_signed(record: &SealRecord, check: &SealCheck) -> bool {
    ballot_box_sealed_description(
        &record.election.name,
        &record.area.name,
        check.manifest.ballots_counted(),
        check.manifest.ballots_in_box(),
    ) == check.message.statement.head.description
}

/// The verified box: ids, times and counts from the manifest.
fn print_header(record: &SealRecord, check: &SealCheck, names_signed: bool) {
    let manifest = &check.manifest;
    header(
        "Ballot box",
        &format!(
            "{}, {} ({})",
            record.election.name,
            record.area.name,
            if names_signed {
                "names as in the signed statement"
            } else {
                "names NOT those of the signed statement"
            }
        ),
    );
    header(
        "Event",
        &format!(
            "{} (name not signed), ID {}",
            record.election_event.name, manifest.election_event_id
        ),
    );
    header(
        "IDs",
        &format!(
            "election {}, area {}",
            manifest.election_id, manifest.area_id
        ),
    );
    header(
        "Closed at",
        &format!(
            "{}, grace period until {}",
            readable_time(manifest.closed_at),
            readable_time(manifest.grace_deadline)
        ),
    );
    header("Sealed at", &readable_time(manifest.sealed_at));
    header(
        "Ballots",
        &format!(
            "{} in the box, {} counted, {} eligible voters",
            thousands(manifest.ballots_in_box()),
            thousands(manifest.ballots_counted()),
            thousands(manifest.eligible_voters)
        ),
    );
}

/// Unix seconds as `2028-05-08 11:01:14 UTC`.
fn readable_time(unix_seconds: u64) -> String {
    i64::try_from(unix_seconds)
        .ok()
        .and_then(|seconds| DateTime::<Utc>::from_timestamp(seconds, 0))
        .map(|time| time.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| unix_seconds.to_string())
}

fn step_label(step: SealCheckStep) -> &'static str {
    match step {
        SealCheckStep::Format => "Format",
        SealCheckStep::Entries => "Entries",
        SealCheckStep::Manifest => "Manifest",
        SealCheckStep::Message => "Message",
        SealCheckStep::SealHash => "Seal hash",
        SealCheckStep::Statement => "Statement",
        SealCheckStep::Time => "Time",
        SealCheckStep::CloseRequest => "Close request",
        SealCheckStep::SystemSignature => "System signature",
        SealCheckStep::SenderSignature => "Sender signature",
        SealCheckStep::Key => "Expected key",
    }
}

/// The verification stops at the first failure, and its checks do not run
/// in a fixed order, so only the failed one is reported.
fn print_failure(error: &SealCheckError) {
    line(Mark::Fail, step_label(error.step), &error.reason);
    line(
        Mark::NotChecked,
        "Other checks",
        "not reported: the verification stops at the first failure",
    );
}

/// Prints what a valid record proves. Without an expected key, nothing ties
/// the record's key to the election event.
fn print_checks(
    record: &SealRecord,
    check: &SealCheck,
    key_was_expected: bool,
    names_signed: bool,
) {
    let manifest = &check.manifest;
    let fingerprint = &check.system_key_fingerprint;
    let indent = " ".repeat(LABEL_WIDTH + 3);

    line(
        Mark::Pass,
        "Entries",
        &format!(
            "{} entries, sorted by ballot hash; weights and counts follow the seal rules",
            thousands(manifest.ballots_in_box())
        ),
    );
    line(
        Mark::Pass,
        "Manifest",
        "rebuilt from the header and entries: the same bytes as the record's",
    );
    line(
        Mark::Pass,
        "Seal hash",
        "the SHA-512 of the manifest is the hash in the signed statement:",
    );
    println!("{indent}{}", hex::encode(check.seal_hash));
    line(
        Mark::Pass,
        "Statement",
        "names this election, area, counts and close request",
    );
    if names_signed {
        line(
            Mark::Pass,
            "Names",
            &format!(
                "{}, {}: the names in the signed statement",
                record.election.name, record.area.name
            ),
        );
    } else {
        line(
            Mark::Fail,
            "Names",
            &format!(
                "the record says {}, {}; the signed statement says \"{}\"",
                record.election.name, record.area.name, check.message.statement.head.description
            ),
        );
    }
    line(
        Mark::Pass,
        "System signature",
        &if key_was_expected {
            format!("Ed25519, election event key {fingerprint}")
        } else {
            format!("Ed25519, signed by the record's key {fingerprint}")
        },
    );
    if key_was_expected {
        line(
            Mark::Pass,
            "Expected key",
            &format!("{fingerprint} is the key you gave"),
        );
    } else {
        line(
            Mark::Warn,
            "Expected key",
            &format!(
                "not checked (no --expected-key): compare {fingerprint} with the key you trust"
            ),
        );
    }
    line(
        Mark::Pass,
        "Sender signature",
        "a system entry: the sender is the same key",
    );
    let signed_time = format_time(check.message.statement.head.timestamp)
        .unwrap_or_else(|_| check.message.statement.head.timestamp.to_string());
    line(
        Mark::Pass,
        "Time",
        &format!(
            "the signed statement says {signed_time}; closed, grace deadline and seal in order"
        ),
    );
    let close_request = match &record.close_request {
        Some(request) => {
            let shown: String = request.id.chars().take(REQUEST_ID_SHOWN).collect();
            let code = request
                .signing_code
                .as_deref()
                .map(|code| format!(", signing code {code}"))
                .unwrap_or_default();
            format!(
                "{shown}…{code}, {} certificate signatures (signing code and signers not signed)",
                request.signers.len()
            )
        }
        None => "none: closed without a signed Close voting request".to_string(),
    };
    line(Mark::Pass, "Close request", &close_request);
    line(
        Mark::NotChecked,
        "Log entry",
        &format!(
            "#{} (not signed): check that the bulletin board's BallotBoxSealed entry for this box has this seal hash",
            record.log_entry.id
        ),
    );
}

fn normalize_id(id: &str) -> String {
    id.trim().to_ascii_lowercase()
}

fn disposition_text(entry: &SealEntry) -> String {
    match entry.disposition {
        SealDisposition::Counted if entry.weight != 1 => {
            format!("counted (weight {})", entry.weight)
        }
        SealDisposition::Counted => "counted".to_string(),
        SealDisposition::Replaced => "replaced by a later ballot, not counted".to_string(),
        SealDisposition::NotEligible => "not eligible, not counted".to_string(),
        SealDisposition::Discarded => "discarded, not counted".to_string(),
    }
}

/// Returns whether the Ballot ID is in the box.
fn print_ballot_id(entries: &[SealEntry], ballot_id: &str) -> bool {
    let wanted = normalize_id(ballot_id);
    let found: Vec<String> = entries
        .iter()
        .filter(|entry| normalize_id(&entry.ballot_id) == wanted)
        .map(disposition_text)
        .collect();
    let label = format!("Ballot ID {}", ballot_id.trim());
    match found.len() {
        0 => {
            println!("{} {label} not in the ballot box", "✗".red());
            false
        }
        1 => {
            println!("{} {label} in the ballot box: {}", "✓".green(), found[0]);
            true
        }
        times => {
            println!(
                "{} {label} in the ballot box {times} times: {}",
                "✓".green(),
                found.join(", ")
            );
            true
        }
    }
}

/// Ballot IDs (normalized) and how many times each appears.
fn multiset<'a>(ids: impl Iterator<Item = &'a str>) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    for id in ids {
        *counts.entry(normalize_id(id)).or_default() += 1;
    }
    counts
}

/// The ids `left` has more times than `right`, repeated by the difference.
fn surplus(left: &BTreeMap<String, u64>, right: &BTreeMap<String, u64>) -> Vec<String> {
    let mut ids = Vec::new();
    for (id, count) in left {
        let extra = count.saturating_sub(right.get(id).copied().unwrap_or(0));
        for _ in 0..extra {
            ids.push(id.clone());
        }
    }
    ids
}

fn list_ids(ids: &[String]) -> String {
    let shown = ids.iter().take(IDS_LISTED).cloned().collect::<Vec<_>>();
    let more = ids.len().saturating_sub(IDS_LISTED);
    let mut text = shown.join(", ");
    if more > 0 {
        text.push_str(&format!(" and {more} more"));
    }
    text
}

/// Compares every sealed ballot, in any disposition (each went through the
/// cast path), with the CSV's CastVote entries of this election and area.
/// Returns whether both sides match.
fn print_cast_votes(check: &SealCheck, path: &str) -> Result<bool> {
    let manifest = &check.manifest;
    let file = File::open(path).with_context(|| format!("cannot read {path}"))?;
    let mut reader = csv::Reader::from_reader(file);
    let mut box_rows = Vec::new();
    for row in reader.deserialize::<CastVoteRow>() {
        let row = row.with_context(|| format!("{path} is not an export-cast-votes CSV"))?;
        if row.election_id == manifest.election_id && row.area_id == manifest.area_id {
            box_rows.push(row.ballot_id);
        }
    }
    let sealed = multiset(
        manifest
            .entries
            .iter()
            .map(|entry| entry.ballot_id.as_str()),
    );
    let logged = multiset(box_rows.iter().map(String::as_str));
    let without_entry = surplus(&sealed, &logged);
    let without_ballot = surplus(&logged, &sealed);
    let name = Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string());
    let matches = without_entry.is_empty() && without_ballot.is_empty();
    line(
        if matches { Mark::Pass } else { Mark::Fail },
        "CastVote entries",
        &format!(
            "{} in {name}; sealed ballots without an entry: {}; entries without a sealed ballot: {}",
            thousands(box_rows.len() as u64),
            thousands(without_entry.len() as u64),
            thousands(without_ballot.len() as u64)
        ),
    );
    let indent = " ".repeat(LABEL_WIDTH + 3);
    if !without_entry.is_empty() {
        println!(
            "{indent}without an entry: Ballot IDs {}",
            list_ids(&without_entry)
        );
    }
    if !without_ballot.is_empty() {
        println!(
            "{indent}without a sealed ballot: Ballot IDs {}",
            list_ids(&without_ballot)
        );
    }
    Ok(matches)
}

/// `1342` as `1,342`.
fn thousands(value: u64) -> String {
    let digits = value.to_string();
    let mut text = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            text.push(',');
        }
        text.push(digit);
    }
    text
}
